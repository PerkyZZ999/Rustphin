//! What the toolbar holds, in which order, and what fits (W-106f).
//!
//! The toolbar is a list of items chosen in Settings, Toolbar, as in
//! Dolphin's "Configure Toolbars…": built-in buttons, any palette command,
//! and separators. The location field and the Command palette button always
//! stay: one is where the folder is shown, the other reaches every action.
//! The Settings button sits after the palette's, at the right end, by
//! default (W-154 replaced the worded Actions button with these two icon
//! buttons; W-156 put Settings last). When the
//! window is too narrow, items leave from the right end inwards.

use crate::metrics::{size, space};

/// How settings saved before W-154 named the Actions button.
const LEGACY_ACTIONS: &str = "actions";

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
    /// Settings (W-154).
    Settings,
    /// The command palette; always there. It replaced the Actions button
    /// (W-154), whose saved id it still reads.
    Palette,
    /// A thin line between groups.
    Separator,
    /// A palette command, by its id (`commands::COMMANDS`).
    Command(String),
}

/// The built-in items in the order "Available actions" lists them.
pub const BUILT_IN: [Item; 13] = [
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
    Item::Settings,
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
            Item::Settings => Icon::Settings,
            Item::Palette => Icon::Command,
            Item::Separator => Icon::Minus,
            Item::Command(id) => crate::palette::command_icon(&self.name_of_command(id)),
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
            Self::Settings => "settings".to_owned(),
            Self::Palette => "palette".to_owned(),
            Self::Separator => "separator".to_owned(),
            Self::Command(id) => format!("command:{id}"),
        }
    }

    /// The item an id names, if this build knows it. A command saved by
    /// its label, as before W-123, is read as its id.
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
            "settings" => Self::Settings,
            // The Actions button of before W-154 opened the palette.
            "palette" | LEGACY_ACTIONS => Self::Palette,
            "separator" => Self::Separator,
            other => {
                let named = other
                    .strip_prefix("command:")
                    .filter(|named| !named.is_empty())?;
                let by_label = crate::commands::COMMANDS
                    .iter()
                    .find(|command| command.label == named);
                Self::Command(by_label.map_or(named, |command| command.id).to_owned())
            }
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
            Self::Settings => "Settings".to_owned(),
            Self::Palette => "Command palette".to_owned(),
            Self::Separator => "Separator".to_owned(),
            Self::Command(id) => self.name_of_command(id),
        }
    }

    /// A command's label, or the id of one this build does not know.
    fn name_of_command(&self, id: &str) -> String {
        crate::commands::find(id).map_or_else(|| id.to_owned(), |command| command.label.to_owned())
    }

    /// The command the button runs (`commands::COMMANDS`), if it runs one.
    pub fn command_id(&self) -> Option<&'static str> {
        Some(match self {
            Self::Back => "go-back",
            Self::Forward => "go-forward",
            Self::Up => "go-up",
            Self::Reload => "reload",
            Self::Terminal => "open-terminal",
            Self::Split => "toggle-split",
            Self::Hidden => "toggle-hidden",
            Self::Filter => "show-filter",
            Self::Settings => "open-settings",
            Self::Palette => "open-palette",
            Self::Command(id) => return crate::commands::find(id).map(|command| command.id),
            Self::Admin | Self::Location | Self::New | Self::View | Self::Separator => {
                return None;
            }
        })
    }

    /// The key that does the same, as its tooltip names it: its command's
    /// (`commands::COMMANDS`).
    pub fn keys(&self) -> Option<String> {
        crate::commands::find(self.command_id()?)?.shown_key()
    }

    /// Whether it must stay on the toolbar.
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Location | Self::Palette)
    }

    /// Whether it can show its words beside its icon.
    pub fn can_show_words(&self) -> bool {
        !matches!(self, Self::Location | Self::Separator)
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
            Item::Palette,
            Item::Settings,
        ];
        Self {
            items: items.into_iter().map(|item| (item, false)).collect(),
        }
    }
}

impl Toolbar {
    /// Reads the saved lists: item ids in order, and those showing words.
    /// Unknown ids and repeated buttons are dropped; the location field
    /// and the palette button are put back if missing. A list saved with
    /// the Actions button, before W-154, gains Settings after the palette
    /// as today's toolbar has it. An empty list is today's.
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
        let palette = match list.iter().position(|(item, _)| *item == Item::Palette) {
            Some(at) => at,
            None => {
                list.push((Item::Palette, false));
                list.len() - 1
            }
        };
        let from_actions = items.iter().any(|id| id == LEGACY_ACTIONS);
        if from_actions && !list.iter().any(|(item, _)| *item == Item::Settings) {
            list.insert(palette + 1, (Item::Settings, false));
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

    /// Adds `item` after position `after`, or at the end before the
    /// palette button.
    /// Returns where it went; `None` when it is there already.
    pub fn add(&mut self, item: Item, after: Option<usize>) -> Option<usize> {
        if self.contains(&item) {
            return None;
        }
        let palette = self
            .items
            .iter()
            .position(|(on, _)| *on == Item::Palette)
            .unwrap_or(self.items.len());
        let at = after.map_or(palette, |after| (after + 1).min(self.items.len()));
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
        let total = |shown: &[bool]| -> f32 {
            self.items
                .iter()
                .zip(shown)
                .filter(|(_, on)| **on)
                .map(|((item, words), _)| item.width(*words))
                .sum()
        };
        while total(&shown) > available {
            // The rightmost item that may leave, the palette aside.
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
        Fitted { shown }
    }
}

/// What fits: which items are drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fitted {
    /// Per item, whether it is drawn.
    pub shown: Vec<bool>,
}

/// What "Available actions" offers: the built-in buttons and every palette
/// command not on the toolbar yet (a separator always), matching `search`.
pub fn available(toolbar: &Toolbar, search: &str) -> Vec<Item> {
    let search = search.trim().to_lowercase();
    BUILT_IN
        .iter()
        .cloned()
        .chain(
            crate::commands::COMMANDS
                .iter()
                .map(|command| Item::Command(command.id.to_owned())),
        )
        .filter(|item| !toolbar.contains(item))
        .filter(|item| search.is_empty() || item.name().to_lowercase().contains(&search))
        .collect()
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
        toolbar.add(Item::Command("copy-location".to_owned()), None);
        toolbar.set_words(2, true);
        let (items, words) = toolbar.to_ids();
        assert_eq!(Toolbar::from_ids(&items, &words), toolbar);
        // Unknown ids and repeated buttons go; Location and the palette
        // return.
        let odd = Toolbar::from_ids(
            &ids(&["back", "wormhole", "back", "separator", "separator"]),
            &ids(&["back", "separator"]),
        );
        let names: Vec<String> = odd.items.iter().map(|(item, _)| item.id()).collect();
        assert_eq!(
            names,
            ["back", "location", "separator", "separator", "palette"]
        );
        assert!(odd.items[0].1, "back shows its words");
        assert!(!odd.items[2].1, "a separator has none");
        assert_eq!(Toolbar::from_ids(&[], &[]), Toolbar::default());
        assert_eq!(Item::from_id("command:"), None);
    }

    #[test]
    fn a_command_button_is_saved_by_its_id_and_read_from_its_old_label() {
        let item = Item::Command("copy-location".to_owned());
        assert_eq!(item.id(), "command:copy-location");
        assert_eq!(Item::from_id("command:copy-location"), Some(item.clone()));
        assert_eq!(
            Item::from_id("command:Files: Copy location"),
            Some(item.clone())
        );
        assert_eq!(item.name(), "Files: Copy location");
        assert_eq!(item.keys().as_deref(), Some("Ctrl+Alt+C"));
        let gone = Item::Command("Files: Gone".to_owned());
        assert_eq!(Item::from_id("command:Files: Gone"), Some(gone.clone()));
        assert_eq!(gone.command_id(), None);
    }

    #[test]
    fn a_toolbar_saved_with_actions_gets_settings_and_the_palette_in_its_place() {
        let names = |toolbar: &Toolbar| {
            toolbar
                .items
                .iter()
                .map(|(item, _)| item.id())
                .collect::<Vec<_>>()
        };
        let old = Toolbar::from_ids(&ids(&["back", "location", "actions", "filter"]), &[]);
        assert_eq!(
            names(&old),
            ["back", "location", "palette", "settings", "filter"]
        );
        // Saved again, it says so in today's words, and stays as it is.
        let (items, words) = old.to_ids();
        assert_eq!(Toolbar::from_ids(&items, &words), old);
        // Settings taken off today stays off; the palette always returns.
        let without = Toolbar::from_ids(&ids(&["location", "filter"]), &[]);
        assert_eq!(names(&without), ["location", "filter", "palette"]);
        // Both are icon buttons running their commands, with their keys.
        assert_eq!(Item::Settings.keys().as_deref(), Some("Ctrl+Shift+,"));
        assert_eq!(Item::Palette.keys().as_deref(), Some("Ctrl+Shift+P"));
        assert_eq!(Item::Palette.icon(), crate::icons::Icon::Command);
        assert_eq!(Item::Settings.icon(), crate::icons::Icon::Settings);
        assert!(Item::Palette.is_fixed() && !Item::Settings.is_fixed());
        // Today's toolbar ends with the palette, then Settings (W-156).
        let mut toolbar = Toolbar::default();
        let settings = toolbar.items.len() - 1;
        assert_eq!(toolbar.items[settings].0, Item::Settings);
        assert_eq!(toolbar.items[settings - 1].0, Item::Palette);
        assert!(toolbar.remove(settings), "Settings can go");
        assert!(available(&toolbar, "").contains(&Item::Settings));
    }

    #[test]
    fn location_and_the_palette_cannot_be_removed_and_buttons_appear_once() {
        let mut toolbar = Toolbar::default();
        let location = toolbar
            .items
            .iter()
            .position(|(i, _)| *i == Item::Location)
            .unwrap();
        assert!(!toolbar.remove(location));
        let palette = toolbar
            .items
            .iter()
            .position(|(i, _)| *i == Item::Palette)
            .unwrap();
        assert!(!toolbar.remove(palette), "the palette stays");
        assert_eq!(toolbar.add(Item::Back, None), None);
        let at = toolbar.add(Item::Separator, None).unwrap();
        assert_eq!(
            toolbar.items[at + 1].0,
            Item::Palette,
            "added before the palette"
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
        let narrow = drawn(&toolbar, 560.);
        assert!(narrow.contains(&"location".to_owned()));
        assert!(narrow.contains(&"palette".to_owned()));
        assert!(
            narrow.contains(&"back".to_owned()),
            "left items stay longest"
        );
        assert!(
            !narrow.contains(&"filter".to_owned()),
            "right items leave first"
        );
        assert_eq!(drawn(&toolbar, 320.), ["location", "palette"]);
    }

    #[test]
    fn a_separator_left_alone_is_not_drawn() {
        let toolbar = Toolbar::from_ids(
            &ids(&["location", "separator", "filter", "separator", "palette"]),
            &[],
        );
        assert_eq!(
            drawn(&toolbar, 1400.),
            ["location", "separator", "filter", "separator", "palette"]
        );
        let lonely = Toolbar::from_ids(
            &ids(&["separator", "location", "separator", "separator", "palette"]),
            &[],
        );
        assert_eq!(drawn(&lonely, 1400.), ["location", "separator", "palette"]);
    }

    #[test]
    fn available_actions_leave_out_what_is_on_the_toolbar_but_offer_separators() {
        let toolbar = Toolbar::default();
        let offered = available(&toolbar, "");
        assert!(!offered.contains(&Item::Back), "Back is on the toolbar");
        assert!(offered.contains(&Item::Separator), "a separator always");
        let found = available(&toolbar, "  COPY LOCATION ");
        assert!(!found.is_empty());
        assert!(
            found
                .iter()
                .all(|item| item.name().to_lowercase().contains("copy location"))
        );
        assert!(available(&toolbar, "no such action at all").is_empty());
    }
}
