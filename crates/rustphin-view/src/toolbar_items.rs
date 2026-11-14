//! What the toolbar holds, in which order, and what fits (W-106f).
//!
//! The toolbar is a list of items chosen in Settings, Toolbar, as in
//! Dolphin's "Configure Toolbars…": built-in buttons, any palette command,
//! and separators. The location field and Actions always stay: one is
//! where the folder is shown, the other reaches every action. When the
//! window is too narrow, Actions first loses its key hint and its words,
//! then items leave from the right end inwards.

use crate::metrics::{size, space};

/// One thing on the toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    /// Act as administrator (the shield).
    Admin,
    /// Back.
    Back,
    /// Forward.
    Forward,
    /// Up one folder.
    Up,
    /// Reload.
    Reload,
    /// The location field; always there.
    Location,
    /// The New menu.
    New,
    /// Open terminal here.
    Terminal,
    /// The View menu.
    View,
    /// Split view.
    Split,
    /// Show hidden files.
    Hidden,
    /// Filter by name.
    Filter,
    /// Actions (the palette); always there.
    Actions,
    /// A thin line between groups.
    Separator,
    /// A palette command, by its label.
    Command(String),
}

/// The built-in items in the order "Available actions" lists them.
pub const BUILT_IN: [Item; 12] = [
    Item::Admin,
    Item::Back,
    Item::Forward,
    Item::Up,
    Item::Reload,
    Item::New,
    Item::Terminal,
    Item::View,
    Item::Split,
    Item::Hidden,
    Item::Filter,
    Item::Separator,
];

impl Item {
    /// The icon the item shows on the toolbar and in Settings.
    pub fn icon(&self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Item::Admin => Icon::ShieldAlert,
            Item::Back => Icon::ArrowLeft,
            Item::Forward => Icon::ArrowRight,
            Item::Up => Icon::ArrowUp,
            Item::Reload => Icon::RefreshCw,
            Item::Location => Icon::Folder,
            Item::New => Icon::Plus,
            Item::Terminal => Icon::SquareTerminal,
            Item::View => Icon::List,
            Item::Split => Icon::SquareSplitHorizontal,
            Item::Hidden => Icon::Eye,
            Item::Filter => Icon::Funnel,
            Item::Actions => Icon::Menu,
            Item::Separator => Icon::Minus,
            Item::Command(label) => crate::palette::command_icon(label),
        }
    }

    /// How the settings file names it.
    pub fn id(&self) -> String {
        match self {
            Self::Admin => "admin".to_owned(),
            Self::Back => "back".to_owned(),
            Self::Forward => "forward".to_owned(),
            Self::Up => "up".to_owned(),
            Self::Reload => "reload".to_owned(),
            Self::Location => "location".to_owned(),
            Self::New => "new".to_owned(),
            Self::Terminal => "terminal".to_owned(),
            Self::View => "view".to_owned(),
            Self::Split => "split".to_owned(),
            Self::Hidden => "hidden".to_owned(),
            Self::Filter => "filter".to_owned(),
            Self::Actions => "actions".to_owned(),
            Self::Separator => "separator".to_owned(),
            Self::Command(label) => format!("command:{label}"),
        }
    }

    /// The item an id names, if this build knows it.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "admin" => Self::Admin,
            "back" => Self::Back,
            "forward" => Self::Forward,
            "up" => Self::Up,
            "reload" => Self::Reload,
            "location" => Self::Location,
            "new" => Self::New,
            "terminal" => Self::Terminal,
            "view" => Self::View,
            "split" => Self::Split,
            "hidden" => Self::Hidden,
            "filter" => Self::Filter,
            "actions" => Self::Actions,
            "separator" => Self::Separator,
            other => Self::Command(
                other
                    .strip_prefix("command:")
                    .filter(|label| !label.is_empty())?
                    .to_owned(),
            ),
        })
    }

    /// Its name in the lists.
    pub fn name(&self) -> String {
        match self {
            Self::Admin => "Act as administrator".to_owned(),
            Self::Back => "Back".to_owned(),
            Self::Forward => "Forward".to_owned(),
            Self::Up => "Up one folder".to_owned(),
            Self::Reload => "Reload".to_owned(),
            Self::Location => "Location".to_owned(),
            Self::New => "New".to_owned(),
            Self::Terminal => "Open terminal here".to_owned(),
            Self::View => "View".to_owned(),
            Self::Split => "Split view".to_owned(),
            Self::Hidden => "Show hidden files".to_owned(),
            Self::Filter => "Filter by name".to_owned(),
            Self::Actions => "Actions".to_owned(),
            Self::Separator => "Separator".to_owned(),
            Self::Command(label) => label.clone(),
        }
    }

    /// The key that does the same, as its tooltip names it (KEYBOARD.md).
    pub fn keys(&self) -> Option<&'static str> {
        match self {
            Self::Back => Some("Alt+Left"),
            Self::Forward => Some("Alt+Right"),
            Self::Up => Some("Alt+Up"),
            Self::Reload => Some("F5"),
            Self::Terminal => Some("Shift+F4"),
            Self::Split => Some("F3"),
            Self::Hidden => Some("Ctrl+H"),
            Self::Filter => Some("Ctrl+I"),
            Self::Actions => Some("Ctrl+Shift+P"),
            _ => None,
        }
    }

    /// Whether it must stay on the toolbar.
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Location | Self::Actions)
    }

    /// Whether it can show its words beside its icon.
    pub fn can_show_words(&self) -> bool {
        !matches!(self, Self::Location | Self::Actions | Self::Separator)
    }

    /// About how wide it is drawn, for deciding what fits.
    fn width(&self, words: bool) -> f32 {
        let gap = space::XS;
        let base = match self {
            Self::Separator => size::HAIRLINE + 2. * space::XS,
            // The menus carry a chevron.
            Self::New | Self::View => size::TOOL_BUTTON + space::LG,
            Self::Location => size::TOOLBAR_LOCATION_MIN,
            _ => size::TOOL_BUTTON,
        };
        let words = if words && self.can_show_words() {
            // About 7 pixels a character at the body size, and a gap.
            self.name().chars().count() as f32 * 7. + space::SM
        } else {
            0.
        };
        base + words + gap
    }
}

/// The toolbar's items, each with whether it shows its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolbar {
    /// Left to right.
    pub items: Vec<(Item, bool)>,
}

impl Default for Toolbar {
    /// Today's toolbar (W-096, W-107, W-108).
    fn default() -> Self {
        let items = [
            Item::Admin,
            Item::Separator,
            Item::Back,
            Item::Forward,
            Item::Up,
            Item::Reload,
            Item::Location,
            Item::New,
            Item::Terminal,
            Item::Separator,
            Item::View,
            Item::Split,
            Item::Hidden,
            Item::Filter,
            Item::Separator,
            Item::Actions,
        ];
        Self {
            items: items.into_iter().map(|item| (item, false)).collect(),
        }
    }
}

impl Toolbar {
    /// Reads the saved lists: item ids in order, and those showing words.
    /// Unknown ids and repeated buttons are dropped; the location field
    /// and Actions are put back if missing. An empty list is today's.
    pub fn from_ids(items: &[String], words: &[String]) -> Self {
        if items.is_empty() {
            return Self::default();
        }
        let mut seen = std::collections::HashSet::new();
        let mut list: Vec<(Item, bool)> = items
            .iter()
            .filter_map(|id| Item::from_id(id))
            .filter(|item| *item == Item::Separator || seen.insert(item.clone()))
            .map(|item| {
                let shows = item.can_show_words() && words.contains(&item.id());
                (item, shows)
            })
            .collect();
        if !list.iter().any(|(item, _)| *item == Item::Location) {
            let at = list.len().min(1);
            list.insert(at, (Item::Location, false));
        }
        if !list.iter().any(|(item, _)| *item == Item::Actions) {
            list.push((Item::Actions, false));
        }
        Self { items: list }
    }

    /// The lists to save.
    pub fn to_ids(&self) -> (Vec<String>, Vec<String>) {
        let items = self.items.iter().map(|(item, _)| item.id()).collect();
        let words = self
            .items
            .iter()
            .filter(|(_, words)| *words)
            .map(|(item, _)| item.id())
            .collect();
        (items, words)
    }

    /// Whether `item` is on the toolbar (a separator always may be added).
    pub fn contains(&self, item: &Item) -> bool {
        *item != Item::Separator && self.items.iter().any(|(on, _)| on == item)
    }

    /// Adds `item` after position `after`, or at the end before Actions.
    /// Returns where it went; `None` when it is there already.
    pub fn add(&mut self, item: Item, after: Option<usize>) -> Option<usize> {
        if self.contains(&item) {
            return None;
        }
        let actions = self
            .items
            .iter()
            .position(|(on, _)| *on == Item::Actions)
            .unwrap_or(self.items.len());
        let at = after.map_or(actions, |after| (after + 1).min(self.items.len()));
        self.items.insert(at, (item, false));
        Some(at)
    }

    /// Takes the item at `index` off, unless it must stay.
    pub fn remove(&mut self, index: usize) -> bool {
        if self
            .items
            .get(index)
            .is_some_and(|(item, _)| !item.is_fixed())
        {
            self.items.remove(index);
            return true;
        }
        false
    }

    /// Moves the item at `index` one place left (negative) or right.
    /// Returns its new index.
    pub fn shift(&mut self, index: usize, delta: isize) -> usize {
        if index >= self.items.len() {
            return index;
        }
        let to = index.saturating_add_signed(delta).min(self.items.len() - 1);
        let item = self.items.remove(index);
        self.items.insert(to, item);
        to
    }

    /// Shows or hides the words of the item at `index`.
    pub fn set_words(&mut self, index: usize, words: bool) {
        if let Some((item, shows)) = self.items.get_mut(index) {
            *shows = words && item.can_show_words();
        }
    }

    /// What is drawn in a window `width` wide.
    pub fn fit(&self, width: f32) -> Fitted {
        let available = width - 2. * space::SM;
        let mut shown: Vec<bool> = vec![true; self.items.len()];
        let mut fitted = Fitted {
            shown: Vec::new(),
            actions_words: true,
            actions_hint: true,
        };
        let total = |shown: &[bool], fitted: &Fitted| -> f32 {
            let items: f32 = self
                .items
                .iter()
                .zip(shown)
                .filter(|(_, on)| **on)
                .map(|((item, words), _)| match item {
                    Item::Actions => {
                        if !fitted.actions_words {
                            size::TOOL_BUTTON
                        } else if fitted.actions_hint {
                            size::TOOLBAR_ACTIONS_HINTED
                        } else {
                            size::TOOLBAR_ACTIONS
                        }
                    }
                    other => other.width(*words),
                })
                .sum();
            items
        };
        if total(&shown, &fitted) > available {
            fitted.actions_hint = false;
        }
        if total(&shown, &fitted) > available {
            fitted.actions_words = false;
        }
        while total(&shown, &fitted) > available {
            // The rightmost item that may leave, Actions aside.
            let Some(index) = self
                .items
                .iter()
                .enumerate()
                .rev()
                .find(|(index, (item, _))| shown[*index] && !item.is_fixed())
                .map(|(index, _)| index)
            else {
                break;
            };
            shown[index] = false;
        }
        // A separator after nothing drawn, or after another, goes too, and
        // so does one left at the end.
        let mut last_drawn: Option<usize> = None;
        for (index, on) in shown.iter_mut().enumerate() {
            if !*on {
                continue;
            }
            if self.items[index].0 == Item::Separator
                && last_drawn.is_none_or(|last| self.items[last].0 == Item::Separator)
            {
                *on = false;
                continue;
            }
            last_drawn = Some(index);
        }
        if let Some(last) = last_drawn
            && self.items[last].0 == Item::Separator
        {
            shown[last] = false;
        }
        fitted.shown = shown;
        fitted
    }
}

/// How a pane draws its toolbar items this frame.
#[derive(Debug, Clone)]
pub struct Look {
    /// The window's width.
    pub width: f32,
    /// Actions shows its words and chevron.
    pub actions_words: bool,
    /// Actions shows its key hint.
    pub actions_hint: bool,
    /// The items that show their words beside their icons.
    pub words: Vec<Item>,
}

impl Look {
    /// The words `item` shows beside its icon, if it shows them.
    pub fn words_of(&self, item: &Item) -> Option<String> {
        self.words.contains(item).then(|| item.name())
    }
}

/// What fits: which items are drawn, and how Actions is drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fitted {
    /// Per item, whether it is drawn.
    pub shown: Vec<bool>,
    /// Actions shows its words and chevron.
    pub actions_words: bool,
    /// Actions shows its key hint.
    pub actions_hint: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    fn drawn(toolbar: &Toolbar, width: f32) -> Vec<String> {
        let fitted = toolbar.fit(width);
        toolbar
            .items
            .iter()
            .zip(&fitted.shown)
            .filter(|(_, on)| **on)
            .map(|((item, _), _)| item.id())
            .collect()
    }

    #[test]
    fn the_saved_lists_round_trip_and_keep_what_must_stay() {
        let mut toolbar = Toolbar::default();
        toolbar.add(Item::Command("Edit: Copy location".to_owned()), None);
        toolbar.set_words(2, true);
        let (items, words) = toolbar.to_ids();
        assert_eq!(Toolbar::from_ids(&items, &words), toolbar);
        // Unknown ids and repeated buttons go; Location and Actions return.
        let odd = Toolbar::from_ids(
            &ids(&["back", "wormhole", "back", "separator", "separator"]),
            &ids(&["back", "separator"]),
        );
        let names: Vec<String> = odd.items.iter().map(|(item, _)| item.id()).collect();
        assert_eq!(
            names,
            ["back", "location", "separator", "separator", "actions"]
        );
        assert!(odd.items[0].1, "back shows its words");
        assert!(!odd.items[2].1, "a separator has none");
        assert_eq!(Toolbar::from_ids(&[], &[]), Toolbar::default());
        assert_eq!(Item::from_id("command:"), None);
    }

    #[test]
    fn location_and_actions_cannot_be_removed_and_buttons_appear_once() {
        let mut toolbar = Toolbar::default();
        let location = toolbar
            .items
            .iter()
            .position(|(i, _)| *i == Item::Location)
            .unwrap();
        assert!(!toolbar.remove(location));
        assert_eq!(toolbar.add(Item::Back, None), None);
        let at = toolbar.add(Item::Separator, None).unwrap();
        assert_eq!(
            toolbar.items[at + 1].0,
            Item::Actions,
            "added before Actions"
        );
        assert!(toolbar.remove(0), "the shield can go");
        assert_eq!(toolbar.shift(0, -1), 0);
        let last = toolbar.items.len() - 1;
        assert_eq!(toolbar.shift(0, 100), last);
    }

    #[test]
    fn a_wide_window_shows_everything_and_a_narrow_one_drops_from_the_right() {
        let toolbar = Toolbar::default();
        let wide = toolbar.fit(1400.);
        assert!(wide.shown.iter().all(|on| *on));
        assert!(wide.actions_words && wide.actions_hint);
        let narrow = drawn(&toolbar, 560.);
        assert!(narrow.contains(&"location".to_owned()));
        assert!(narrow.contains(&"actions".to_owned()));
        assert!(
            narrow.contains(&"back".to_owned()),
            "left items stay longest"
        );
        assert!(
            !narrow.contains(&"filter".to_owned()),
            "right items leave first"
        );
        let tiny = toolbar.fit(320.);
        assert!(!tiny.actions_words && !tiny.actions_hint);
        assert_eq!(drawn(&toolbar, 320.), ["location", "actions"]);
    }

    #[test]
    fn a_separator_left_alone_is_not_drawn() {
        let toolbar = Toolbar::from_ids(
            &ids(&["location", "separator", "filter", "separator", "actions"]),
            &[],
        );
        assert_eq!(
            drawn(&toolbar, 1400.),
            ["location", "separator", "filter", "separator", "actions"]
        );
        let lonely = Toolbar::from_ids(
            &ids(&["separator", "location", "separator", "separator", "actions"]),
            &[],
        );
        assert_eq!(drawn(&lonely, 1400.), ["location", "separator", "actions"]);
    }
}
