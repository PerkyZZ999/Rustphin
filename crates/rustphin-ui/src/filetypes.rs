//! File types and their icons, resolved through the platform layer.
//!
//! Loading the databases and looking up an icon or description read files, so
//! both run on a background thread; only the in-memory name match
//! ([`Database::mime_type`]) is called while rendering.

use std::{path::Path, sync::Arc};

use gpui_kit::SharedString;
use rustphin_platform::{apps::Associations, icons::IconTheme, mime::MimeDb, xdg};

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
}

impl Database {
    /// Reads the databases from disk. Blocking: call from a background thread.
    pub fn load() -> Self {
        let data_dirs = xdg::data_dirs();
        let theme = xdg::config_home()
            .and_then(|config| rustphin_platform::icons::system_theme_name(&config))
            .unwrap_or_else(|| DEFAULT_THEME.to_owned());
        Self {
            mimes: MimeDb::load(&data_dirs),
            theme: IconTheme::load(&theme, &data_dirs),
            apps: Associations::load(&xdg::config_dirs(), &data_dirs),
        }
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
