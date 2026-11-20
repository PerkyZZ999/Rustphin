//! The application chooser's object (W-169): Open with › Other
//! application…, and later Properties' Change…. The state is
//! `rustphin_view::app_chooser`; the file comes as its `file:` URI.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "chooser_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{path::PathBuf, pin::Pin};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use rustphin_core::display_name;
use rustphin_platform::apps::Application;
use rustphin_view::{app_chooser::AppChooser, menu::AppChoice};

use crate::{qml_path, types};

/// The Rust side of `ChooserModel`.
#[derive(Default)]
pub struct ChooserModelRust {
    view: QString,
    remembering: bool,
    open: bool,
    /// The file, its type, and the chooser's state.
    state: Option<(PathBuf, String, AppChooser)>,
}

impl qobject::ChooserModel {
    fn start(self: Pin<&mut Self>, uri: &QString, remember: bool) {
        let Some(path) = qml_path::from_qml(uri) else {
            return;
        };
        let thread = self.qt_thread();
        types::when_ready(move |types| {
            let name = path
                .file_name()
                .map(|name| display_name(name.as_encoded_bytes()).into_owned())
                .unwrap_or_default();
            let mime = path
                .file_name()
                .map(|name| types.mimes.mime_type(name.as_encoded_bytes(), false))
                .unwrap_or("application/octet-stream")
                .to_owned();
            let choice = |app: Application| AppChoice {
                icon: types.app_icon(&app.id),
                id: app.id,
                name: app.name,
            };
            let chooser = AppChooser::new(
                name,
                types.mimes.description(&mime),
                types.applications(&mime).into_iter().map(choice).collect(),
                types.all_applications().into_iter().map(choice).collect(),
                remember,
            );
            // The object may be gone if the dialog closed meanwhile.
            let _ = thread.queue(move |mut model| {
                model.as_mut().rust_mut().state = Some((path, mime, chooser));
                model.as_mut().show();
                model.set_open(true);
            });
        });
    }

    fn show(mut self: Pin<&mut Self>) {
        let Some((_, _, chooser)) = self.rust().state.as_ref() else {
            return;
        };
        let json = serde_json::to_string(&chooser.view()).unwrap_or_default();
        let remember = chooser.remember;
        self.as_mut().set_view(QString::from(&json));
        self.set_remembering(remember);
    }

    fn change(mut self: Pin<&mut Self>, change: impl FnOnce(&mut AppChooser)) {
        if let Some((_, _, chooser)) = self.as_mut().rust_mut().state.as_mut() {
            change(chooser);
        }
        self.show();
    }

    fn search(self: Pin<&mut Self>, text: &QString) {
        let text = text.to_string();
        self.change(|chooser| chooser.set_search(&text));
    }

    fn choose(self: Pin<&mut Self>, id: &QString) {
        let id = id.to_string();
        self.change(|chooser| chooser.choose(&id));
    }

    fn keep(self: Pin<&mut Self>, on: bool) {
        self.change(|chooser| chooser.remember = on);
    }

    fn accept(mut self: Pin<&mut Self>) -> bool {
        let Some((path, mime, (id, remember))) =
            self.rust()
                .state
                .as_ref()
                .and_then(|(path, mime, chooser)| {
                    Some((path.clone(), mime.clone(), chooser.accept()?))
                })
        else {
            return false;
        };
        if remember && let Some(types) = types::when_ready_now() {
            types.chose_default(&mime, &id);
        }
        std::thread::spawn(move || {
            if remember {
                let written = rustphin_platform::xdg::config_home()
                    .ok_or_else(|| std::io::Error::other("no configuration folder"))
                    .and_then(|home| rustphin_platform::apps::set_default(&home, &mime, &id));
                if let Err(error) = written {
                    crate::hub::say(
                        format!(
                            "The file was opened, but the choice could not be kept: {error}. Choose the application again in System Settings, Default Applications."
                        ),
                        true,
                    );
                }
            }
            if let Err(error) = rustphin_platform::apps::open_with(&id, &path) {
                crate::hub::say(error.to_string(), true);
            }
        });
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
        true
    }

    fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
    }
}
