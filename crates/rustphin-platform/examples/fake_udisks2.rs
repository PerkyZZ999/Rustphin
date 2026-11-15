//! A stand-in for the UDisks2 device service, for interface tests only
//! (W-148, UI_SCENARIOS.md "Drive cards").
//!
//! It claims `org.freedesktop.UDisks2` on the bus `DBUS_SYSTEM_BUS_ADDRESS`
//! names, which must be a private bus started for the test: it refuses to
//! start without that variable, so it never stands beside the machine's
//! real service. Rustphin started with the same variable lists its five
//! volumes and sends it every Mount, Unmount and PowerOff, which it prints.
//! Nothing is mounted or powered off for real: a "mount point" is a folder
//! under the folder given, created at start.
//!
//! ```sh
//! dbus-daemon --session --nofork --print-address --address=unix:path=/tmp/home/system-bus &
//! DBUS_SYSTEM_BUS_ADDRESS=unix:path=/tmp/home/system-bus \
//!     cargo run -p rustphin-platform --example fake_udisks2 -- /tmp/home/drives
//! ```

use std::{
    collections::HashMap,
    io::Write as _,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use zbus::{
    blocking::connection::Builder,
    fdo::ObjectManager,
    interface,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue},
};

const ROOT: &str = "/org/freedesktop/UDisks2";

#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.UDisks2.Error")]
enum Refusal {
    #[zbus(error)]
    ZBus(zbus::Error),
    DeviceBusy(String),
    Failed(String),
}

/// Prints one request, at once, for the test to read.
fn log(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

fn object_path(path: &str) -> zbus::fdo::Result<OwnedObjectPath> {
    ObjectPath::try_from(path)
        .map(OwnedObjectPath::from)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}

struct Block {
    label: &'static str,
    size: u64,
    drive: &'static str,
    id_type: &'static str,
    crypto_backing: &'static str,
    /// The device file, for Properties (W-153).
    device: &'static str,
    uuid: &'static str,
    read_only: bool,
}

#[interface(name = "org.freedesktop.UDisks2.Block")]
impl Block {
    #[zbus(property)]
    fn id_label(&self) -> String {
        self.label.to_owned()
    }
    #[zbus(property)]
    fn hint_name(&self) -> String {
        String::new()
    }
    #[zbus(property)]
    fn size(&self) -> u64 {
        self.size
    }
    #[zbus(property)]
    fn drive(&self) -> zbus::fdo::Result<OwnedObjectPath> {
        object_path(self.drive)
    }
    #[zbus(property)]
    fn hint_ignore(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn id_type(&self) -> String {
        self.id_type.to_owned()
    }
    #[zbus(property)]
    fn crypto_backing_device(&self) -> zbus::fdo::Result<OwnedObjectPath> {
        object_path(self.crypto_backing)
    }
    #[zbus(property)]
    fn preferred_device(&self) -> Vec<u8> {
        [self.device.as_bytes(), b"\0"].concat()
    }
    #[zbus(property, name = "IdUUID")]
    fn id_uuid(&self) -> String {
        self.uuid.to_owned()
    }
    #[zbus(property)]
    fn read_only(&self) -> bool {
        self.read_only
    }
}

/// A volume's state, shared by its filesystem object.
struct Volume {
    name: &'static str,
    /// Where Mount puts it.
    mount_at: PathBuf,
    mounted: bool,
    /// The system's own volume: never unmounted.
    fixed_mount: bool,
}

struct Filesystem(Arc<Mutex<Volume>>);

impl Filesystem {
    fn volume(&self) -> std::sync::MutexGuard<'_, Volume> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[interface(name = "org.freedesktop.UDisks2.Filesystem")]
impl Filesystem {
    #[zbus(property)]
    fn mount_points(&self) -> Vec<Vec<u8>> {
        let volume = self.volume();
        if volume.mounted {
            vec![[volume.mount_at.as_os_str().as_encoded_bytes(), b"\0"].concat()]
        } else {
            Vec::new()
        }
    }

    async fn mount(
        &self,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, Refusal> {
        let path = {
            let mut volume = self.volume();
            log(&format!("Mount {}", volume.name));
            if volume.mounted {
                return Err(Refusal::Failed("already mounted".into()));
            }
            volume.mounted = true;
            volume.mount_at.to_string_lossy().into_owned()
        };
        self.mount_points_changed(&emitter).await?;
        Ok(path)
    }

    async fn unmount(
        &self,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Refusal> {
        {
            let mut volume = self.volume();
            log(&format!("Unmount {}", volume.name));
            if volume.fixed_mount {
                return Err(Refusal::DeviceBusy("the system runs from it".into()));
            }
            volume.mounted = false;
        }
        self.mount_points_changed(&emitter).await?;
        Ok(())
    }
}

struct Drive {
    name: &'static str,
    removable: bool,
    can_power_off: bool,
    vendor: &'static str,
    model: &'static str,
    bus: &'static str,
    rotation: i32,
}

#[interface(name = "org.freedesktop.UDisks2.Drive")]
impl Drive {
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
    #[zbus(property)]
    fn vendor(&self) -> String {
        self.vendor.to_owned()
    }
    #[zbus(property)]
    fn model(&self) -> String {
        self.model.to_owned()
    }
    #[zbus(property)]
    fn connection_bus(&self) -> String {
        self.bus.to_owned()
    }
    #[zbus(property)]
    fn rotation_rate(&self) -> i32 {
        self.rotation
    }
    fn power_off(&self, _options: HashMap<String, OwnedValue>) -> Result<(), Refusal> {
        log(&format!("PowerOff {}", self.name));
        if self.can_power_off {
            Ok(())
        } else {
            Err(Refusal::Failed("this drive cannot be powered off".into()))
        }
    }
}

/// One volume of the stand-in.
struct Spec {
    object: &'static str,
    block: Block,
    mounted: bool,
    /// Its mount point: "/" for the system's, else a folder under the root given.
    system: bool,
}

const NVME: &str = "/org/freedesktop/UDisks2/drives/nvme";
const STICK: &str = "/org/freedesktop/UDisks2/drives/usb_stick";
const DISK: &str = "/org/freedesktop/UDisks2/drives/usb_disk";

fn specs() -> Vec<Spec> {
    let block = |label, size, drive, id_type, device, uuid| Block {
        label,
        size,
        drive,
        id_type,
        crypto_backing: "/",
        device,
        uuid,
        read_only: false,
    };
    vec![
        Spec {
            object: "nvme0n1p2",
            block: block(
                "",
                1_000_000_000_000,
                NVME,
                "btrfs",
                "/dev/nvme0n1p2",
                "0f6a3c1e-5b7d-4e2a-9c8b-1d2e3f4a5b6c",
            ),
            mounted: true,
            system: true,
        },
        Spec {
            object: "dm_2d0",
            block: Block {
                crypto_backing: "/org/freedesktop/UDisks2/block_devices/nvme0n1p3",
                ..block(
                    "Data",
                    512_000_000_000,
                    NVME,
                    "btrfs",
                    "/dev/dm-0",
                    "7c1d2e3f-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
                )
            },
            mounted: true,
            system: false,
        },
        Spec {
            object: "sdb1",
            block: block(
                "STICK",
                64_000_000_000,
                STICK,
                "exfat",
                "/dev/sdb1",
                "1A2B-3C4D",
            ),
            mounted: true,
            system: false,
        },
        Spec {
            object: "sdc1",
            block: block(
                "BACKUP",
                2_000_000_000_000,
                DISK,
                "ext4",
                "/dev/sdc1",
                "a1b2c3d4-e5f6-4a7b-8c9d-0e1f2a3b4c5d",
            ),
            mounted: false,
            system: false,
        },
        Spec {
            object: "nvme0n1p4",
            block: Block {
                read_only: true,
                ..block(
                    "FULL",
                    8_000_000_000,
                    NVME,
                    "squashfs",
                    "/dev/nvme0n1p4",
                    "",
                )
            },
            mounted: true,
            system: false,
        },
    ]
}

fn serve(root: &Path) -> Result<zbus::blocking::Connection, Box<dyn std::error::Error>> {
    let mut builder = Builder::system()?
        .name("org.freedesktop.UDisks2")?
        .serve_at(ROOT, ObjectManager)?;
    for spec in specs() {
        let name = if spec.block.label.is_empty() {
            "System"
        } else {
            spec.block.label
        };
        let mount_at = if spec.system {
            PathBuf::from("/")
        } else {
            let folder = root.join(name);
            std::fs::create_dir_all(&folder)?;
            folder
        };
        let volume = Arc::new(Mutex::new(Volume {
            name,
            mount_at,
            mounted: spec.mounted,
            fixed_mount: spec.system,
        }));
        let path = format!("{ROOT}/block_devices/{}", spec.object);
        builder = builder
            .serve_at(path.clone(), spec.block)?
            .serve_at(path, Filesystem(volume))?;
    }
    for (path, name, removable, can_power_off, vendor, model, bus, rotation) in [
        (NVME, "nvme", false, false, "", "Example NVMe SSD", "", 0),
        (
            STICK,
            "usb_stick",
            true,
            true,
            "Example",
            "Flash Drive",
            "usb",
            0,
        ),
        (
            DISK,
            "usb_disk",
            true,
            true,
            "Example",
            "Portable Disk",
            "usb",
            5400,
        ),
    ] {
        builder = builder.serve_at(
            path,
            Drive {
                name,
                removable,
                can_power_off,
                vendor,
                model,
                bus,
                rotation,
            },
        )?;
    }
    Ok(builder.build()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("DBUS_SYSTEM_BUS_ADDRESS").is_none() {
        return Err("set DBUS_SYSTEM_BUS_ADDRESS to a private test bus; \
                    the stand-in never serves the machine's system bus"
            .into());
    }
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: fake_udisks2 <folder for the mount points>")?;
    std::fs::create_dir_all(&root)?;
    let _connection = serve(&root.canonicalize()?)?;
    log(&format!("fake UDisks2 serving under {}", root.display()));
    loop {
        std::thread::park();
    }
}
