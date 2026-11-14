//! A window's tabs as a QML list model (W-114e, W-130). The tabs, their
//! panes, the active tab and pane, the tabs closed lately and the session
//! they make are `rustphin_view::workspace::Workspace`; this model turns
//! its changes into row inserts, removals and moves (QML keeps a delegate,
//! with its panes, per row, so the model is never reset) and saves the
//! session as it changes. Folders cross to QML as byte-preserving `file:`
//! URIs (`PaneStart::to_text`, `FolderModel.locationUri`).

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
use rustphin_platform::{remote::RemoteAddress, uri::path_from_uri};
use rustphin_view::{
    location::Location,
    preferences::Store,
    session::{self, Session, SessionSaves},
    settings::Start,
    workspace::{Closing, PaneStart, TabStart, Transfer, Workspace},
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

/// The remote address of a folder under a kio-fuse mount.
fn remote_of(path: &Path) -> Option<RemoteAddress> {
    crate::remote::address_of(path).and_then(|text| RemoteAddress::parse(&text))
}

fn text(words: &str) -> QString {
    QString::from(words)
}

fn strings(list: &QStringList) -> Vec<String> {
    QList::<QString>::from(list)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn row_number(row: usize) -> i32 {
    i32::try_from(row).unwrap_or(i32::MAX)
}

/// The Rust side of `TabsModel`.
#[derive(Default)]
pub struct TabsModelRust {
    count: i32,
    active: i32,
    saves: bool,
    workspace: Option<Workspace>,
    session_saves: SessionSaves,
    saved: Session,
}

impl qobject::TabsModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(workspace) = self.rust().workspace.as_ref() else {
            return QVariant::default();
        };
        let (Ok(row), Some(role)) = (usize::try_from(index.row()), Role::from_number(role)) else {
            return QVariant::default();
        };
        let Some(tab) = workspace.tab(row) else {
            return QVariant::default();
        };
        let start = |side: usize| {
            text(
                &tab.panes()
                    .get(side)
                    .map(|pane| pane.start().to_text())
                    .unwrap_or_default(),
            )
        };
        match role {
            Role::Key => QVariant::from(&i32::try_from(tab.key()).unwrap_or(-1)),
            Role::Title => QVariant::from(&text(tab.active().title())),
            Role::Icon => QVariant::from(&text(match tab.active().icon() {
                "" => "folder",
                icon => icon,
            })),
            Role::Split => QVariant::from(&tab.is_split()),
            Role::ActivePane => QVariant::from(&i32::try_from(tab.active_pane()).unwrap_or(0)),
            Role::LeftStart => QVariant::from(&start(0)),
            Role::RightStart => QVariant::from(&start(1)),
            Role::Select => {
                let list: QList<QString> = tab.select().iter().map(|name| text(name)).collect();
                QVariant::from(&QStringList::from(&list))
            }
            Role::Current => QVariant::from(&(workspace.active_index() == row)),
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
        let (count, active) = self.rust().workspace.as_ref().map_or((0, 0), |workspace| {
            (
                row_number(workspace.len()),
                row_number(workspace.active_index()),
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

    /// Changes the workspace, then publishes the result.
    fn update(mut self: Pin<&mut Self>, change: impl FnOnce(&mut Workspace)) {
        if let Some(workspace) = self.as_mut().rust_mut().workspace.as_mut() {
            change(workspace);
        }
        self.settle();
    }

    /// Stores the session if it changed, on another thread.
    fn save_session(mut self: Pin<&mut Self>) {
        if !self.rust().saves {
            return;
        }
        let Some(path) = session_path() else {
            return;
        };
        let Some(session) = self
            .rust()
            .workspace
            .as_ref()
            .map(|workspace| workspace.session(remote_of))
        else {
            return;
        };
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
        if self.rust().workspace.is_some() {
            return;
        }
        let home = PaneStart::parse(&home.to_string());
        let folders: Vec<PaneStart> = strings(folders)
            .iter()
            .filter(|folder| !folder.is_empty())
            .map(|folder| PaneStart::parse(folder))
            .collect();
        let (tabs, active): (Vec<TabStart>, usize) = if start {
            // The folders named at launch replace the saved tabs, which the
            // start window then saves as they change, as in the GPUI app.
            let starts: Vec<PathBuf> = if folders.is_empty() {
                match &home {
                    PaneStart::At(Location::Folder(home)) => start_folders(home),
                    _ => Vec::new(),
                }
            } else {
                folders
                    .iter()
                    .filter_map(|folder| match folder {
                        PaneStart::At(Location::Folder(path)) => Some(path.clone()),
                        _ => None,
                    })
                    .collect()
            };
            let saved =
                session_path().map_or(Ok(None), |path| rustphin_platform::config::read(&path));
            let restored = session::restore(saved, &starts);
            // The settings file, read before any window, is kept when it
            // could not be used (W-140); the first window says so.
            let mut notices: Vec<String> =
                crate::prefs::with(Store::take_notice).into_iter().collect();
            if let Some(issue) = &restored.issue {
                tracing::warn!(%issue, "saved session is protected for this run");
                notices.push(issue.notice().to_owned());
            }
            if !notices.is_empty() {
                hub::say(notices.join(" "), true);
            }
            let can_save = restored.can_save();
            self.as_mut().set_saves(can_save);
            if can_save {
                self.as_mut().rust_mut().saved = restored.session.clone();
            }
            let tabs = restored.session.tabs.iter().map(TabStart::from).collect();
            (tabs, restored.session.active)
        } else {
            let tabs = folders
                .into_iter()
                .map(|folder| TabStart {
                    panes: vec![folder],
                    active_pane: 0,
                })
                .collect();
            (tabs, 0)
        };
        let workspace = Workspace::new(tabs, active, home, strings(select));
        let last = row_number(workspace.len()) - 1;
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), 0, last);
        self.as_mut().rust_mut().workspace = Some(workspace);
        self.as_mut().rust_mut().count = last + 1;
        self.as_mut().end_insert_rows();
        self.settle();
    }

    /// Inserts the tab `add` opens after the active one, if `add` does.
    fn insert(mut self: Pin<&mut Self>, add: impl FnOnce(&mut Workspace) -> bool) {
        let Some(at) = self.rust().workspace.as_ref().map(Workspace::insert_row) else {
            return;
        };
        let at = row_number(at);
        self.as_mut()
            .begin_insert_rows(&QModelIndex::default(), at, at);
        let added = self.as_mut().rust_mut().workspace.as_mut().is_some_and(add);
        if added {
            self.as_mut().rust_mut().count += 1;
        }
        self.as_mut().end_insert_rows();
        self.settle();
    }

    fn open(self: Pin<&mut Self>, location: &QString, activate: bool, select: &QStringList) {
        let start = PaneStart::parse(&location.to_string());
        let select = strings(select);
        self.insert(move |workspace| {
            workspace.open(start, select, activate);
            true
        });
    }

    fn close(mut self: Pin<&mut Self>, index: i32) -> bool {
        let Ok(row) = usize::try_from(index) else {
            return true;
        };
        let closing = self
            .rust()
            .workspace
            .as_ref()
            .map_or(Closing::Missing, |workspace| workspace.closing(row));
        match closing {
            Closing::Missing => true,
            Closing::LastTab => false,
            Closing::Closes => {
                self.as_mut()
                    .begin_remove_rows(&QModelIndex::default(), index, index);
                if let Some(workspace) = self.as_mut().rust_mut().workspace.as_mut() {
                    workspace.close(row);
                }
                self.as_mut().rust_mut().count -= 1;
                self.as_mut().end_remove_rows();
                self.settle();
                true
            }
        }
    }

    fn reopen(self: Pin<&mut Self>) -> bool {
        if !self
            .rust()
            .workspace
            .as_ref()
            .is_some_and(Workspace::can_reopen)
        {
            hub::say("There are no closed tabs to reopen.".to_owned(), false);
            return false;
        }
        self.insert(Workspace::reopen);
        true
    }

    fn activate(self: Pin<&mut Self>, index: i32) {
        self.update(|workspace| {
            if let Ok(row) = usize::try_from(index) {
                workspace.activate(row);
            }
        });
    }

    fn activate_number(self: Pin<&mut Self>, number: i32) {
        if let Ok(number) = usize::try_from(number) {
            self.update(|workspace| {
                workspace.activate_number(number);
            });
        }
    }

    fn cycle(self: Pin<&mut Self>, step: i32) {
        self.update(|workspace| workspace.cycle(step as isize));
    }

    fn shift_active(self: Pin<&mut Self>, step: i32) {
        let Some((from, to)) = self
            .rust()
            .workspace
            .as_ref()
            .and_then(|workspace| workspace.shift_target(step as isize))
        else {
            return;
        };
        self.move_tab(row_number(from), row_number(to));
    }

    fn move_tab(mut self: Pin<&mut Self>, from: i32, to: i32) {
        let (Ok(source), Ok(target)) = (usize::try_from(from), usize::try_from(to)) else {
            return;
        };
        if !self
            .rust()
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.can_move(source, target))
        {
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
        if let Some(workspace) = self.as_mut().rust_mut().workspace.as_mut() {
            workspace.move_tab(source, target);
        }
        self.as_mut().end_move_rows();
        self.settle();
    }

    fn set_pane(
        self: Pin<&mut Self>,
        key: i32,
        side: i32,
        location: &QString,
        title: &QString,
        icon: &QString,
    ) {
        let (Ok(key), Ok(side)) = (u32::try_from(key), usize::try_from(side)) else {
            return;
        };
        let location = location.to_string();
        let folder = path_from_uri(location.as_bytes())
            .or_else(|| (!location.is_empty()).then(|| PathBuf::from(&location)));
        let (title, icon) = (title.to_string(), icon.to_string());
        self.update(|workspace| workspace.report(key, side, folder, title, icon));
    }

    fn set_active_pane(self: Pin<&mut Self>, key: i32, side: i32) {
        let (Ok(key), Ok(side)) = (u32::try_from(key), usize::try_from(side)) else {
            return;
        };
        self.update(|workspace| workspace.set_active_pane(key, side));
    }

    fn toggle_split(mut self: Pin<&mut Self>) {
        if let Some(workspace) = self.as_mut().rust_mut().workspace.as_mut() {
            workspace.toggle_split();
        }
        self.as_mut().changed(&[Role::LeftStart, Role::RightStart]);
        self.settle();
    }

    fn row_of(&self, key: i32) -> i32 {
        u32::try_from(key)
            .ok()
            .and_then(|key| self.rust().workspace.as_ref()?.row_of(key))
            .map_or(-1, row_number)
    }

    fn palette_json(&self) -> QString {
        let tabs: Vec<serde_json::Value> = self
            .rust()
            .workspace
            .as_ref()
            .map(|workspace| {
                workspace
                    .tabs()
                    .map(|tab| {
                        let pane = tab.active();
                        serde_json::json!({
                            "title": pane.title(),
                            "path": pane
                                .folder()
                                .map(|path| path.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        text(&serde_json::Value::Array(tabs).to_string())
    }

    fn transfer(self: Pin<&mut Self>, selected: &QString, moving: bool) {
        let Some(workspace) = self.rust().workspace.as_ref() else {
            return;
        };
        let selected = rustphin_platform::uri::paths_from_uri_list(selected.to_string().as_bytes());
        let how = if moving {
            Transfer::Move
        } else {
            Transfer::Copy
        };
        match workspace.transfer(how, selected) {
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
}
