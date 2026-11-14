//! The command palette as a QML list model (S8, W-114e): every command,
//! the window's tabs and places, and the recent folders, ranked on another
//! thread by the core's fuzzy matching. Choosing hands QML what to do.

// cxx-qt generates the unsafe FFI glue for the model; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "palette_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{pin::Pin, sync::Arc};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use rustphin_core::{SearchQuery, display_name};
use rustphin_platform::uri::{file_uri, path_from_uri};
use rustphin_view::{
    commands::COMMANDS,
    location::Location,
    palette::{Candidate, Results, Target, command_candidates, result_icon, split_label},
    workspace::folder_label,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Label = 0x100,
    Category,
    Name,
    Trail,
    Shortcut,
    Icon,
    Selected,
}

impl Role {
    const ALL: [Role; 7] = [
        Role::Label,
        Role::Category,
        Role::Name,
        Role::Trail,
        Role::Shortcut,
        Role::Icon,
        Role::Selected,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Label => "label",
            Role::Category => "category",
            Role::Name => "name",
            Role::Trail => "trail",
            Role::Shortcut => "shortcut",
            Role::Icon => "icon",
            Role::Selected => "chosen",
        }
    }
}

/// The candidates for these tabs (a JSON list of `{title, path}`, the path
/// shown only), places (a JSON list of `{title, uri}`) and recent folders.
fn candidates(tabs: &str, places: &str, recent: &[Location]) -> Vec<Candidate> {
    let listed = |json: &str, key: &str| -> Vec<(String, String)> {
        serde_json::from_str::<Vec<serde_json::Value>>(json)
            .unwrap_or_default()
            .iter()
            .filter_map(|item| {
                Some((
                    item.get("title")?.as_str()?.to_owned(),
                    item.get(key)?.as_str()?.to_owned(),
                ))
            })
            .collect()
    };
    let mut candidates: Vec<Candidate> = command_candidates();
    for (index, (title, path)) in listed(tabs, "path").into_iter().enumerate() {
        if !path.is_empty() {
            candidates.push(Candidate::new(
                format!("Tab: {title}"),
                path,
                String::new(),
                Target::Tab(index),
            ));
        }
    }
    for (title, uri) in listed(places, "uri") {
        // The URI keeps every byte of the folder's name (W-132).
        let Some(path) = path_from_uri(uri.as_bytes()) else {
            continue;
        };
        candidates.push(Candidate::new(
            format!("Place: {title}"),
            display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
            String::new(),
            Target::Folder(path),
        ));
    }
    for location in recent {
        candidates.push(match location {
            Location::Folder(path) => Candidate::new(
                format!("Recent: {}", folder_label(path)),
                display_name(path.as_os_str().as_encoded_bytes()).into_owned(),
                String::new(),
                Target::Folder(path.clone()),
            ),
            Location::Remote(address) => {
                let shown = address.to_string();
                Candidate::new(
                    format!("Recent: {}", rustphin_view::remote::short_name(&shown)),
                    shown,
                    String::new(),
                    Target::Remote(address.clone()),
                )
            }
        });
    }
    candidates.push(Candidate::new(
        "History: Clear recent folders".to_owned(),
        "Forget saved folder visits on this computer".to_owned(),
        String::new(),
        Target::ClearHistory,
    ));
    candidates
}

/// What choosing `target` does, for QML. A folder goes as its `file://`
/// URI, which keeps every byte, or as a remote address; QML hands it to
/// `FolderModel.openTyped`.
fn target_json(target: &Target) -> serde_json::Value {
    match target {
        Target::Command(index) => serde_json::json!({
            "kind": "command",
            "id": COMMANDS.get(*index).map_or("", |command| command.id),
        }),
        Target::Tab(index) => serde_json::json!({ "kind": "tab", "index": index }),
        Target::Folder(path) => {
            serde_json::json!({ "kind": "folder", "location": file_uri(path) })
        }
        Target::Remote(address) => {
            serde_json::json!({ "kind": "folder", "location": address.to_string() })
        }
        Target::ClearHistory => serde_json::json!({ "kind": "clear" }),
    }
}

/// The Rust side of `PaletteModel`.
#[derive(Default)]
pub struct PaletteModelRust {
    count: i32,
    selected: i32,
    status: QString,
    candidates: Arc<Vec<Candidate>>,
    results: Results,
}

impl qobject::PaletteModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        *self.count()
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let (Ok(row), Some(role)) = (usize::try_from(index.row()), Role::from_number(role)) else {
            return QVariant::default();
        };
        let Some(candidate) = rust
            .results
            .indices
            .get(row)
            .and_then(|at| rust.candidates.get(*at))
        else {
            return QVariant::default();
        };
        let (category, name) = split_label(&candidate.label);
        let words = |words: &str| QVariant::from(&QString::from(words));
        match role {
            Role::Label => words(&candidate.label),
            Role::Category => words(category),
            Role::Name => words(name),
            Role::Trail => words(&if category.is_empty() {
                candidate.detail.clone()
            } else {
                format!("{category} \u{203a} {}", candidate.detail)
            }),
            Role::Shortcut => words(&candidate.shortcut),
            Role::Icon => words(&result_icon(&candidate.label).lucide_name()),
            Role::Selected => QVariant::from(&(rust.results.selected == row)),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    /// Shows the results and the selection as they are.
    fn publish(mut self: Pin<&mut Self>) {
        let count = i32::try_from(self.rust().results.indices.len()).unwrap_or(i32::MAX);
        let selected = i32::try_from(self.rust().results.selected).unwrap_or(0);
        self.as_mut().begin_reset_model();
        self.as_mut().rust_mut().count = count;
        self.as_mut().end_reset_model();
        self.as_mut().count_changed();
        self.as_mut().set_selected(selected);
    }

    fn gather(mut self: Pin<&mut Self>, tabs: &QString, places: &QString) {
        crate::history::start();
        let (recent, status) = crate::history::snapshot();
        let gathered = candidates(&tabs.to_string(), &places.to_string(), recent.paths());
        self.as_mut().rust_mut().candidates = Arc::new(gathered);
        self.as_mut()
            .set_status(QString::from(status.as_deref().unwrap_or("")));
    }

    fn search(mut self: Pin<&mut Self>, query: &QString) {
        let generation = self.as_mut().rust_mut().results.begin();
        let query = SearchQuery::new(&query.to_string());
        let candidates = Arc::clone(&self.rust().candidates);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let ranked = query.rank(
                candidates.iter().map(|candidate| candidate.search.as_str()),
                usize::MAX,
            );
            let _ = thread.queue(move |mut model| {
                if model
                    .as_mut()
                    .rust_mut()
                    .results
                    .complete(generation, ranked)
                {
                    model.publish();
                }
            });
        });
    }

    fn move_by(mut self: Pin<&mut Self>, delta: i32) {
        self.as_mut().rust_mut().results.move_by(delta as isize);
        self.publish_selection();
    }

    fn select(mut self: Pin<&mut Self>, row: i32) {
        let generation = self.rust().results.generation;
        if let Ok(row) = usize::try_from(row)
            && self.as_mut().rust_mut().results.select(generation, row)
        {
            self.publish_selection();
        }
    }

    /// Redraws which row is selected.
    fn publish_selection(mut self: Pin<&mut Self>) {
        let selected = i32::try_from(self.rust().results.selected).unwrap_or(0);
        self.as_mut().set_selected(selected);
        // Few rows are on screen; redrawing them all is cheap.
        self.as_mut().begin_reset_model();
        self.as_mut().end_reset_model();
    }

    fn chosen(&self) -> QString {
        let rust = self.rust();
        rust.results
            .chosen()
            .and_then(|index| rust.candidates.get(index))
            .map(|candidate| QString::from(&target_json(&candidate.target).to_string()))
            .unwrap_or_default()
    }

    fn clear_history(mut self: Pin<&mut Self>) {
        crate::history::clear();
        self.as_mut()
            .set_status(QString::from("Recent folders cleared."));
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf};

    use rustphin_platform::places::Place;

    use super::*;

    #[test]
    fn tabs_and_places_join_the_commands_and_choosing_says_what_to_do() {
        let tabs = r#"[{"title": "Pictures", "path": "/home/u/Pictures"}]"#;
        let places = r#"[{"title": "Home", "uri": "file:///home/u"}]"#;
        let all = candidates(tabs, places, &[Location::Folder("/srv/old".into())]);
        assert!(all.iter().any(|candidate| candidate.label == "Recent: old"));
        assert!(
            all.iter()
                .any(|candidate| candidate.label == "Tab: Pictures")
        );
        assert!(all.iter().any(|candidate| candidate.label == "Place: Home"));
        assert_eq!(
            all.last().map(|candidate| candidate.label.as_str()),
            Some("History: Clear recent folders")
        );
        assert_eq!(target_json(&Target::Command(0))["id"], "go-back");
        assert_eq!(target_json(&Target::Tab(2))["index"], 2);
        assert_eq!(
            target_json(&Target::Folder("/srv".into()))["location"],
            "file:///srv"
        );
    }

    /// What `FolderModel.openTyped` opens for the target QML is handed.
    fn opened(target: &Target) -> Option<PathBuf> {
        path_from_uri(target_json(target)["location"].as_str()?.as_bytes())
    }

    #[test]
    fn a_place_outside_utf8_opens_from_the_palette_byte_for_byte() {
        let folder = PathBuf::from(OsStr::from_bytes(b"/srv/caf\xe9"));
        let place = Place {
            id: String::new(),
            title: "Caf".to_owned(),
            href: file_uri(&folder),
            path: Some(folder.clone()),
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        };
        let places = crate::places::palette_entries(&[place]).to_string();
        let all = candidates("[]", &places, &[]);
        let Some(chosen) = all.iter().find(|candidate| candidate.label == "Place: Caf") else {
            panic!("the place is offered");
        };
        assert_eq!(opened(&chosen.target), Some(folder));
    }

    #[test]
    fn a_recent_folder_outside_utf8_opens_from_the_palette_byte_for_byte() {
        let folder = PathBuf::from(OsStr::from_bytes(b"/srv/caf\xe9"));
        let all = candidates("[]", "[]", &[Location::Folder(folder.clone())]);
        let Some(chosen) = all
            .iter()
            .find(|candidate| matches!(candidate.target, Target::Folder(_)))
        else {
            panic!("the recent folder is offered");
        };
        assert_eq!(opened(&chosen.target), Some(folder));
    }
}
