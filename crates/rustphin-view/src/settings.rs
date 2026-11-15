//! Rustphin's settings, stored as TOML (D-029): how it starts and works
//! with the desktop, and the view folders open in.
//!
//! Reading never fails: a missing, damaged or partly unknown file yields the
//! defaults for whatever could not be understood, and [`Settings::read`]
//! says when the file must be kept rather than replaced.

use rustphin_core::{SortKey, ViewOptions};
use serde::{Deserialize, Serialize};

use crate::{
    format::{Dates, Units},
    layout::{self, ViewMode},
    saved_file::{self, FileIssue},
};

/// The format version this build writes. Version 2 has seven zoom levels
/// where version 1 had five (W-107); version 3 adds `[general]` (W-106c).
const VERSION: u32 = 3;

/// Where each of version 1's five zoom levels (16, 20, 24, 32 and 48-pixel
/// icons) is among today's seven.
const ZOOM_FROM_VERSION_1: [usize; 5] = [0, 1, 2, 4, 6];

/// Everything Rustphin remembers between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    version: u32,
    general: General,
    appearance: Appearance,
    confirm: Confirm,
    places: Places,
    toolbar: ToolbarSaved,
    view: View,
}

/// The toolbar's items as ids, and those that show words (W-106f). Empty
/// is today's toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
struct ToolbarSaved {
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "lenient")]
    items: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", deserialize_with = "lenient")]
    words: Vec<String>,
}

/// The places panel (W-106e2). Groups live in their own file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Places {
    #[serde(deserialize_with = "lenient_true")]
    devices: bool,
    #[serde(deserialize_with = "lenient_true")]
    remote: bool,
    #[serde(deserialize_with = "lenient")]
    show_hidden: bool,
    /// The panel's width in pixels, within 200–360.
    #[serde(deserialize_with = "lenient_width")]
    width: u32,
    /// How each drive is drawn (W-159).
    #[serde(deserialize_with = "lenient")]
    drive_style: DriveStyle,
}

impl Default for Places {
    fn default() -> Self {
        Self {
            devices: true,
            remote: true,
            show_hidden: false,
            width: crate::metrics::size::SIDEBAR as u32,
            drive_style: DriveStyle::default(),
        }
    }
}

/// How the places panel draws each drive (W-159, the maintainer's three
/// mockups of 2026-11-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DriveStyle {
    /// Its name and what is free, a bar across the row, then the
    /// filesystem and size.
    #[default]
    Bars,
    /// One short line with a small bar and the percentage; the drive whose
    /// folder is shown opens its figures under it.
    Compact,
    /// A ring of the used space around its icon, its name and what is free.
    Rings,
}

impl DriveStyle {
    /// The name QML and the settings form use.
    pub fn id(self) -> &'static str {
        match self {
            DriveStyle::Bars => "bars",
            DriveStyle::Compact => "compact",
            DriveStyle::Rings => "rings",
        }
    }

    /// The style a name stands for; bars for any other.
    pub fn from_id(id: &str) -> Self {
        match id {
            "compact" => DriveStyle::Compact,
            "rings" => DriveStyle::Rings,
            _ => DriveStyle::Bars,
        }
    }
}

/// The Places page's choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacesChoice {
    /// The Devices section is shown.
    pub devices: bool,
    /// The Remote section is shown.
    pub remote: bool,
    /// Hidden places are shown, faded.
    pub show_hidden: bool,
    /// The panel's width in pixels.
    pub width: u32,
    /// How each drive is drawn.
    pub drive_style: DriveStyle,
}

/// A panel width within what the window allows, on the 4-pixel grid.
fn clamp_width(width: u32) -> u32 {
    let (min, max) = (
        crate::metrics::size::SIDEBAR_MIN as u32,
        crate::metrics::size::SIDEBAR_MAX as u32,
    );
    (width.clamp(min, max) / 4) * 4
}

fn lenient_width<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value
        .as_integer()
        .and_then(|width| u32::try_from(width).ok())
        .map_or(crate::metrics::size::SIDEBAR as u32, clamp_width))
}

/// What Rustphin asks before doing (W-106e). Deleting for good always asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Confirm {
    #[serde(deserialize_with = "lenient")]
    trash: bool,
    #[serde(deserialize_with = "lenient_true")]
    empty_trash: bool,
    #[serde(deserialize_with = "lenient")]
    programs: Programs,
    #[serde(deserialize_with = "lenient")]
    close_tabs: bool,
}

impl Default for Confirm {
    fn default() -> Self {
        Self {
            trash: false,
            empty_trash: true,
            programs: Programs::default(),
            close_tabs: false,
        }
    }
}

/// What activating a program file does, as KDE's own choice for
/// executable files. A shortcut has nothing to open: it asks unless Run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Programs {
    /// Ask each time.
    #[default]
    Ask,
    /// Hand it to the default application, as a document.
    Open,
    /// Run it.
    Run,
}

/// The Confirmations page's choices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confirmations {
    /// Ask before moving to the trash.
    pub trash: bool,
    /// Ask before emptying the trash.
    pub empty_trash: bool,
    /// What activating a program does.
    pub programs: Programs,
    /// Ask before closing a window that has several tabs.
    pub close_tabs: bool,
}

/// How the window looks (W-106d). The design stays dark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Appearance {
    #[serde(deserialize_with = "lenient")]
    look: Look,
    #[serde(deserialize_with = "lenient")]
    title_bar: TitleBar,
    #[serde(deserialize_with = "lenient")]
    text_size: TextSize,
    /// Hidden entries' opacity, in percent.
    #[serde(deserialize_with = "lenient_hidden")]
    hidden_opacity: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            look: Look::default(),
            title_bar: TitleBar::default(),
            text_size: TextSize::default(),
            hidden_opacity: HIDDEN_OPACITY_PERCENT,
        }
    }
}

/// Hidden entries' opacity by default, in percent.
pub const HIDDEN_OPACITY_PERCENT: u32 = 60;

/// Glass or solid surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Look {
    /// Translucent, blurred where the desktop offers blur.
    #[default]
    Glass,
    /// Solid everywhere.
    Opaque,
}

/// The theme a window is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Translucent surfaces over the blurred desktop; solid ones otherwise.
    pub glass: bool,
    /// Glass was wanted but the desktop offers no blur: the user is told
    /// why the window is solid (R-109).
    pub blur_missing: bool,
}

impl Look {
    /// The theme this look gives: glass only where it is wanted, not
    /// switched off for the run (`forced_opaque`, `RUSTPHIN_OPAQUE`) and
    /// the desktop offers blur. Missing blur is reported only when nothing
    /// else turned the glass off.
    pub fn theme(self, forced_opaque: bool, blur_offered: bool) -> Theme {
        let wanted = self == Self::Glass && !forced_opaque;
        Theme {
            glass: wanted && blur_offered,
            blur_missing: wanted && !blur_offered,
        }
    }
}

/// Who draws the title bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleBar {
    /// Rustphin's own (W-096).
    #[default]
    Rustphin,
    /// The desktop's.
    Kde,
}

/// The interface's text, one step down or up the type scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextSize {
    /// One step down: 11 px body text.
    Smaller,
    /// The design's sizes: 12 px body text.
    #[default]
    Normal,
    /// One step up: 14 px body text.
    Larger,
}

impl TextSize {
    /// What every interface text size is multiplied by.
    pub fn scale(self) -> f32 {
        match self {
            Self::Smaller => 11. / 12.,
            Self::Normal => 1.,
            Self::Larger => 14. / 12.,
        }
    }
}

/// How Rustphin starts and works with the rest of the desktop.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
struct General {
    #[serde(deserialize_with = "lenient")]
    start: Start,
    /// The folder for `Start::Folder`, saved as a session location.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    start_folder: Option<String>,
    #[serde(deserialize_with = "lenient")]
    shown_folders: ShownIn,
    /// A terminal command chosen here instead of KDE's.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    terminal: Option<String>,
    /// How files open; KDE's setting when absent.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    click: Option<Click>,
    /// Keep running after the last window closes (W-110, D-031); on when
    /// absent.
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "lenient")]
    keep_running: Option<bool>,
    /// Start without a window when the user logs in (W-110c).
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        deserialize_with = "lenient"
    )]
    start_at_login: bool,
}

/// What Rustphin shows when it starts without folders to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Start {
    /// The tabs open when it last closed.
    #[default]
    Restore,
    /// The home folder.
    Home,
    /// The chosen folder.
    Folder,
}

/// Where folders other applications ask for are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShownIn {
    /// A new tab of the open window.
    #[default]
    Tab,
    /// A window of their own.
    Window,
}

/// How many clicks open a file or folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Click {
    /// A double click; one click selects.
    Double,
    /// A single click; Ctrl or Shift and a click select.
    Single,
}

/// Like [`lenient`] for a choice that is on unless the file says otherwise.
fn lenient_true<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value.as_bool().unwrap_or(true))
}

/// Like [`lenient`] for the hidden entries' opacity, kept readable.
fn lenient_hidden<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value
        .as_integer()
        .and_then(|percent| u32::try_from(percent).ok())
        .map_or(HIDDEN_OPACITY_PERCENT, clamp_hidden))
}

/// A hidden opacity within what the contrast tests allow.
fn clamp_hidden(percent: u32) -> u32 {
    percent.clamp(crate::metrics::HIDDEN_OPACITY_MIN_PERCENT, 100)
}

/// Like [`lenient`] for the preview limit, which is at least 1 MB.
fn lenient_limit<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = toml::Value::deserialize(deserializer)?;
    Ok(value
        .as_integer()
        .and_then(|mb| u32::try_from(mb).ok())
        .filter(|mb| *mb >= 1)
        .unwrap_or(THUMBNAIL_LIMIT_MB))
}

/// Reads a value, or its default when it is not one this build knows, so
/// one strange value does not cost the rest of the file.
fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let value = toml::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct View {
    mode: Mode,
    zoom: usize,
    sort: Sort,
    descending: bool,
    show_hidden: bool,
    folders_first: bool,
    /// Each folder keeps the view it was last shown in (W-106c2).
    #[serde(deserialize_with = "lenient")]
    per_folder: bool,
    /// The details view's optional columns.
    #[serde(deserialize_with = "lenient_true")]
    column_size: bool,
    #[serde(deserialize_with = "lenient_true")]
    column_modified: bool,
    #[serde(deserialize_with = "lenient_true")]
    column_type: bool,
    #[serde(deserialize_with = "lenient")]
    dates: Dates,
    #[serde(deserialize_with = "lenient")]
    sizes: Units,
    /// Pictures show their thumbnails.
    #[serde(deserialize_with = "lenient_true")]
    thumbnails: bool,
    /// Pictures larger than this many megabytes keep their type icon.
    #[serde(deserialize_with = "lenient_limit")]
    thumbnail_limit_mb: u32,
}

/// The largest picture previewed by default, in megabytes.
pub const THUMBNAIL_LIMIT_MB: u32 = 50;

/// The details columns that can be hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailColumns {
    /// Size.
    pub size: bool,
    /// Modified.
    pub modified: bool,
    /// Type.
    pub kind: bool,
}

/// Picture previews: whether they are shown and up to which file size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Previews {
    /// Thumbnails are shown at all.
    pub shown: bool,
    /// Largest file previewed, in megabytes.
    pub limit_mb: u32,
}

impl Previews {
    /// The largest file previewed, in bytes.
    pub fn limit_bytes(self) -> u64 {
        u64::from(self.limit_mb) * 1_000_000
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Icons,
    Compact,
    #[default]
    Details,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Sort {
    #[default]
    Name,
    Size,
    Modified,
    Type,
}

impl Default for View {
    fn default() -> Self {
        let options = ViewOptions::default();
        Self {
            mode: Mode::default(),
            zoom: layout::DEFAULT_ZOOM,
            sort: Sort::default(),
            descending: options.descending,
            show_hidden: options.show_hidden,
            folders_first: options.folders_first,
            per_folder: false,
            column_size: true,
            column_modified: true,
            column_type: true,
            dates: Dates::default(),
            sizes: Units::default(),
            thumbnails: true,
            thumbnail_limit_mb: THUMBNAIL_LIMIT_MB,
        }
    }
}

impl Settings {
    /// Captures the current view state.
    pub fn new(mode: ViewMode, zoom: usize, options: ViewOptions) -> Self {
        Self {
            version: VERSION,
            general: General::default(),
            appearance: Appearance::default(),
            confirm: Confirm::default(),
            places: Places::default(),
            toolbar: ToolbarSaved::default(),
            view: View {
                mode: match mode {
                    ViewMode::Icons => Mode::Icons,
                    ViewMode::Compact => Mode::Compact,
                    ViewMode::Details => Mode::Details,
                },
                zoom,
                sort: match options.sort {
                    SortKey::Name => Sort::Name,
                    SortKey::Size => Sort::Size,
                    SortKey::Modified => Sort::Modified,
                    SortKey::Type => Sort::Type,
                },
                descending: options.descending,
                show_hidden: options.show_hidden,
                folders_first: options.folders_first,
                ..View::default()
            },
        }
    }

    /// Replaces the default view with `mode`, `zoom` and `options`, keeping
    /// the Views page's other choices.
    pub fn set_view(&mut self, mode: ViewMode, zoom: usize, options: ViewOptions) {
        let current = Self::new(mode, zoom, options).view;
        self.view = View {
            per_folder: self.view.per_folder,
            column_size: self.view.column_size,
            column_modified: self.view.column_modified,
            column_type: self.view.column_type,
            dates: self.view.dates,
            sizes: self.view.sizes,
            thumbnails: self.view.thumbnails,
            thumbnail_limit_mb: self.view.thumbnail_limit_mb,
            ..current
        };
        self.version = VERSION;
    }

    /// Whether each folder keeps its own view.
    pub fn per_folder(&self) -> bool {
        self.view.per_folder
    }

    /// Chooses whether each folder keeps its own view.
    pub fn set_per_folder(&mut self, on: bool) {
        self.view.per_folder = on;
        self.version = VERSION;
    }

    /// The details columns chosen.
    pub fn detail_columns(&self) -> DetailColumns {
        DetailColumns {
            size: self.view.column_size,
            modified: self.view.column_modified,
            kind: self.view.column_type,
        }
    }

    /// Chooses the details columns.
    pub fn set_detail_columns(&mut self, columns: DetailColumns) {
        self.view.column_size = columns.size;
        self.view.column_modified = columns.modified;
        self.view.column_type = columns.kind;
        self.version = VERSION;
    }

    /// How dates and sizes are written.
    pub fn formats(&self) -> (Dates, Units) {
        (self.view.dates, self.view.sizes)
    }

    /// Chooses how dates and sizes are written.
    pub fn set_formats(&mut self, dates: Dates, sizes: Units) {
        self.view.dates = dates;
        self.view.sizes = sizes;
        self.version = VERSION;
    }

    /// Picture previews.
    pub fn previews(&self) -> Previews {
        Previews {
            shown: self.view.thumbnails,
            limit_mb: self.view.thumbnail_limit_mb,
        }
    }

    /// Chooses picture previews.
    pub fn set_previews(&mut self, previews: Previews) {
        self.view.thumbnails = previews.shown;
        self.view.thumbnail_limit_mb = previews.limit_mb.max(1);
        self.version = VERSION;
    }

    /// Puts the Views page back as it was before anything was chosen.
    pub fn reset_views(&mut self) {
        self.view = View::default();
        self.version = VERSION;
    }

    /// What Rustphin shows when it starts without folders to open, and the
    /// chosen folder as a session location.
    pub fn start(&self) -> (Start, Option<&str>) {
        (self.general.start, self.general.start_folder.as_deref())
    }

    /// Sets what Rustphin shows at start; `folder` is kept when `None`.
    pub fn set_start(&mut self, start: Start, folder: Option<String>) {
        self.general.start = start;
        if folder.is_some() {
            self.general.start_folder = folder;
        }
        self.version = VERSION;
    }

    /// Where folders other applications ask for are shown.
    pub fn shown_folders(&self) -> ShownIn {
        self.general.shown_folders
    }

    /// Sets where folders other applications ask for are shown.
    pub fn set_shown_folders(&mut self, shown: ShownIn) {
        self.general.shown_folders = shown;
        self.version = VERSION;
    }

    /// The terminal command chosen here, if any.
    pub fn terminal(&self) -> Option<&str> {
        self.general.terminal.as_deref()
    }

    /// Chooses a terminal command; empty words go back to KDE's terminal.
    pub fn set_terminal(&mut self, command: &str) {
        let command = command.trim();
        self.general.terminal = (!command.is_empty()).then(|| command.to_owned());
        self.version = VERSION;
    }

    /// Whether Rustphin keeps running after its last window closes.
    pub fn keep_running(&self) -> bool {
        self.general.keep_running.unwrap_or(true)
    }

    /// Chooses whether Rustphin keeps running without windows.
    pub fn set_keep_running(&mut self, keep: bool) {
        self.general.keep_running = Some(keep);
        self.version = VERSION;
    }

    /// Whether Rustphin starts without a window at login. Only meaningful
    /// while it keeps running.
    pub fn start_at_login(&self) -> bool {
        self.general.start_at_login && self.keep_running()
    }

    /// Chooses whether Rustphin starts at login.
    pub fn set_start_at_login(&mut self, start: bool) {
        self.general.start_at_login = start;
        self.version = VERSION;
    }

    /// How files open, when chosen here; `None` follows KDE.
    pub fn click(&self) -> Option<Click> {
        self.general.click
    }

    /// Chooses how files open; `None` follows KDE.
    pub fn set_click(&mut self, click: Option<Click>) {
        self.general.click = click;
        self.version = VERSION;
    }

    /// Glass or opaque.
    pub fn look(&self) -> Look {
        self.appearance.look
    }

    /// Who draws the title bar.
    pub fn title_bar(&self) -> TitleBar {
        self.appearance.title_bar
    }

    /// The interface's text size.
    pub fn text_size(&self) -> TextSize {
        self.appearance.text_size
    }

    /// Hidden entries' opacity, in percent, always readable.
    pub fn hidden_opacity(&self) -> u32 {
        clamp_hidden(self.appearance.hidden_opacity)
    }

    /// Chooses the Appearance page's values; the opacity is kept readable.
    pub fn set_appearance(
        &mut self,
        look: Look,
        title_bar: TitleBar,
        text_size: TextSize,
        hidden_opacity: u32,
    ) {
        self.appearance = Appearance {
            look,
            title_bar,
            text_size,
            hidden_opacity: clamp_hidden(hidden_opacity),
        };
        self.version = VERSION;
    }

    /// The toolbar's items.
    pub fn toolbar(&self) -> crate::toolbar_items::Toolbar {
        crate::toolbar_items::Toolbar::from_ids(&self.toolbar.items, &self.toolbar.words)
    }

    /// Chooses the toolbar's items; today's toolbar is saved as nothing.
    pub fn set_toolbar(&mut self, toolbar: &crate::toolbar_items::Toolbar) {
        self.toolbar = if *toolbar == crate::toolbar_items::Toolbar::default() {
            ToolbarSaved::default()
        } else {
            let (items, words) = toolbar.to_ids();
            ToolbarSaved { items, words }
        };
        self.version = VERSION;
    }

    /// Puts the Toolbar page back as it was before anything was chosen.
    pub fn reset_toolbar(&mut self) {
        self.toolbar = ToolbarSaved::default();
        self.version = VERSION;
    }

    /// The places panel's choices.
    pub fn places(&self) -> PlacesChoice {
        PlacesChoice {
            devices: self.places.devices,
            remote: self.places.remote,
            show_hidden: self.places.show_hidden,
            width: clamp_width(self.places.width),
            drive_style: self.places.drive_style,
        }
    }

    /// Chooses the places panel's look; the width is kept in bounds.
    pub fn set_places(&mut self, choice: PlacesChoice) {
        self.places = Places {
            devices: choice.devices,
            remote: choice.remote,
            show_hidden: choice.show_hidden,
            width: clamp_width(choice.width),
            drive_style: choice.drive_style,
        };
        self.version = VERSION;
    }

    /// Puts the Places page back as it was before anything was chosen.
    pub fn reset_places(&mut self) {
        self.places = Places::default();
        self.version = VERSION;
    }

    /// What Rustphin asks before doing.
    pub fn confirmations(&self) -> Confirmations {
        Confirmations {
            trash: self.confirm.trash,
            empty_trash: self.confirm.empty_trash,
            programs: self.confirm.programs,
            close_tabs: self.confirm.close_tabs,
        }
    }

    /// Chooses what Rustphin asks before doing.
    pub fn set_confirmations(&mut self, confirmations: Confirmations) {
        self.confirm = Confirm {
            trash: confirmations.trash,
            empty_trash: confirmations.empty_trash,
            programs: confirmations.programs,
            close_tabs: confirmations.close_tabs,
        };
        self.version = VERSION;
    }

    /// Puts the Confirmations page back as it was before anything was chosen.
    pub fn reset_confirmations(&mut self) {
        self.confirm = Confirm::default();
        self.version = VERSION;
    }

    /// Puts the Appearance page back as it was before anything was chosen.
    pub fn reset_appearance(&mut self) {
        self.appearance = Appearance::default();
        self.version = VERSION;
    }

    /// Puts the General page back as it was before anything was chosen.
    pub fn reset_general(&mut self) {
        self.general = General::default();
        self.version = VERSION;
    }

    /// Reads settings from TOML text, falling back to the defaults for the
    /// whole file if it cannot be parsed.
    pub fn parse(text: &str) -> Self {
        toml::from_str(text).unwrap_or_else(|error| {
            tracing::warn!(%error, "ignoring unreadable settings");
            Self::default()
        })
    }

    /// Reads the saved file, `None` when there is none, as far as it can
    /// be: the defaults stand in for a file that cannot be used, and the
    /// issue says why the file must be kept as it is (W-140).
    pub fn read(saved: std::io::Result<Option<String>>) -> (Self, Option<FileIssue>) {
        let decoded = saved_file::decode(saved, VERSION);
        (decoded.value.unwrap_or_default(), decoded.issue)
    }

    /// The settings as TOML text.
    pub fn to_toml(&self) -> String {
        // Serialising plain booleans, integers and unit enums cannot fail; an
        // empty file would simply read back as the defaults.
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Whether the file was written by a later version of Rustphin. Such a
    /// file is read as far as it is understood but must not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }

    /// The remembered view mode.
    pub fn mode(&self) -> ViewMode {
        match self.view.mode {
            Mode::Icons => ViewMode::Icons,
            Mode::Compact => ViewMode::Compact,
            Mode::Details => ViewMode::Details,
        }
    }

    /// The remembered zoom level, kept within the valid levels. A file
    /// from before the seven levels keeps its icon size.
    pub fn zoom(&self) -> usize {
        if self.version < 2 {
            ZOOM_FROM_VERSION_1[self.view.zoom.min(ZOOM_FROM_VERSION_1.len() - 1)]
        } else {
            self.view.zoom.min(layout::ZOOM_LEVELS - 1)
        }
    }

    /// The remembered ordering and filtering, with `details` set for the mode.
    pub fn options(&self) -> ViewOptions {
        ViewOptions {
            show_hidden: self.view.show_hidden,
            folders_first: self.view.folders_first,
            sort: match self.view.sort {
                Sort::Name => SortKey::Name,
                Sort::Size => SortKey::Size,
                Sort::Modified => SortKey::Modified,
                Sort::Type => SortKey::Type,
            },
            descending: self.view.descending,
            details: self.mode() == ViewMode::Details,
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn glass_needs_the_look_the_run_and_the_desktop_to_agree() {
        let glass = Theme {
            glass: true,
            blur_missing: false,
        };
        let solid = Theme {
            glass: false,
            blur_missing: false,
        };
        let told = Theme {
            glass: false,
            blur_missing: true,
        };
        assert_eq!(Look::Glass.theme(false, true), glass);
        assert_eq!(Look::Glass.theme(false, false), told);
        // Switched off, by the setting or for the run: nothing to explain.
        assert_eq!(Look::Glass.theme(true, true), solid);
        assert_eq!(Look::Glass.theme(true, false), solid);
        assert_eq!(Look::Opaque.theme(false, true), solid);
        assert_eq!(Look::Opaque.theme(false, false), solid);
    }

    #[test]
    fn rustphin_keeps_running_by_default_and_starts_at_login_only_when_asked() {
        let fresh = Settings::default();
        assert!(fresh.keep_running());
        assert!(!fresh.start_at_login());
        let mut chosen = Settings::default();
        chosen.set_start_at_login(true);
        let read = Settings::parse(&chosen.to_toml());
        assert!(read.keep_running() && read.start_at_login());
        // Login start means nothing once Rustphin no longer keeps running.
        chosen.set_keep_running(false);
        let read = Settings::parse(&chosen.to_toml());
        assert!(!read.keep_running() && !read.start_at_login());
        // A damaged value keeps the default.
        let damaged = Settings::parse("version = 3\n[general]\nkeep_running = \"yes\"\n");
        assert!(damaged.keep_running());
    }
    use super::*;

    fn sample() -> Settings {
        Settings::new(
            ViewMode::Icons,
            3,
            ViewOptions {
                show_hidden: true,
                folders_first: false,
                sort: SortKey::Modified,
                descending: true,
                details: false,
            },
        )
    }

    #[test]
    fn settings_survive_being_written_and_read_back() {
        let saved = sample();
        let loaded = Settings::parse(&saved.to_toml());
        assert_eq!(loaded, saved);
        assert_eq!(loaded.mode(), ViewMode::Icons);
        assert_eq!(loaded.zoom(), 3);
        assert_eq!(loaded.options().sort, SortKey::Modified);
        assert!(loaded.options().descending && loaded.options().show_hidden);
        assert!(!loaded.options().folders_first);
    }

    #[test]
    fn the_file_uses_plain_readable_names() {
        let text = sample().to_toml();
        for line in [
            "version = 3",
            "[view]",
            "mode = \"icons\"",
            "sort = \"modified\"",
            "zoom = 3",
        ] {
            assert!(text.contains(line), "missing {line:?} in:\n{text}");
        }
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let settings = Settings::parse("");
        assert_eq!(settings.mode(), ViewMode::Details);
        assert_eq!(settings.zoom(), layout::DEFAULT_ZOOM);
        assert_eq!(
            settings.options(),
            ViewOptions {
                details: true,
                ..ViewOptions::default()
            }
        );
    }

    #[test]
    fn missing_keys_keep_their_defaults() {
        let settings = Settings::parse("[view]\nmode = \"compact\"\n");
        assert_eq!(settings.mode(), ViewMode::Compact);
        assert_eq!(settings.zoom(), layout::DEFAULT_ZOOM);
        assert!(settings.options().folders_first);
        assert!(!settings.options().details);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let settings = Settings::parse("future = true\n[view]\nzoom = 2\ncolour = \"teal\"\n");
        assert_eq!(settings.zoom(), 2);
    }

    #[test]
    fn damaged_files_and_wrong_values_fall_back_to_the_defaults() {
        for text in [
            "this is not toml [",
            "[view]\nmode = \"sideways\"\n",
            "[view]\nzoom = \"big\"\n",
            "[view]\nzoom = -1\n",
        ] {
            assert_eq!(Settings::parse(text), Settings::default(), "for {text:?}");
        }
    }

    #[test]
    fn setting_the_view_changes_only_the_view_and_writes_todays_version() {
        let mut settings = Settings::parse("version = 1\n[view]\nzoom = 3\n");
        settings.set_view(
            ViewMode::Compact,
            5,
            ViewOptions {
                show_hidden: true,
                ..ViewOptions::default()
            },
        );
        assert_eq!(settings.mode(), ViewMode::Compact);
        assert_eq!(settings.zoom(), 5);
        assert!(settings.options().show_hidden);
        assert!(settings.to_toml().contains("version = 3"));
    }

    #[test]
    fn a_zoom_saved_with_five_levels_keeps_its_icon_size() {
        let icon = |text: &str| ViewMode::Details.layout(Settings::parse(text).zoom()).icon;
        // Version 1 had 16, 20, 24, 32 and 48-pixel levels.
        let old: Vec<f32> = (0..5)
            .map(|zoom| icon(&format!("version = 1\n[view]\nzoom = {zoom}\n")))
            .collect();
        assert_eq!(old, [16., 20., 24., 32., 48.]);
        // Today's files are taken as they are.
        assert_eq!(icon("version = 2\n[view]\nzoom = 3\n"), 28.);
    }

    #[test]
    fn an_out_of_range_zoom_is_clamped() {
        assert_eq!(
            Settings::parse("[view]\nzoom = 99\n").zoom(),
            layout::ZOOM_LEVELS - 1
        );
    }

    #[test]
    fn a_file_from_a_newer_version_is_recognised() {
        assert!(Settings::parse("version = 4\n").is_from_newer_version());
        assert!(!sample().is_from_newer_version());
        assert!(!Settings::parse("").is_from_newer_version());
    }

    #[test]
    fn the_general_settings_survive_being_written_and_read_back() {
        let mut settings = sample();
        settings.set_start(Start::Folder, Some("/home/u/Work".to_owned()));
        settings.set_shown_folders(ShownIn::Window);
        settings.set_terminal("  kitty --single-instance ");
        settings.set_click(Some(Click::Single));
        let text = settings.to_toml();
        assert!(text.contains("[general]") && text.contains("start = \"folder\""));
        let loaded = Settings::parse(&text);
        assert_eq!(loaded, settings);
        assert_eq!(loaded.start(), (Start::Folder, Some("/home/u/Work")));
        assert_eq!(loaded.shown_folders(), ShownIn::Window);
        assert_eq!(loaded.terminal(), Some("kitty --single-instance"));
        assert_eq!(loaded.click(), Some(Click::Single));
    }

    #[test]
    fn a_value_this_build_does_not_know_falls_back_alone() {
        let settings = Settings::parse(
            "version = 3\n[general]\nstart = \"sideways\"\nshown_folders = \"window\"\nclick = 2\n[view]\nzoom = 4\n",
        );
        assert_eq!(settings.start().0, Start::Restore);
        assert_eq!(settings.shown_folders(), ShownIn::Window);
        assert_eq!(settings.click(), None);
        assert_eq!(settings.zoom(), 4);
    }

    #[test]
    fn choosing_no_terminal_or_resetting_the_page_goes_back_to_kdes() {
        let mut settings = sample();
        settings.set_terminal("foot");
        settings.set_terminal("   ");
        assert_eq!(settings.terminal(), None);
        settings.set_start(Start::Home, None);
        settings.set_click(Some(Click::Double));
        settings.reset_general();
        assert_eq!(settings.start(), (Start::Restore, None));
        assert_eq!(settings.click(), None);
        // The view is another page's.
        assert_eq!(settings.mode(), ViewMode::Icons);
        assert!(!settings.to_toml().contains("terminal"));
    }

    #[test]
    fn the_views_page_choices_survive_and_a_pane_does_not_undo_them() {
        let mut settings = sample();
        settings.set_per_folder(true);
        settings.set_detail_columns(DetailColumns {
            size: true,
            modified: false,
            kind: false,
        });
        settings.set_formats(Dates::Relative, Units::Decimal);
        settings.set_previews(Previews {
            shown: false,
            limit_mb: 10,
        });
        // A pane telling its view keeps the page's other choices.
        settings.set_view(ViewMode::Compact, 1, ViewOptions::default());
        let loaded = Settings::parse(&settings.to_toml());
        assert_eq!(loaded, settings);
        assert!(loaded.per_folder());
        assert!(!loaded.detail_columns().modified && loaded.detail_columns().size);
        assert_eq!(loaded.formats(), (Dates::Relative, Units::Decimal));
        assert_eq!(loaded.previews().limit_mb, 10);
        assert_eq!(loaded.mode(), ViewMode::Compact);
    }

    #[test]
    fn missing_or_strange_view_choices_take_their_defaults() {
        let old = Settings::parse("version = 2\n[view]\nmode = \"icons\"\n");
        assert!(!old.per_folder());
        assert_eq!(
            old.detail_columns(),
            DetailColumns {
                size: true,
                modified: true,
                kind: true
            }
        );
        assert!(old.previews().shown);
        assert_eq!(old.previews().limit_mb, THUMBNAIL_LIMIT_MB);
        let odd = Settings::parse(
            "version = 3\n[view]\nmode = \"icons\"\ncolumn_size = \"no\"\nthumbnail_limit_mb = -4\ndates = \"lunar\"\n",
        );
        assert!(odd.detail_columns().size);
        assert_eq!(odd.previews().limit_mb, THUMBNAIL_LIMIT_MB);
        assert_eq!(odd.formats().0, Dates::Iso);
        assert_eq!(odd.mode(), ViewMode::Icons);
    }

    #[test]
    fn previews_stop_at_their_limit_and_resetting_the_page_restores_them() {
        let previews = Previews {
            shown: true,
            limit_mb: 50,
        };
        assert_eq!(previews.limit_bytes(), 50_000_000);
        let mut settings = sample();
        settings.set_previews(Previews {
            shown: false,
            limit_mb: 0,
        });
        assert_eq!(settings.previews().limit_mb, 1);
        settings.set_start(Start::Home, None);
        settings.reset_views();
        assert!(settings.previews().shown);
        assert_eq!(settings.mode(), ViewMode::Details);
        // The General page is another page's.
        assert_eq!(settings.start().0, Start::Home);
    }

    #[test]
    fn appearance_survives_and_hidden_fading_stays_readable() {
        let mut settings = sample();
        settings.set_appearance(Look::Opaque, TitleBar::Kde, TextSize::Larger, 80);
        let loaded = Settings::parse(&settings.to_toml());
        assert_eq!(loaded, settings);
        assert_eq!(loaded.look(), Look::Opaque);
        assert_eq!(loaded.title_bar(), TitleBar::Kde);
        assert_eq!(loaded.text_size(), TextSize::Larger);
        assert_eq!(loaded.hidden_opacity(), 80);
        // Too faint to read is raised to the readable floor, in the file too.
        settings.set_appearance(Look::Glass, TitleBar::Rustphin, TextSize::Normal, 10);
        assert_eq!(
            settings.hidden_opacity(),
            crate::metrics::HIDDEN_OPACITY_MIN_PERCENT
        );
        let faint =
            Settings::parse("version = 3\n[appearance]\nhidden_opacity = 5\nlook = \"neon\"\n");
        assert_eq!(
            faint.hidden_opacity(),
            crate::metrics::HIDDEN_OPACITY_MIN_PERCENT
        );
        assert_eq!(faint.look(), Look::Glass);
        assert_eq!(Settings::default().hidden_opacity(), HIDDEN_OPACITY_PERCENT);
        settings.reset_appearance();
        assert_eq!(settings.text_size(), TextSize::Normal);
        assert_eq!(TextSize::Larger.scale() * 12., 14.);
    }

    #[test]
    fn confirmations_default_to_todays_behaviour_and_survive() {
        let defaults = Settings::default().confirmations();
        assert_eq!(
            defaults,
            Confirmations {
                trash: false,
                empty_trash: true,
                programs: Programs::Ask,
                close_tabs: false,
            }
        );
        let mut settings = sample();
        let chosen = Confirmations {
            trash: true,
            empty_trash: false,
            programs: Programs::Open,
            close_tabs: true,
        };
        settings.set_confirmations(chosen);
        assert_eq!(Settings::parse(&settings.to_toml()).confirmations(), chosen);
        let odd = Settings::parse(
            "version = 3\n[confirm]\nempty_trash = 3\nprograms = \"always\"\ntrash = true\n",
        );
        assert!(
            odd.confirmations().empty_trash,
            "a strange value keeps asking"
        );
        assert_eq!(odd.confirmations().programs, Programs::Ask);
        assert!(odd.confirmations().trash);
        settings.reset_confirmations();
        assert_eq!(settings.confirmations(), defaults);
    }

    #[test]
    fn the_places_panel_choices_survive_and_its_width_stays_in_bounds() {
        let defaults = Settings::default().places();
        assert!(defaults.devices && defaults.remote && !defaults.show_hidden);
        assert_eq!(defaults.width, 240);
        // The drives are drawn as bars until another style is chosen (W-159).
        assert_eq!(defaults.drive_style, DriveStyle::Bars);
        let mut settings = sample();
        settings.set_places(PlacesChoice {
            devices: false,
            remote: true,
            show_hidden: true,
            width: 301,
            drive_style: DriveStyle::Rings,
        });
        let loaded = Settings::parse(&settings.to_toml()).places();
        assert!(!loaded.devices && loaded.show_hidden);
        assert_eq!(loaded.drive_style, DriveStyle::Rings);
        assert_eq!(loaded.width, 300, "on the 4-pixel grid");
        settings.set_places(PlacesChoice {
            width: 1000,
            ..loaded
        });
        assert_eq!(settings.places().width, 360);
        let narrow = Settings::parse("version = 3\n[places]\nwidth = 12\nremote = \"x\"\n");
        assert_eq!(narrow.places().width, 200);
        assert!(narrow.places().remote);
        // A style this build does not know is read as bars, the rest kept.
        let unknown =
            Settings::parse("version = 3\n[places]\ndrive_style = \"tiles\"\nwidth = 280\n");
        assert_eq!(unknown.places().drive_style, DriveStyle::Bars);
        assert_eq!(unknown.places().width, 280);
        settings.reset_places();
        assert_eq!(settings.places(), defaults);
    }

    #[test]
    fn a_chosen_toolbar_survives_and_todays_is_not_written_out() {
        use crate::toolbar_items::{Item, Toolbar};
        let mut settings = sample();
        assert_eq!(settings.toolbar(), Toolbar::default());
        let mut toolbar = Toolbar::default();
        toolbar.add(Item::Command("Edit: Copy location".to_owned()), None);
        toolbar.set_words(2, true);
        settings.set_toolbar(&toolbar);
        let loaded = Settings::parse(&settings.to_toml());
        assert_eq!(loaded.toolbar(), toolbar);
        settings.set_toolbar(&Toolbar::default());
        assert!(!settings.to_toml().contains("[toolbar]\nitems"));
        settings.set_toolbar(&toolbar);
        settings.reset_toolbar();
        assert_eq!(settings.toolbar(), Toolbar::default());
    }
}
