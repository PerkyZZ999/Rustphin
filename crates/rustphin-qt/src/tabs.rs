//! A window's tabs as a QML list model (W-114e): one or two panes per tab,
//! the active tab and pane, the tabs closed lately, and the session saved
//! as it changes. The rules are `rustphin_view::tabs` and `::workspace`;
//! QML keeps a delegate (with its panes) per row, so rows are inserted,
//! removed and moved, never reset.

// cxx-qt generates the unsafe FFI glue for the model; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "tabs_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QStringList, QVariant,
};
use rustphin_platform::remote::RemoteAddress;
use rustphin_view::{
    location::Location,
    session::{self, Session, SessionSaves, SessionTab},
    settings::Start,
    tabs::Tabs,
    workspace::{
        Transfer, folder_label, remember_closed_tab, tab_for_number, transfer_to_other_view,
    },
};

use crate::{hub, prefs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Key = 0x100,
    Title,
    Icon,
    Split,
    ActivePane,
    LeftStart,
    RightStart,
    Select,
    Current,
}

impl Role {
    const ALL: [Role; 9] = [
        Role::Key,
        Role::Title,
        Role::Icon,
        Role::Split,
        Role::ActivePane,
        Role::LeftStart,
        Role::RightStart,
        Role::Select,
        Role::Current,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Key => "key",
            Role::Title => "title",
            Role::Icon => "icon",
            Role::Split => "split",
            Role::ActivePane => "activePane",
            Role::LeftStart => "leftStart",
            Role::RightStart => "rightStart",
            Role::Select => "select",
            Role::Current => "current",
        }
    }
}

/// One pane of a tab, as last reported.
#[derive(Debug, Clone, Default)]
struct PaneState {
    /// Where it started: a path or a remote address, as QML opens it.
    start: String,
    /// The folder it shows.
    path: Option<PathBuf>,
    title: String,
    icon: String,
}

/// One tab.
#[derive(Debug, Clone)]
struct Tab {
    key: i32,
    panes: Vec<PaneState>,
    active_pane: usize,
    /// Names to select when the first pane arrives.
    select: Vec<String>,
}

impl Tab {
    fn new(key: i32, starts: Vec<String>, active_pane: usize) -> Self {
        let panes: Vec<PaneState> = starts
            .into_iter()
            .map(|start| PaneState {
                title: start_title(&start),
                start,
                ..PaneState::default()
            })
            .collect();
        Self {
            key,
            active_pane: active_pane.min(panes.len().saturating_sub(1)),
            panes,
            select: Vec::new(),
        }
    }

    fn active(&self) -> Option<&PaneState> {
        self.panes.get(self.active_pane)
    }

    /// The tab as the session keeps it.
    fn saved(&self) -> SessionTab {
        SessionTab {
            panes: self.panes.iter().filter_map(saved_location).collect(),
            active_pane: self.active_pane,
        }
    }
}

/// What a tab is called before its pane has said: its folder's name.
fn start_title(start: &str) -> String {
    match RemoteAddress::parse(start).filter(|_| start.contains("://")) {
        Some(_) => rustphin_view::remote::short_name(start).to_owned(),
        None => folder_label(Path::new(start)),
    }
}

/// How a pane is saved: a remote folder by its address, else its path.
fn saved_location(pane: &PaneState) -> Option<Location> {
    let path = pane.path.clone().or_else(|| {
        (!pane.start.contains("://"))
            .then(|| PathBuf::from(&pane.start))
            .filter(|path| path.is_absolute())
    })?;
    match crate::remote::address_of(&path).and_then(|text| RemoteAddress::parse(&text)) {
        Some(address) => Some(Location::Remote(address)),
        None => Some(Location::Folder(path)),
    }
}

/// How a saved location is opened: its path, or its remote address.
fn start_of(location: &Location) -> String {
    match location {
        Location::Folder(path) => path.to_string_lossy().into_owned(),
        Location::Remote(address) => address.to_string(),
    }
}

/// The folders a start window opens with: none to restore the session.
fn start_folders(home: &Path) -> Vec<PathBuf> {
    let settings = prefs::settings();
    match settings.start() {
        (Start::Restore, _) => Vec::new(),
        (Start::Home, _) => vec![home.to_owned()],
        (Start::Folder, folder) => match folder.and_then(Location::from_saved) {
            Some(Location::Folder(path)) => vec![path],
            _ => vec![home.to_owned()],
        },
    }
}

fn session_path() -> Option<PathBuf> {
    rustphin_platform::config::state_dir().map(|dir| dir.join("session.toml"))
}

fn text(words: &str) -> QString {
    QString::from(words)
}

/// The Rust side of `TabsModel`.
pub struct TabsModelRust {
    count: i32,
    active: i32,
    saves: bool,
    tabs: Option<Tabs<Tab>>,
    closed: Vec<SessionTab>,
    next_key: i32,
    session_saves: SessionSaves,
    saved: Session,
}

impl Default for TabsModelRust {
    fn default() -> Self {
        Self {
            count: 0,
            active: 0,
            saves: false,
            tabs: None,
            closed: Vec::new(),
            next_key: 1,
            session_saves: SessionSaves::default(),
            saved: Session::default(),
        }
    }
}

impl TabsModelRust {
    fn key(&mut self) -> i32 {
        let key = self.next_key;
        self.next_key += 1;
        key
    }

    fn get(&self, row: usize) -> Option<&Tab> {
        self.tabs.as_ref()?.iter().nth(row)
    }

    fn row_of_key(&self, key: i32) -> Option<usize> {
        self.tabs.as_ref()?.iter().position(|tab| tab.key == key)
    }
}

impl qobject::TabsModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let (Ok(row), Some(role)) = (usize::try_from(index.row()), Role::from_number(role)) else {
            return QVariant::default();
        };
        let Some(tab) = rust.get(row) else {
            return QVariant::default();
        };
        let start = |side: usize| tab.panes.get(side).map_or("", |pane| pane.start.as_str());
        match role {
            Role::Key => QVariant::from(&tab.key),
            Role::Title => {
                QVariant::from(&text(tab.active().map_or("", |pane| pane.title.as_str())))
            }
            Role::Icon => {
                QVariant::from(&text(match tab.active().map(|pane| pane.icon.as_str()) {
                    Some(icon) if !icon.is_empty() => icon,
                    _ => "folder",
                }))
            }
            Role::Split => QVariant::from(&(tab.panes.len() > 1)),
            Role::ActivePane => QVariant::from(&i32::try_from(tab.active_pane).unwrap_or(0)),
            Role::LeftStart => QVariant::from(&text(start(0))),
            Role::RightStart => QVariant::from(&text(start(1))),
            Role::Select => {
                let list: QList<QString> = tab.select.iter().map(|name| text(name)).collect();
                QVariant::from(&QStringList::from(&list))
            }
            Role::Current => QVariant::from(
                &(rust
                    .tabs
                    .as_ref()
                    .is_some_and(|tabs| tabs.active_index() == row)),
            ),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Redraws the given roles of every tab.
    fn changed(mut self: Pin<&mut Self>, roles: &[Role]) {
        let last = *self.count() - 1;
        if last >= 0 {
            let first = self.index(0, 0, &QModelIndex::default());
            let last = self.index(last, 0, &QModelIndex::default());
            let roles: QList<i32> = roles.iter().map(|role| *role as i32).collect();
            self.as_mut().data_changed(&first, &last, &roles);
        }
    }

    /// Publishes the count and the active tab, and saves the session.
    fn settle(mut self: Pin<&mut Self>) {
        let (count, active) = self.rust().tabs.as_ref().map_or((0, 0), |tabs| {
            (
                i32::try_from(tabs.len()).unwrap_or(i32::MAX),
                i32::try_from(tabs.active_index()).unwrap_or(0),
            )
        });
        self.as_mut().set_count(count);
        self.as_mut().set_active(active);
        self.as_mut().changed(&[
            Role::Current,
            Role::Title,
            Role::Icon,
            Role::Split,
            Role::ActivePane,
        ]);
        self.save_session();
    }

    /// Stores the session if it changed, on another thread.
    fn save_session(mut self: Pin<&mut Self>) {
        if !self.rust().saves {
            return;
        }
        let Some(path) = session_path() else {
            return;
        };
        let Some(tabs) = self.rust().tabs.as_ref() else {
            return;
        };
        let session = Session::new(
            tabs.iter()
                .map(Tab::saved)
                .filter(|tab| !tab.panes.is_empty())
                .collect(),
            tabs.active_index(),
        );
        if session == self.rust().saved || session.tabs.is_empty() {
            return;
        }
        let save = match self
            .as_mut()
            .rust_mut()
            .session_saves
            .prepare(session.clone())
        {
            Ok(save) => save,
            Err(error) => {
                tracing::warn!(%error, "could not queue the session save");
                return;
            }
        };
        self.as_mut().rust_mut().saved = session;
        std::thread::spawn(move || {
            if let Err(error) = save.write(&path) {
                tracing::warn!(%error, path = %path.display(), "could not save the session");
            }
        });
    }

    fn restore(
        mut self: Pin<&mut Self>,
        folders: &QStringList,
        select: &QStringList,
        start: bool,
        home: &QString,
    ) {
        if self.rust().tabs.is_some() {
            return;
        }
        let home = PathBuf::from(home.to_string());
        let folders: Vec<String> = QList::<QString>::from(folders)
            .iter()
            .map(ToString::to_string)
            .filter(|folder| !folder.is_empty())
            .collect();
        let mut tabs: Vec<(Vec<String>, usize)> = Vec::new();
        let mut active = 0;
        if !folders.is_empty() {
            tabs = folders
                .into_iter()
                .map(|folder| (vec![folder], 0))
                .collect();
        } else if start {
            let starts = start_folders(&home);
            let saved =
                session_path().map_or(Ok(None), |path| rustphin_platform::config::read(&path));
            let restored = session::restore(saved, &starts);
            if let Some(issue) = &restored.issue {
                tracing::warn!(%issue, "saved session is protected for this run");
                hub::say(issue.notice().to_owned(), true);
            }
            let can_save = restored.can_save();
            self.as_mut().set_saves(can_save);
            if can_save {
                self.as_mut().rust_mut().saved = restored.session.clone();
            }
            active = restored.session.active;
            tabs = restored
                .session
                .tabs
                .iter()
                .map(|tab| (tab.panes.iter().map(start_of).collect(), tab.active_pane))
                .collect();
        }
        if tabs.is_empty() {
            tabs.push((vec![home.to_string_lossy().into_owned()], 0));
            active = 0;
        }
        let mut items: Vec<Tab> = tabs
            .into_iter()
            .map(|(starts, pane)| {
                let key = self.as_mut().rust_mut().key();
                Tab::new(key, starts, pane)
            })
            .collect();
        if let Some(first) = items.first_mut() {
            first.select = QList::<QString>::from(select)
                .iter()
                .map(ToString::to_string)
                .collect();
        }
        let last = i32::try_from(items.len()).unwrap_or(1) - 1;
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), 0, last);
        self.as_mut().rust_mut().tabs = Tabs::from_parts(items, active);
        self.as_mut().rust_mut().count = last + 1;
        self.as_mut().end_insert_rows();
        self.settle();
    }

    /// Inserts `tab` after the active one; keeps the previous one active
    /// unless `activate`.
    fn insert(mut self: Pin<&mut Self>, tab: Tab, activate: bool) {
        let Some(previous) = self.rust().tabs.as_ref().map(Tabs::active_index) else {
            return;
        };
        let at = i32::try_from(previous + 1).unwrap_or(i32::MAX);
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), at, at);
        if let Some(tabs) = self.as_mut().rust_mut().tabs.as_mut() {
            tabs.open(tab);
            if !activate {
                tabs.activate(previous);
            }
        }
        self.as_mut().rust_mut().count += 1;
        self.as_mut().end_insert_rows();
        self.settle();
    }

    fn open(mut self: Pin<&mut Self>, location: &QString, activate: bool, select: &QStringList) {
        let key = self.as_mut().rust_mut().key();
        let mut tab = Tab::new(key, vec![location.to_string()], 0);
        tab.select = QList::<QString>::from(select)
            .iter()
            .map(ToString::to_string)
            .collect();
        self.insert(tab, activate);
    }

    fn close(mut self: Pin<&mut Self>, index: i32) -> bool {
        let Ok(row) = usize::try_from(index) else {
            return true;
        };
        if self.rust().tabs.as_ref().is_none_or(|tabs| tabs.len() <= 1) {
            return false;
        }
        self.as_mut()
            .begin_remove_rows(&QModelIndex::default(), index, index);
        let closed = self
            .as_mut()
            .rust_mut()
            .tabs
            .as_mut()
            .and_then(|tabs| tabs.close(row));
        if let Some(closed) = closed {
            let saved = closed.saved();
            remember_closed_tab(&mut self.as_mut().rust_mut().closed, saved);
        }
        self.as_mut().rust_mut().count -= 1;
        self.as_mut().end_remove_rows();
        self.settle();
        true
    }

    fn reopen(mut self: Pin<&mut Self>) -> bool {
        if self.rust().closed.is_empty() {
            hub::say("There are no closed tabs to reopen.".to_owned(), false);
            return false;
        }
        let saved = self.as_mut().rust_mut().closed.remove(0);
        let key = self.as_mut().rust_mut().key();
        let tab = Tab::new(
            key,
            saved.panes.iter().map(start_of).collect(),
            saved.active_pane,
        );
        self.insert(tab, true);
        true
    }

    fn activate(mut self: Pin<&mut Self>, index: i32) {
        if let (Ok(index), Some(tabs)) = (
            usize::try_from(index),
            self.as_mut().rust_mut().tabs.as_mut(),
        ) {
            tabs.activate(index);
        }
        self.settle();
    }

    fn activate_number(self: Pin<&mut Self>, number: i32) {
        let len = self.rust().tabs.as_ref().map_or(0, Tabs::len);
        if let Some(index) = usize::try_from(number)
            .ok()
            .and_then(|number| tab_for_number(number, len))
        {
            self.activate(i32::try_from(index).unwrap_or(0));
        }
    }

    fn cycle(mut self: Pin<&mut Self>, step: i32) {
        if let Some(tabs) = self.as_mut().rust_mut().tabs.as_mut() {
            tabs.cycle(step as isize);
        }
        self.settle();
    }

    fn shift_active(self: Pin<&mut Self>, step: i32) {
        let Some(active) = self.rust().tabs.as_ref().map(Tabs::active_index) else {
            return;
        };
        let Some(to) = active.checked_add_signed(step as isize) else {
            return;
        };
        self.move_tab(
            i32::try_from(active).unwrap_or(0),
            i32::try_from(to).unwrap_or(0),
        );
    }

    fn move_tab(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let len = self.rust().tabs.as_ref().map_or(0, Tabs::len);
        let (Ok(source), Ok(target)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        if source >= len || target >= len || source == target {
            return;
        }
        // Qt counts the destination before the move: one past when moving down.
        let destination = if target > source { to + 1 } else { to };
        let parent = QModelIndex::default();
        if !self
            .as_mut()
            .begin_move_rows(&parent, from, from, &parent, destination)
        {
            return;
        }
        if let Some(tabs) = self.as_mut().rust_mut().tabs.as_mut() {
            tabs.move_tab(source, target);
        }
        self.as_mut().end_move_rows();
        self.settle();
    }

    fn set_pane(
        mut self: Pin<&mut Self>,
        key: i32,
        side: i32,
        path: &QString,
        title: &QString,
        icon: &QString,
    ) {
        let (Some(row), Ok(side)) = (self.rust().row_of_key(key), usize::try_from(side)) else {
            return;
        };
        if let Some(tab) = self
            .as_mut()
            .rust_mut()
            .tabs
            .as_mut()
            .and_then(|tabs| tabs.get_mut(row))
            && let Some(pane) = tab.panes.get_mut(side)
        {
            let path = path.to_string();
            pane.path = (!path.is_empty()).then(|| PathBuf::from(path));
            pane.title = title.to_string();
            pane.icon = icon.to_string();
        }
        self.settle();
    }

    fn set_active_pane(mut self: Pin<&mut Self>, key: i32, side: i32) {
        let (Some(row), Ok(side)) = (self.rust().row_of_key(key), usize::try_from(side)) else {
            return;
        };
        if let Some(tab) = self
            .as_mut()
            .rust_mut()
            .tabs
            .as_mut()
            .and_then(|tabs| tabs.get_mut(row))
        {
            tab.active_pane = side.min(tab.panes.len().saturating_sub(1));
        }
        self.settle();
    }

    fn toggle_split(mut self: Pin<&mut Self>) {
        let Some(index) = self.rust().tabs.as_ref().map(Tabs::active_index) else {
            return;
        };
        if let Some(tab) = self
            .as_mut()
            .rust_mut()
            .tabs
            .as_mut()
            .and_then(|tabs| tabs.get_mut(index))
        {
            if tab.panes.len() == 1 {
                // The new view opens on the same folder, as in Dolphin.
                let mut second = tab.panes[0].clone();
                if let Some(path) = &second.path {
                    second.start = path.to_string_lossy().into_owned();
                }
                tab.panes.push(second);
                tab.active_pane = 1;
            } else {
                let keep = tab.active_pane.min(tab.panes.len() - 1);
                tab.panes.swap(0, keep);
                tab.panes.truncate(1);
                tab.active_pane = 0;
                // The kept view goes on from where it is.
                if let Some(path) = tab.panes[0].path.clone() {
                    tab.panes[0].start = path.to_string_lossy().into_owned();
                }
            }
        }
        self.as_mut().changed(&[Role::LeftStart, Role::RightStart]);
        self.settle();
    }

    fn row_of(&self, key: i32) -> i32 {
        self.rust()
            .row_of_key(key)
            .and_then(|row| i32::try_from(row).ok())
            .unwrap_or(-1)
    }

    fn palette_json(&self) -> QString {
        let tabs: Vec<serde_json::Value> = self
            .rust()
            .tabs
            .as_ref()
            .map(|tabs| {
                tabs.iter()
                    .map(|tab| {
                        let pane = tab.active();
                        serde_json::json!({
                            "title": pane.map_or("", |pane| pane.title.as_str()),
                            "path": pane
                                .and_then(|pane| pane.path.as_ref())
                                .map(|path| path.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        text(&serde_json::Value::Array(tabs).to_string())
    }

    fn transfer(self: Pin<&mut Self>, selected: &QStringList, moving: bool) {
        let Some(tab) = self.rust().tabs.as_ref().map(Tabs::active).cloned() else {
            return;
        };
        let here = tab.active().and_then(|pane| pane.path.clone());
        let there = (tab.panes.len() == 2)
            .then(|| tab.panes[1 - tab.active_pane.min(1)].path.clone())
            .flatten();
        let selected: Vec<PathBuf> = QList::<QString>::from(selected)
            .iter()
            .map(|path| PathBuf::from(path.to_string()))
            .collect();
        let how = if moving {
            Transfer::Move
        } else {
            Transfer::Copy
        };
        match transfer_to_other_view(how, selected, here.as_deref(), there.as_deref()) {
            Ok(request) => hub::start(request),
            Err(reason) => hub::say(reason.to_owned(), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_found_by_number_and_named_once() {
        for role in Role::ALL {
            assert_eq!(Role::from_number(role as i32), Some(role));
        }
        let mut names: Vec<&str> = Role::ALL.iter().map(|role| role.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Role::ALL.len());
    }

    #[test]
    fn tabs_are_titled_by_their_folder_or_remote_name_until_the_pane_says() {
        assert_eq!(start_title("/srv/data"), "data");
        assert_eq!(start_title("sftp://me@host/srv/www"), "www");
        assert_eq!(start_title("/"), "/");
    }

    #[test]
    fn a_pane_is_saved_by_where_it_is_or_where_it_started() {
        let pane = PaneState {
            start: "/srv".into(),
            path: Some("/srv/data".into()),
            ..PaneState::default()
        };
        assert_eq!(
            saved_location(&pane),
            Some(Location::Folder("/srv/data".into()))
        );
        let unvisited = PaneState {
            start: "/srv".into(),
            ..PaneState::default()
        };
        assert_eq!(
            saved_location(&unvisited),
            Some(Location::Folder("/srv".into()))
        );
        let relative = PaneState {
            start: "docs".into(),
            ..PaneState::default()
        };
        assert_eq!(saved_location(&relative), None);
    }
}
