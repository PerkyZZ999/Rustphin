//! The shared settings as QML follows them (W-114e, W-114f): every window
//! holds a `Preferences` object whose revision changes with the settings.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "preferences_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{cell::RefCell, pin::Pin};

use cxx_qt::{CxxQtThread, CxxQtType, Threading};

thread_local! {
    static WATCHERS: RefCell<Vec<CxxQtThread<qobject::Preferences>>> = const { RefCell::new(Vec::new()) };
}

/// The Rust side of `Preferences`.
#[derive(Default)]
pub struct PreferencesRust {
    revision: i32,
    watching: bool,
}

/// Tells every window that the settings changed.
pub fn changed() {
    WATCHERS.with(|watchers| {
        watchers
            .borrow_mut()
            .retain(|thread| thread.queue(|object| object.bump()).is_ok());
    });
}

impl qobject::Preferences {
    fn watch(mut self: Pin<&mut Self>) {
        if self.rust().watching {
            return;
        }
        self.as_mut().rust_mut().watching = true;
        let thread = self.qt_thread();
        WATCHERS.with(|watchers| watchers.borrow_mut().push(thread));
    }

    fn bump(mut self: Pin<&mut Self>) {
        let next = self.revision().wrapping_add(1);
        self.as_mut().set_revision(next);
    }
}
