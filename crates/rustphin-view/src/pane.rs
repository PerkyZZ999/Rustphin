//! What one browsing pane says and decides, free of any toolkit (W-114b3,
//! W-114d): why a folder could not be shown, the status line, which detail
//! columns fit, type-ahead, typed locations, the location's crumbs, fading
//! of hidden and cut entries.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rustphin_core::{LoadError, Selection, Snapshot, display_name};
use rustphin_platform::remote::RemoteAddress;

use crate::{
    format::{self, count, plural},
    metrics::size,
    settings::DetailColumns,
};

/// A failed attempt to open a location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The location that failed.
    pub path: PathBuf,
    /// Why.
    pub error: LoadError,
    /// The folder being shown went away and the view moved to a parent,
    /// rather than an attempt to open something failing.
    pub moved: bool,
}

/// Where the button of a folder error shown in place leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// Back to the previous folder (Alt+Left).
    Back,
    /// Up to the containing folder (Alt+Up), when there is nothing to go back to.
    Up,
    /// Nowhere: no history and no parent.
    Nowhere,
}

impl Way {
    /// Where the in-place error's button leads, from what navigation allows.
    pub fn out(can_go_back: bool, can_go_up: bool) -> Self {
        if can_go_back {
            Self::Back
        } else if can_go_up {
            Self::Up
        } else {
            Self::Nowhere
        }
    }

    /// The button's label, if there is a button.
    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::Back => Some("Go back"),
            Self::Up => Some("Go up"),
            Self::Nowhere => None,
        }
    }
}

impl Failure {
    /// Whether to explain the failure in place of the list, as mockup 28
    /// draws it, rather than in a banner over a listing. Only an attempt to
    /// open the folder now shown, with nothing listed for it, is said in
    /// place.
    pub fn in_place(&self, location: Option<&Path>, listed: bool) -> bool {
        !self.moved && !listed && location == Some(self.path.as_path())
    }

    /// The in-place explanation: what happened, then what can be done.
    pub fn explanation(&self, way: Way) -> String {
        let path = display_name(self.path.as_os_str().as_encoded_bytes());
        let (what, advice) = match &self.error {
            LoadError::PermissionDenied => (
                format!("You do not have permission to open {path}."),
                "Ask its owner for access",
            ),
            LoadError::NotFound => (format!("{path} does not exist."), "Check the address"),
            LoadError::NotADirectory => {
                return format!(
                    "{path} is a file, not a folder. Open it from the folder that contains it."
                );
            }
            LoadError::Io(reason) => (
                format!("{path} could not be read: {reason}."),
                "Try again with F5",
            ),
        };
        let way = match way {
            Way::Back => ", or go back to the previous folder",
            Way::Up => ", or go up to the folder that contains it",
            Way::Nowhere => "",
        };
        format!("{what} {advice}{way}.")
    }

    /// The in-place error's heading.
    pub fn title(&self) -> &'static str {
        match self.error {
            LoadError::PermissionDenied => "This folder is not readable",
            LoadError::NotFound => "This folder does not exist",
            LoadError::NotADirectory => "This is not a folder",
            LoadError::Io(_) => "This folder could not be read",
        }
    }

    /// The status line's text while the failure is shown in place.
    pub fn status(&self) -> &'static str {
        match self.error {
            LoadError::PermissionDenied => "Not readable",
            LoadError::NotFound => "Does not exist",
            LoadError::NotADirectory => "Not a folder",
            LoadError::Io(_) => "Could not be read",
        }
    }

    /// What happened, in a sentence, for a banner over a listing.
    pub fn message(&self) -> String {
        let path = display_name(self.path.as_os_str().as_encoded_bytes());
        if self.moved {
            format!("{path} no longer exists. Showing the nearest folder that does.")
        } else {
            format!("Could not open {path}: {}.", self.error)
        }
    }
}

/// What the status line counts.
#[derive(Debug, Clone, Default)]
pub struct StatusFacts<'a> {
    /// Rows shown.
    pub shown: usize,
    /// Rows there would be without the name filter.
    pub unfiltered: usize,
    /// Entries listed, including hidden ones.
    pub total: usize,
    /// Rows selected.
    pub selected: usize,
    /// The name filter's text, while one applies.
    pub filter: Option<&'a str>,
    /// The listing is still arriving.
    pub loading: bool,
    /// Total size of the files shown, when any is known.
    pub shown_size: Option<u64>,
    /// Total size of the selected files, when any is known.
    pub selected_size: Option<u64>,
    /// The name being edited in place, if any.
    pub renaming: Option<&'a str>,
}

/// The status line's words: the selection, when there is one, and the
/// totals ("12 items (150.4 MiB), 1 hidden").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusParts {
    /// What is selected, or how to finish a rename.
    pub selection: Option<String>,
    /// The totals.
    pub total: String,
}

/// The status line's words for `facts`.
pub fn status_parts(facts: &StatusFacts<'_>) -> StatusParts {
    let items = |n: usize| plural(n, "item", "items");
    let sized = |text: String, size: Option<u64>| match size {
        Some(size) => format!("{text} ({})", format::size(size)),
        None => text,
    };
    // While a name is edited, the status says how to finish, as Dolphin's does.
    let renaming = facts.renaming.map(|name| {
        format!("Renaming \u{201c}{name}\u{201d}: Enter keeps the new name, Esc cancels.")
    });
    let selection = renaming.or_else(|| {
        (facts.selected > 0).then(|| {
            sized(
                format!(
                    "{} {} selected",
                    count(facts.selected),
                    items(facts.selected)
                ),
                facts.selected_size,
            )
        })
    });
    let mut total = match facts.filter {
        Some(text) => format!(
            "{} of {} {} match \u{201c}{text}\u{201d}",
            count(facts.shown),
            count(facts.unfiltered),
            items(facts.unfiltered)
        ),
        None => sized(
            format!("{} {}", count(facts.shown), items(facts.shown)),
            facts.shown_size,
        ),
    };
    let hidden = facts.total.saturating_sub(facts.unfiltered);
    if hidden > 0 {
        total.push_str(&format!(", {} hidden", count(hidden)));
    }
    if facts.loading {
        total.push_str(" \u{2014} loading\u{2026}");
    }
    StatusParts { selection, total }
}

/// The total size of the files among `rows` whose size is known, if any
/// is; folders count nothing.
pub fn total_size(snapshot: &Snapshot, rows: impl IntoIterator<Item = usize>) -> Option<u64> {
    let mut total = None;
    for row in rows {
        if snapshot.is_container(row) {
            continue;
        }
        if let Some(meta) = snapshot.metadata(row) {
            total = Some(total.unwrap_or(0) + meta.size);
        }
    }
    total
}

/// Which parts of the status line fit a pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusZones {
    /// The free space on the right.
    pub free_space: bool,
    /// The totals after the selection.
    pub totals: bool,
}

impl StatusZones {
    /// The free space leaves first; then, while entries are selected, the
    /// totals, so the selection is never cut off.
    pub fn for_width(width: f32, selected: bool) -> Self {
        Self {
            free_space: width >= size::STATUS_FREE_SPACE_MIN,
            totals: !selected || width >= size::STATUS_SELECTION_ONLY_BELOW,
        }
    }
}

/// The free space on the status line.
pub fn free_text(total: u64, free: u64, volume: Option<&str>) -> String {
    let text = format!("Free: {} of {}", format::size(free), format::size(total));
    match volume {
        Some(name) => format!("{text} ({name})"),
        None => text,
    }
}

/// Which optional detail columns show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Columns {
    /// Size.
    pub size: bool,
    /// Date modified.
    pub modified: bool,
    /// Type.
    pub kind: bool,
}

impl Columns {
    /// Only the columns chosen in Settings, of those that fit.
    pub fn within(self, chosen: DetailColumns) -> Self {
        Self {
            size: self.size && chosen.size,
            modified: self.modified && chosen.modified,
            kind: self.kind && chosen.kind,
        }
    }

    /// Narrow panes drop type, modified and then size to preserve names.
    pub fn for_width(width: f32) -> Self {
        Self {
            size: width >= size::WIDTH_FOR_SIZE,
            modified: width >= size::WIDTH_FOR_MODIFIED,
            kind: width >= size::WIDTH_FOR_TYPE,
        }
    }
}

/// A pause longer than this starts a new type-ahead search.
pub const TYPE_AHEAD_PAUSE: Duration = Duration::from_millis(1000);

/// What a type-ahead buffer searches for: the prefix, and whether to start
/// after the focused row. Pressing one letter repeatedly cycles through the
/// names starting with it; anything else narrows the search in place.
pub fn type_ahead_query(typed: &str) -> (&str, bool) {
    let mut chars = typed.chars();
    match chars.next() {
        Some(first) if chars.all(|c| c == first) => (&typed[..first.len_utf8()], true),
        _ => (typed, false),
    }
}

/// Letters typed over the list, to jump to a name.
#[derive(Debug, Default)]
pub struct TypeAhead {
    typed: String,
    at: Option<Instant>,
}

impl TypeAhead {
    /// The row `typed`, added to what came before, jumps to from `focus`.
    pub fn find(
        &mut self,
        typed: &str,
        now: Instant,
        snapshot: &Snapshot,
        focus: Option<usize>,
    ) -> Option<usize> {
        if self
            .at
            .is_none_or(|at| now.duration_since(at) > TYPE_AHEAD_PAUSE)
        {
            self.typed.clear();
        }
        self.typed.push_str(typed);
        self.at = Some(now);
        let (prefix, advance) = type_ahead_query(&self.typed);
        let from = focus.map_or(0, |row| row + usize::from(advance));
        snapshot.find_prefix(prefix.as_bytes(), from)
    }
}

/// Whether a typed location is a trash address, which Rustphin opens itself,
/// as Dolphin does, instead of asking kio-fuse to mount it.
pub fn names_trash(typed: &str) -> bool {
    typed
        .split_once(':')
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("trash"))
}

/// What text typed into the location bar opens (W-140).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    /// Nothing was typed.
    Nothing,
    /// The trash, which Rustphin opens itself.
    Trash,
    /// A local folder.
    Folder(PathBuf),
    /// A remote address, mounted through kio-fuse first (W-080).
    Remote(RemoteAddress),
    /// Text written as an address (`scheme://…`) that is not one Rustphin
    /// can open.
    NotAnAddress,
}

/// Reads typed text, trimmed, in this order: a trash address, a local file
/// URI, a remote address, then a path, `~` standing for `home` and a
/// relative path starting at `base`.
pub fn interpret_typed(typed: &str, base: &Path, home: Option<&Path>) -> Typed {
    let typed = typed.trim();
    if names_trash(typed) {
        return Typed::Trash;
    }
    if let Some(path) = rustphin_platform::uri::path_from_uri(typed.as_bytes()) {
        return Typed::Folder(path);
    }
    if typed.contains("://") {
        return RemoteAddress::parse(typed).map_or(Typed::NotAnAddress, Typed::Remote);
    }
    expand_path(typed, Some(base), home).map_or(Typed::Nothing, Typed::Folder)
}

/// What the location bar says of text that is not an address.
pub fn not_an_address(typed: &str) -> String {
    format!(
        "\u{201c}{}\u{201d} is not an address Rustphin can open.",
        typed.trim()
    )
}

/// Turns typed text into an absolute path: expands a leading `~` to `home`
/// and resolves relative paths against `base`. `None` for empty input.
pub fn expand_path(typed: &str, base: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    if typed.is_empty() {
        return None;
    }
    let path = match (typed.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home.to_owned(),
        (Some(rest), Some(home)) if rest.starts_with('/') => {
            home.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(typed),
    };
    Some(match base {
        Some(base) if path.is_relative() => base.join(path),
        _ => path,
    })
}

/// The folders the location bar offers, outermost first, with their labels.
/// Inside a remote folder the trail starts at its mount, labelled with the
/// remote address, instead of kio-fuse's local mount path.
pub fn crumb_trail(path: &Path, remote: Option<(&Path, &str)>) -> Vec<(PathBuf, String)> {
    let mut trail: Vec<(PathBuf, String)> = path
        .ancestors()
        .map(|ancestor| {
            let label = match ancestor.file_name() {
                Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
                None => "/".to_owned(),
            };
            (ancestor.to_path_buf(), label)
        })
        .collect();
    trail.reverse();
    if let Some((root, address)) = remote.filter(|(root, _)| path.starts_with(root)) {
        trail.retain(|(ancestor, _)| ancestor.starts_with(root));
        if let Some(first) = trail.first_mut() {
            first.1 = address.to_owned();
        }
    }
    trail
}

/// How many trailing folders of the location fit the view; the rest collapse
/// into an ellipsis. The current folder is always shown.
pub fn crumbs_for_width(width: f32) -> usize {
    match width {
        w if w >= 1000. => 4,
        w if w >= 780. => 3,
        w if w >= 520. => 2,
        _ => 1,
    }
}

/// The trash folder's title: the home trash, or the volume it belongs to.
pub fn trash_label(dir: &Path) -> String {
    let in_volume = dir
        .file_name()
        .is_some_and(|name| name.as_encoded_bytes().starts_with(b".Trash"));
    let parent = dir.parent();
    // `$top/.Trash/$uid` or `$top/.Trash-$uid`.
    let top = match parent {
        Some(shared) if shared.file_name() == Some(OsStr::new(".Trash")) => shared.parent(),
        Some(top) if in_volume => Some(top),
        _ => None,
    };
    match top {
        Some(top) => {
            let name = top
                .file_name()
                .map_or_else(|| "/".into(), |name| display_name(name.as_encoded_bytes()));
            format!("Trash on \u{201c}{name}\u{201d}")
        }
        None => "Home trash".to_owned(),
    }
}

/// Opacity of an entry that was cut and is waiting to be pasted.
pub const CUT_OPACITY: f32 = 0.5;

/// How faded an entry is drawn: 1 when not at all. Cut entries are on their
/// way out: half opacity until pasted. Hidden ones are faded as in Dolphin,
/// by `hidden_opacity`, except while selected, where the selection's own
/// contrast must hold.
pub fn entry_opacity(hidden: bool, cut: bool, selected: bool, hidden_opacity: f32) -> f32 {
    let hidden = if hidden && !selected {
        hidden_opacity
    } else {
        1.
    };
    let cut = if cut { CUT_OPACITY } else { 1. };
    hidden * cut
}

/// Selects the entries named `names`, replacing the selection.
pub fn select_names(selection: &mut Selection, snapshot: &Snapshot, names: &[Vec<u8>]) {
    let mut selected = false;
    for row in snapshot.rows_of(names).into_iter().flatten() {
        if selected {
            selection.toggle(snapshot, row);
        } else {
            selection.select_only(snapshot, row);
            selected = true;
        }
    }
}

/// The size to ask the icon theme for, given the size drawn. Themes ship a
/// fixed set of sizes; asking for one of them avoids scaling a neighbour.
pub fn theme_icon_size(drawn: f32) -> u32 {
    const SIZES: [u32; 9] = [16, 22, 24, 32, 48, 64, 96, 128, 256];
    SIZES
        .into_iter()
        .find(|&size| size as f32 >= drawn)
        .unwrap_or(256)
}

/// Thumbnails are not drawn for icons smaller than this: a picture that
/// small says less than the type's icon.
pub const MIN_THUMBNAIL_EDGE: f32 = 24.;

/// What an empty pane says: an icon, a heading and what can be done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptyState {
    /// Lucide icon name.
    pub icon: &'static str,
    /// The heading.
    pub heading: String,
    /// Why, and what can be done.
    pub text: String,
    /// Offer to show hidden files.
    pub offer_hidden: bool,
    /// Offer to clear the filter.
    pub offer_clear_filter: bool,
}

/// The empty pane's words for a snapshot with no rows. `total` counts the
/// entries listed, hidden ones included; `unfiltered` the rows there would
/// be without the filter.
pub fn empty_state(
    loading: bool,
    total: usize,
    unfiltered: usize,
    filter: Option<&str>,
    path: &Path,
    trash: bool,
) -> EmptyState {
    if let Some(text) = filter.filter(|_| !loading) {
        return EmptyState {
            icon: "search-x",
            heading: format!("Nothing matches \u{201c}{text}\u{201d}"),
            text: format!(
                "Change the filter, or clear it to see all {} {}.",
                count(unfiltered),
                plural(unfiltered, "item", "items")
            ),
            offer_hidden: false,
            offer_clear_filter: true,
        };
    }
    if loading {
        return EmptyState {
            icon: "loader",
            heading: "Loading\u{2026}".to_owned(),
            text: format!(
                "Reading {}. Entries appear as soon as the first ones are read.",
                path.display()
            ),
            offer_hidden: false,
            offer_clear_filter: false,
        };
    }
    if total > 0 {
        return EmptyState {
            icon: "eye-off",
            heading: "This folder has only hidden items".to_owned(),
            text: format!(
                "{} hidden {} not shown.",
                count(total),
                plural(total, "item is", "items are")
            ),
            offer_hidden: true,
            offer_clear_filter: false,
        };
    }
    EmptyState {
        icon: "folder-open",
        heading: if trash {
            "The trash is empty".to_owned()
        } else {
            "This folder is empty".to_owned()
        },
        text: if trash {
            String::new()
        } else {
            "Drop files here, or make a new folder with F10.".to_owned()
        },
        offer_hidden: false,
        offer_clear_filter: false,
    }
}

/// The scroll benchmark's verdict on frame intervals in milliseconds: an
/// interval over 1.5 times the median is a dropped frame, and at most 5%
/// may drop (D-019).
pub fn frame_report(intervals: &[f64]) -> String {
    if intervals.is_empty() {
        return "no frames".to_owned();
    }
    let mut sorted = intervals.to_vec();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[sorted.len() / 2];
    let dropped = intervals.iter().filter(|&&ms| ms > median * 1.5).count();
    let percent = dropped as f64 * 100. / intervals.len() as f64;
    format!(
        "{} intervals, median {median:.2} ms, max {:.2} ms, {dropped} dropped ({percent:.2}%) => {}",
        intervals.len(),
        sorted[sorted.len() - 1],
        if percent <= 5. { "PASS" } else { "FAIL" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn failure(path: &str, error: LoadError, moved: bool) -> Failure {
        Failure {
            path: PathBuf::from(path),
            error,
            moved,
        }
    }

    #[test]
    fn messages_say_what_happened_to_which_folder() {
        assert_eq!(
            failure("/data/locked", LoadError::PermissionDenied, false).message(),
            "Could not open /data/locked: you do not have permission to open this location."
        );
        assert_eq!(
            failure("/data/old", LoadError::NotFound, true).message(),
            "/data/old no longer exists. Showing the nearest folder that does."
        );
    }

    #[test]
    fn a_folder_that_could_not_be_opened_is_explained_in_place() {
        let locked = failure("/data/locked", LoadError::PermissionDenied, false);
        let here = Some(Path::new("/data/locked"));
        assert!(locked.in_place(here, false));
        assert!(!locked.in_place(here, true));
        assert!(!locked.in_place(Some(Path::new("/data")), false));
        assert!(!failure("/data/locked", LoadError::NotFound, true).in_place(here, false));
    }

    #[test]
    fn the_in_place_error_says_why_and_where_its_button_leads() {
        let locked = failure("/data/locked", LoadError::PermissionDenied, false);
        assert_eq!(
            locked.explanation(Way::Back),
            "You do not have permission to open /data/locked. Ask its owner \
             for access, or go back to the previous folder."
        );
        assert_eq!(
            locked.explanation(Way::Up),
            "You do not have permission to open /data/locked. Ask its owner \
             for access, or go up to the folder that contains it."
        );
        assert_eq!(
            locked.explanation(Way::Nowhere),
            "You do not have permission to open /data/locked. Ask its owner for access."
        );
        assert_eq!(
            failure("/notes.txt", LoadError::NotADirectory, false).explanation(Way::Nowhere),
            "/notes.txt is a file, not a folder. Open it from the folder that contains it."
        );
        assert_eq!(Way::out(true, true), Way::Back);
        assert_eq!(Way::out(false, true), Way::Up);
        assert_eq!(Way::out(false, false).label(), None);
    }

    #[test]
    fn the_status_line_names_the_failure_while_it_is_shown_in_place() {
        let status = |error| failure("/x", error, false).status();
        assert_eq!(status(LoadError::PermissionDenied), "Not readable");
        assert_eq!(status(LoadError::NotFound), "Does not exist");
        assert_eq!(status(LoadError::NotADirectory), "Not a folder");
        assert_eq!(status(LoadError::Io("broken".into())), "Could not be read");
    }

    #[test]
    fn hidden_entries_are_faded_unless_selected_and_cut_ones_fade_further() {
        let hidden = 0.6;
        assert_eq!(entry_opacity(false, false, false, hidden), 1.);
        assert_eq!(entry_opacity(true, false, false, hidden), hidden);
        assert_eq!(entry_opacity(true, false, true, hidden), 1.);
        assert_eq!(entry_opacity(false, true, true, hidden), CUT_OPACITY);
        assert_eq!(
            entry_opacity(true, true, false, hidden),
            hidden * CUT_OPACITY
        );
    }

    #[test]
    fn repeating_one_letter_cycles_and_a_longer_prefix_narrows() {
        assert_eq!(type_ahead_query("f"), ("f", true));
        assert_eq!(type_ahead_query("fff"), ("f", true));
        assert_eq!(type_ahead_query("fi"), ("fi", false));
        assert_eq!(type_ahead_query("\u{e9}\u{e9}"), ("\u{e9}", true));
        assert_eq!(type_ahead_query(""), ("", false));
    }

    #[test]
    fn icons_are_requested_at_the_theme_size_at_or_above_the_drawn_size() {
        assert_eq!(theme_icon_size(16.), 16);
        assert_eq!(theme_icon_size(20.), 22);
        assert_eq!(theme_icon_size(192.), 256);
    }

    #[test]
    fn narrower_views_show_fewer_folders_of_the_location_but_always_one() {
        assert_eq!(crumbs_for_width(1280.), 4);
        assert_eq!(crumbs_for_width(899.), 3);
        assert_eq!(crumbs_for_width(639.), 2);
        assert_eq!(crumbs_for_width(0.), 1);
    }

    #[test]
    fn a_typed_trash_address_opens_the_trash_not_a_mount() {
        for typed in ["trash:/", "trash:", "TRASH:/", "trash:/old"] {
            assert!(names_trash(typed), "{typed}");
        }
        for typed in ["trash", "/trash", "./trash:", "sftp:/"] {
            assert!(!names_trash(typed), "{typed}");
        }
    }

    #[test]
    fn columns_leave_as_the_pane_narrows_and_only_chosen_ones_show() {
        assert!(!Columns::for_width(size::WIDTH_FOR_SIZE - 1.).size);
        let medium = Columns::for_width(size::WIDTH_FOR_TYPE - 1.);
        assert!(medium.modified && !medium.kind);
        let chosen = DetailColumns {
            size: true,
            modified: false,
            kind: true,
        };
        let wide = Columns::for_width(1200.).within(chosen);
        assert!(wide.size && !wide.modified && wide.kind);
    }

    #[test]
    fn trash_directories_are_named_after_their_volume() {
        assert_eq!(
            trash_label(Path::new("/mnt/data/.Trash-1000")),
            "Trash on \u{201c}data\u{201d}"
        );
        assert_eq!(
            trash_label(Path::new("/media/usb stick/.Trash/1000")),
            "Trash on \u{201c}usb stick\u{201d}"
        );
        assert_eq!(
            trash_label(Path::new("/home/u/.local/share/Trash")),
            "Home trash"
        );
    }

    #[test]
    fn the_status_counts_matches_hidden_and_selected_entries() {
        const MIB: u64 = 1024 * 1024;
        let facts = |shown, unfiltered, total, selected, filter| StatusFacts {
            shown,
            unfiltered,
            total,
            selected,
            filter,
            ..StatusFacts::default()
        };
        let parts = |facts: &StatusFacts<'_>| {
            let parts = status_parts(facts);
            (parts.selection, parts.total)
        };
        assert_eq!(parts(&facts(7, 7, 7, 0, None)), (None, "7 items".into()));
        assert_eq!(
            parts(&StatusFacts {
                shown_size: Some(150 * MIB),
                selected_size: Some(6 * MIB),
                ..facts(7, 7, 9, 2, None)
            }),
            (
                Some("2 items selected (6.0 MiB)".into()),
                "7 items (150.0 MiB), 2 hidden".into()
            )
        );
        assert_eq!(
            parts(&facts(2, 7, 9, 0, Some("txt"))),
            (
                None,
                "2 of 7 items match \u{201c}txt\u{201d}, 2 hidden".into()
            )
        );
        let loading = StatusFacts {
            loading: true,
            ..facts(1, 1, 1, 0, None)
        };
        assert_eq!(parts(&loading).1, "1 item \u{2014} loading\u{2026}");
        let renaming = StatusFacts {
            renaming: Some("a.jpg"),
            ..facts(1, 1, 1, 1, None)
        };
        assert_eq!(
            parts(&renaming).0.as_deref(),
            Some("Renaming \u{201c}a.jpg\u{201d}: Enter keeps the new name, Esc cancels.")
        );
    }

    #[test]
    fn the_status_line_drops_free_space_then_the_totals_as_the_pane_narrows() {
        let wide = StatusZones::for_width(size::STATUS_FREE_SPACE_MIN, true);
        assert!(wide.free_space && wide.totals);
        let narrow = StatusZones::for_width(size::STATUS_SELECTION_ONLY_BELOW - 1., true);
        assert!(!narrow.free_space && !narrow.totals);
        assert!(StatusZones::for_width(size::STATUS_SELECTION_ONLY_BELOW - 1., false).totals);
        let gib = 1024 * 1024 * 1024;
        assert_eq!(
            free_text(4 * gib, gib, Some("Data")),
            "Free: 1.0 GiB of 4.0 GiB (Data)"
        );
    }

    #[test]
    fn the_crumb_trail_starts_at_a_remote_mount() {
        let labels = |path: &str, remote: Option<(&Path, &str)>| -> Vec<String> {
            crumb_trail(Path::new(path), remote)
                .into_iter()
                .map(|(_, label)| label)
                .collect()
        };
        assert_eq!(labels("/srv/data", None), ["/", "srv", "data"]);
        let mount = Some((
            Path::new("/run/kio-fuse-x/sftp/host/projects"),
            "sftp://me@host/projects",
        ));
        assert_eq!(
            labels("/run/kio-fuse-x/sftp/host/projects/sub dir", mount),
            ["sftp://me@host/projects", "sub dir"]
        );
    }

    #[test]
    fn an_empty_pane_says_why_and_what_to_do() {
        let here = Path::new("/srv");
        let empty = empty_state(false, 0, 0, None, here, false);
        assert_eq!(empty.heading, "This folder is empty");
        assert!(empty.text.contains("F10"));
        let hidden = empty_state(false, 3, 0, None, here, false);
        assert!(hidden.offer_hidden);
        assert_eq!(hidden.text, "3 hidden items are not shown.");
        let filtered = empty_state(false, 5, 5, Some("x"), here, false);
        assert!(filtered.offer_clear_filter);
        assert_eq!(filtered.heading, "Nothing matches \u{201c}x\u{201d}");
        assert!(
            empty_state(true, 0, 0, None, here, false)
                .text
                .starts_with("Reading /srv.")
        );
        assert_eq!(empty_state(false, 0, 0, None, here, true).text, "");
    }

    #[test]
    fn more_than_five_percent_dropped_frames_fail() {
        let mut steady = vec![16.7; 100];
        assert!(frame_report(&steady).ends_with("PASS"));
        steady[..6].fill(40.);
        assert!(frame_report(&steady).contains("6 dropped (6.00%) => FAIL"));
        assert_eq!(frame_report(&[]), "no frames");
    }

    const BASE: &str = "/srv/data";
    const HOME: &str = "/home/me";

    fn typed(text: &str) -> Typed {
        interpret_typed(text, Path::new(BASE), Some(Path::new(HOME)))
    }

    #[test]
    fn typed_paths_are_made_absolute() {
        let base = Path::new(BASE);
        let home = Some(Path::new(HOME));
        assert_eq!(expand_path("", Some(base), home), None);
        assert_eq!(
            expand_path("/etc", Some(base), home),
            Some(PathBuf::from("/etc"))
        );
        assert_eq!(
            expand_path("sub/dir", Some(base), home),
            Some(PathBuf::from("/srv/data/sub/dir"))
        );
    }

    #[test]
    fn a_tilde_stands_for_the_home_folder() {
        let base = Some(Path::new(BASE));
        let home = Some(Path::new(HOME));
        assert_eq!(expand_path("~", base, home), Some(PathBuf::from(HOME)));
        assert_eq!(
            expand_path("~/Music", base, home),
            Some(PathBuf::from("/home/me/Music"))
        );
        // Another user's home is not expanded: it is a relative name.
        assert_eq!(
            expand_path("~bob", base, home),
            Some(PathBuf::from("/srv/data/~bob"))
        );
        // Without a home, `~` is an ordinary name.
        assert_eq!(
            expand_path("~", base, None),
            Some(PathBuf::from("/srv/data/~"))
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn typed_text_is_read_as_trash_then_file_uri_then_remote_then_path() {
        assert_eq!(typed("  trash:/  "), Typed::Trash);
        assert_eq!(
            typed("file:///tmp/a%20b"),
            Typed::Folder(PathBuf::from("/tmp/a b"))
        );
        assert_eq!(
            typed("sftp://me@host/srv"),
            Typed::Remote(RemoteAddress::parse("sftp://me@host/srv").unwrap())
        );
        assert_eq!(typed("~/x"), Typed::Folder(PathBuf::from("/home/me/x")));
        assert_eq!(typed("sub"), Typed::Folder(PathBuf::from("/srv/data/sub")));
        assert_eq!(typed("   "), Typed::Nothing);
    }

    #[test]
    fn a_malformed_address_is_not_taken_for_a_relative_path() {
        // A scheme must start with a letter: this was opened as the
        // folder "/srv/data/1sftp:/host" before W-140.
        assert_eq!(typed("1sftp://host/srv"), Typed::NotAnAddress);
        assert_eq!(
            not_an_address(" 1sftp://host "),
            "\u{201c}1sftp://host\u{201d} is not an address Rustphin can open."
        );
    }
}
