//! Acting as administrator through KDE's kio-admin helper (W-098, W-114f).
//! The mode belongs to a window (D-028): its panes list and its operations
//! run through the helper's filesystem while it is on, and it ends with the
//! window. The helper's connection is shared by the process. Nothing asks
//! for a password until the user turns the mode on; the helper's own dialog
//! asks.

use std::{cell::RefCell, collections::HashSet, sync::Arc};

use rustphin_core::Fs;
use rustphin_platform::admin::{AdminError, AdminFs, AdminHelper};

use crate::{hub, post};

#[derive(Default)]
struct State {
    fs: Option<Arc<dyn Fs>>,
    helper: Option<Arc<AdminHelper>>,
    pending: bool,
    /// The windows acting as administrator.
    windows: HashSet<i32>,
    /// The number the next window gets.
    next_window: i32,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// A number for a new window, never used before in this run.
pub fn new_window() -> i32 {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.next_window += 1;
        state.next_window
    })
}

/// Whether `window` acts as administrator.
pub fn active(window: i32) -> bool {
    STATE.with(|state| {
        let state = state.borrow();
        state.fs.is_some() && state.windows.contains(&window)
    })
}

/// Whether access is being asked for.
pub fn pending() -> bool {
    STATE.with(|state| state.borrow().pending)
}

/// The helper's filesystem, while `window` acts as administrator.
pub fn filesystem(window: i32) -> Option<Arc<dyn Fs>> {
    active(window)
        .then(|| STATE.with(|state| state.borrow().fs.clone()))
        .flatten()
}

/// The helper, to describe entries while `window` acts as administrator.
pub fn helper(window: i32) -> Option<Arc<AdminHelper>> {
    active(window)
        .then(|| STATE.with(|state| state.borrow().helper.clone()))
        .flatten()
}

/// Ends the mode of `window`, which is closing, without a word.
pub fn leave(window: i32) {
    if STATE.with(|state| state.borrow_mut().windows.remove(&window)) {
        hub::admin_changed();
    }
}

/// Makes `window` act as administrator (asking for access) or stop.
pub fn toggle(window: i32) {
    if STATE.with(|state| state.borrow_mut().windows.remove(&window)) {
        hub::admin_changed();
        hub::say("No longer acting as administrator.".into(), false);
        return;
    }
    if pending() || !rustphin_platform::admin::installed() {
        return;
    }
    STATE.with(|state| state.borrow_mut().pending = true);
    let ticket = hub::say_until_withdrawn(
        "Asking for administrator access\u{2026} Enter your password in KDE's dialog.".into(),
    );
    let helper = STATE.with(|state| state.borrow().helper.clone());
    std::thread::spawn(move || {
        let granted = (|| {
            let helper = match helper {
                Some(helper) => helper,
                None => Arc::new(AdminHelper::connect()?),
            };
            helper.stat(std::path::Path::new("/"))?;
            Ok::<_, AdminError>(helper)
        })();
        post::on_qt(move || {
            STATE.with(|state| state.borrow_mut().pending = false);
            hub::withdraw(ticket);
            match granted {
                Ok(helper) => {
                    // One helper connection and one runner serve every
                    // window that acts as administrator.
                    let made = STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.windows.insert(window);
                        if state.fs.is_some() {
                            return None;
                        }
                        let fs: Arc<dyn Fs> = Arc::new(AdminFs::new(Arc::clone(&helper)));
                        state.fs = Some(Arc::clone(&fs));
                        state.helper = Some(helper);
                        Some(fs)
                    });
                    match made {
                        Some(fs) => hub::set_admin(fs),
                        None => hub::admin_changed(),
                    }
                }
                Err(error) => {
                    let words = match error {
                        AdminError::Denied => {
                            "Administrator access was not granted: the password was not given."
                                .to_owned()
                        }
                        AdminError::Unavailable => "KDE's administrator helper could not be reached. Install the kio-admin package, then try again.".to_owned(),
                        other => format!("Could not act as administrator: {other}."),
                    };
                    hub::say(words, true);
                    hub::admin_changed();
                }
            }
        });
    });
    hub::admin_changed();
}
