//! Permission bits, as shown and as edited.
//!
//! Only the nine read, write and execute bits are toggled. Setuid, setgid and
//! the sticky bit are kept when a new mode is combined with the one on disk.

/// The setuid, setgid and sticky bits.
const SPECIAL: u32 = 0o7000;
/// The nine read, write and execute bits.
const ORDINARY: u32 = 0o777;

/// `mode` with only its permission bits.
pub fn permission_bits(mode: u32) -> u32 {
    mode & 0o7777
}

/// The mode to write: the edited read, write and execute bits, and the special
/// bits from `original`.
pub fn combine_mode(edited: u32, original: u32) -> u32 {
    (edited & ORDINARY) | (original & SPECIAL)
}

/// Whether `mode` has `bit` set. `bit` is one permission bit, such as `0o400`.
pub fn has_bit(mode: u32, bit: u32) -> bool {
    mode & bit != 0
}

/// `mode` with `bit` set or cleared.
pub fn with_bit(mode: u32, bit: u32, on: bool) -> u32 {
    if on { mode | bit } else { mode & !bit }
}

/// Four-digit octal, such as `0644`.
pub fn octal_mode(mode: u32) -> String {
    format!("{:04o}", permission_bits(mode))
}

/// The symbolic form, such as `rwxr-xr-x`. Setuid, setgid and the sticky bit
/// replace the execute character with `s`, `s` or `t`, or with the capital
/// when execute is not also set.
pub fn symbolic_mode(mode: u32) -> String {
    let mode = permission_bits(mode);
    let mut out = String::with_capacity(9);
    for (shift, special, special_char) in [(6, 0o4000, 's'), (3, 0o2000, 's'), (0, 0o1000, 't')] {
        let bits = (mode >> shift) & 0o7;
        out.push(if bits & 0o4 != 0 { 'r' } else { '-' });
        out.push(if bits & 0o2 != 0 { 'w' } else { '-' });
        let execute = bits & 0o1 != 0;
        out.push(match (mode & special != 0, execute) {
            (true, true) => special_char,
            (true, false) => special_char.to_ascii_uppercase(),
            (false, true) => 'x',
            (false, false) => '-',
        });
    }
    out
}

/// Whether setuid, setgid or the sticky bit is set.
pub fn has_special(mode: u32) -> bool {
    mode & SPECIAL != 0
}

/// The read bit for owner (`0`), group (`1`) or others (`2`).
pub fn read_bit(class: u8) -> u32 {
    0o400 >> (class * 3)
}

/// The write bit for owner (`0`), group (`1`) or others (`2`).
pub fn write_bit(class: u8) -> u32 {
    0o200 >> (class * 3)
}

/// The execute bit for owner (`0`), group (`1`) or others (`2`).
pub fn execute_bit(class: u8) -> u32 {
    0o100 >> (class * 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_is_shown_in_words_and_octal() {
        assert_eq!(symbolic_mode(0o755), "rwxr-xr-x");
        assert_eq!(octal_mode(0o644), "0644");
        assert_eq!(symbolic_mode(0o4755), "rwsr-xr-x");
        assert_eq!(symbolic_mode(0o4644), "rwSr--r--");
        assert_eq!(symbolic_mode(0o1755), "rwxr-xr-t");
    }

    #[test]
    fn editing_keeps_special_bits_and_changes_only_the_nine() {
        let original = 0o4755;
        let edited = with_bit(original, write_bit(0), false);
        assert_eq!(combine_mode(edited, original), 0o4555);
        assert!(has_special(original));
        assert!(!has_bit(edited, write_bit(0)));
    }
}
