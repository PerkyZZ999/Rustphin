//! The desktop's file types, icon theme and applications, read once in
//! the background and shared by every pane (W-114e): which type a name
//! has, what describes it, which applications open it, with their icons.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

use rustphin_platform::{
    apps::{Application, Associations},
    icons::IconTheme,
    mime::MimeDb,
    xdg,
};

use crate::post;

/// Size of an application's icon in menus.
const APP_ICON_SIZE: u32 = rustphin_view::metrics::size::ICON as u32;
/// The icon theme when the desktop names none.
const DEFAULT_THEME: &str = "breeze-dark";

/// The shared database.
pub struct Types {
    /// File types by name and content.
    pub mimes: MimeDb,
    apps: Associations,
    app_icons: HashMap<String, Arc<Path>>,
    /// Defaults the user set during this run (W-169), type to desktop id:
    /// the desktop files are not read again for them.
    chosen_defaults: Mutex<HashMap<String, String>>,
}

impl Types {
    fn load() -> Self {
        let data_dirs = xdg::data_dirs();
        let theme = xdg::config_home()
            .and_then(|config| rustphin_platform::icons::system_theme_name(&config))
            .unwrap_or_else(|| DEFAULT_THEME.to_owned());
        let theme = IconTheme::load(&theme, &data_dirs);
        let apps = Associations::load(&xdg::config_dirs(), &data_dirs);
        let app_icons = apps
            .all()
            .filter_map(|app| {
                let icon = theme.find_app_icon(app.icon.as_deref()?, APP_ICON_SIZE)?;
                Some((app.id.clone(), Arc::from(icon)))
            })
            .collect();
        Self {
            mimes: MimeDb::load(&data_dirs),
            apps,
            app_icons,
            chosen_defaults: Mutex::default(),
        }
    }

    /// The applications registered for `mime`, the default first.
    pub fn applications(&self, mime: &str) -> Vec<Application> {
        let mut apps = self.apps.for_type(mime);
        let chosen = self
            .chosen_defaults
            .lock()
            .ok()
            .and_then(|chosen| chosen.get(mime).cloned());
        if let Some(chosen) = chosen {
            put_first(&mut apps, self.apps.get(&chosen), &chosen);
        }
        apps
    }

    /// Every installed application, in no particular order.
    pub fn all_applications(&self) -> Vec<Application> {
        self.apps.all().cloned().collect()
    }

    /// Notes that `id` is now the default for `mime` (written to the
    /// user's `mimeapps.list` by the caller).
    pub fn chose_default(&self, mime: &str, id: &str) {
        if let Ok(mut chosen) = self.chosen_defaults.lock() {
            chosen.insert(mime.to_owned(), id.to_owned());
        }
    }

    /// An application's icon file.
    pub fn app_icon(&self, id: &str) -> Option<Arc<Path>> {
        self.app_icons.get(id).cloned()
    }
}

/// Moves the application `id` to the front of `apps`, adding `installed`
/// when the type did not list it.
fn put_first(apps: &mut Vec<Application>, installed: Option<&Application>, id: &str) {
    let app = match apps.iter().position(|app| app.id == id) {
        Some(at) => apps.remove(at),
        None => match installed {
            Some(app) => app.clone(),
            None => return,
        },
    };
    apps.insert(0, app);
}

static TYPES: OnceLock<Arc<Types>> = OnceLock::new();

/// Someone waiting for the database.
type Waiter = Box<dyn FnOnce(Arc<Types>)>;

thread_local! {
    static WAITING: RefCell<Option<Vec<Waiter>>> = const { RefCell::new(None) };
}

/// Calls `ready` on the Qt thread with the database: at once when it is
/// loaded, else once it is. The first call starts loading it.
pub fn when_ready(ready: impl FnOnce(Arc<Types>) + 'static) {
    if let Some(types) = TYPES.get() {
        return ready(Arc::clone(types));
    }
    WAITING.with(|waiting| {
        let mut waiting = waiting.borrow_mut();
        let start = waiting.is_none();
        waiting.get_or_insert_with(Vec::new).push(Box::new(ready));
        if start {
            std::thread::spawn(|| {
                let types = Arc::new(Types::load());
                post::on_qt(move || {
                    let types = Arc::clone(TYPES.get_or_init(|| types));
                    let callbacks = WAITING.with(|waiting| waiting.borrow_mut().take());
                    for callback in callbacks.into_iter().flatten() {
                        callback(Arc::clone(&types));
                    }
                });
            });
        }
    });
}

/// The database if it is loaded already.
pub fn when_ready_now() -> Option<Arc<Types>> {
    TYPES.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> Application {
        Application {
            id: id.to_owned(),
            name: id.to_owned(),
            icon: None,
        }
    }

    /// W-169: an application made the default in the chooser leads Open
    /// with at once, listed for the type or not.
    #[test]
    fn a_default_chosen_now_leads_the_list() {
        let mut apps = vec![app("a"), app("b")];
        put_first(&mut apps, Some(&app("b")), "b");
        assert_eq!(apps, [app("b"), app("a")]);
        put_first(&mut apps, Some(&app("c")), "c");
        assert_eq!(apps, [app("c"), app("b"), app("a")]);
        put_first(&mut apps, None, "gone");
        assert_eq!(apps.len(), 3, "an uninstalled default is left out");
    }
}
