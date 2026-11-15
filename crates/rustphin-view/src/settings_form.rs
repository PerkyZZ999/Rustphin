//! The Settings dialog's pages as data (W-106, W-114f, W-155), free of
//! any toolkit: each page is sections of rows, each row what the setting
//! is, what it means and one control with its value; [`apply`] carries out
//! a change by the row's key. The interface draws the rows and reports
//! changes.

use jiff::{Timestamp, tz::TimeZone};
use rustphin_core::SortKey;
use serde::Serialize;

use crate::{
    format::{self, Dates, Units},
    layout::{ViewMode, ZOOM_LEVELS},
    metrics::{HIDDEN_OPACITY_MIN_PERCENT, size},
    settings::{
        Click, Confirmations, DetailColumns, DriveStyle, Look, PlacesChoice, Previews, Programs,
        Settings, ShownIn, Start, TextSize, TitleBar,
    },
};

/// A page of the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// How Rustphin starts and works with the desktop.
    General,
    /// How the window looks.
    Appearance,
    /// How folders are shown.
    Views,
    /// The toolbar's buttons.
    Toolbar,
    /// The places panel.
    Places,
    /// Which actions ask first.
    Confirmations,
    /// The keys.
    Shortcuts,
    /// Rustphin itself.
    About,
}

impl Page {
    /// Every page, in the dialog's order.
    pub const ALL: [Page; 8] = [
        Page::General,
        Page::Appearance,
        Page::Views,
        Page::Toolbar,
        Page::Places,
        Page::Confirmations,
        Page::Shortcuts,
        Page::About,
    ];

    /// The page's name.
    pub fn title(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Appearance => "Appearance",
            Page::Views => "Views",
            Page::Toolbar => "Toolbar",
            Page::Places => "Places",
            Page::Confirmations => "Confirmations",
            Page::Shortcuts => "Shortcuts",
            Page::About => "About",
        }
    }

    /// The line under the page's title.
    pub fn description(self) -> &'static str {
        match self {
            Page::General => "How Rustphin starts and works with the rest of the desktop.",
            Page::Views => "How folders are shown when they open.",
            Page::Toolbar => "Which buttons the toolbar shows, and in which order.",
            Page::Places => "What the places panel shows, and how it is grouped.",
            Page::Confirmations => "Which actions ask before they act.",
            Page::Shortcuts => "The keys Rustphin understands. Changing them comes later.",
            Page::Appearance => {
                "How the window looks. The colours are Rustphin\u{2019}s dark design."
            }
            Page::About => "Rustphin and the work it builds on.",
        }
    }

    /// The Lucide icon in the page list.
    pub fn icon(self) -> &'static str {
        match self {
            Page::General => "settings",
            Page::Appearance => "palette",
            Page::Views => "layout-grid",
            Page::Toolbar => "panel-top",
            Page::Places => "panel-left",
            Page::Confirmations => "circle-check",
            Page::Shortcuts => "keyboard",
            Page::About => "info",
        }
    }

    /// The heading the page is listed under; About has none, being kept
    /// apart at the foot of the list.
    pub fn section(self) -> &'static str {
        match self {
            Page::General | Page::Appearance => "Rustphin",
            Page::Views | Page::Toolbar | Page::Places => "Browsing",
            Page::Confirmations | Page::Shortcuts => "Behaviour",
            Page::About => "",
        }
    }

    /// Words the dialog's search finds the page by, beyond its name: what
    /// its settings are about.
    pub fn keywords(self) -> &'static str {
        match self {
            Page::General => {
                "start starting startup restore tabs home folder window default file manager \
                 single double click open terminal command background keep running login memory quit"
            }
            Page::Appearance => {
                "look glass opaque blur transparent title bar text size font larger smaller \
                 hidden files faded opacity"
            }
            Page::Views => {
                "view icons compact details size zoom remember each folder sort order direction \
                 folders first hidden files columns modified type dates sizes units thumbnails \
                 previews pictures"
            }
            Page::Toolbar => "buttons actions items separator order words labels palette",
            Page::Places => {
                "places panel sidebar devices drives remote network hidden places width groups \
                 separators style bars compact rings removable"
            }
            Page::Confirmations => {
                "ask confirm trash delete empty close tabs window executable programs run"
            }
            Page::Shortcuts => "keys keyboard hotkeys shortcuts",
            Page::About => {
                "version licence license credits privacy telemetry network settings file fonts icons"
            }
        }
    }

    /// Whether the dialog's search finds the page: every word of `query`
    /// is in its name or its keywords, whatever the case.
    pub fn matches(self, query: &str) -> bool {
        let haystack = format!("{} {}", self.title(), self.keywords()).to_lowercase();
        query
            .to_lowercase()
            .split_whitespace()
            .all(|word| haystack.contains(word))
    }

    /// Whether it holds settings, and so a "Reset to defaults".
    pub fn resettable(self) -> bool {
        !matches!(self, Page::Shortcuts | Page::About)
    }

    /// The page a QML number names.
    pub fn from_number(number: i32) -> Option<Page> {
        usize::try_from(number)
            .ok()
            .and_then(|index| Page::ALL.get(index).copied())
    }
}

/// The numbers of the pages the dialog's search finds, in the dialog's
/// order; every page while the search is empty.
pub fn matching_pages(query: &str) -> Vec<usize> {
    Page::ALL
        .iter()
        .enumerate()
        .filter(|(_, page)| page.matches(query))
        .map(|(number, _)| number)
        .collect()
}

/// What the dialog knows of the desktop, read in the background.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// The folder the window shows, as saved (a `file://` address).
    pub shown_folder: Option<String>,
    /// KDE's terminal, as its command words.
    pub kde_terminal: Vec<String>,
    /// The application that opens folders now.
    pub file_manager: Option<String>,
    /// Rustphin is that application.
    pub is_default: bool,
    /// Rustphin's desktop entry is installed.
    pub installed: bool,
    /// KDE opens with one click.
    pub kde_single_click: bool,
    /// The desktop facts have been read.
    pub read: bool,
    /// Rustphin's own memory now, in bytes.
    pub memory: Option<u64>,
    /// What "Make Rustphin the default" is doing or why it failed.
    pub make_default: Option<String>,
    /// The terminal field is shown although no command is chosen.
    pub editing_terminal: bool,
}

/// One option of a row of segments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Segment {
    /// What [`apply`] is told.
    pub id: &'static str,
    /// Its words.
    pub words: String,
    /// A Lucide icon before the words, or empty.
    pub icon: &'static str,
}

/// One chip of a row of chips: a setting of its own, on or off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chip {
    /// The row key [`apply`] is told, with "true" or "false".
    pub key: &'static str,
    /// Its words.
    pub words: &'static str,
    /// Whether it is on.
    pub on: bool,
}

/// One control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Control {
    /// One of several, as a drop-down: for many options or long words.
    Choice {
        /// The current option's id.
        value: String,
        /// Each option's id and words.
        options: Vec<(String, String)>,
    },
    /// One of two or three short options, side by side.
    Segmented {
        /// The current option's id.
        value: &'static str,
        /// The options.
        options: Vec<Segment>,
    },
    /// On or off, as a switch.
    Toggle {
        /// Whether it is on.
        on: bool,
        /// Whether it can be changed.
        enabled: bool,
    },
    /// A number in steps, as a slider with the value in words beside it.
    Slider {
        /// The value.
        value: i64,
        /// The smallest value.
        min: i64,
        /// The largest value.
        max: i64,
        /// The step.
        step: i64,
        /// The value in words ("96 px").
        text: String,
        /// Each step's value in words, from `min` up, shown while the
        /// slider is dragged.
        steps: Vec<String>,
    },
    /// The window's look, as a drawing of each: `glass` or `opaque`.
    Looks {
        /// The current look.
        value: &'static str,
    },
    /// Several settings on or off, as a row of chips.
    Chips {
        /// The chips.
        chips: Vec<Chip>,
    },
    /// A line of text, applied on Enter.
    Text {
        /// What it holds.
        value: String,
        /// What it says while empty.
        placeholder: String,
    },
    /// A button that does something once.
    Button {
        /// Its words.
        label: String,
    },
    /// A state that is as it should be ("Default"), with a check mark.
    Badge {
        /// Its words.
        label: &'static str,
    },
    /// Words only.
    Note,
}

/// One row: what the setting is, what it means, and its control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    /// What [`apply`] is told when it changes.
    pub key: &'static str,
    /// What the setting is.
    pub label: String,
    /// The control.
    pub control: Control,
    /// What it means, under the label; empty for none.
    pub hint: String,
    /// The hint is a warning.
    pub warning: bool,
}

/// A titled section of rows, drawn as one panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    /// Its title.
    pub title: &'static str,
    /// A line under the title; empty for none.
    pub note: &'static str,
    /// Its rows.
    pub rows: Vec<Row>,
}

/// Each step the thumbnail limit offers, in megabytes.
const LIMITS_MB: [u32; 4] = [5, 10, 50, 100];
/// The hidden-file opacity's step, in percent.
const HIDDEN_STEP: i64 = 5;
/// The places panel width's step, in pixels.
const WIDTH_STEP: i64 = 4;

fn row(key: &'static str, label: &str, control: Control) -> Row {
    Row {
        key,
        label: label.to_owned(),
        control,
        hint: String::new(),
        warning: false,
    }
}

fn hinted(mut row: Row, hint: impl Into<String>) -> Row {
    row.hint = hint.into();
    row
}

fn group(title: &'static str, rows: Vec<Row>) -> Group {
    Group {
        title,
        note: "",
        rows,
    }
}

fn choice(value: &str, options: &[(&str, &str)]) -> Control {
    Control::Choice {
        value: value.to_owned(),
        options: options
            .iter()
            .map(|(id, words)| ((*id).to_owned(), (*words).to_owned()))
            .collect(),
    }
}

fn segmented(value: &'static str, options: &[(&'static str, &str, &'static str)]) -> Control {
    Control::Segmented {
        value,
        options: options
            .iter()
            .map(|(id, words, icon)| Segment {
                id,
                words: (*words).to_owned(),
                icon,
            })
            .collect(),
    }
}

/// A slider from `min` to `max` by `step` at `value`, each value written
/// by `words`.
fn slider(value: i64, min: i64, max: i64, step: i64, words: impl Fn(i64) -> String) -> Control {
    let steps = (min..=max)
        .step_by(usize::try_from(step.max(1)).unwrap_or(1))
        .map(&words)
        .collect();
    Control::Slider {
        value,
        min,
        max,
        step,
        text: words(value),
        steps,
    }
}

fn toggle(on: bool) -> Control {
    Control::Toggle { on, enabled: true }
}

fn mode_id(mode: ViewMode) -> &'static str {
    match mode {
        ViewMode::Icons => "icons",
        ViewMode::Compact => "compact",
        ViewMode::Details => "details",
    }
}

fn sort_id(sort: SortKey) -> &'static str {
    match sort {
        SortKey::Size => "size",
        SortKey::Modified => "modified",
        SortKey::Type => "type",
        _ => "name",
    }
}

/// The sections of `page` for these settings and facts. The Toolbar,
/// Shortcuts and About pages are drawn by the interface from their own
/// lists; they have no rows here.
pub fn page(page: Page, settings: &Settings, facts: &Facts) -> Vec<Group> {
    match page {
        Page::General => general(settings, facts),
        Page::Appearance => appearance(settings),
        Page::Views => views(settings),
        Page::Places => places(settings),
        Page::Confirmations => confirmations(settings),
        Page::Toolbar | Page::Shortcuts | Page::About => Vec::new(),
    }
}

fn general(settings: &Settings, facts: &Facts) -> Vec<Group> {
    let (start, folder) = settings.start();
    let mut starting = vec![row(
        "start",
        "When Rustphin starts",
        choice(
            match start {
                Start::Restore => "restore",
                Start::Home => "home",
                Start::Folder => "folder",
            },
            &[
                ("restore", "Restore the last tabs"),
                ("home", "Open the home folder"),
                ("folder", "Open a chosen folder"),
            ],
        ),
    )];
    if start == Start::Folder {
        let shown = folder
            .and_then(crate::location::Location::from_saved)
            .map(|location| match location {
                crate::location::Location::Folder(path) => path.display().to_string(),
                crate::location::Location::Remote(address) => address.to_string(),
            });
        starting.push(hinted(
            row(
                "start-folder-shown",
                "The chosen folder",
                Control::Button {
                    label: "Use the folder shown".to_owned(),
                },
            ),
            shown.unwrap_or_else(|| "No folder chosen: Rustphin opens the home folder.".to_owned()),
        ));
    }
    starting.push(hinted(
        row(
            "shown-in",
            "Folders opened by other applications",
            segmented(
                match settings.shown_folders() {
                    ShownIn::Tab => "tab",
                    ShownIn::Window => "window",
                },
                &[("tab", "New tab", ""), ("window", "New window", "")],
            ),
        ),
        "From a browser\u{2019}s downloads or a desktop link.",
    ));
    let memory = facts
        .memory
        .map(|bytes| format!(" Rustphin uses {} MB of memory now.", bytes / (1024 * 1024)))
        .unwrap_or_default();
    let keep_running = settings.keep_running();
    let background = vec![
        hinted(
            row(
                "keep-running",
                "Keep running after the last window closes",
                toggle(keep_running),
            ),
            "New windows then open at once.",
        ),
        hinted(
            row(
                "start-at-login",
                "Start when you log in",
                Control::Toggle {
                    on: settings.start_at_login(),
                    enabled: keep_running,
                },
            ),
            if keep_running {
                format!("Quit with Ctrl+Q.{memory}")
            } else {
                "Needs \u{201c}Keep running after the last window closes\u{201d} on.".to_owned()
            },
        ),
    ];
    let default_manager = if !facts.read {
        hinted(
            row("", "Default file manager", Control::Note),
            "Checking which application opens folders\u{2026}",
        )
    } else if facts.is_default {
        hinted(
            row(
                "",
                "Default file manager",
                Control::Badge { label: "Default" },
            ),
            "Rustphin opens folders for this desktop.",
        )
    } else if !facts.installed {
        let mut note = hinted(
            row("", "Default file manager", Control::Note),
            "Rustphin\u{2019}s desktop entry (rustphin.desktop) is not installed, so the desktop cannot choose it.",
        );
        note.warning = true;
        note
    } else {
        let now = match &facts.file_manager {
            Some(name) => format!("{name} opens folders now."),
            None => "No application is set to open folders.".to_owned(),
        };
        let mut button = hinted(
            row(
                "make-default",
                "Default file manager",
                Control::Button {
                    label: if facts.make_default.as_deref() == Some("") {
                        "Making it the default\u{2026}".to_owned()
                    } else {
                        "Make Rustphin the default".to_owned()
                    },
                },
            ),
            facts
                .make_default
                .clone()
                .filter(|reason| !reason.is_empty())
                .unwrap_or(now),
        );
        button.warning = facts
            .make_default
            .as_ref()
            .is_some_and(|reason| !reason.is_empty());
        button
    };
    let click = settings.click();
    let mut desktop = vec![
        default_manager,
        row(
            "click",
            "Open files and folders with",
            choice(
                match click {
                    None => "kde",
                    Some(Click::Double) => "double",
                    Some(Click::Single) => "single",
                },
                &[
                    (
                        "kde",
                        if facts.kde_single_click {
                            "As KDE is set (a single click)"
                        } else {
                            "As KDE is set (a double click)"
                        },
                    ),
                    ("double", "A double click"),
                    ("single", "A single click"),
                ],
            ),
        ),
    ];
    let chosen = settings.terminal();
    let (name, source) = match chosen {
        Some(command) => (
            rustphin_platform::terminal::words(command)
                .map(|words| rustphin_platform::terminal::display_name(&words))
                .unwrap_or_else(|| command.to_owned()),
            "chosen here",
        ),
        None => (
            if facts.read {
                rustphin_platform::terminal::display_name(&facts.kde_terminal)
            } else {
                "\u{2026}".to_owned()
            },
            "from KDE\u{2019}s settings",
        ),
    };
    desktop.push(hinted(
        row(
            if chosen.is_some() || facts.editing_terminal {
                "terminal-kde"
            } else {
                "terminal-other"
            },
            "Terminal",
            Control::Button {
                label: if chosen.is_some() || facts.editing_terminal {
                    "Use KDE\u{2019}s".to_owned()
                } else {
                    "Use another\u{2026}".to_owned()
                },
            },
        ),
        format!("{name}, {source}"),
    ));
    if chosen.is_some() || facts.editing_terminal {
        let text = chosen.unwrap_or_default();
        let broken = !text.is_empty() && rustphin_platform::terminal::words(text).is_none();
        let mut field = hinted(
            row(
                "terminal",
                "Terminal command",
                Control::Text {
                    value: text.to_owned(),
                    placeholder: "konsole --separate".to_owned(),
                },
            ),
            if broken {
                "A quote is not closed: the command cannot be run."
            } else {
                "The program and its options; Rustphin adds the folder."
            },
        );
        field.warning = broken;
        desktop.push(field);
    }
    vec![
        group("Starting", starting),
        group("Desktop", desktop),
        group("Background", background),
    ]
}

fn appearance(settings: &Settings) -> Vec<Group> {
    let hidden = settings.hidden_opacity();
    vec![
        group(
            "Window",
            vec![
                hinted(
                    row(
                        "look",
                        "Look",
                        Control::Looks {
                            value: match settings.look() {
                                Look::Glass => "glass",
                                Look::Opaque => "opaque",
                            },
                        },
                    ),
                    "Blur shows only where the desktop offers it.",
                ),
                hinted(
                    row(
                        "title-bar",
                        "Title bar",
                        segmented(
                            match settings.title_bar() {
                                TitleBar::Rustphin => "rustphin",
                                TitleBar::Kde => "kde",
                            },
                            &[
                                ("rustphin", "Rustphin\u{2019}s own", ""),
                                ("kde", "KDE\u{2019}s", ""),
                            ],
                        ),
                    ),
                    "Applies to new windows.",
                ),
            ],
        ),
        group(
            "Text and files",
            vec![
                row(
                    "text-size",
                    "Text size",
                    segmented(
                        match settings.text_size() {
                            TextSize::Smaller => "smaller",
                            TextSize::Normal => "normal",
                            TextSize::Larger => "larger",
                        },
                        &[
                            ("smaller", "Smaller", ""),
                            ("normal", "Normal", ""),
                            ("larger", "Larger", ""),
                        ],
                    ),
                ),
                hinted(
                    row(
                        "hidden-opacity",
                        "Hidden files are drawn at",
                        slider(
                            i64::from(hidden),
                            i64::from(HIDDEN_OPACITY_MIN_PERCENT),
                            100,
                            HIDDEN_STEP,
                            |percent| format!("{percent} %"),
                        ),
                    ),
                    "Lower is fainter.",
                ),
            ],
        ),
    ]
}

fn views(settings: &Settings) -> Vec<Group> {
    let (mode, zoom, options) = (settings.mode(), settings.zoom(), settings.options());
    let columns = settings.detail_columns();
    let (dates, units) = settings.formats();
    let previews = settings.previews();
    let now = Timestamp::now();
    let zone = TimeZone::system();
    let iso = format::timestamp_in(now.as_second(), &zone, Dates::Iso, now);
    let relative = format::timestamp_in(now.as_second(), &zone, Dates::Relative, now);
    let mut limits: Vec<u32> = LIMITS_MB.to_vec();
    if !limits.contains(&previews.limit_mb) {
        limits.push(previews.limit_mb);
        limits.sort_unstable();
    }
    let limit_words: Vec<(String, String)> = limits
        .iter()
        .map(|mb| (mb.to_string(), format!("up to {mb} MB a file")))
        .collect();
    vec![
        group(
            "Default view",
            vec![
                row(
                    "view-mode",
                    "View",
                    segmented(
                        mode_id(mode),
                        &[
                            ("icons", "Icons", "layout-grid"),
                            ("compact", "Compact", "columns-3"),
                            ("details", "Details", "layout-list"),
                        ],
                    ),
                ),
                row(
                    "zoom",
                    "Icon size",
                    slider(
                        i64::try_from(zoom).unwrap_or(0),
                        0,
                        i64::try_from(ZOOM_LEVELS - 1).unwrap_or(6),
                        1,
                        |level| {
                            let level = usize::try_from(level).unwrap_or(0);
                            format!("{} px", mode.layout(level).icon as u32)
                        },
                    ),
                ),
                hinted(
                    row(
                        "per-folder",
                        "Remember the view for each folder",
                        toggle(settings.per_folder()),
                    ),
                    "Otherwise every folder opens in the view above.",
                ),
            ],
        ),
        group(
            "Order",
            vec![
                row(
                    "sort",
                    "Sort by",
                    choice(
                        sort_id(options.sort),
                        &[
                            ("name", "Name"),
                            ("size", "Size"),
                            ("modified", "Modified"),
                            ("type", "Type"),
                        ],
                    ),
                ),
                row(
                    "direction",
                    "Direction",
                    segmented(
                        if options.descending { "down" } else { "up" },
                        &[("up", "A to Z", ""), ("down", "Z to A", "")],
                    ),
                ),
                row(
                    "folders-first",
                    "Folders first",
                    toggle(options.folders_first),
                ),
                hinted(
                    row("hidden", "Show hidden files", toggle(options.show_hidden)),
                    "Ctrl+H switches this in any window.",
                ),
            ],
        ),
        Group {
            title: "Details view",
            note: "The name is always shown.",
            rows: vec![
                row(
                    "",
                    "Columns",
                    Control::Chips {
                        chips: vec![
                            Chip {
                                key: "column-size",
                                words: "Size",
                                on: columns.size,
                            },
                            Chip {
                                key: "column-modified",
                                words: "Modified",
                                on: columns.modified,
                            },
                            Chip {
                                key: "column-type",
                                words: "Type",
                                on: columns.kind,
                            },
                        ],
                    },
                ),
                row(
                    "dates",
                    "Dates",
                    Control::Choice {
                        value: match dates {
                            Dates::Iso => "iso",
                            Dates::Relative => "relative",
                        }
                        .to_owned(),
                        options: vec![("iso".to_owned(), iso), ("relative".to_owned(), relative)],
                    },
                ),
                row(
                    "sizes",
                    "Sizes",
                    choice(
                        match units {
                            Units::Binary => "binary",
                            Units::Decimal => "decimal",
                        },
                        &[
                            ("binary", "Binary (KiB, MiB)"),
                            ("decimal", "Decimal (kB, MB)"),
                        ],
                    ),
                ),
            ],
        },
        group(
            "Previews",
            vec![
                row(
                    "thumbnails",
                    "Show pictures\u{2019} thumbnails",
                    toggle(previews.shown),
                ),
                row(
                    "thumbnail-limit",
                    "Largest picture previewed",
                    Control::Choice {
                        value: previews.limit_mb.to_string(),
                        options: limit_words,
                    },
                ),
            ],
        ),
    ]
}

fn places(settings: &Settings) -> Vec<Group> {
    let choice_now = settings.places();
    vec![
        group(
            "Sections",
            vec![
                hinted(
                    row("places-devices", "Devices", toggle(choice_now.devices)),
                    "Drives that are plugged in.",
                ),
                hinted(
                    row("places-remote", "Remote", toggle(choice_now.remote)),
                    "Network folders you have connected.",
                ),
                row(
                    "hidden-places",
                    "Hidden places",
                    choice(
                        if choice_now.show_hidden {
                            "show"
                        } else {
                            "hide"
                        },
                        &[("hide", "Keep them hidden"), ("show", "Show them faded")],
                    ),
                ),
            ],
        ),
        group(
            "Drives",
            vec![hinted(
                row(
                    "drive-style",
                    "Style",
                    segmented(
                        choice_now.drive_style.id(),
                        &[
                            ("bars", "Bars", ""),
                            ("compact", "Compact", ""),
                            ("rings", "Rings", ""),
                        ],
                    ),
                ),
                "How each drive is drawn in the places panel.",
            )],
        ),
        group(
            "Panel",
            vec![hinted(
                row(
                    "places-width",
                    "Width",
                    slider(
                        i64::from(choice_now.width),
                        size::SIDEBAR_MIN as i64,
                        size::SIDEBAR_MAX as i64,
                        WIDTH_STEP,
                        |width| format!("{width} px"),
                    ),
                ),
                "Or drag the panel\u{2019}s edge.",
            )],
        ),
    ]
}

fn confirmations(settings: &Settings) -> Vec<Group> {
    let asked = settings.confirmations();
    vec![
        group(
            "Ask before",
            vec![
                hinted(
                    row("ask-trash", "Moving to the trash", toggle(asked.trash)),
                    "It can be put back from the Trash.",
                ),
                hinted(
                    row(
                        "ask-delete",
                        "Deleting for good",
                        Control::Toggle {
                            on: true,
                            enabled: false,
                        },
                    ),
                    "Always asks: it cannot be undone.",
                ),
                row("ask-empty", "Emptying the trash", toggle(asked.empty_trash)),
                row(
                    "ask-close",
                    "Closing a window with several tabs",
                    toggle(asked.close_tabs),
                ),
            ],
        ),
        group(
            "Executable files",
            vec![hinted(
                row(
                    "programs",
                    "Opening one",
                    choice(
                        match asked.programs {
                            Programs::Ask => "ask",
                            Programs::Open => "open",
                            Programs::Run => "run",
                        },
                        &[
                            ("ask", "Ask each time"),
                            ("open", "Open it"),
                            ("run", "Run it"),
                        ],
                    ),
                ),
                "Open hands it to the application for its type. A shortcut still asks unless Run it.",
            )],
        ),
    ]
}

/// What a change does besides the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// Nothing changed.
    Nothing,
    /// The settings changed.
    Changed,
    /// The default view changed: open panes take it up.
    DefaultView,
    /// The settings changed and the login entry must follow `start_at_login`.
    Login(bool),
    /// A button asks the interface to act: `make-default`, `terminal-other`.
    Act(&'static str),
}

/// Applies the change of row `key` to `value` (an option's id, "true" or
/// "false", a number, a text). `facts` gives the folder shown.
pub fn apply(settings: &mut Settings, key: &str, value: &str, facts: &Facts) -> Applied {
    let on = value == "true";
    let number = value.parse::<i64>().ok();
    let before = settings.clone();
    let mut default_view = false;
    let mut login = None;
    match key {
        "start" => {
            let start = match value {
                "home" => Start::Home,
                "folder" => Start::Folder,
                _ => Start::Restore,
            };
            // The first choice of a folder takes the one shown.
            let keep = settings.start().1.is_some();
            let folder = (start == Start::Folder && !keep)
                .then(|| facts.shown_folder.clone())
                .flatten();
            settings.set_start(start, folder);
        }
        "start-folder-shown" => {
            if let Some(folder) = facts.shown_folder.clone() {
                settings.set_start(Start::Folder, Some(folder));
            }
        }
        "shown-in" => settings.set_shown_folders(if value == "window" {
            ShownIn::Window
        } else {
            ShownIn::Tab
        }),
        "click" => settings.set_click(match value {
            "double" => Some(Click::Double),
            "single" => Some(Click::Single),
            _ => None,
        }),
        "keep-running" => {
            let starts = settings.start_at_login();
            settings.set_keep_running(on);
            if !on && starts {
                settings.set_start_at_login(false);
                login = Some(false);
            }
        }
        "start-at-login" => {
            settings.set_start_at_login(on);
            login = Some(on);
        }
        "terminal" => settings.set_terminal(value.trim()),
        "terminal-kde" => settings.set_terminal(""),
        "make-default" => return Applied::Act("make-default"),
        "terminal-other" => return Applied::Act("terminal-other"),
        "view-mode" | "zoom" | "sort" | "direction" | "folders-first" | "hidden" => {
            let mut mode = settings.mode();
            let mut zoom = settings.zoom();
            let mut options = settings.options();
            match key {
                "view-mode" => {
                    mode = match value {
                        "icons" => ViewMode::Icons,
                        "compact" => ViewMode::Compact,
                        _ => ViewMode::Details,
                    }
                }
                "zoom" => {
                    zoom = number
                        .and_then(|zoom| usize::try_from(zoom).ok())
                        .unwrap_or(zoom)
                        .min(ZOOM_LEVELS - 1);
                }
                "sort" => {
                    options.sort = match value {
                        "size" => SortKey::Size,
                        "modified" => SortKey::Modified,
                        "type" => SortKey::Type,
                        _ => SortKey::Name,
                    }
                }
                "direction" => options.descending = value == "down",
                "folders-first" => options.folders_first = on,
                _ => options.show_hidden = on,
            }
            settings.set_view(mode, zoom, options);
            default_view = true;
        }
        "per-folder" => settings.set_per_folder(on),
        "column-size" | "column-modified" | "column-type" => {
            let mut columns: DetailColumns = settings.detail_columns();
            match key {
                "column-size" => columns.size = on,
                "column-modified" => columns.modified = on,
                _ => columns.kind = on,
            }
            settings.set_detail_columns(columns);
        }
        "dates" | "sizes" => {
            let (mut dates, mut units) = settings.formats();
            if key == "dates" {
                dates = if value == "relative" {
                    Dates::Relative
                } else {
                    Dates::Iso
                };
            } else {
                units = if value == "decimal" {
                    Units::Decimal
                } else {
                    Units::Binary
                };
            }
            settings.set_formats(dates, units);
        }
        "thumbnails" | "thumbnail-limit" => {
            let mut previews: Previews = settings.previews();
            if key == "thumbnails" {
                previews.shown = on;
            } else if let Some(mb) = value.parse::<u32>().ok().filter(|mb| *mb >= 1) {
                previews.limit_mb = mb;
            }
            settings.set_previews(previews);
        }
        "look" | "title-bar" | "text-size" | "hidden-opacity" => {
            let (mut look, mut title_bar, mut text_size, mut hidden) = (
                settings.look(),
                settings.title_bar(),
                settings.text_size(),
                settings.hidden_opacity(),
            );
            match key {
                "look" => {
                    look = if value == "opaque" {
                        Look::Opaque
                    } else {
                        Look::Glass
                    }
                }
                "title-bar" => {
                    title_bar = if value == "kde" {
                        TitleBar::Kde
                    } else {
                        TitleBar::Rustphin
                    }
                }
                "text-size" => {
                    text_size = match value {
                        "smaller" => TextSize::Smaller,
                        "larger" => TextSize::Larger,
                        _ => TextSize::Normal,
                    }
                }
                _ => {
                    hidden = number
                        .and_then(|percent| u32::try_from(percent).ok())
                        .unwrap_or(hidden);
                }
            }
            settings.set_appearance(look, title_bar, text_size, hidden);
        }
        "places-devices" | "places-remote" | "hidden-places" | "places-width" | "drive-style" => {
            let mut places: PlacesChoice = settings.places();
            match key {
                "places-devices" => places.devices = on,
                "places-remote" => places.remote = on,
                "hidden-places" => places.show_hidden = value == "show",
                "drive-style" => places.drive_style = DriveStyle::from_id(value),
                _ => {
                    places.width = number
                        .and_then(|width| u32::try_from(width).ok())
                        .unwrap_or(places.width);
                }
            }
            settings.set_places(places);
        }
        "ask-trash" | "ask-empty" | "ask-close" | "programs" => {
            let mut asked: Confirmations = settings.confirmations();
            match key {
                "ask-trash" => asked.trash = on,
                "ask-empty" => asked.empty_trash = on,
                "ask-close" => asked.close_tabs = on,
                _ => {
                    asked.programs = match value {
                        "open" => Programs::Open,
                        "run" => Programs::Run,
                        _ => Programs::Ask,
                    }
                }
            }
            settings.set_confirmations(asked);
        }
        _ => return Applied::Nothing,
    }
    if *settings == before {
        return Applied::Nothing;
    }
    match (login, default_view) {
        (Some(on), _) => Applied::Login(on),
        (None, true) => Applied::DefaultView,
        (None, false) => Applied::Changed,
    }
}

/// Puts `page`'s settings back to their defaults.
pub fn reset(settings: &mut Settings, page: Page) -> Applied {
    let before = settings.clone();
    match page {
        Page::General => settings.reset_general(),
        Page::Appearance => settings.reset_appearance(),
        Page::Views => settings.reset_views(),
        Page::Toolbar => settings.reset_toolbar(),
        Page::Places => settings.reset_places(),
        Page::Confirmations => settings.reset_confirmations(),
        Page::Shortcuts | Page::About => {}
    }
    if *settings == before {
        Applied::Nothing
    } else if page == Page::Views {
        Applied::DefaultView
    } else if page == Page::General && before.start_at_login() != settings.start_at_login() {
        Applied::Login(settings.start_at_login())
    } else {
        Applied::Changed
    }
}

/// One shortcut of the Shortcuts page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shortcut {
    /// The action's name without its category: "Back".
    pub name: String,
    /// Its keys: "Alt+Left".
    pub keys: Vec<String>,
}

/// The Shortcuts page: each category with its shortcuts, those matching
/// `search` (in the name or the keys).
pub fn shortcut_groups(search: &str) -> Vec<(String, Vec<Shortcut>)> {
    let search = search.trim().to_lowercase();
    let mut groups: Vec<(String, Vec<Shortcut>)> = Vec::new();
    for command in crate::commands::COMMANDS {
        let keys = command.shown_keys();
        if keys.is_empty() {
            continue;
        }
        let (category, name) = command
            .label
            .split_once(": ")
            .unwrap_or(("Other", command.label));
        let matches = search.is_empty()
            || command.label.to_lowercase().contains(&search)
            || keys.iter().any(|key| key.to_lowercase().contains(&search));
        if !matches {
            continue;
        }
        let shortcut = Shortcut {
            name: name.to_owned(),
            keys,
        };
        match groups.iter_mut().find(|(known, _)| known == category) {
            Some((_, list)) => list.push(shortcut),
            None => groups.push((category.to_owned(), vec![shortcut])),
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(groups: &[Group]) -> Vec<&'static str> {
        groups
            .iter()
            .flat_map(|group| &group.rows)
            .flat_map(|row| match &row.control {
                Control::Chips { chips } => chips.iter().map(|chip| chip.key).collect(),
                _ => vec![row.key],
            })
            .collect()
    }

    #[test]
    fn every_page_with_settings_shows_rows_that_apply_changes() {
        let facts = Facts {
            read: true,
            installed: true,
            ..Facts::default()
        };
        let settings = Settings::default();
        for page_now in [
            Page::General,
            Page::Appearance,
            Page::Views,
            Page::Places,
            Page::Confirmations,
        ] {
            assert!(
                !page(page_now, &settings, &facts).is_empty(),
                "{page_now:?}"
            );
        }
        let views = keys(&page(Page::Views, &settings, &facts));
        assert!(views.contains(&"thumbnail-limit") && views.contains(&"column-type"));
    }

    #[test]
    fn a_change_is_applied_and_says_what_it_asks_of_the_interface() {
        let facts = Facts {
            shown_folder: Some("file:///srv".into()),
            ..Facts::default()
        };
        let mut settings = Settings::default();
        assert_eq!(
            apply(&mut settings, "ask-trash", "true", &facts),
            Applied::Changed
        );
        assert!(settings.confirmations().trash);
        assert_eq!(
            apply(&mut settings, "ask-trash", "true", &facts),
            Applied::Nothing
        );
        assert_eq!(
            apply(&mut settings, "view-mode", "icons", &facts),
            Applied::DefaultView
        );
        assert_eq!(settings.mode(), ViewMode::Icons);
        assert_eq!(
            apply(&mut settings, "start", "folder", &facts),
            Applied::Changed
        );
        assert_eq!(settings.start(), (Start::Folder, Some("file:///srv")));
        assert_eq!(
            apply(&mut settings, "start-at-login", "true", &facts),
            Applied::Login(true)
        );
        // Leaving the background also stops starting at login.
        assert_eq!(
            apply(&mut settings, "keep-running", "false", &facts),
            Applied::Login(false)
        );
        assert!(!settings.start_at_login());
        assert_eq!(
            apply(&mut settings, "make-default", "", &facts),
            Applied::Act("make-default")
        );
        assert_eq!(
            apply(&mut settings, "hidden-opacity", "80", &facts),
            Applied::Changed
        );
        assert_eq!(settings.hidden_opacity(), 80);
        assert_eq!(
            apply(&mut settings, "no-such-row", "1", &facts),
            Applied::Nothing
        );
    }

    #[test]
    fn resetting_a_page_puts_back_only_its_settings() {
        let facts = Facts::default();
        let mut settings = Settings::default();
        apply(&mut settings, "ask-close", "true", &facts);
        apply(&mut settings, "text-size", "larger", &facts);
        assert_eq!(reset(&mut settings, Page::Confirmations), Applied::Changed);
        assert!(!settings.confirmations().close_tabs);
        assert_eq!(settings.text_size(), TextSize::Larger);
        assert_eq!(reset(&mut settings, Page::Confirmations), Applied::Nothing);
        assert_eq!(reset(&mut settings, Page::Appearance), Applied::Changed);
        assert_eq!(settings.text_size(), TextSize::default());
        apply(&mut settings, "hidden", "true", &facts);
        // The views page changes the default view, which panes follow.
        assert_eq!(reset(&mut settings, Page::Views), Applied::DefaultView);
    }

    #[test]
    fn the_search_finds_pages_by_name_or_by_what_they_hold() {
        assert_eq!(matching_pages(""), (0..Page::ALL.len()).collect::<Vec<_>>());
        assert_eq!(matching_pages("  "), matching_pages(""));
        assert_eq!(matching_pages("BLUR"), [1]);
        assert_eq!(matching_pages("Shortcuts"), [6]);
        assert_eq!(matching_pages("terminal"), [0]);
        // Every word must be found, in any order.
        assert_eq!(matching_pages("files hidden"), [1, 2]);
        assert_eq!(matching_pages("hidden telemetry"), Vec::<usize>::new());
        assert!(matching_pages("nothing like this").is_empty());
    }

    #[test]
    fn pages_are_listed_under_their_headings_with_about_apart() {
        let sections: Vec<&str> = Page::ALL.iter().map(|page| page.section()).collect();
        assert_eq!(
            sections,
            [
                "Rustphin",
                "Rustphin",
                "Browsing",
                "Browsing",
                "Browsing",
                "Behaviour",
                "Behaviour",
                ""
            ]
        );
    }

    /// Every row of every page with settings, for a desktop in each state
    /// the default file manager row knows.
    fn every_row() -> Vec<Row> {
        let mut settings = Settings::default();
        settings.set_start(Start::Folder, None);
        settings.set_terminal("konsole");
        let states = [
            Facts::default(),
            Facts {
                read: true,
                is_default: true,
                installed: true,
                ..Facts::default()
            },
            Facts {
                read: true,
                installed: true,
                ..Facts::default()
            },
            Facts {
                read: true,
                ..Facts::default()
            },
        ];
        let mut rows = Vec::new();
        for facts in &states {
            for page_now in Page::ALL {
                for group in page(page_now, &settings, facts) {
                    rows.extend(group.rows);
                }
            }
        }
        rows
    }

    #[test]
    fn every_row_says_what_its_setting_is() {
        for row in every_row() {
            assert!(!row.label.is_empty(), "{row:?}");
        }
    }

    #[test]
    fn every_segment_and_chip_applies_its_own_change() {
        let facts = Facts::default();
        for row in every_row() {
            match &row.control {
                Control::Segmented { value, options } => {
                    for option in options.iter().filter(|option| option.id != *value) {
                        let mut settings = Settings::default();
                        apply(&mut settings, row.key, value, &facts);
                        assert_ne!(
                            apply(&mut settings, row.key, option.id, &facts),
                            Applied::Nothing,
                            "{} {}",
                            row.key,
                            option.id
                        );
                    }
                }
                Control::Chips { chips } => {
                    for chip in chips {
                        let mut settings = Settings::default();
                        let flipped = if chip.on { "false" } else { "true" };
                        assert_eq!(
                            apply(&mut settings, chip.key, flipped, &facts),
                            Applied::Changed,
                            "{}",
                            chip.key
                        );
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn the_default_file_manager_shows_a_badge_only_when_it_is_rustphin() {
        let rows = every_row();
        let manager: Vec<&Row> = rows
            .iter()
            .filter(|row| row.label == "Default file manager")
            .collect();
        let badges = manager
            .iter()
            .filter(|row| matches!(row.control, Control::Badge { .. }))
            .count();
        assert_eq!(manager.len(), 4);
        assert_eq!(badges, 1);
        assert!(manager.iter().any(|row| row.key == "make-default"));
    }

    #[test]
    fn a_slider_words_each_of_its_steps_as_its_value_is_worded() {
        let mut sliders = 0;
        for row in every_row() {
            if let Control::Slider {
                value,
                min,
                max,
                step,
                text,
                steps,
            } = &row.control
            {
                sliders += 1;
                assert_eq!(steps.len() as i64, (max - min) / step + 1, "{}", row.key);
                let at = usize::try_from((value - min) / step).unwrap_or(usize::MAX);
                assert_eq!(steps.get(at), Some(text), "{}", row.key);
            }
        }
        assert!(sliders >= 3);
    }

    #[test]
    fn starting_at_login_says_why_it_cannot_be_changed() {
        let mut settings = Settings::default();
        settings.set_keep_running(false);
        let general = page(Page::General, &settings, &Facts::default());
        let login = general
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.key == "start-at-login");
        assert!(login.is_some_and(|row| {
            matches!(row.control, Control::Toggle { enabled: false, .. })
                && row.hint.contains("Keep running")
        }));
    }

    #[test]
    fn a_broken_terminal_command_is_said_under_its_field() {
        let mut settings = Settings::default();
        settings.set_terminal("konsole -e 'unclosed");
        let facts = Facts {
            read: true,
            ..Facts::default()
        };
        let general = page(Page::General, &settings, &facts);
        let field = general
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.key == "terminal");
        assert!(field.is_some_and(|row| row.warning));
    }

    #[test]
    fn shortcuts_are_grouped_and_found_by_name_or_key() {
        let all = shortcut_groups("");
        assert!(all.iter().any(|(category, list)| {
            category == "Navigation"
                && list
                    .iter()
                    .any(|s| s.name == "Open terminal here" && s.keys == ["Shift+F4"])
        }));
        let by_key = shortcut_groups("shift+f4");
        assert_eq!(by_key[0].1[0].name, "Open terminal here");
        assert!(shortcut_groups("no such shortcut anywhere").is_empty());
    }
}
