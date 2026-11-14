//! Acting as administrator through KDE's kio-admin helper (W-098, W-114f):
//! one state for the process; panes list and operations run through the
//! helper's filesystem while it is on. Nothing asks for a password until
//! the user turns it on; the helper's own dialog asks.

use std::{cell::RefCell, sync::Arc};

use rustphin_core::Fs;
use rustphin_platform::admin::{AdminError, AdminFs, AdminHelper};

use crate::{hub, post};

#[derive(Default)]
struct State {
    fs: Option<Arc<dyn Fs>>,
    helper: Option<Arc<AdminHelper>>,
    pending: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Whether the window acts as administrator.
pub fn active() -> bool {
    STATE.with(|state| state.borrow().fs.is_some())
}

/// Whether access is being asked for.
pub fn pending() -> bool {
    STATE.with(|state| state.borrow().pending)
}

/// The helper's filesystem while acting as administrator.
pub fn filesystem() -> Option<Arc<dyn Fs>> {
    STATE.with(|state| state.borrow().fs.clone())
}

/// The helper, to describe entries as administrator.
pub fn helper() -> Option<Arc<AdminHelper>> {
    STATE.with(|state| {
        state
            .borrow()
            .helper
            .clone()
            .filter(|_| state.borrow().fs.is_some())
    })
}

/// Starts acting as administrator (asking for access) or stops.
pub fn toggle() {
    if STATE.with(|state| state.borrow_mut().fs.take().is_some()) {
        hub::set_admin(None);
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
                    let fs: Arc<dyn Fs> = Arc::new(AdminFs::new(Arc::clone(&helper)));
                    STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.fs = Some(Arc::clone(&fs));
                        state.helper = Some(helper);
                    });
                    hub::set_admin(Some(fs));
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
                    hub::set_admin(None);
                }
            }
        });
    });
    hub::set_admin_pending();
}
