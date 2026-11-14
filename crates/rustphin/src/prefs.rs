//! The settings store every window shares (D-029), on the Qt thread: read
//! the first time something asks, written in the background on each change.

use std::cell::RefCell;

use rustphin_view::preferences::{Changed, Loaded, Store};
use rustphin_view::settings::Settings;

thread_local! {
    static STORE: RefCell<Option<Store>> = const { RefCell::new(None) };
}

/// Runs `look` on the store, reading it the first time.
pub fn with<R>(look: impl FnOnce(&mut Store) -> R) -> R {
    STORE.with(|store| {
        let mut store = store.borrow_mut();
        let store = store.get_or_insert_with(|| Store::new(Loaded::read()));
        look(store)
    })
}

/// The settings as they are now.
pub fn settings() -> Settings {
    with(|store| store.settings().clone())
}

/// Changes the settings and saves them.
pub fn update(change: impl FnOnce(&mut Settings)) -> Changed {
    with(|store| store.update(change))
}
