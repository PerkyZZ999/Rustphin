//! The places panel's rows and words, free of any toolkit (W-114e): which
//! bookmarks show, in what order with the groups (W-105) and the Remote
//! and Devices sections, their icons, and how volumes are named.

use std::path::Path;

use rustphin_platform::{
    devices::Device,
    places::Place,
    remote::{RemoteAddress, network_folder_name},
};

use crate::{format, groups::Groups};

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

/// The rows of the panel, top to bottom: local places outside groups, then
/// each group's heading followed by its places unless it is collapsed, then
/// the Remote heading and the network places outside groups unless folded.
pub fn rows(
    places: &[Place],
    groups: &Groups,
    show_hidden: bool,
    remote_folded: bool,
    show_remote: bool,
) -> Vec<Row> {
    let layout = groups.layout(places);
    let (remote, local): (Vec<&Place>, Vec<&Place>) = layout
        .ungrouped
        .iter()
        .copied()
        .filter(|place| is_shown(place, show_hidden))
        .partition(|place| is_remote(place));
    let mut rows: Vec<Row> = local
        .iter()
        .map(|place| Row::Place(place.id.clone()))
        .collect();
    for (index, (group, members)) in layout.groups.iter().enumerate() {
        rows.push(Row::Group(index));
        if !group.collapsed {
            rows.extend(
                members
                    .iter()
                    .filter(|place| is_shown(place, show_hidden))
                    .map(|place| Row::Place(place.id.clone())),
            );
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

/// How full a volume is, in words short enough for the panel: "120.6 of
/// 476.9 GiB free", or both units when they differ.
pub fn usage_text(total: u64, free: u64) -> String {
    let (free, total) = (format::size(free), format::size(total));
    let unit = |size: &str| size.rsplit_once(' ').map(|(_, unit)| unit.to_owned());
    match (free.rsplit_once(' '), unit(&total)) {
        (Some((number, free_unit)), Some(total_unit)) if free_unit == total_unit => {
            format!("{number} of {total} free")
        }
        _ => format!("{free} of {total} free"),
    }
}

/// Whether the panel offers to unmount or remove `device`: anything mounted
/// or unpluggable, except the volumes the system and the user's home run from.
pub fn can_eject(device: &Device, home: Option<&Path>) -> bool {
    match &device.mount_point {
        Some(mount) => mount != Path::new("/") && home.is_none_or(|home| !home.starts_with(mount)),
        None => device.removable && device.can_power_off(),
    }
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
            },
            Group {
                name: "Media".into(),
                collapsed: true,
                places: vec!["media".into()],
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
    fn the_system_and_home_volumes_are_never_offered_for_unmounting() {
        let home = Some(Path::new("/home/u"));
        assert!(!can_eject(&device(Some("/"), false, false), home));
        assert!(!can_eject(&device(Some("/home"), false, false), home));
        assert!(!can_eject(&device(Some("/home/u"), false, false), home));
        assert!(can_eject(&device(Some("/mnt/data"), false, false), home));
        assert!(can_eject(&device(Some("/home/u/stick"), true, true), home));
        // Not mounted: only a drive that can be powered off has anything to do.
        assert!(can_eject(&device(None, true, true), home));
        assert!(!can_eject(&device(None, true, false), home));
        assert!(!can_eject(&device(None, false, false), home));
    }

    #[test]
    fn a_volume_without_a_label_is_named_by_its_size_and_usage_is_put_in_words() {
        let mut volume = device(None, true, true);
        assert_eq!(device_name(&volume), "64.0 GiB volume");
        volume.label = Some("STICK".into());
        assert_eq!(device_name(&volume), "STICK");
        const GIB: u64 = 1024 * 1024 * 1024;
        // One unit when both figures share it, so the line fits the panel.
        assert_eq!(usage_text(64 * GIB, 16 * GIB), "16.0 of 64.0 GiB free");
        // Both units when they differ.
        assert_eq!(
            usage_text(GIB + GIB / 5, 512 * 1024 * 1024),
            "512.0 MiB of 1.2 GiB free"
        );
        assert_eq!(device_name(&device(Some("/"), false, false)), "System");
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
