//! A drive's Properties (W-153): what its dialog shows, from the volume the
//! device service lists and the size and free space of its filesystem.

use std::path::Path;

use rustphin_platform::devices::Device;
use serde::Serialize;

use crate::{
    format,
    places_panel::{NEARLY_FULL_PERCENT, device_name, filesystem_name, percent_used},
};

/// How full a mounted volume is, for the dialog's ring and its legend.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DriveUsage {
    /// The used part, 0 to 1: the ring's arc.
    pub fraction: f64,
    /// "22%", in the ring's middle; rounded down, as on the card.
    pub percent: String,
    /// The used space, with its exact bytes.
    pub used: String,
    /// The space still free for the user, with its exact bytes.
    pub free: String,
    /// The filesystem's size, with its exact bytes.
    pub total: String,
    /// At least 90 % used: the arc warns, as the card's bar does.
    pub nearly_full: bool,
    /// The ring in one sentence, for screen readers.
    pub description: String,
}

/// Everything the dialog shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DriveSheet {
    /// The volume's name, as its card shows it.
    pub title: String,
    /// The Lucide icon: `usb` for a removable drive, else `hard-drive`.
    pub icon: &'static str,
    /// Under the name: the filesystem and its traits, or "Not mounted".
    pub kind: String,
    /// How full it is, when mounted and measured.
    pub usage: Option<DriveUsage>,
    /// Why there is no ring, and what to do; empty when there is one.
    pub note: String,
    /// Label and value pairs, only those the service told.
    pub facts: Vec<(String, String)>,
}

/// How a connection, as the device service names it, is written.
fn connection_name(bus: &str) -> String {
    match bus {
        "usb" => "USB",
        "sdio" => "SD card reader",
        "ieee1394" => "FireWire",
        "thunderbolt" => "Thunderbolt",
        other => other,
    }
    .to_owned()
}

/// The Properties of `device`, with the `(total, free)` bytes of its
/// filesystem when mounted and measured.
pub fn drive_sheet(device: &Device, usage: Option<(u64, u64)>) -> DriveSheet {
    let details = &device.details;
    let mounted = device.mount_point.as_deref();
    let measured = usage.filter(|_| mounted.is_some());
    let filesystem = device.filesystem.as_deref().map(filesystem_name);

    let mut traits: Vec<String> = filesystem.iter().cloned().collect();
    if device.encrypted {
        traits.push("Encrypted".to_owned());
    }
    if device.removable {
        traits.push("Removable".to_owned());
    }
    let kind = if mounted.is_some() {
        traits.join(" \u{b7} ")
    } else {
        "Not mounted".to_owned()
    };

    let usage = measured.map(|(total, free)| {
        let free = free.min(total);
        let used = total - free;
        let percent = percent_used(total, free);
        DriveUsage {
            fraction: if total == 0 {
                0.
            } else {
                used as f64 / total as f64
            },
            percent: format!("{percent}%"),
            used: format::size_exact(used),
            free: format::size_exact(free),
            total: format::size_exact(total),
            nearly_full: percent >= NEARLY_FULL_PERCENT,
            description: format!(
                "{} of {} used ({percent}%), {} free",
                format::size(used),
                format::size(total),
                format::size(free)
            ),
        }
    });
    let note = match (mounted, &usage) {
        (None, _) => "Not mounted. Mount it to see how full it is.".to_owned(),
        (Some(_), None) => "Its free space could not be read.".to_owned(),
        (Some(_), Some(_)) => String::new(),
    };

    let mut facts: Vec<(String, String)> = Vec::new();
    let mut fact = |label: &str, value: String| facts.push((label.to_owned(), value));
    fact(
        "Mounted at",
        mounted.map_or_else(|| "Not mounted".to_owned(), shown),
    );
    if mounted.is_none() && device.size > 0 {
        fact("Size", format::size_exact(device.size));
    }
    if let Some(file) = &details.device_file {
        fact("Device", shown(file));
    }
    if let Some(filesystem) = filesystem {
        fact("Filesystem", filesystem);
    }
    if let Some(uuid) = &details.uuid {
        fact("UUID", uuid.clone());
    }
    fact(
        "Access",
        if details.read_only {
            "Read only"
        } else {
            "Read and write"
        }
        .to_owned(),
    );
    fact(
        "Encrypted",
        if device.encrypted {
            "Yes, unlocked"
        } else {
            "No"
        }
        .to_owned(),
    );
    if let Some(model) = &details.model {
        fact("Drive", model.clone());
    }
    match details.rotational {
        Some(true) => fact("Kind", "Hard disk".to_owned()),
        Some(false) => fact("Kind", "Solid-state drive".to_owned()),
        None => {}
    }
    if let Some(bus) = &details.connection {
        fact("Connection", connection_name(bus));
    }
    fact(
        "Removable",
        if device.removable { "Yes" } else { "No" }.to_owned(),
    );

    DriveSheet {
        title: device_name(device),
        icon: if device.removable {
            "usb"
        } else {
            "hard-drive"
        },
        kind,
        usage,
        note,
        facts,
    }
}

/// A path as text, for display only: bytes outside UTF-8 are replaced.
fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
// A test cannot go on without what it unwraps.
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use rustphin_platform::devices::DriveDetails;

    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn stick() -> Device {
        Device {
            id: "/org/freedesktop/UDisks2/block_devices/sdb1".into(),
            label: Some("STICK".into()),
            mount_point: Some(PathBuf::from("/run/media/u/STICK")),
            size: 64 * GIB,
            removable: true,
            power_off: Some("/drive".into()),
            filesystem: Some("exfat".into()),
            encrypted: false,
            details: DriveDetails {
                device_file: Some(PathBuf::from("/dev/sdb1")),
                uuid: Some("1A2B-3C4D".into()),
                read_only: false,
                model: Some("SanDisk Ultra".into()),
                connection: Some("usb".into()),
                rotational: Some(false),
            },
        }
    }

    fn fact<'a>(sheet: &'a DriveSheet, label: &str) -> Option<&'a str> {
        sheet
            .facts
            .iter()
            .find(|(name, _)| name == label)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn a_mounted_drive_shows_how_full_it_is_with_exact_bytes() {
        let sheet = drive_sheet(&stick(), Some((64 * GIB, 48 * GIB)));
        let usage = sheet.usage.as_ref().unwrap();
        assert!((usage.fraction - 0.25).abs() < 1e-9, "{}", usage.fraction);
        assert_eq!(usage.percent, "25%");
        assert_eq!(usage.used, "16.0 GiB (17,179,869,184 bytes)");
        assert_eq!(usage.free, "48.0 GiB (51,539,607,552 bytes)");
        assert_eq!(usage.total, "64.0 GiB (68,719,476,736 bytes)");
        assert!(!usage.nearly_full);
        assert_eq!(
            usage.description,
            "16.0 GiB of 64.0 GiB used (25%), 48.0 GiB free"
        );
        assert_eq!(sheet.note, "");
        assert_eq!(sheet.title, "STICK");
        assert_eq!(sheet.icon, "usb");
        assert_eq!(sheet.kind, "exFAT \u{b7} Removable");
    }

    #[test]
    fn the_facts_say_what_the_device_service_told_in_words() {
        let sheet = drive_sheet(&stick(), Some((64 * GIB, 48 * GIB)));
        let labels: Vec<&str> = sheet
            .facts
            .iter()
            .map(|(label, _)| label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "Mounted at",
                "Device",
                "Filesystem",
                "UUID",
                "Access",
                "Encrypted",
                "Drive",
                "Kind",
                "Connection",
                "Removable"
            ]
        );
        assert_eq!(fact(&sheet, "Mounted at"), Some("/run/media/u/STICK"));
        assert_eq!(fact(&sheet, "Device"), Some("/dev/sdb1"));
        assert_eq!(fact(&sheet, "Filesystem"), Some("exFAT"));
        assert_eq!(fact(&sheet, "Access"), Some("Read and write"));
        assert_eq!(fact(&sheet, "Kind"), Some("Solid-state drive"));
        assert_eq!(fact(&sheet, "Connection"), Some("USB"));
        assert_eq!(fact(&sheet, "Removable"), Some("Yes"));
    }

    #[test]
    fn what_the_service_did_not_tell_is_left_out() {
        let mut disk = stick();
        disk.removable = false;
        disk.encrypted = true;
        disk.details = DriveDetails {
            read_only: true,
            rotational: Some(true),
            ..DriveDetails::default()
        };
        let sheet = drive_sheet(&disk, Some((100, 50)));
        for absent in ["Device", "UUID", "Drive", "Connection"] {
            assert_eq!(fact(&sheet, absent), None, "{absent}");
        }
        assert_eq!(fact(&sheet, "Access"), Some("Read only"));
        assert_eq!(fact(&sheet, "Encrypted"), Some("Yes, unlocked"));
        assert_eq!(fact(&sheet, "Kind"), Some("Hard disk"));
        assert_eq!(sheet.icon, "hard-drive");
        // No kind at all when the service says nothing of spinning.
        disk.details.rotational = None;
        assert_eq!(fact(&drive_sheet(&disk, None), "Kind"), None);
    }

    #[test]
    fn an_unmounted_drive_has_no_ring_and_says_how_to_get_one() {
        let mut stick = stick();
        stick.mount_point = None;
        // A measure left over from before it was unmounted is not drawn.
        let sheet = drive_sheet(&stick, Some((100, 50)));
        assert_eq!(sheet.usage, None);
        assert_eq!(sheet.note, "Not mounted. Mount it to see how full it is.");
        assert_eq!(sheet.kind, "Not mounted");
        assert_eq!(fact(&sheet, "Mounted at"), Some("Not mounted"));
        assert_eq!(
            fact(&sheet, "Size"),
            Some("64.0 GiB (68,719,476,736 bytes)")
        );
    }

    #[test]
    fn a_mounted_drive_not_measured_says_so_and_a_full_one_warns() {
        let sheet = drive_sheet(&stick(), None);
        assert_eq!(sheet.usage, None);
        assert_eq!(sheet.note, "Its free space could not be read.");
        let full = drive_sheet(&stick(), Some((1000, 100))).usage.unwrap();
        assert!(full.nearly_full);
        assert_eq!(full.percent, "90%");
        // Free space above the size counts as all free, never below zero.
        let odd = drive_sheet(&stick(), Some((1000, 5000))).usage.unwrap();
        assert_eq!(odd.percent, "0%");
        assert_eq!(odd.used, "0 bytes");
    }

    #[test]
    fn connections_are_written_as_people_know_them() {
        assert_eq!(connection_name("usb"), "USB");
        assert_eq!(connection_name("sdio"), "SD card reader");
        assert_eq!(connection_name("ieee1394"), "FireWire");
        assert_eq!(connection_name("other"), "other");
    }
}
