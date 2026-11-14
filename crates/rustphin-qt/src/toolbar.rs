//! The toolbar as QML sees it (W-114c2b): the items Settings arranges
//! (W-106f), each with its icon, name and key, and which of them fit the
//! window, decided by the same rule as the GPUI toolbar.

// cxx-qt generates the unsafe FFI glue between Rust and the C++ QObject
// for the toolbar; nothing in the bridge file is written by hand but
// declarations, and the code here stays under the lint (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "toolbar_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QModelIndex, QString, QVariant};
use rustphin_view::toolbar_items::{Item, Toolbar};

/// The roles QML reads, after Qt's own (`Qt::UserRole` is 0x100).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Kind = 0x100,
    ItemId,
    Name,
    Tip,
    Icon,
    Shown,
    Available,
}

impl Role {
    const ALL: [Role; 7] = [
        Role::Kind,
        Role::ItemId,
        Role::Name,
        Role::Tip,
        Role::Icon,
        Role::Shown,
        Role::Available,
    ];

    fn from_number(number: i32) -> Option<Role> {
        Role::ALL.into_iter().find(|role| *role as i32 == number)
    }

    fn name(self) -> &'static str {
        match self {
            Role::Kind => "kind",
            Role::ItemId => "itemId",
            Role::Name => "name",
            Role::Tip => "tip",
            Role::Icon => "icon",
            Role::Shown => "shown",
            Role::Available => "available",
        }
    }
}

/// How QML draws an item.
fn kind(item: &Item) -> &'static str {
    match item {
        Item::Separator => "separator",
        Item::Location => "location",
        Item::Actions => "actions",
        Item::New | Item::View => "menu",
        _ => "button",
    }
}

/// Whether the Qt interface does what the item asks yet; the others are
/// drawn in their place but unavailable until their work item (W-114d–f).
fn available(item: &Item) -> bool {
    match item {
        Item::Admin => false,
        Item::Command(label) => command_id(label).is_some(),
        _ => true,
    }
}

/// The palette command a toolbar button runs, by its label.
fn command_id(label: &str) -> Option<&'static str> {
    rustphin_view::commands::COMMANDS
        .iter()
        .find(|command| command.label == label)
        .map(|command| command.id)
}

/// The id QML acts on: a command button by its command's id.
fn qml_id(item: &Item) -> String {
    match item {
        Item::Command(label) => {
            command_id(label).map_or_else(|| item.id(), |id| format!("command:{id}"))
        }
        _ => item.id(),
    }
}

/// The tooltip: the name and its key, or why it cannot be used yet.
fn tip(item: &Item) -> String {
    let name = item.name();
    if !available(item) {
        return format!("{name}: not in the Qt interface yet");
    }
    match item.keys() {
        Some(keys) => format!("{name} ({keys})"),
        None => name,
    }
}

/// The toolbar the settings arrange, or today's.
fn saved_toolbar() -> Toolbar {
    crate::prefs::settings().toolbar()
}

/// The Rust side of `ToolbarModel`.
pub struct ToolbarModelRust {
    actions_words: bool,
    actions_hint: bool,
    toolbar: Toolbar,
    shown: Vec<bool>,
}

impl Default for ToolbarModelRust {
    fn default() -> Self {
        let toolbar = saved_toolbar();
        let shown = vec![true; toolbar.items.len()];
        Self {
            actions_words: true,
            actions_hint: true,
            toolbar,
            shown,
        }
    }
}

impl qobject::ToolbarModel {
    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        i32::try_from(self.rust().toolbar.items.len()).unwrap_or(i32::MAX)
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let rust = self.rust();
        let (Ok(row), Some(role)) = (usize::try_from(index.row()), Role::from_number(role)) else {
            return QVariant::default();
        };
        let Some((item, _)) = rust.toolbar.items.get(row) else {
            return QVariant::default();
        };
        let text = |words: &str| QVariant::from(&QString::from(words));
        match role {
            Role::Kind => text(kind(item)),
            Role::ItemId => text(&qml_id(item)),
            Role::Name => text(&item.name()),
            Role::Tip => text(&tip(item)),
            Role::Icon => text(&item.icon().lucide_name()),
            Role::Shown => QVariant::from(&rust.shown.get(row).copied().unwrap_or(true)),
            Role::Available => QVariant::from(&available(item)),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for role in Role::ALL {
            roles.insert(role as i32, QByteArray::from(role.name()));
        }
        roles
    }

    fn fit(mut self: Pin<&mut Self>, width: f64) {
        let fitted = self.rust().toolbar.fit(width as f32);
        if fitted.shown != self.rust().shown {
            self.as_mut().rust_mut().shown = fitted.shown;
            let last = self.row_count(&QModelIndex::default()) - 1;
            if last >= 0 {
                let first = self.index(0, 0, &QModelIndex::default());
                let last = self.index(last, 0, &QModelIndex::default());
                self.as_mut().data_changed(&first, &last);
            }
        }
        if *self.actions_words() != fitted.actions_words {
            self.as_mut().rust_mut().actions_words = fitted.actions_words;
            self.as_mut().actions_words_changed();
        }
        if *self.actions_hint() != fitted.actions_hint {
            self.as_mut().rust_mut().actions_hint = fitted.actions_hint;
            self.as_mut().actions_hint_changed();
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
    fn tooltips_name_the_key_or_say_the_item_is_not_built_yet() {
        assert_eq!(tip(&Item::Back), "Back (Alt+Left)");
        assert_eq!(tip(&Item::Hidden), "Show hidden files (Ctrl+H)");
        assert_eq!(
            tip(&Item::Admin),
            "Act as administrator: not in the Qt interface yet"
        );
    }

    #[test]
    fn a_command_button_runs_its_palette_command() {
        let item = Item::Command("Files: Copy location".to_owned());
        assert_eq!(qml_id(&item), "command:copy-location");
        assert!(available(&item));
        assert!(!available(&Item::Command("Files: Gone".to_owned())));
    }

    #[test]
    fn menus_separators_and_the_fixed_items_are_drawn_their_own_way() {
        assert_eq!(kind(&Item::New), "menu");
        assert_eq!(kind(&Item::View), "menu");
        assert_eq!(kind(&Item::Separator), "separator");
        assert_eq!(kind(&Item::Location), "location");
        assert_eq!(kind(&Item::Actions), "actions");
        assert_eq!(kind(&Item::Reload), "button");
    }
}
