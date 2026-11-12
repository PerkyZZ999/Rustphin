//! Text forms of numbers, sizes and dates shown in the interface.
//!
//! Sizes and dates follow the style chosen in Settings, Views (W-106c2),
//! kept here for the whole program so every place that shows one agrees;
//! the `_in` forms take the style and are what the tests check.

use std::sync::atomic::{AtomicBool, Ordering};

use jiff::{Timestamp, tz::TimeZone};
use serde::{Deserialize, Serialize};

/// Units for sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    /// Powers of 1024: KiB, MiB, as Dolphin shows by default.
    #[default]
    Binary,
    /// Powers of 1000: kB, MB.
    Decimal,
}

/// How dates are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dates {
    /// `2026-11-12 14:32`.
    #[default]
    Iso,
    /// `Today 14:32`, `Yesterday 09:10`, else as `Iso`.
    Relative,
}

static DECIMAL: AtomicBool = AtomicBool::new(false);
static RELATIVE: AtomicBool = AtomicBool::new(false);

/// Sets the style every size and date is shown in from now on.
pub fn set_style(units: Units, dates: Dates) {
    DECIMAL.store(units == Units::Decimal, Ordering::Relaxed);
    RELATIVE.store(dates == Dates::Relative, Ordering::Relaxed);
}

fn units() -> Units {
    if DECIMAL.load(Ordering::Relaxed) {
        Units::Decimal
    } else {
        Units::Binary
    }
}

fn dates() -> Dates {
    if RELATIVE.load(Ordering::Relaxed) {
        Dates::Relative
    } else {
        Dates::Iso
    }
}

/// Formats a count with thousands separators.
pub fn count(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Picks the singular or plural word for a count.
pub fn plural<'a>(value: usize, one: &'a str, many: &'a str) -> &'a str {
    if value == 1 { one } else { many }
}

/// Formats a size in the chosen units with one decimal, as Dolphin does:
/// `512 B`, `1.5 KiB`, `2.0 GiB`.
pub fn size(bytes: u64) -> String {
    size_in(bytes, units())
}

/// [`size`] in `units`: binary `1.5 KiB` or decimal `1.5 kB`.
pub fn size_in(bytes: u64, units: Units) -> String {
    let (base, names) = match units {
        Units::Binary => (1024., ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"]),
        Units::Decimal => (1000., ["kB", "MB", "GB", "TB", "PB", "EB"]),
    };
    if (bytes as f64) < base {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / base;
    let mut unit = 0;
    // Step up before rounding would print "1024.0" of the smaller unit.
    while value >= base - 0.05 && unit + 1 < names.len() {
        value /= base;
        unit += 1;
    }
    format!("{value:.1} {}", names[unit])
}

/// Formats seconds since the Unix epoch as a date and time in `zone`, in
/// the chosen style: `2026-10-12 14:03` or `Today 14:03`. Out-of-range
/// values give an empty string.
pub fn timestamp(seconds: i64, zone: &TimeZone) -> String {
    timestamp_in(seconds, zone, dates(), Timestamp::now())
}

/// [`timestamp`] in `style`, with `now` deciding what today is.
pub fn timestamp_in(seconds: i64, zone: &TimeZone, style: Dates, now: Timestamp) -> String {
    let Ok(instant) = Timestamp::from_second(seconds) else {
        return String::new();
    };
    let when = instant.to_zoned(zone.clone());
    if style == Dates::Relative {
        let today = now.to_zoned(zone.clone()).date();
        let day = when.date();
        let name = if day == today {
            Some("Today")
        } else if today.yesterday().ok() == Some(day) {
            Some("Yesterday")
        } else {
            None
        };
        if let Some(name) = name {
            return format!("{name} {}", when.strftime("%H:%M"));
        }
    }
    when.strftime("%Y-%m-%d %H:%M").to_string()
}

/// Like [`timestamp`] with seconds, for Properties: `2026-10-12 14:03:27`.
pub fn timestamp_precise(seconds: i64, zone: &TimeZone) -> String {
    formatted(seconds, zone, "%Y-%m-%d %H:%M:%S")
}

fn formatted(seconds: i64, zone: &TimeZone, format: &str) -> String {
    Timestamp::from_second(seconds)
        .map(|instant| instant.to_zoned(zone.clone()).strftime(format).to_string())
        .unwrap_or_default()
}

/// A size with its exact bytes, for Properties: `6.3 MiB (6,611,968 bytes)`.
/// Below 1 KiB the bytes alone say it all.
pub fn size_exact(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} {}", plural(bytes as usize, "byte", "bytes"));
    }
    format!("{} ({} bytes)", size(bytes), count(bytes as usize))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn exact_sizes_add_the_bytes_with_separators() {
        assert_eq!(size_exact(6_611_968), "6.3 MiB (6,611,968 bytes)");
        assert_eq!(size_exact(1), "1 byte");
        assert_eq!(size_exact(512), "512 bytes");
    }

    #[test]
    fn counts_use_thousands_separators() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(500_000), "500,000");
        assert_eq!(count(1_234_567), "1,234,567");
    }

    #[test]
    fn plural_picks_the_word_for_the_count() {
        assert_eq!(plural(1, "item", "items"), "item");
        assert_eq!(plural(0, "item", "items"), "items");
        assert_eq!(plural(2, "item", "items"), "items");
    }

    #[test]
    fn sizes_below_one_kibibyte_are_exact_bytes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
    }

    #[test]
    fn sizes_use_binary_units_with_one_decimal() {
        assert_eq!(size(1024), "1.0 KiB");
        assert_eq!(size(1536), "1.5 KiB");
        assert_eq!(size(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(size(3 * 1024 * 1024 * 1024 + 512 * 1024 * 1024), "3.5 GiB");
        assert_eq!(size(u64::MAX), "16.0 EiB");
    }

    #[test]
    fn sizes_never_print_1024_of_a_unit() {
        assert_eq!(size(1024 * 1024 - 1), "1.0 MiB");
        assert_eq!(size(1023 * 1024 + 900), "1023.9 KiB");
    }

    #[test]
    fn timestamps_are_shown_in_the_given_zone() {
        // 2026-10-12 13:30:45 UTC.
        let seconds = 1_791_811_845;
        assert_eq!(timestamp(seconds, &TimeZone::UTC), "2026-10-12 13:30");
        let montreal = TimeZone::fixed(jiff::tz::offset(-4));
        assert_eq!(timestamp(seconds, &montreal), "2026-10-12 09:30");
        assert_eq!(timestamp(0, &TimeZone::UTC), "1970-01-01 00:00");
        assert_eq!(
            timestamp_precise(seconds + 7, &TimeZone::UTC),
            "2026-10-12 13:30:52"
        );
    }

    #[test]
    fn decimal_sizes_count_in_thousands() {
        assert_eq!(size_in(999, Units::Decimal), "999 B");
        assert_eq!(size_in(1000, Units::Decimal), "1.0 kB");
        assert_eq!(size_in(1_500_000, Units::Decimal), "1.5 MB");
        assert_eq!(size_in(999_990, Units::Decimal), "1.0 MB");
        assert_eq!(size_in(1536, Units::Binary), "1.5 KiB");
    }

    #[test]
    fn relative_dates_name_today_and_yesterday_only() {
        let zone = TimeZone::UTC;
        // 2026-10-12 13:30:45 UTC.
        let now = Timestamp::from_second(1_791_811_845).unwrap();
        let hour = 3600;
        let at = |seconds: i64| timestamp_in(seconds, &zone, Dates::Relative, now);
        assert_eq!(at(1_791_811_845 - hour), "Today 12:30");
        assert_eq!(at(1_791_811_845 - 24 * hour), "Yesterday 13:30");
        assert_eq!(at(1_791_811_845 - 48 * hour), "2026-10-10 13:30");
        assert_eq!(
            timestamp_in(1_791_811_845, &zone, Dates::Iso, now),
            "2026-10-12 13:30"
        );
        assert_eq!(timestamp_in(i64::MAX, &zone, Dates::Relative, now), "");
    }

    #[test]
    fn timestamps_outside_the_supported_range_are_blank() {
        assert_eq!(timestamp(i64::MAX, &TimeZone::UTC), "");
    }
}
