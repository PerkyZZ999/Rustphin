//! The window's tabs and split views, free of any toolkit (W-114e, W-130):
//! a [`Workspace`] of tabs that each hold one or two [`PaneSlot`]s, the
//! active tab and pane, splitting and joining, the tabs closed lately for
//! Ctrl+Shift+T, the session they make; which tab Alt+number picks, how
//! wide tabs are, and copying or moving to the other view (Shift+F5/F6).
//!
//! Folders are kept as paths, never as text, so a name that is not UTF-8
//! survives a split, a reopened tab and a restart (W-077f).

use std::path::{Path, PathBuf};

use rustphin_core::{display_name, ops::Request};
use rustphin_platform::{remote::RemoteAddress, uri::path_from_uri};

use crate::{
    location::Location,
    metrics::size,
    session::{Session, SessionTab},
    tabs::Tabs,
};

/// Whether the other view receives a copy or the entries themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    /// Shift+F5.
    Copy,
    /// Shift+F6.
    Move,
}

/// The operation that sends the active view's selection into the other
/// view's folder, as Dolphin's copy and move to the other view, or why
/// there is nothing to do.
pub fn transfer_to_other_view(
    transfer: Transfer,
    selected: Vec<PathBuf>,
    from: Option<&Path>,
    to: Option<&Path>,
) -> Result<Request, &'static str> {
    let Some(into) = to else {
        return Err("Split the view with F3 to copy or move to the other side.");
    };
    if selected.is_empty() {
        return Err("Select what to copy or move first.");
    }
    if from == Some(into) {
        return Err(
            "Both views show the same folder. Open another folder in the other view first.",
        );
    }
    let into = into.to_path_buf();
    Ok(match transfer {
        Transfer::Copy => Request::Copy {
            sources: selected,
            into,
        },
        Transfer::Move => Request::Move {
            sources: selected,
            into,
        },
    })
}

/// How many closed tabs Ctrl+Shift+T can bring back.
pub const CLOSED_TAB_LIMIT: usize = 6;

/// The tab Alt+`number` activates, as in Dolphin: Alt+1 to Alt+9 pick that
/// tab if it exists, Alt+0 the last one.
pub fn tab_for_number(number: usize, tabs: usize) -> Option<usize> {
    match number {
        0 => tabs.checked_sub(1),
        n => (n <= tabs).then(|| n - 1),
    }
}

/// How wide each of `count` tabs is in a strip `available` pixels wide:
/// 160 while they fit beside the "+", shrinking evenly to 96, never less
/// (past that the strip scrolls).
pub fn tab_width(available: f32, count: usize) -> f32 {
    let room = (available - size::TAB_ADD).max(0.) / count.max(1) as f32;
    room.clamp(size::TAB_MIN, size::TAB)
}

/// A folder's label: its last component, or "/".
pub fn folder_label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
        None => "/".to_owned(),
    }
}

/// Where a pane was asked to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneStart {
    /// A local folder or a remote address.
    At(Location),
    /// Text the pane resolves itself, such as `trash:/` or a relative path.
    Typed(String),
}

impl PaneStart {
    /// Reads the text the interface hands over: a `file:` URI (which keeps
    /// every byte of a name), an absolute path, a remote address, or else
    /// text the pane resolves.
    pub fn parse(text: &str) -> Self {
        if let Some(path) = path_from_uri(text.as_bytes()) {
            Self::At(Location::Folder(path))
        } else if text.starts_with('/') {
            Self::At(Location::Folder(PathBuf::from(text)))
        } else if let Some(address) = RemoteAddress::parse(text).filter(|_| text.contains("://")) {
            Self::At(Location::Remote(address))
        } else {
            Self::Typed(text.to_owned())
        }
    }

    /// The text a pane opens: a byte-preserving `file://` URI, a remote
    /// address without password, or the typed text. [`PaneStart::parse`]
    /// reads it back unchanged.
    pub fn to_text(&self) -> String {
        match self {
            Self::At(location) => location.to_saved(),
            Self::Typed(text) => text.clone(),
        }
    }

    /// What a tab is called before its pane has said: the folder's name.
    fn title(&self) -> String {
        match self {
            Self::At(Location::Folder(path)) => folder_label(path),
            Self::At(Location::Remote(address)) => {
                crate::remote::short_name(&address.to_string()).to_owned()
            }
            Self::Typed(text) => folder_label(Path::new(text)),
        }
    }
}

impl From<Location> for PaneStart {
    fn from(location: Location) -> Self {
        Self::At(location)
    }
}

/// One pane of a tab: where it started and what it last reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSlot {
    start: PaneStart,
    folder: Option<PathBuf>,
    title: String,
    icon: String,
}

impl PaneSlot {
    fn new(start: PaneStart) -> Self {
        Self {
            title: start.title(),
            start,
            folder: None,
            icon: String::new(),
        }
    }

    /// Where the pane was asked to open.
    pub fn start(&self) -> &PaneStart {
        &self.start
    }

    /// The folder it shows, once it has said.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Its title, the folder's name until the pane says otherwise.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The icon name it reported; empty until then.
    pub fn icon(&self) -> &str {
        &self.icon
    }

    /// Where the pane goes on from: its folder, else where it started.
    fn resumed(&self) -> PaneStart {
        match &self.folder {
            Some(path) => PaneStart::At(Location::Folder(path.clone())),
            None => self.start.clone(),
        }
    }

    /// Opens a pane where this one is, with its title and icon.
    fn resumed_slot(&self) -> Self {
        Self {
            start: self.resumed(),
            ..self.clone()
        }
    }

    /// How the pane is saved: a folder under a remote mount by the mount's
    /// address (`remote`), else its path. Typed or relative starts that the
    /// pane has not yet resolved are not saved.
    fn saved(&self, remote: &impl Fn(&Path) -> Option<RemoteAddress>) -> Option<Location> {
        let path = match (&self.folder, &self.start) {
            (Some(path), _) => path.clone(),
            (None, PaneStart::At(Location::Folder(path))) if path.is_absolute() => path.clone(),
            (None, PaneStart::At(Location::Remote(address))) => {
                return Some(Location::Remote(address.clone()));
            }
            _ => return None,
        };
        Some(remote(&path).map_or(Location::Folder(path), Location::Remote))
    }
}

/// A tab as it is opened: one or two panes and which has the keyboard.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabStart {
    /// One pane, or two when the tab is split.
    pub panes: Vec<PaneStart>,
    /// Index of the pane that has the keyboard.
    pub active_pane: usize,
}

impl From<&SessionTab> for TabStart {
    fn from(tab: &SessionTab) -> Self {
        Self {
            panes: tab.panes.iter().cloned().map(PaneStart::At).collect(),
            active_pane: tab.active_pane,
        }
    }
}

/// One tab of a [`Workspace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTab {
    key: u32,
    panes: Vec<PaneSlot>,
    active_pane: usize,
    select: Vec<String>,
}

impl WorkspaceTab {
    /// A key that stays with the tab while it moves; never reused.
    pub fn key(&self) -> u32 {
        self.key
    }

    /// Its panes, left to right: one, or two when split.
    pub fn panes(&self) -> &[PaneSlot] {
        &self.panes
    }

    /// Whether it shows two panes.
    pub fn is_split(&self) -> bool {
        self.panes.len() > 1
    }

    /// Index of the pane that has the keyboard.
    pub fn active_pane(&self) -> usize {
        self.active_pane
    }

    /// The pane that has the keyboard.
    pub fn active(&self) -> &PaneSlot {
        &self.panes[self.active_pane]
    }

    /// Names to select when its first pane arrives, as the window hands
    /// them over (percent-encoded bytes); kept as they are.
    pub fn select(&self) -> &[String] {
        &self.select
    }

    /// The tab as it would be opened again.
    fn resumed(&self) -> TabStart {
        let panes: Vec<PaneStart> = self.panes.iter().map(PaneSlot::resumed).collect();
        TabStart {
            active_pane: self.active_pane.min(panes.len().saturating_sub(1)),
            panes,
        }
    }

    fn saved(&self, remote: &impl Fn(&Path) -> Option<RemoteAddress>) -> SessionTab {
        SessionTab {
            panes: self
                .panes
                .iter()
                .filter_map(|pane| pane.saved(remote))
                .collect(),
            active_pane: self.active_pane,
        }
    }
}

/// What closing a tab does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closing {
    /// The tab closes and is remembered for Ctrl+Shift+T.
    Closes,
    /// It is the window's only tab: the window closes instead.
    LastTab,
    /// There is no such tab.
    Missing,
}

/// A window's tabs: each one or two panes, one tab active, and the tabs
/// closed lately. A window always has a tab.
#[derive(Debug, Clone)]
pub struct Workspace {
    tabs: Tabs<WorkspaceTab>,
    closed: Vec<TabStart>,
    next_key: u32,
}

impl Workspace {
    /// The given tabs with `active` showing and `select` chosen in the
    /// first; a tab without panes is skipped, and with none left the
    /// window opens one tab on `fallback`.
    pub fn new(
        tabs: Vec<TabStart>,
        active: usize,
        fallback: PaneStart,
        select: Vec<String>,
    ) -> Self {
        let mut next_key = 1;
        let mut items: Vec<WorkspaceTab> = tabs
            .into_iter()
            .filter(|tab| !tab.panes.is_empty())
            .map(|tab| Self::make_tab(&mut next_key, tab))
            .collect();
        let active = if items.is_empty() {
            items.push(Self::make_tab(
                &mut next_key,
                TabStart {
                    panes: vec![fallback],
                    active_pane: 0,
                },
            ));
            0
        } else {
            active
        };
        if let Some(first) = items.first_mut() {
            first.select = select;
        }
        let tabs = match Tabs::from_parts(items, active) {
            Some(tabs) => tabs,
            // Not reached: a tab was added above when there was none.
            None => Tabs::new(Self::make_tab(&mut next_key, TabStart::default())),
        };
        Self {
            tabs,
            closed: Vec::new(),
            next_key,
        }
    }

    fn make_tab(next_key: &mut u32, tab: TabStart) -> WorkspaceTab {
        let key = *next_key;
        *next_key += 1;
        let mut panes: Vec<PaneSlot> = tab.panes.into_iter().take(2).map(PaneSlot::new).collect();
        if panes.is_empty() {
            panes.push(PaneSlot::new(PaneStart::Typed(String::new())));
        }
        WorkspaceTab {
            key,
            active_pane: tab.active_pane.min(panes.len() - 1),
            panes,
            select: Vec::new(),
        }
    }

    /// Number of tabs; never zero.
    #[allow(clippy::len_without_is_empty)] // A window always has a tab.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Index of the active tab.
    pub fn active_index(&self) -> usize {
        self.tabs.active_index()
    }

    /// The active tab.
    pub fn active(&self) -> &WorkspaceTab {
        self.tabs.active()
    }

    /// The tabs in order.
    pub fn tabs(&self) -> impl Iterator<Item = &WorkspaceTab> {
        self.tabs.iter()
    }

    /// The tab at `row`.
    pub fn tab(&self, row: usize) -> Option<&WorkspaceTab> {
        self.tabs.iter().nth(row)
    }

    /// The row of the tab with `key`.
    pub fn row_of(&self, key: u32) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.key == key)
    }

    /// The row a new or reopened tab is inserted at: after the active one.
    pub fn insert_row(&self) -> usize {
        self.tabs.active_index() + 1
    }

    /// Opens a tab on `start` at [`Workspace::insert_row`], selecting
    /// `select` there; the active tab stays unless `activate`.
    pub fn open(&mut self, start: PaneStart, select: Vec<String>, activate: bool) {
        let mut tab = Self::make_tab(
            &mut self.next_key,
            TabStart {
                panes: vec![start],
                active_pane: 0,
            },
        );
        tab.select = select;
        self.insert(tab, activate);
    }

    fn insert(&mut self, tab: WorkspaceTab, activate: bool) {
        let previous = self.tabs.active_index();
        self.tabs.open(tab);
        if !activate {
            self.tabs.activate(previous);
        }
    }

    /// What closing the tab at `row` would do.
    pub fn closing(&self, row: usize) -> Closing {
        if row >= self.tabs.len() {
            Closing::Missing
        } else if self.tabs.len() == 1 {
            Closing::LastTab
        } else {
            Closing::Closes
        }
    }

    /// Closes the tab at `row` (see [`Workspace::closing`]) and remembers
    /// it, newest first, up to [`CLOSED_TAB_LIMIT`]. A tab with nowhere to
    /// open again is not kept.
    pub fn close(&mut self, row: usize) -> Closing {
        let closing = self.closing(row);
        if closing == Closing::Closes
            && let Some(tab) = self.tabs.close(row)
        {
            let start = tab.resumed();
            if start
                .panes
                .iter()
                .any(|pane| matches!(pane, PaneStart::At(_)))
            {
                self.closed.insert(0, start);
                self.closed.truncate(CLOSED_TAB_LIMIT);
            }
        }
        closing
    }

    /// Whether a closed tab can be opened again.
    pub fn can_reopen(&self) -> bool {
        !self.closed.is_empty()
    }

    /// Opens the most recently closed tab again, split as it was and with
    /// the same pane active, at [`Workspace::insert_row`]; false when
    /// there is none.
    pub fn reopen(&mut self) -> bool {
        if self.closed.is_empty() {
            return false;
        }
        let start = self.closed.remove(0);
        let tab = Self::make_tab(&mut self.next_key, start);
        self.insert(tab, true);
        true
    }

    /// Makes the tab at `row` active, if there is one.
    pub fn activate(&mut self, row: usize) {
        self.tabs.activate(row);
    }

    /// Activates the tab Alt+`number` picks; false when there is none.
    pub fn activate_number(&mut self, number: usize) -> bool {
        match tab_for_number(number, self.tabs.len()) {
            Some(row) => {
                self.tabs.activate(row);
                true
            }
            None => false,
        }
    }

    /// Makes the next (`+1`) or previous (`-1`) tab active, wrapping.
    pub fn cycle(&mut self, step: isize) {
        self.tabs.cycle(step);
    }

    /// Whether the tab at `from` can move to `to`.
    pub fn can_move(&self, from: usize, to: usize) -> bool {
        let len = self.tabs.len();
        from < len && to < len && from != to
    }

    /// Where moving the active tab by `step` takes it, if it can move.
    pub fn shift_target(&self, step: isize) -> Option<(usize, usize)> {
        let from = self.tabs.active_index();
        let to = from.checked_add_signed(step)?;
        self.can_move(from, to).then_some((from, to))
    }

    /// Moves the tab at `from` to `to`, keeping the same tab active.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        self.tabs.move_tab(from, to);
    }

    /// Records what the pane on `side` of the tab with `key` shows.
    pub fn report(
        &mut self,
        key: u32,
        side: usize,
        folder: Option<PathBuf>,
        title: String,
        icon: String,
    ) {
        if let Some(pane) = self.tab_mut(key).and_then(|tab| tab.panes.get_mut(side)) {
            pane.folder = folder;
            pane.title = title;
            pane.icon = icon;
        }
    }

    /// Gives the keyboard to the pane on `side` of the tab with `key`.
    pub fn set_active_pane(&mut self, key: u32, side: usize) {
        if let Some(tab) = self.tab_mut(key) {
            tab.active_pane = side.min(tab.panes.len() - 1);
        }
    }

    fn tab_mut(&mut self, key: u32) -> Option<&mut WorkspaceTab> {
        let row = self.row_of(key)?;
        self.tabs.get_mut(row)
    }

    /// Splits the active tab, the new pane opening where the active one
    /// is and taking the keyboard, as in Dolphin; or joins it, keeping the
    /// pane that has the keyboard, which goes on from where it is.
    pub fn toggle_split(&mut self) {
        let row = self.tabs.active_index();
        let Some(tab) = self.tabs.get_mut(row) else {
            return;
        };
        if tab.panes.len() == 1 {
            let second = tab.panes[0].resumed_slot();
            tab.panes.push(second);
            tab.active_pane = 1;
        } else {
            let kept = tab.panes.swap_remove(tab.active_pane.min(1));
            tab.panes = vec![kept.resumed_slot()];
            tab.active_pane = 0;
        }
    }

    /// The session these tabs make: each pane by its folder, a folder
    /// under a remote mount by the address `remote` gives for it. Tabs
    /// with nothing to save are left out.
    pub fn session(&self, remote: impl Fn(&Path) -> Option<RemoteAddress>) -> Session {
        Session::new(
            self.tabs
                .iter()
                .map(|tab| tab.saved(&remote))
                .filter(|tab| !tab.panes.is_empty())
                .collect(),
            self.tabs.active_index(),
        )
    }

    /// The operation that copies or moves `selected` from the active pane
    /// into the other pane of the active tab, or why there is none.
    pub fn transfer(
        &self,
        transfer: Transfer,
        selected: Vec<PathBuf>,
    ) -> Result<Request, &'static str> {
        let tab = self.tabs.active();
        let here = tab.active().folder();
        let there = tab
            .is_split()
            .then(|| tab.panes[1 - tab.active_pane.min(1)].folder())
            .flatten();
        transfer_to_other_view(transfer, selected, here, there)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_other_view_receives_the_selection_or_a_reason() {
        let a = vec![PathBuf::from("/left/a"), PathBuf::from("/left/b")];
        let left = Some(Path::new("/left"));
        let right = Some(Path::new("/right"));
        assert!(matches!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, right),
            Ok(Request::Copy { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(matches!(
            transfer_to_other_view(Transfer::Move, a.clone(), left, right),
            Ok(Request::Move { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, None)
                .is_err_and(|reason| reason.contains("F3"))
        );
        assert!(
            transfer_to_other_view(Transfer::Copy, Vec::new(), left, right)
                .is_err_and(|reason| reason.contains("Select"))
        );
        assert!(
            transfer_to_other_view(Transfer::Move, a, left, left)
                .is_err_and(|reason| reason.contains("same folder"))
        );
    }

    fn folder(path: &str) -> PaneStart {
        PaneStart::At(Location::Folder(PathBuf::from(path)))
    }

    fn one(path: &str) -> TabStart {
        TabStart {
            panes: vec![folder(path)],
            active_pane: 0,
        }
    }

    fn workspace(tabs: &[&str], active: usize) -> Workspace {
        Workspace::new(
            tabs.iter().map(|path| one(path)).collect(),
            active,
            folder("/home"),
            Vec::new(),
        )
    }

    /// Each tab's pane folders (or starts) and which pane is active.
    fn shown(workspace: &Workspace) -> Vec<(Vec<PaneStart>, usize)> {
        workspace
            .tabs()
            .map(|tab| {
                let panes = tab.panes().iter().map(PaneSlot::resumed).collect();
                (panes, tab.active_pane())
            })
            .collect()
    }

    fn report(workspace: &mut Workspace, row: usize, side: usize, path: &str) {
        let key = workspace.tab(row).map(WorkspaceTab::key).unwrap_or(0);
        let title = folder_label(Path::new(path));
        workspace.report(key, side, Some(path.into()), title, "folder".into());
    }

    fn no_remote(_: &Path) -> Option<RemoteAddress> {
        None
    }

    #[test]
    fn splitting_opens_a_second_pane_where_the_active_one_is() {
        let mut tabs = workspace(&["/srv"], 0);
        report(&mut tabs, 0, 0, "/srv/data");
        tabs.toggle_split();
        let tab = tabs.active();
        assert!(tab.is_split());
        assert_eq!(tab.active_pane(), 1);
        assert_eq!(tab.panes()[1].start(), &folder("/srv/data"));
        assert_eq!(tab.panes()[1].title(), "data");
        assert_eq!(tab.panes()[0].folder(), Some(Path::new("/srv/data")));
    }

    #[test]
    fn joining_keeps_the_pane_with_the_keyboard_from_where_it_is() {
        for (active, kept) in [(0, "/left/now"), (1, "/right/now")] {
            let mut tabs = workspace(&["/left"], 0);
            tabs.toggle_split();
            report(&mut tabs, 0, 0, "/left/now");
            report(&mut tabs, 0, 1, "/right/now");
            let key = tabs.active().key();
            tabs.set_active_pane(key, active);
            tabs.toggle_split();
            let tab = tabs.active();
            assert!(!tab.is_split());
            assert_eq!(tab.active_pane(), 0);
            assert_eq!(tab.panes()[0].start(), &folder(kept));
            assert_eq!(tab.panes()[0].folder(), Some(Path::new(kept)));
        }
    }

    #[test]
    fn a_closed_split_tab_reopens_split_on_its_folders_with_the_same_pane_active() {
        let mut tabs = workspace(&["/a", "/b"], 1);
        tabs.toggle_split();
        report(&mut tabs, 1, 0, "/b/left");
        report(&mut tabs, 1, 1, "/b/right");
        let key = tabs.active().key();
        tabs.set_active_pane(key, 0);
        tabs.set_active_pane(key, 1);
        assert_eq!(tabs.close(1), Closing::Closes);
        assert_eq!(tabs.len(), 1);
        tabs.activate(0);
        assert!(tabs.reopen());
        assert_eq!(tabs.active_index(), 1);
        assert_eq!(
            shown(&tabs)[1],
            (vec![folder("/b/left"), folder("/b/right")], 1)
        );
        assert!(!tabs.reopen());
    }

    #[test]
    fn closed_tabs_are_kept_newest_first_up_to_the_limit() {
        let mut tabs = workspace(&["/keep"], 0);
        for index in 0..CLOSED_TAB_LIMIT + 2 {
            tabs.open(folder(&format!("/{index}")), Vec::new(), true);
            tabs.close(tabs.active_index());
        }
        // Text the pane has not resolved gives nowhere to reopen.
        tabs.open(PaneStart::Typed("relative".into()), Vec::new(), true);
        tabs.close(tabs.active_index());
        let mut reopened = Vec::new();
        while tabs.reopen() {
            reopened.push(tabs.active().active().start().clone());
        }
        let expected: Vec<PaneStart> = (2..CLOSED_TAB_LIMIT + 2)
            .rev()
            .map(|index| folder(&format!("/{index}")))
            .collect();
        assert_eq!(reopened, expected);
    }

    #[test]
    fn the_only_tab_closes_the_window_instead() {
        let mut tabs = workspace(&["/only"], 0);
        assert_eq!(tabs.closing(0), Closing::LastTab);
        assert_eq!(tabs.close(0), Closing::LastTab);
        assert_eq!(tabs.close(3), Closing::Missing);
        assert_eq!(tabs.len(), 1);
        assert!(!tabs.can_reopen());
    }

    #[test]
    fn moving_a_tab_keeps_the_active_tab_and_its_key() {
        let mut tabs = workspace(&["/a", "/b", "/c"], 1);
        let key = tabs.active().key();
        assert!(tabs.can_move(1, 2));
        tabs.move_tab(1, 2);
        assert_eq!(tabs.active_index(), 2);
        assert_eq!(tabs.active().key(), key);
        assert_eq!(tabs.row_of(key), Some(2));
        assert_eq!(tabs.shift_target(1), None);
        assert_eq!(tabs.shift_target(-1), Some((2, 1)));
        assert!(!tabs.can_move(0, 3));
    }

    #[test]
    fn new_tabs_open_after_the_active_one_and_take_it_only_when_asked() {
        let mut tabs = workspace(&["/a", "/b"], 0);
        assert_eq!(tabs.insert_row(), 1);
        tabs.open(folder("/x"), vec!["note".into()], false);
        assert_eq!(tabs.active_index(), 0);
        assert_eq!(
            tabs.tab(1).map(WorkspaceTab::select),
            Some(&["note".to_owned()][..])
        );
        tabs.open(folder("/y"), Vec::new(), true);
        assert_eq!(tabs.active_index(), 1);
        assert_eq!(shown(&tabs)[1].0, vec![folder("/y")]);
    }

    #[test]
    fn the_session_round_trips_folders_whose_names_are_not_utf8() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt as _};

        let raw = PathBuf::from(OsStr::from_bytes(b"/srv/caf\xe9"));
        let mut tabs = workspace(&["/a"], 0);
        tabs.toggle_split();
        let key = tabs.active().key();
        tabs.report(
            key,
            1,
            Some(raw.clone()),
            "caf\u{fffd}".into(),
            String::new(),
        );
        let session = tabs.session(no_remote);
        let text = session.to_toml().unwrap_or_default();
        let read = Session::parse(&text).unwrap_or_default();
        assert_eq!(read, session);
        let restored = Workspace::new(
            read.tabs.iter().map(TabStart::from).collect(),
            read.active,
            folder("/home"),
            Vec::new(),
        );
        let start = restored.active().panes()[1].start();
        assert_eq!(start, &PaneStart::At(Location::Folder(raw)));
        // The text the pane opens carries every byte back.
        assert_eq!(&PaneStart::parse(&start.to_text()), start);
        assert_eq!(restored.active().active_pane(), 1);
    }

    #[test]
    fn a_pane_is_saved_by_where_it_is_its_remote_address_or_where_it_started() {
        let mount = |path: &Path| {
            path.starts_with("/run/fuse")
                .then(|| RemoteAddress::parse("sftp://host/www"))
                .flatten()
        };
        let mut tabs = workspace(&["/srv", "/unvisited", "/mounted"], 0);
        tabs.open(PaneStart::Typed("docs".into()), Vec::new(), false);
        tabs.open(PaneStart::parse("sftp://host/www"), Vec::new(), false);
        report(&mut tabs, 0, 0, "/srv/data");
        report(&mut tabs, 4, 0, "/run/fuse/sftp/host/www");
        let saved: Vec<Vec<Location>> = tabs
            .session(mount)
            .tabs
            .into_iter()
            .map(|tab| tab.panes)
            .collect();
        let remote = RemoteAddress::parse("sftp://host/www").map(Location::Remote);
        assert_eq!(
            saved,
            vec![
                vec![Location::Folder("/srv/data".into())],
                remote.clone().into_iter().collect(),
                vec![Location::Folder("/unvisited".into())],
                remote.into_iter().collect(),
            ]
        );
    }

    #[test]
    fn handed_over_text_is_read_as_a_uri_path_address_or_typed_text() {
        assert_eq!(PaneStart::parse("file:///a%20b"), folder("/a b"));
        assert_eq!(PaneStart::parse("/plain"), folder("/plain"));
        assert!(matches!(
            PaneStart::parse("sftp://me@host/srv"),
            PaneStart::At(Location::Remote(_))
        ));
        assert_eq!(
            PaneStart::parse("trash:/"),
            PaneStart::Typed("trash:/".into())
        );
        assert_eq!(folder("/srv/data").title(), "data");
        assert_eq!(PaneStart::parse("sftp://me@host/srv/www").title(), "www");
    }

    #[test]
    fn a_workspace_without_tabs_opens_one_on_the_fallback() {
        let tabs = Workspace::new(
            vec![TabStart::default()],
            4,
            folder("/home"),
            vec!["a".into()],
        );
        assert_eq!(shown(&tabs), vec![(vec![folder("/home")], 0)]);
        assert_eq!(tabs.active().select(), ["a".to_owned()]);
    }

    #[test]
    fn alt_numbers_pick_existing_tabs_and_alt_0_the_last() {
        assert_eq!(tab_for_number(1, 3), Some(0));
        assert_eq!(tab_for_number(3, 3), Some(2));
        assert_eq!(tab_for_number(4, 3), None);
        assert_eq!(tab_for_number(0, 3), Some(2));
        assert_eq!(tab_for_number(0, 0), None);
    }

    #[test]
    fn tabs_keep_their_width_shrink_evenly_then_stop_at_the_narrowest() {
        assert_eq!(tab_width(1000., 2), size::TAB);
        assert_eq!(tab_width(size::TAB_ADD + 300., 2), 150.);
        assert_eq!(tab_width(200., 9), size::TAB_MIN);
        assert_eq!(folder_label(Path::new("/srv/data")), "data");
        assert_eq!(folder_label(Path::new("/")), "/");
    }
}
