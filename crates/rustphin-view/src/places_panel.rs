//! The places panel's rows and words, free of any toolkit (W-114e): which
//! bookmarks show, in what order with the groups (W-105) and the Remote
//! and Devices sections, their icons, and how volumes are named; and
//! [`PlacesPanel`], which holds the panel's state and decides what each
//! key, menu entry, drop and rename does (W-127).

use std::path::Path;

use rustphin_platform::{
    devices::Device,
    places::Place,
    remote::{RemoteAddress, network_folder_name},
};

use crate::{
    format,
    groups::{Groups, Member},
};

mod panel;

pub use panel::{Effect, Opening, PlacesEdit, PlacesPanel, RowKind, RowView};

/// One line of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    /// A place, by its identifier in the shared file.
    Place(String),
    /// A group heading, by the group's position.
    Group(usize),
    /// A storage volume, by the device service's name for it.
    Device(String),
    /// The Devices heading, which folds the volumes away.
    Devices,
    /// The Remote heading, which folds the network places away.
    Remote,
    /// A separator, by its identifier (W-152).
    Separator(String),
}

/// The message shown when the device service cannot be reached.
pub const DEVICES_UNAVAILABLE: &str = "Devices could not be listed. Open folders by path, or restart Rustphin to reconnect to the device service.";

/// The address of a network bookmark (`sftp://…`, `smb://…`), opened
/// through kio-fuse.
pub fn network_address(place: &Place) -> Option<RemoteAddress> {
    RemoteAddress::parse(&place.href).filter(RemoteAddress::is_network_folder)
}

/// A network folder made in Dolphin (`remote:/NAME`), whose address KDE
/// keeps in a file of its own.
pub fn is_network_folder(place: &Place) -> bool {
    network_folder_name(&place.href).is_some()
}

/// Whether `place` is somewhere else on the network: it goes in the Remote
/// section, as in Dolphin, unless the user put it in a group.
pub fn is_remote(place: &Place) -> bool {
    network_address(place).is_some() || is_network_folder(place)
}

/// Whether the panel shows `place`: local folders, network folders and the
/// trash, unless another application keeps the bookmark to itself. Dolphin
/// marks the places added in it as its own; Rustphin stands in for Dolphin,
/// so it shows those as Dolphin does (W-101).
pub fn is_shown(place: &Place, show_hidden: bool) -> bool {
    let reachable = place.path.is_some() || place.href.starts_with("trash:") || is_remote(place);
    let ours = place
        .only_in_app
        .as_deref()
        .is_none_or(|app| app == "rustphin" || app == "dolphin");
    reachable && ours && (show_hidden || !place.hidden)
}

/// The local places outside groups that the panel shows, in the shared
/// file's order: the places the separators outside groups sit between.
pub(crate) fn shown_loose<'a>(
    places: &'a [Place],
    groups: &'a Groups,
    show_hidden: bool,
) -> Vec<&'a Place> {
    groups
        .layout(places)
        .ungrouped
        .into_iter()
        .filter(|place| is_shown(place, show_hidden) && !is_remote(place))
        .collect()
}

/// Where each separator outside groups is drawn among `shown` (the places
/// of [`shown_loose`]), in the order of [`Groups::separators`]: below that
/// many of them, 0 above them all. One below a place that is not shown
/// (hidden, in a group, remote) sits below the last shown place before it
/// in the shared file; one below a place gone from the file is not drawn.
pub(crate) fn loose_slots(
    places: &[Place],
    shown: &[&Place],
    groups: &Groups,
) -> Vec<Option<usize>> {
    let position = |id: &str| places.iter().position(|place| place.id == id);
    let shown_at: Vec<usize> = shown
        .iter()
        .filter_map(|place| position(&place.id))
        .collect();
    groups
        .separators
        .iter()
        .map(|separator| match separator.after.as_deref() {
            None => Some(0),
            Some(after) => {
                let anchor = position(after)?;
                Some(shown_at.iter().filter(|at| **at <= anchor).count())
            }
        })
        .collect()
}

/// The rows of the panel, top to bottom: local places outside groups with
/// the separators between them, then each entry of the group list (a
/// group's heading followed by its places and separators unless it is
/// collapsed, or a separator between groups), then the Remote heading and
/// the network places outside groups unless folded.
pub fn rows(
    places: &[Place],
    groups: &Groups,
    show_hidden: bool,
    remote_folded: bool,
    show_remote: bool,
) -> Vec<Row> {
    let layout = groups.layout(places);
    let local = shown_loose(places, groups, show_hidden);
    let remote: Vec<&Place> = layout
        .ungrouped
        .iter()
        .copied()
        .filter(|place| is_shown(place, show_hidden) && is_remote(place))
        .collect();
    let slots = loose_slots(places, &local, groups);
    let separators_at = |slot: usize| {
        groups
            .separators
            .iter()
            .zip(&slots)
            .filter(move |(_, at)| **at == Some(slot))
            .map(|(separator, _)| Row::Separator(separator.id.clone()))
    };
    let mut rows: Vec<Row> = separators_at(0).collect();
    for (index, place) in local.iter().enumerate() {
        rows.push(Row::Place(place.id.clone()));
        rows.extend(separators_at(index + 1));
    }
    for (index, (group, members)) in layout.groups.iter().enumerate() {
        if let Some(id) = &group.separator {
            rows.push(Row::Separator(id.clone()));
            continue;
        }
        rows.push(Row::Group(index));
        if !group.collapsed {
            rows.extend(members.iter().filter_map(|member| match member {
                Member::Place(place) => {
                    is_shown(place, show_hidden).then(|| Row::Place(place.id.clone()))
                }
                Member::Separator(id) => Some(Row::Separator((*id).to_owned())),
            }));
        }
    }
    if show_remote && !remote.is_empty() {
        rows.push(Row::Remote);
        if !remote_folded {
            rows.extend(remote.iter().map(|place| Row::Place(place.id.clone())));
        }
    }
    rows
}

/// The Lucide icon standing in for a place's theme icon name.
pub fn icon_for(place: &Place) -> &'static str {
    let name = place.icon.as_deref().unwrap_or_default();
    let has = |part: &str| name.contains(part);
    if place.href.starts_with("trash:") || has("trash") {
        "trash"
    } else if is_remote(place) {
        "server"
    } else if has("home") {
        "house"
    } else if has("desktop") {
        "monitor"
    } else if has("download") {
        "download"
    } else if has("document") {
        "file-text"
    } else if has("music") || has("audio") {
        "music"
    } else if has("picture") || has("image") {
        "image"
    } else if has("video") {
        "film"
    } else if has("game") {
        "gamepad-2"
    } else {
        "folder"
    }
}

/// The name shown for a volume: its label, or its size when it has none.
pub fn device_name(device: &Device) -> String {
    match &device.label {
        Some(label) => label.clone(),
        None if device.mount_point.as_deref() == Some(Path::new("/")) => "System".to_owned(),
        None => format!("{} volume", format::size(device.size)),
    }
}

/// The listed volume mounted at `mount`, if one is: a folder on a
/// filesystem that is not a listed volume (tmpfs, a network mount) has none.
pub fn volume_at<'a>(devices: &'a [Device], mount: &Path) -> Option<&'a Device> {
    devices
        .iter()
        .find(|device| device.mount_point.as_deref() == Some(mount))
}

/// From this many percent used, a volume's usage bar warns (W-148).
pub const NEARLY_FULL_PERCENT: u64 = 90;

/// How much of a volume is used, in whole percent rounded down, so "90%"
/// means at least 90 %: 0 for a volume of no size.
pub fn percent_used(total: u64, free: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let used = u128::from(total - free.min(total));
    // At most 100: used never exceeds total.
    (used * 100 / u128::from(total)) as u64
}

/// A volume's "used / total" figures, short enough for its card: one unit
/// when both share it ("211.3 / 927.0 GiB"), else both ("512.0 MiB / 1.2
/// GiB"), in the units Settings chose, as the status line.
pub fn used_of_total(total: u64, free: u64) -> String {
    let (used, total) = (format::size(total - free.min(total)), format::size(total));
    match (used.rsplit_once(' '), total.rsplit_once(' ')) {
        (Some((number, used_unit)), Some((_, total_unit))) if used_unit == total_unit => {
            format!("{number} / {total}")
        }
        _ => format!("{used} / {total}"),
    }
}

/// How a filesystem type, as UDisks2 names it (`IdType`), is written.
pub fn filesystem_name(id_type: &str) -> String {
    match id_type {
        "btrfs" => "Btrfs",
        "vfat" | "fat" | "msdos" => "FAT",
        "exfat" => "exFAT",
        "ntfs" | "ntfs3" => "NTFS",
        "xfs" => "XFS",
        "f2fs" => "F2FS",
        "iso9660" => "ISO 9660",
        "udf" => "UDF",
        "hfsplus" => "HFS+",
        "apfs" => "APFS",
        "squashfs" => "SquashFS",
        "zfs_member" | "zfs" => "ZFS",
        "bcachefs" => "Bcachefs",
        // ext2, ext3, ext4 and anything else: as the system names it.
        other => other,
    }
    .to_owned()
}

/// What a volume's card shows (W-148), besides its name and icon.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceCard {
    /// Right of the name: "used / total" when mounted, the size when not.
    pub figures: String,
    /// The last line's left: the filesystem and its traits ("Btrfs ·
    /// Encrypted"), "Not mounted", or "Working…" while it is mounted or
    /// removed.
    pub traits: String,
    /// The last line's right: "22% used", or empty.
    pub percent: String,
    /// How full it is, 0 to 1; -1 when there is no bar to draw.
    pub usage: f64,
    /// At least [`NEARLY_FULL_PERCENT`] used: the bar warns.
    pub nearly_full: bool,
    /// The card in one sentence, for screen readers.
    pub description: String,
}

/// The card of `device`, with the `(total, free)` bytes of its filesystem
/// when mounted and measured; `busy` while it is being mounted or removed.
pub fn device_card(device: &Device, usage: Option<(u64, u64)>, busy: bool) -> DeviceCard {
    let mounted = device.mount_point.is_some();
    let mut traits: Vec<String> = device
        .filesystem
        .as_deref()
        .map(filesystem_name)
        .into_iter()
        .collect();
    if device.encrypted {
        traits.push("Encrypted".to_owned());
    }
    if device.removable {
        traits.push("Removable".to_owned());
    }
    let traits = traits.join(" \u{b7} ");
    let measured = usage.filter(|_| mounted);
    let (figures, percent) = match measured {
        Some((total, free)) => (used_of_total(total, free), Some(percent_used(total, free))),
        None if !mounted && device.size > 0 => (format::size(device.size), None),
        None => (String::new(), None),
    };
    let line = if busy {
        "Working\u{2026}".to_owned()
    } else if mounted {
        traits.clone()
    } else {
        "Not mounted".to_owned()
    };
    let description = match (measured, percent) {
        (Some((total, free)), Some(percent)) => {
            let used = format::size(total - free.min(total));
            let total = format::size(total);
            [format!("{used} of {total} used ({percent}%)"), traits]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ => [line.clone(), figures.clone()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
    };
    DeviceCard {
        figures,
        traits: line,
        percent: percent.map_or_else(String::new, |percent| format!("{percent}% used")),
        usage: measured.map_or(-1., |(total, free)| {
            if total == 0 {
                0.
            } else {
                (total - free.min(total)) as f64 / total as f64
            }
        }),
        nearly_full: percent.is_some_and(|percent| percent >= NEARLY_FULL_PERCENT),
        description,
    }
}

/// Whether `device` holds the system or the user's home folder: those
/// volumes are never offered for unmounting or removal.
fn runs_the_system(device: &Device, home: Option<&Path>) -> bool {
    device.mount_point.as_deref().is_some_and(|mount| {
        mount == Path::new("/") || home.is_some_and(|home| home.starts_with(mount))
    })
}

/// Whether the drive menu offers Unmount: a mounted volume the system and
/// the home folder do not run from.
pub fn can_unmount(device: &Device, home: Option<&Path>) -> bool {
    device.mount_point.is_some() && !runs_the_system(device, home)
}

/// Whether the drive menu offers Safely remove: a removable or power-off
/// capable drive the system and the home folder do not run from.
pub fn can_remove(device: &Device, home: Option<&Path>) -> bool {
    (device.removable || device.can_power_off()) && !runs_the_system(device, home)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::groups::Group;

    use super::*;

    fn place(id: &str, href: &str) -> Place {
        Place {
            id: id.to_owned(),
            title: id.to_owned(),
            href: href.to_owned(),
            path: rustphin_platform::uri::path_from_uri(href.as_bytes()),
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        }
    }

    #[test]
    fn local_and_network_folders_and_the_trash_are_shown_and_other_places_are_not() {
        let mut restricted = place("r", "file:///x");
        restricted.only_in_app = Some("gwenview".into());
        // Dolphin marks the places added in it as its own; Rustphin stands
        // in for Dolphin, so they are its too (W-101: the maintainer's NAS).
        let mut dolphins = place("d", "remote:/NAS");
        dolphins.only_in_app = Some("dolphin".into());
        let mut ours = place("o", "file:///x");
        ours.only_in_app = Some("rustphin".into());
        let mut hidden = place("h", "file:///x");
        hidden.hidden = true;

        assert!(is_shown(&place("a", "file:///home/u"), false));
        assert!(is_shown(&place("t", "trash:/"), false));
        assert!(is_shown(&ours, false));
        assert!(is_shown(&dolphins, false));
        assert!(is_shown(&place("ssh", "sftp://me@host/srv"), false));
        assert!(is_shown(&place("nas", "smb://nas/share"), false));
        assert!(is_shown(&place("lab", "remote:/NAS"), false));
        assert!(!is_shown(&place("n", "remote:/"), false));
        assert!(!is_shown(&place("s", "recentlyused:/files"), false));
        assert!(!is_shown(&place("web", "https://example.org/"), false));
        assert!(!is_shown(&restricted, false));
        assert!(!is_shown(&hidden, false));
        assert!(is_shown(&hidden, true));
    }

    #[test]
    fn network_places_follow_the_groups_under_their_own_foldable_heading() {
        let places = [
            place("lab", "remote:/NAS"),
            place("home", "file:///home/u"),
            place("nas", "smb://nas/share"),
            place("vps", "sftp://me@host/"),
            place("network", "remote:/"),
            place("docs", "file:///d"),
        ];
        let mut groups = Groups::default();
        groups.groups = vec![Group {
            name: "Servers".into(),
            collapsed: false,
            places: vec!["vps".into()],
            separator: None,
        }];
        let p = |id: &str| Row::Place(id.to_owned());

        assert_eq!(
            rows(&places, &groups, false, false, true),
            [
                p("home"),
                p("docs"),
                Row::Group(0),
                p("vps"),
                Row::Remote,
                p("lab"),
                p("nas"),
            ]
        );
        assert_eq!(
            rows(&places, &groups, false, true, true),
            [p("home"), p("docs"), Row::Group(0), p("vps"), Row::Remote]
        );
        // Settings, Places: without the Remote section its heading and
        // places leave the keyboard's list; a grouped one stays.
        assert_eq!(
            rows(&places, &groups, false, false, false),
            [p("home"), p("docs"), Row::Group(0), p("vps")]
        );
        // No network places outside groups: no Remote heading.
        let local = [place("home", "file:///home/u")];
        assert_eq!(
            rows(&local, &Groups::default(), false, false, true),
            [p("home")]
        );
    }

    #[test]
    fn rows_list_ungrouped_places_then_each_group_and_skip_collapsed_members() {
        let mut hidden = place("hidden", "file:///h");
        hidden.hidden = true;
        let places = [
            place("home", "file:///home/u"),
            place("work-a", "file:///a"),
            place("net", "remote:/"),
            place("work-b", "file:///b"),
            place("media", "file:///m"),
            hidden,
        ];
        let mut groups = Groups::default();
        groups.groups = vec![
            Group {
                name: "Work".into(),
                collapsed: false,
                places: vec!["work-b".into(), "work-a".into(), "hidden".into()],
                separator: None,
            },
            Group {
                name: "Media".into(),
                collapsed: true,
                places: vec!["media".into()],
                separator: None,
            },
        ];
        let p = |id: &str| Row::Place(id.to_owned());

        assert_eq!(
            rows(&places, &groups, false, false, true),
            [
                p("home"),
                Row::Group(0),
                p("work-b"),
                p("work-a"),
                Row::Group(1)
            ]
        );
        assert_eq!(
            rows(&places, &groups, true, false, true),
            [
                p("home"),
                Row::Group(0),
                p("work-b"),
                p("work-a"),
                p("hidden"),
                Row::Group(1)
            ]
        );
    }

    fn device(mount: Option<&str>, removable: bool, power_off: bool) -> Device {
        Device {
            id: "/dev/x".into(),
            label: None,
            mount_point: mount.map(PathBuf::from),
            size: 64 * 1024 * 1024 * 1024,
            removable,
            power_off: power_off.then(|| "/drive".to_owned()),
            filesystem: None,
            encrypted: false,
            details: Default::default(),
        }
    }

    #[test]
    fn only_a_listed_volume_mounted_exactly_there_names_a_folders_filesystem() {
        let mut data = device(Some("/mnt/data"), true, false);
        data.label = Some("Data".into());
        let devices = vec![device(Some("/"), false, false), data];
        let name = |mount: &str| volume_at(&devices, Path::new(mount)).map(device_name);
        assert_eq!(name("/mnt/data").as_deref(), Some("Data"));
        assert_eq!(name("/").as_deref(), Some("System"));
        // tmpfs under the root volume is not the root volume.
        assert_eq!(name("/tmp"), None);
    }

    #[test]
    fn the_system_and_home_volumes_are_never_offered_for_unmounting_or_removal() {
        let home = Some(Path::new("/home/u"));
        for runs_the_system in [
            device(Some("/"), false, false),
            device(Some("/home"), true, true),
            device(Some("/home/u"), false, false),
        ] {
            assert!(!can_unmount(&runs_the_system, home), "{runs_the_system:?}");
            assert!(!can_remove(&runs_the_system, home), "{runs_the_system:?}");
        }
        // A volume mounted inside home is not the one home runs from.
        assert!(can_unmount(
            &device(Some("/home/u/stick"), true, true),
            home
        ));
        assert!(can_remove(&device(Some("/home/u/stick"), true, true), home));
    }

    #[test]
    fn unmount_needs_a_mount_and_safe_removal_a_removable_or_power_off_capable_drive() {
        let home = Some(Path::new("/home/u"));
        let fixed = device(Some("/mnt/data"), false, false);
        assert!(can_unmount(&fixed, home));
        assert!(!can_remove(&fixed, home));
        // Removable, or able to power off: either is enough.
        assert!(can_remove(&device(Some("/mnt/card"), true, false), home));
        assert!(can_remove(&device(Some("/mnt/disk"), false, true), home));
        // Not mounted: nothing to unmount, but it can still be powered off.
        let unplugged = device(None, true, true);
        assert!(!can_unmount(&unplugged, home));
        assert!(can_remove(&unplugged, home));
    }

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn a_volume_without_a_label_is_named_by_its_size() {
        let mut volume = device(None, true, true);
        assert_eq!(device_name(&volume), "64.0 GiB volume");
        volume.label = Some("STICK".into());
        assert_eq!(device_name(&volume), "STICK");
        assert_eq!(device_name(&device(Some("/"), false, false)), "System");
    }

    #[test]
    fn used_and_total_share_their_unit_when_they_can() {
        assert_eq!(used_of_total(64 * GIB, 16 * GIB), "48.0 / 64.0 GiB");
        assert_eq!(
            used_of_total(GIB + GIB / 5, GIB + GIB / 5 - 512 * 1024 * 1024),
            "512.0 MiB / 1.2 GiB"
        );
        // Free space the size of the volume, or more, is nothing used.
        assert_eq!(used_of_total(4096, 8192), "0 B / 4.0 KiB");
    }

    #[test]
    fn percent_used_is_rounded_down_so_ninety_means_at_least_ninety() {
        assert_eq!(percent_used(1000, 101), 89);
        assert_eq!(percent_used(1000, 100), 90);
        assert_eq!(percent_used(1000, 0), 100);
        assert_eq!(percent_used(0, 0), 0);
        // No overflow on the largest sizes.
        assert_eq!(percent_used(u64::MAX, u64::MAX / 2), 50);
    }

    #[test]
    fn a_mounted_card_shows_used_and_total_its_filesystem_and_traits_and_percent() {
        let mut data = device(Some("/mnt/data"), false, false);
        data.filesystem = Some("btrfs".into());
        data.encrypted = true;

        let card = device_card(&data, Some((1000 * GIB, 780 * GIB)), false);

        assert_eq!(card.figures, "220.0 / 1000.0 GiB");
        assert_eq!(card.traits, "Btrfs \u{b7} Encrypted");
        assert_eq!(card.percent, "22% used");
        assert!((card.usage - 0.22).abs() < 1e-9, "{}", card.usage);
        assert!(!card.nearly_full);
        assert_eq!(
            card.description,
            "220.0 GiB of 1000.0 GiB used (22%), Btrfs \u{b7} Encrypted"
        );
    }

    #[test]
    fn a_removable_card_says_so_after_its_filesystem() {
        let mut stick = device(Some("/run/media/u/STICK"), true, true);
        stick.filesystem = Some("exfat".into());
        let card = device_card(&stick, Some((64 * GIB, 32 * GIB)), false);
        assert_eq!(card.traits, "exFAT \u{b7} Removable");
        assert_eq!(card.percent, "50% used");
    }

    #[test]
    fn a_volume_warns_from_ninety_percent_used() {
        let data = device(Some("/mnt/data"), false, false);
        let at = |free| device_card(&data, Some((1000, free)), false);
        assert!(!at(101).nearly_full, "89 %");
        assert!(at(100).nearly_full, "90 %");
        assert!(at(0).nearly_full, "full");
        // Its words say it too, so the warning is never colour alone.
        assert_eq!(at(0).percent, "100% used");
    }

    #[test]
    fn an_unmounted_card_shows_its_size_and_not_mounted_without_a_bar() {
        let mut stick = device(None, true, true);
        stick.filesystem = Some("ext4".into());
        // A measure left over from before it was unmounted is not drawn.
        let card = device_card(&stick, Some((100, 50)), false);
        assert_eq!(card.figures, "64.0 GiB");
        assert_eq!(card.traits, "Not mounted");
        assert_eq!(card.percent, "");
        assert!((card.usage + 1.).abs() < 1e-9, "no bar: {}", card.usage);
        assert!(!card.nearly_full);
        assert_eq!(card.description, "Not mounted, 64.0 GiB");
    }

    #[test]
    fn a_card_being_mounted_or_removed_says_it_is_working() {
        let stick = device(None, true, true);
        assert_eq!(device_card(&stick, None, true).traits, "Working\u{2026}");
        let data = device(Some("/mnt/data"), false, false);
        let card = device_card(&data, Some((100, 50)), true);
        assert_eq!(card.traits, "Working\u{2026}");
        assert_eq!(card.percent, "50% used");
    }

    #[test]
    fn a_mounted_card_not_yet_measured_draws_no_bar_and_no_figures() {
        let card = device_card(&device(Some("/mnt/data"), false, false), None, false);
        assert_eq!((card.figures.as_str(), card.percent.as_str()), ("", ""));
        assert!((card.usage + 1.).abs() < 1e-9);
    }

    #[test]
    fn filesystem_types_are_written_as_people_know_them() {
        assert_eq!(filesystem_name("btrfs"), "Btrfs");
        assert_eq!(filesystem_name("vfat"), "FAT");
        assert_eq!(filesystem_name("exfat"), "exFAT");
        assert_eq!(filesystem_name("ntfs3"), "NTFS");
        assert_eq!(filesystem_name("ext4"), "ext4");
        assert_eq!(filesystem_name("something"), "something");
    }

    #[test]
    fn theme_icon_names_map_to_bundled_icons() {
        let with_icon = |icon: &str| {
            let mut place = place("x", "file:///x");
            place.icon = Some(icon.to_owned());
            icon_for(&place)
        };
        assert_eq!(with_icon("user-home-symbolic"), "house");
        assert_eq!(with_icon("folder-downloads"), "download");
        assert_eq!(with_icon("folder-documents"), "file-text");
        assert_eq!(with_icon("applications-games-symbolic"), "gamepad-2");
        assert_eq!(with_icon("something-else"), "folder");
        assert_eq!(icon_for(&place("t", "trash:/")), "trash");
        // Dolphin shows network bookmarks with a server icon whatever their theme icon.
        let mut network = place("n", "sftp://host/");
        network.icon = Some("folder-documents".into());
        assert_eq!(icon_for(&network), "server");
    }
}
