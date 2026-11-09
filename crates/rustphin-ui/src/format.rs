//! Text forms of numbers, sizes and dates shown in the interface.

use jiff::{Timestamp, tz::TimeZone};

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

/// Formats a size in binary units with one decimal, as Dolphin does:
/// `512 B`, `1.5 KiB`, `2.0 GiB`.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.;
    let mut unit = 0;
    // Step up before rounding would print "1024.0" of the smaller unit.
    while value >= 1023.95 && unit + 1 < UNITS.len() {
        value /= 1024.;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

/// Formats seconds since the Unix epoch as a date and time in `zone`:
/// `2026-10-12 14:03`. Out-of-range values give an empty string.
pub fn timestamp(seconds: i64, zone: &TimeZone) -> String {
    formatted(seconds, zone, "%Y-%m-%d %H:%M")
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
    fn timestamps_outside_the_supported_range_are_blank() {
        assert_eq!(timestamp(i64::MAX, &TimeZone::UTC), "");
    }
}
