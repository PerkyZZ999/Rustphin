//! Storage volumes, through the UDisks2 service on the system bus: listing,
//! mounting, unmounting and powering off, and noticing when they change.
//!
//! Calls block until the service answers; callers run them off the UI thread.

use std::{
    collections::HashMap,
    ffi::OsString,
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
    thread,
};

use zbus::{
    blocking::{Connection, MessageIterator, fdo::ObjectManagerProxy},
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value},
};

const SERVICE: &str = "org.freedesktop.UDisks2";
const ROOT: &str = "/org/freedesktop/UDisks2";
const BLOCK: &str = "org.freedesktop.UDisks2.Block";
const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
const LOOP: &str = "org.freedesktop.UDisks2.Loop";
const DRIVE: &str = "org.freedesktop.UDisks2.Drive";

/// A volume that holds a filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// The service's name for it; stable while it is plugged in.
    pub id: String,
    /// Its label, when it has one.
    pub label: Option<String>,
    /// Where it is mounted, when it is.
    pub mount_point: Option<PathBuf>,
    /// Its size in bytes.
    pub size: u64,
    /// Whether it sits on a drive that can be unplugged or whose medium can
    /// be taken out.
    pub removable: bool,
    /// The drive it sits on, when that drive can be powered off for safe removal.
    pub power_off: Option<String>,
}

impl Device {
    /// Whether the drive can be powered off once this volume is unmounted.
    pub fn can_power_off(&self) -> bool {
        self.power_off.is_some()
    }
}

/// Why a device request failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeviceError {
    /// Something still uses the volume: an open file, a terminal inside it.
    #[error("it is still in use ({0})")]
    Busy(String),
    /// The user may not do this, or dismissed the password prompt.
    #[error("permission was not given ({0})")]
    NotAuthorized(String),
    /// The service refused for another reason, or cannot be reached.
    #[error("{0}")]
    Other(String),
}

impl From<zbus::Error> for DeviceError {
    fn from(error: zbus::Error) -> Self {
        match &error {
            zbus::Error::MethodError(name, message, _) => {
                let message = message.clone().unwrap_or_else(|| name.to_string());
                match name.as_str().rsplit('.').next() {
                    Some("DeviceBusy") => Self::Busy(message),
                    Some("NotAuthorized" | "NotAuthorizedCanObtain" | "NotAuthorizedDismissed") => {
                        Self::NotAuthorized(message)
                    }
                    _ => Self::Other(message),
                }
            }
            _ => Self::Other(error.to_string()),
        }
    }
}

/// Stops a device watch when dropped.
pub type WatchGuard = Box<dyn Send>;

/// The storage devices of this machine.
pub trait Devices: Send + Sync {
    /// The volumes that hold a filesystem, fixed ones first, then by label.
    fn list(&self) -> Result<Vec<Device>, DeviceError>;

    /// Mounts the volume and returns where.
    fn mount(&self, device: &Device) -> Result<PathBuf, DeviceError>;

    /// Unmounts the volume. Fails with [`DeviceError::Busy`] while it is in use.
    fn unmount(&self, device: &Device) -> Result<(), DeviceError>;

    /// Powers off the drive of an unmounted volume so it can be unplugged.
    fn power_off(&self, device: &Device) -> Result<(), DeviceError>;

    /// Calls `on_change` from another thread whenever a device appears,
    /// disappears or changes, for as long as the connection lives.
    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Result<(), DeviceError>;
}

/// A system without a device service: no volumes.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoDevices;

impl Devices for NoDevices {
    fn list(&self) -> Result<Vec<Device>, DeviceError> {
        Ok(Vec::new())
    }

    fn mount(&self, _: &Device) -> Result<PathBuf, DeviceError> {
        Err(DeviceError::Other(
            "no device service is available".to_owned(),
        ))
    }

    fn unmount(&self, _: &Device) -> Result<(), DeviceError> {
        Err(DeviceError::Other(
            "no device service is available".to_owned(),
        ))
    }

    fn power_off(&self, _: &Device) -> Result<(), DeviceError> {
        Err(DeviceError::Other(
            "no device service is available".to_owned(),
        ))
    }

    fn watch(&self, _: Box<dyn Fn() + Send>) -> Result<(), DeviceError> {
        Ok(())
    }
}

/// The UDisks2 service.
pub struct UDisks2 {
    connection: Connection,
}

type Properties = HashMap<String, OwnedValue>;

fn text(properties: &Properties, name: &str) -> Option<String> {
    let value = properties.get(name)?;
    <&str>::try_from(value)
        .ok()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn flag(properties: &Properties, name: &str) -> bool {
    properties
        .get(name)
        .and_then(|value| bool::try_from(value).ok())
        .unwrap_or(false)
}

impl UDisks2 {
    /// Connects to the service on the system bus.
    pub fn connect() -> Result<Self, DeviceError> {
        Ok(Self::on(Connection::system()?))
    }

    /// Uses the service reachable over `connection`.
    pub fn on(connection: Connection) -> Self {
        Self { connection }
    }

    /// Calls a method that takes only the usual empty options.
    fn call(
        &self,
        path: &str,
        interface: &str,
        method: &str,
    ) -> Result<zbus::Message, DeviceError> {
        let no_options: HashMap<&str, Value<'_>> = HashMap::new();
        Ok(self.connection.call_method(
            Some(SERVICE),
            path,
            Some(interface),
            method,
            &(no_options,),
        )?)
    }
}

impl Devices for UDisks2 {
    fn list(&self) -> Result<Vec<Device>, DeviceError> {
        let objects = ObjectManagerProxy::builder(&self.connection)
            .destination(SERVICE)?
            .path(ROOT)?
            .build()?
            .get_managed_objects()
            .map_err(zbus::Error::from)?;
        let interface = |path: &str, name: &str| -> Option<&Properties> {
            let path = ObjectPath::try_from(path).ok()?;
            objects
                .get(&OwnedObjectPath::from(path))?
                .iter()
                .find(|(interface, _)| interface.as_str() == name)
                .map(|(_, properties)| properties)
        };

        let mut devices = Vec::new();
        for (path, interfaces) in &objects {
            let has = |name: &str| {
                interfaces
                    .keys()
                    .any(|interface| interface.as_str() == name)
            };
            // Disk images set up as loop devices are not places to browse.
            if !has(FILESYSTEM) || has(LOOP) {
                continue;
            }
            let (Some(block), Some(filesystem)) = (
                interface(path.as_str(), BLOCK),
                interface(path.as_str(), FILESYSTEM),
            ) else {
                continue;
            };
            if flag(block, "HintIgnore") {
                continue;
            }
            let drive_path = block
                .get("Drive")
                .and_then(|value| match &**value {
                    Value::ObjectPath(path) => Some(path.to_string()),
                    _ => None,
                })
                .filter(|path| path != "/");
            let drive = drive_path
                .as_deref()
                .and_then(|path| interface(path, DRIVE));
            // Mount points are byte strings ending in a NUL.
            let mount_point = filesystem
                .get("MountPoints")
                .and_then(|value| Vec::<Vec<u8>>::try_from(value.try_clone().ok()?).ok())
                .and_then(|points| points.into_iter().next())
                .map(|mut bytes| {
                    while bytes.last() == Some(&0) {
                        bytes.pop();
                    }
                    PathBuf::from(OsString::from_vec(bytes))
                })
                .filter(|path| path.is_absolute());
            devices.push(Device {
                id: path.to_string(),
                label: text(block, "IdLabel").or_else(|| text(block, "HintName")),
                mount_point,
                size: block
                    .get("Size")
                    .and_then(|value| u64::try_from(value).ok())
                    .unwrap_or(0),
                removable: drive
                    .is_some_and(|drive| flag(drive, "Removable") || flag(drive, "MediaRemovable")),
                power_off: drive_path
                    .filter(|_| drive.is_some_and(|drive| flag(drive, "CanPowerOff"))),
            });
        }
        devices.sort_by(|a, b| (a.removable, &a.label, &a.id).cmp(&(b.removable, &b.label, &b.id)));
        Ok(devices)
    }

    fn mount(&self, device: &Device) -> Result<PathBuf, DeviceError> {
        let reply = self.call(&device.id, FILESYSTEM, "Mount")?;
        let path: String = reply.body().deserialize()?;
        Ok(PathBuf::from(path))
    }

    fn unmount(&self, device: &Device) -> Result<(), DeviceError> {
        self.call(&device.id, FILESYSTEM, "Unmount").map(drop)
    }

    fn power_off(&self, device: &Device) -> Result<(), DeviceError> {
        let Some(drive) = &device.power_off else {
            return Err(DeviceError::Other(
                "this drive cannot be powered off".to_owned(),
            ));
        };
        self.call(drive, DRIVE, "PowerOff").map(drop)
    }

    fn watch(&self, on_change: Box<dyn Fn() + Send>) -> Result<(), DeviceError> {
        // Objects appearing and disappearing, and their properties changing
        // (a volume being mounted elsewhere), all arrive as signals below the
        // service's root.
        let rule = format!("type='signal',path_namespace='{ROOT}'");
        let signals = MessageIterator::for_match_rule(rule.as_str(), &self.connection, None)?;
        thread::Builder::new()
            .name("rustphin-devices".to_owned())
            .spawn(move || {
                for signal in signals {
                    if signal.is_err() {
                        break;
                    }
                    on_change();
                }
            })
            .map_err(|error| DeviceError::Other(error.to_string()))?;
        Ok(())
    }
}

/// How full the filesystem mounted at `mount_point` is: its size and the
/// space still free for the user, in bytes.
pub fn usage(mount_point: &Path) -> Option<(u64, u64)> {
    let stat = rustix::fs::statvfs(mount_point).ok()?;
    let total = stat.f_blocks.checked_mul(stat.f_frsize)?;
    let free = stat.f_bavail.checked_mul(stat.f_frsize)?;
    (total > 0).then_some((total, free.min(total)))
}

/// Where the filesystem holding `path` is mounted, from the kernel's
/// mount table. Blocking: call from a background thread.
pub fn mount_point_of(path: &Path) -> Option<PathBuf> {
    // Mount points are bytes: one outside UTF-8 must not hide the others.
    let table = std::fs::read("/proc/self/mountinfo").ok()?;
    mount_point_in(&table, path)
}

/// The deepest mount point in a `mountinfo` table that holds `path`.
fn mount_point_in(table: &[u8], path: &Path) -> Option<PathBuf> {
    crate::trash::mount_points(table)
        .into_iter()
        .filter(|mount| path.starts_with(mount))
        .max_by_key(|mount| mount.components().count())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{
        ffi::OsString,
        os::unix::{ffi::OsStringExt, net::UnixStream},
        sync::{Arc, Mutex, mpsc},
        time::Duration,
    };

    use zbus::{blocking::connection::Builder, fdo::ObjectManager, interface};

    use super::*;

    #[derive(Debug, zbus::DBusError)]
    #[zbus(prefix = "org.freedesktop.UDisks2.Error")]
    enum MockError {
        #[zbus(error)]
        ZBus(zbus::Error),
        DeviceBusy(String),
        NotAuthorizedDismissed(String),
    }

    struct MockBlock {
        label: &'static str,
        hint_name: &'static str,
        size: u64,
        drive: &'static str,
        ignore: bool,
    }

    #[interface(name = "org.freedesktop.UDisks2.Block")]
    impl MockBlock {
        #[zbus(property)]
        fn id_label(&self) -> String {
            self.label.to_owned()
        }
        #[zbus(property)]
        fn hint_name(&self) -> String {
            self.hint_name.to_owned()
        }
        #[zbus(property)]
        fn size(&self) -> u64 {
            self.size
        }
        #[zbus(property)]
        fn drive(&self) -> OwnedObjectPath {
            ObjectPath::try_from(self.drive).unwrap().into()
        }
        #[zbus(property)]
        fn hint_ignore(&self) -> bool {
            self.ignore
        }
    }

    /// Shared with the test, to see and steer what the mock does.
    #[derive(Default)]
    struct Volume {
        mounted_at: Option<String>,
        busy: bool,
        locked: bool,
    }

    struct MockFilesystem(Arc<Mutex<Volume>>);

    #[interface(name = "org.freedesktop.UDisks2.Filesystem")]
    impl MockFilesystem {
        #[zbus(property)]
        fn mount_points(&self) -> Vec<Vec<u8>> {
            self.0
                .lock()
                .unwrap()
                .mounted_at
                .iter()
                .map(|path| [path.as_bytes(), b"\0"].concat())
                .collect()
        }

        fn mount(&self, _options: HashMap<String, OwnedValue>) -> Result<String, MockError> {
            let mut volume = self.0.lock().unwrap();
            if volume.locked {
                return Err(MockError::NotAuthorizedDismissed("dismissed".into()));
            }
            volume.mounted_at = Some("/run/media/u/STICK".into());
            Ok("/run/media/u/STICK".into())
        }

        fn unmount(&self, _options: HashMap<String, OwnedValue>) -> Result<(), MockError> {
            let mut volume = self.0.lock().unwrap();
            if volume.busy {
                return Err(MockError::DeviceBusy("target is busy".into()));
            }
            volume.mounted_at = None;
            Ok(())
        }
    }

    struct MockLoop;

    #[interface(name = "org.freedesktop.UDisks2.Loop")]
    impl MockLoop {
        #[zbus(property)]
        fn backing_file(&self) -> Vec<u8> {
            b"/image.iso\0".to_vec()
        }
    }

    struct MockDrive {
        removable: bool,
        can_power_off: bool,
        powered_off: Arc<Mutex<bool>>,
    }

    #[interface(name = "org.freedesktop.UDisks2.Drive")]
    impl MockDrive {
        #[zbus(property)]
        fn removable(&self) -> bool {
            self.removable
        }
        #[zbus(property)]
        fn media_removable(&self) -> bool {
            false
        }
        #[zbus(property)]
        fn can_power_off(&self) -> bool {
            self.can_power_off
        }
        fn power_off(&self, _options: HashMap<String, OwnedValue>) {
            *self.powered_off.lock().unwrap() = true;
        }
    }

    /// A pretend UDisks2 with an internal disk, a USB stick, a volume to
    /// ignore and a disk image, reached over a private connection.
    struct Mock {
        service: Connection,
        client: UDisks2,
        stick: Arc<Mutex<Volume>>,
        powered_off: Arc<Mutex<bool>>,
    }

    fn fs(volume: &Arc<Mutex<Volume>>) -> MockFilesystem {
        MockFilesystem(Arc::clone(volume))
    }

    fn mock() -> Mock {
        let (ours, theirs) = UnixStream::pair().unwrap();
        let stick = Arc::new(Mutex::new(Volume::default()));
        let internal = Arc::new(Mutex::new(Volume {
            mounted_at: Some("/mnt/data".into()),
            ..Volume::default()
        }));
        let powered_off = Arc::new(Mutex::new(false));
        let block = |label, hint_name, drive, ignore| MockBlock {
            label,
            hint_name,
            size: 64_000_000_000,
            drive,
            ignore,
        };
        let guid = zbus::Guid::generate();
        let builder = Builder::async_io_unix_stream(theirs)
            .server(guid)
            .unwrap()
            .p2p()
            .serve_at(ROOT, ObjectManager)
            .unwrap()
            .serve_at(
                format!("{ROOT}/block_devices/sdb1"),
                block("STICK", "", "/org/freedesktop/UDisks2/drives/usb", false),
            )
            .unwrap()
            .serve_at(format!("{ROOT}/block_devices/sdb1"), fs(&stick))
            .unwrap()
            .serve_at(
                format!("{ROOT}/block_devices/sda2"),
                block("", "Data", "/org/freedesktop/UDisks2/drives/ssd", false),
            )
            .unwrap()
            .serve_at(format!("{ROOT}/block_devices/sda2"), fs(&internal))
            .unwrap()
            .serve_at(
                format!("{ROOT}/block_devices/sda1"),
                block("EFI", "", "/org/freedesktop/UDisks2/drives/ssd", true),
            )
            .unwrap()
            .serve_at(format!("{ROOT}/block_devices/sda1"), fs(&Arc::default()))
            .unwrap()
            .serve_at(
                format!("{ROOT}/block_devices/loop0"),
                block("image", "", "/", false),
            )
            .unwrap()
            .serve_at(format!("{ROOT}/block_devices/loop0"), fs(&Arc::default()))
            .unwrap()
            .serve_at(format!("{ROOT}/block_devices/loop0"), MockLoop)
            .unwrap()
            .serve_at(
                format!("{ROOT}/drives/usb"),
                MockDrive {
                    removable: true,
                    can_power_off: true,
                    powered_off: Arc::clone(&powered_off),
                },
            )
            .unwrap()
            .serve_at(
                format!("{ROOT}/drives/ssd"),
                MockDrive {
                    removable: false,
                    can_power_off: false,
                    powered_off: Arc::default(),
                },
            )
            .unwrap();
        // Both ends shake hands at the same time.
        let service = thread::spawn(move || builder.build().unwrap());
        let client = Builder::async_io_unix_stream(ours).p2p().build().unwrap();
        Mock {
            service: service.join().unwrap(),
            client: UDisks2::on(client),
            stick,
            powered_off,
        }
    }

    fn stick(mock: &Mock) -> Device {
        mock.client
            .list()
            .unwrap()
            .into_iter()
            .find(|device| device.label.as_deref() == Some("STICK"))
            .unwrap()
    }

    #[test]
    fn a_path_belongs_to_the_deepest_mount_above_it_with_escaped_names_decoded() {
        let table = b"\
22 1 0:21 / / rw - btrfs /dev/nvme0n1p2 rw
40 22 0:35 / /tmp rw - tmpfs tmpfs rw
41 22 8:1 / /mnt/my\\040drive rw - ext4 /dev/sda1 rw
42 22 8:2 / /mnt/data rw - ext4 /dev/sdb1 rw
";
        let mount = |path: &str| mount_point_in(table, Path::new(path));
        assert_eq!(mount("/home/u"), Some(PathBuf::from("/")));
        assert_eq!(mount("/tmp/x"), Some(PathBuf::from("/tmp")));
        assert_eq!(
            mount("/mnt/my drive/a"),
            Some(PathBuf::from("/mnt/my drive"))
        );
        assert_eq!(mount("/mnt/database"), Some(PathBuf::from("/")));
        assert_eq!(mount_point_in(b"", Path::new("/x")), None);
    }

    #[test]
    fn a_mount_point_outside_utf8_is_found_and_does_not_hide_the_others() {
        // The kernel escapes only spaces, tabs, newlines and backslashes;
        // other bytes of a mount point appear as they are.
        let table = b"\
22 1 0:21 / / rw - btrfs /dev/nvme0n1p2 rw
41 22 8:1 / /media/caf\xe9 rw - vfat /dev/sda1 rw
42 22 8:2 / /mnt/data rw - ext4 /dev/sdb1 rw
";
        let cafe = PathBuf::from(OsString::from_vec(b"/media/caf\xe9".to_vec()));

        assert_eq!(mount_point_in(table, &cafe.join("a")), Some(cafe));
        assert_eq!(
            mount_point_in(table, Path::new("/mnt/data/b")),
            Some(PathBuf::from("/mnt/data"))
        );
    }

    #[test]
    fn the_running_system_has_a_mount_for_its_root() {
        assert_eq!(mount_point_of(Path::new("/")), Some(PathBuf::from("/")));
    }

    #[test]
    fn volumes_are_listed_fixed_first_without_ignored_ones_and_disk_images() {
        let mock = mock();

        let devices = mock.client.list().unwrap();

        let labels: Vec<_> = devices
            .iter()
            .map(|device| device.label.as_deref())
            .collect();
        assert_eq!(labels, [Some("Data"), Some("STICK")]);
        let (data, stick) = (&devices[0], &devices[1]);
        assert_eq!(data.mount_point.as_deref(), Some(Path::new("/mnt/data")));
        assert!(!data.removable && !data.can_power_off());
        assert_eq!(stick.mount_point, None);
        assert_eq!(stick.size, 64_000_000_000);
        assert!(stick.removable && stick.can_power_off());
        assert_eq!(stick.id, "/org/freedesktop/UDisks2/block_devices/sdb1");
    }

    #[test]
    fn mounting_returns_the_mount_point_and_the_list_then_shows_it() {
        let mock = mock();

        let mounted = mock.client.mount(&stick(&mock)).unwrap();

        assert_eq!(mounted, Path::new("/run/media/u/STICK"));
        assert_eq!(
            stick(&mock).mount_point.as_deref(),
            Some(Path::new("/run/media/u/STICK"))
        );
    }

    #[test]
    fn unmounting_a_volume_in_use_fails_with_the_reason_and_leaves_it_mounted() {
        let mock = mock();
        mock.client.mount(&stick(&mock)).unwrap();
        mock.stick.lock().unwrap().busy = true;

        let error = mock.client.unmount(&stick(&mock)).unwrap_err();

        assert_eq!(error, DeviceError::Busy("target is busy".into()));
        assert_eq!(error.to_string(), "it is still in use (target is busy)");
        assert!(stick(&mock).mount_point.is_some());

        mock.stick.lock().unwrap().busy = false;
        mock.client.unmount(&stick(&mock)).unwrap();
        assert_eq!(stick(&mock).mount_point, None);
    }

    #[test]
    fn a_dismissed_password_prompt_is_reported_as_missing_permission() {
        let mock = mock();
        mock.stick.lock().unwrap().locked = true;

        let error = mock.client.mount(&stick(&mock)).unwrap_err();

        assert_eq!(error, DeviceError::NotAuthorized("dismissed".into()));
    }

    #[test]
    fn a_removable_drive_can_be_powered_off_and_a_fixed_one_cannot() {
        let mock = mock();
        let devices = mock.client.list().unwrap();

        mock.client.power_off(&devices[1]).unwrap();
        assert!(*mock.powered_off.lock().unwrap());
        assert!(matches!(
            mock.client.power_off(&devices[0]),
            Err(DeviceError::Other(_))
        ));
    }

    #[test]
    fn plugging_in_a_drive_is_noticed() {
        let mock = mock();
        let (noticed, notices) = mpsc::channel();
        mock.client
            .watch(Box::new(move || {
                let _ = noticed.send(());
            }))
            .unwrap();

        // A new volume appears on the service.
        mock.service
            .object_server()
            .at(
                format!("{ROOT}/block_devices/sdc1"),
                MockBlock {
                    label: "NEW",
                    hint_name: "",
                    size: 1,
                    drive: "/",
                    ignore: false,
                },
            )
            .unwrap();
        mock.service
            .object_server()
            .at(format!("{ROOT}/block_devices/sdc1"), fs(&Arc::default()))
            .unwrap();

        notices.recv_timeout(Duration::from_secs(20)).unwrap();
        let labels: Vec<_> = mock
            .client
            .list()
            .unwrap()
            .into_iter()
            .filter_map(|device| device.label)
            .collect();
        assert!(labels.contains(&"NEW".to_owned()), "{labels:?}");
    }

    #[test]
    fn usage_reports_size_and_free_space_of_a_mounted_filesystem() {
        let (total, free) = usage(Path::new("/")).unwrap();
        assert!(total > 0 && free <= total);
        assert_eq!(usage(Path::new("/nonexistent/mount/point")), None);
    }
}
