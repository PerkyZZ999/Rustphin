//! Acting as administrator through KDE's kio-admin helper (W-098, W-114f).

use std::cell::Cell;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// Whether the window acts as administrator.
pub fn active() -> bool {
    ACTIVE.with(Cell::get)
}

/// Starts or stops acting as administrator.
#[expect(dead_code, reason = "administrator mode arrives with W-114f")]
pub fn set_active(on: bool) {
    ACTIVE.with(|active| active.set(on));
}
