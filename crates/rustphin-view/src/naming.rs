//! Free names for things Rustphin creates.

/// `base`, or the first free `base n` (from 2) among names `is_taken`
/// reports, as Dolphin numbers new folders.
pub fn free_name(base: &str, is_taken: impl Fn(&str) -> bool) -> String {
    if !is_taken(base) {
        return base.to_owned();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !is_taken(name))
        .unwrap_or_else(|| base.to_owned())
}
