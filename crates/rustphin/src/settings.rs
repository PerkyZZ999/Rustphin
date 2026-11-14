//! The Settings dialog's object (W-106, W-114f): the pages come from
//! `rustphin_view::settings_form` as JSON; each change goes through the
//! shared store and every window takes it up.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "settings_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use rustphin_platform::{apps::Associations, kdeglobals, terminal, xdg};
use rustphin_view::{
    location::Location,
    settings_form::{self, Applied, Facts, Page},
    toolbar_items::available,
};

use crate::{preferences, prefs};

/// The desktop entry Rustphin installs.
const DESKTOP_ID: &str = "rustphin.desktop";
/// The type the desktop's file manager opens.
const FOLDER_TYPE: &str = "inode/directory";
/// Rustphin's licence, shown on the About page.
const LICENCE: &str = include_str!("../../../LICENSE");

/// What the desktop says about terminals, folders and clicks.
fn read_facts() -> Facts {
    let config_home = xdg::config_home();
    let kde = kdeglobals::read(config_home.as_deref());
    let apps = Associations::load(&xdg::config_dirs(), &xdg::data_dirs());
    let default = apps.default_for(FOLDER_TYPE);
    Facts {
        kde_terminal: terminal::configured(config_home.as_deref()),
        file_manager: default.map(|app| app.name.clone()),
        is_default: default.is_some_and(|app| app.id == DESKTOP_ID),
        installed: apps.get(DESKTOP_ID).is_some(),
        kde_single_click: kde.is_some_and(|text| kdeglobals::single_click(&text)),
        read: true,
        memory: rustphin_platform::process::own_memory(),
        ..Facts::default()
    }
}

/// The Rust side of `SettingsModel`.
#[derive(Default)]
pub struct SettingsModelRust {
    page: i32,
    pages: QString,
    groups: QString,
    toolbar: QString,
    about: QString,
    revision: i32,
    facts: Facts,
}

fn text(words: &str) -> QString {
    QString::from(words)
}

impl qobject::SettingsModel {
    fn start(mut self: Pin<&mut Self>, shown: &QString) {
        self.as_mut().rust_mut().facts.shown_folder =
            crate::qml_path::from_qml(shown).map(|shown| Location::Folder(shown).to_saved());
        let pages: Vec<serde_json::Value> = Page::ALL
            .iter()
            .map(|page| {
                serde_json::json!({
                    "title": page.title(),
                    "description": page.description(),
                    "icon": page.icon(),
                    "resettable": page.resettable(),
                })
            })
            .collect();
        self.as_mut()
            .set_pages(text(&serde_json::Value::Array(pages).to_string()));
        let about = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "settings": prefs::with(|store| store.saving()),
        });
        self.as_mut().set_about(text(&about.to_string()));
        self.as_mut().refresh();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let facts = read_facts();
            let _ = thread.queue(move |mut model| {
                let shown = model.rust().facts.shown_folder.clone();
                model.as_mut().rust_mut().facts = Facts {
                    shown_folder: shown,
                    ..facts
                };
                model.refresh();
            });
        });
    }

    /// Builds the page shown and the toolbar's list again.
    fn refresh(mut self: Pin<&mut Self>) {
        let page = Page::from_number(*self.page()).unwrap_or(Page::General);
        let settings = prefs::settings();
        let groups = settings_form::page(page, &settings, &self.rust().facts);
        self.as_mut()
            .set_groups(text(&serde_json::to_string(&groups).unwrap_or_default()));
        let toolbar = settings.toolbar();
        let items: Vec<serde_json::Value> = toolbar
            .items
            .iter()
            .map(|(item, words)| {
                serde_json::json!({
                    "name": item.name(),
                    "icon": item.icon().lucide_name(),
                    "fixed": item.is_fixed(),
                    "words": words,
                    "canShowWords": item.can_show_words(),
                })
            })
            .collect();
        self.as_mut()
            .set_toolbar(text(&serde_json::Value::Array(items).to_string()));
        let next = self.revision().wrapping_add(1);
        self.as_mut().set_revision(next);
    }

    fn choose_page(mut self: Pin<&mut Self>, page: i32) {
        if Page::from_number(page).is_some() {
            self.as_mut().set_page(page);
            self.refresh();
        }
    }

    fn set(mut self: Pin<&mut Self>, key: &QString, value: &QString) {
        let (key, value) = (key.to_string(), value.to_string());
        match key.as_str() {
            "terminal-other" => {
                self.as_mut().rust_mut().facts.editing_terminal = true;
                return self.refresh();
            }
            "terminal-kde" => self.as_mut().rust_mut().facts.editing_terminal = false,
            "make-default" => return self.make_default(),
            _ => {}
        }
        let facts = self.rust().facts.clone();
        let mut applied = Applied::Nothing;
        prefs::with(|store| {
            let change = |settings: &mut rustphin_view::settings::Settings| {
                applied = settings_form::apply(settings, &key, &value, &facts);
            };
            if matches!(
                key.as_str(),
                "view-mode" | "zoom" | "sort" | "direction" | "folders-first" | "hidden"
            ) {
                store.update_default_view(change);
            } else {
                store.update(change);
            }
        });
        self.settled(applied);
    }

    /// Tells the windows, and the login entry when it changed.
    fn settled(self: Pin<&mut Self>, applied: Applied) {
        if let Applied::Login(on) = applied {
            std::thread::spawn(move || {
                if let Err(error) = rustphin_platform::autostart::set_for_this_program(on) {
                    crate::hub::say(
                        format!(
                            "Could not change starting at login: {error}. Check ~/.config/autostart."
                        ),
                        true,
                    );
                }
            });
        }
        if applied != Applied::Nothing {
            preferences::changed();
        }
        self.refresh();
    }

    fn make_default(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().facts.make_default = Some(String::new());
        self.as_mut().refresh();
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = xdg::config_home()
                .ok_or_else(|| "There is no configuration folder.".to_owned())
                .and_then(|home| {
                    rustphin_platform::apps::set_default(&home, FOLDER_TYPE, DESKTOP_ID)
                        .map_err(|error| format!("Could not save the choice: {error}."))
                })
                .map(|()| read_facts());
            let _ = thread.queue(move |mut model| {
                let shown = model.rust().facts.shown_folder.clone();
                match result {
                    Ok(facts) => {
                        let failed = (!facts.is_default).then(|| {
                            format!(
                                "Saved, but {} still opens folders. Choose Rustphin in System Settings, Default Applications, File manager.",
                                facts.file_manager.as_deref().unwrap_or("another application")
                            )
                        });
                        model.as_mut().rust_mut().facts = Facts {
                            shown_folder: shown,
                            make_default: failed,
                            ..facts
                        };
                    }
                    Err(reason) => model.as_mut().rust_mut().facts.make_default = Some(reason),
                }
                model.refresh();
            });
        });
    }

    fn reset(self: Pin<&mut Self>) {
        let page = Page::from_number(*self.page()).unwrap_or(Page::General);
        let mut applied = Applied::Nothing;
        prefs::with(|store| {
            let change = |settings: &mut rustphin_view::settings::Settings| {
                applied = settings_form::reset(settings, page);
            };
            if page == Page::Views {
                store.update_default_view(change);
            } else {
                store.update(change);
            }
        });
        self.settled(applied);
    }

    fn shortcuts(&self, search: &QString) -> QString {
        let groups: Vec<serde_json::Value> = settings_form::shortcut_groups(&search.to_string())
            .into_iter()
            .map(|(category, list)| serde_json::json!({ "category": category, "shortcuts": list }))
            .collect();
        text(&serde_json::Value::Array(groups).to_string())
    }

    fn available(&self, search: &QString) -> QString {
        let toolbar = prefs::settings().toolbar();
        let items: Vec<serde_json::Value> = available(&toolbar, &search.to_string())
            .iter()
            .map(|item| serde_json::json!({ "name": item.name(), "icon": item.icon().lucide_name() }))
            .collect();
        text(&serde_json::Value::Array(items).to_string())
    }

    /// Changes the toolbar and saves it.
    fn change_toolbar(
        self: Pin<&mut Self>,
        edit: impl FnOnce(&mut rustphin_view::toolbar_items::Toolbar),
    ) {
        let changed = prefs::update(|settings| {
            let mut toolbar = settings.toolbar();
            edit(&mut toolbar);
            settings.set_toolbar(&toolbar);
        });
        self.settled(if changed.any {
            Applied::Changed
        } else {
            Applied::Nothing
        });
    }

    fn add_item(self: Pin<&mut Self>, search: &QString, index: i32, after: i32) -> i32 {
        let toolbar = prefs::settings().toolbar();
        let Some(item) = usize::try_from(index).ok().and_then(|index| {
            available(&toolbar, &search.to_string())
                .into_iter()
                .nth(index)
        }) else {
            return -1;
        };
        let mut placed = None;
        self.change_toolbar(|toolbar| placed = toolbar.add(item, usize::try_from(after).ok()));
        placed.and_then(|at| i32::try_from(at).ok()).unwrap_or(-1)
    }

    fn remove_item(self: Pin<&mut Self>, index: i32) {
        if let Ok(index) = usize::try_from(index) {
            self.change_toolbar(|toolbar| {
                toolbar.remove(index);
            });
        }
    }

    fn shift_item(self: Pin<&mut Self>, index: i32, delta: i32) -> i32 {
        let Ok(index) = usize::try_from(index) else {
            return -1;
        };
        let mut to = index;
        self.change_toolbar(|toolbar| to = toolbar.shift(index, delta as isize));
        i32::try_from(to).unwrap_or(-1)
    }

    fn set_words(self: Pin<&mut Self>, index: i32, words: bool) {
        if let Ok(index) = usize::try_from(index) {
            self.change_toolbar(|toolbar| toolbar.set_words(index, words));
        }
    }

    fn licence(&self) -> QString {
        text(LICENCE)
    }

    fn open_folder(&self) {
        let folder = prefs::with(|store| {
            store
                .path()
                .and_then(|path| path.parent().map(ToOwned::to_owned))
        });
        if let Some(folder) = folder {
            std::thread::spawn(move || {
                let _ = std::fs::create_dir_all(&folder);
                if let Err(error) = rustphin_platform::apps::open_with_default(&folder) {
                    crate::hub::say(error.to_string(), true);
                }
            });
        }
    }
}
