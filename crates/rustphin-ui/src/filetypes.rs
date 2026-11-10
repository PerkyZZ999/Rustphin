//! File types and their icons, resolved through the platform layer.
//!
//! Loading the databases and looking up an icon or description read files, so
//! both run on a background thread; only the in-memory name match
//! ([`Database::mime_type`]) is called while rendering.

use std::{collections::HashMap, path::Path, sync::Arc};

use gpui_kit::SharedString;
use rustphin_platform::{apps::Associations, icons::IconTheme, mime::MimeDb, xdg};

/// Size at which application icons are looked up: a menu row's icon.
const APP_ICON_SIZE: u32 = crate::theme::size::ICON as u32;

/// Theme used when the desktop settings name none.
const DEFAULT_THEME: &str = "breeze-dark";

/// What the interface shows for one file type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// The icon file from the system theme, if the theme has one.
    pub icon: Option<Arc<Path>>,
    /// A readable name, such as "Plain text document".
    pub description: SharedString,
}

/// The system's file-type and icon databases.
pub struct Database {
    mimes: MimeDb,
    theme: IconTheme,
    apps: Associations,
    /// Each application's icon file, by desktop-file id, looked up when the
    /// databases load so a menu never reads the disk.
    app_icons: HashMap<String, Arc<Path>>,
}

impl Database {
    /// Reads the databases from disk. Blocking: call from a background thread.
    pub fn load() -> Self {
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
            theme,
            apps,
            app_icons,
        }
    }

    /// The icon of the application with desktop-file id `id`, if its theme
    /// or desktop file has one. Does not touch the disk.
    pub fn app_icon(&self, id: &str) -> Option<Arc<Path>> {
        self.app_icons.get(id).cloned()
    }

    /// The MIME type of an entry by its name. Does not touch the disk.
    pub fn mime_type(&self, name: &[u8], is_dir: bool) -> &str {
        self.mimes.mime_type(name, is_dir)
    }

    /// Applications registered for `mime`, default first. Does not touch the disk.
    pub fn applications(&self, mime: &str) -> Vec<rustphin_platform::apps::Application> {
        self.apps.for_type(mime)
    }

    /// Finds the icon and description of a type. Blocking: call from a
    /// background thread.
    pub fn resolve(&self, mime: &str, icon_size: u32) -> TypeInfo {
        TypeInfo {
            icon: self
                .theme
                .find(&self.mimes.icon_names(mime), icon_size)
                .map(Arc::from),
            description: self.mimes.description(mime).into(),
        }
    }
}
