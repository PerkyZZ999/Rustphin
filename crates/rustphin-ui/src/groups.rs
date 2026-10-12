//! Place groups: user-defined, collapsible sections of the places panel.
//!
//! Groups live in Rustphin's own file and refer to places by the identifier
//! KDE gives each bookmark; the shared places file knows nothing of them
//! (decision D-008). A place belongs to at most one group. Reading never
//! fails: a missing or damaged file yields no groups.

use std::collections::HashSet;

use rustphin_platform::places::Place;
use serde::{Deserialize, Serialize};

/// The format version this build writes.
const VERSION: u32 = 1;

/// One group.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    /// The name shown as the section heading.
    pub name: String,
    /// Whether its places are folded away.
    pub collapsed: bool,
    /// Identifiers of its places, in the order shown.
    pub places: Vec<String>,
}

/// All groups, in the order shown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Groups {
    version: u32,
    /// The groups, top to bottom.
    pub groups: Vec<Group>,
}

/// The places panel's content: the places outside any group, then each group
/// with its places.
#[derive(Debug, PartialEq, Eq)]
pub struct Layout<'a> {
    /// Places in no group, in the shared file's order.
    pub ungrouped: Vec<&'a Place>,
    /// Each group with the places it holds that still exist.
    pub groups: Vec<(&'a Group, Vec<&'a Place>)>,
}

impl Groups {
    /// Reads the groups file's text.
    pub fn parse(text: &str) -> Self {
        let mut groups: Self = toml::from_str(text).unwrap_or_default();
        // A place listed twice keeps its first group.
        let mut seen = HashSet::new();
        for group in &mut groups.groups {
            group.places.retain(|id| seen.insert(id.clone()));
        }
        groups
    }

    /// The text to store.
    pub fn to_toml(&self) -> String {
        let stored = Self {
            version: VERSION,
            groups: self.groups.clone(),
        };
        toml::to_string(&stored).unwrap_or_default()
    }

    /// Whether the file was written by a later version and must not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }

    /// The index of the group holding the place `id`.
    pub fn group_of(&self, id: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.places.iter().any(|place| place == id))
    }

    /// A name for a new group that no group has yet.
    pub fn free_name(&self) -> String {
        crate::operations::free_name("New Group", |name| {
            self.groups.iter().any(|group| group.name == name)
        })
    }

    /// Adds an empty group at the end and returns its index. An empty name
    /// is replaced by a free default.
    pub fn add(&mut self, name: &str) -> usize {
        let name = match name.trim() {
            "" => self.free_name(),
            name => name.to_owned(),
        };
        self.groups.push(Group {
            name,
            ..Group::default()
        });
        self.groups.len() - 1
    }

    /// Renames a group. An empty name is ignored.
    pub fn rename(&mut self, index: usize, name: &str) {
        if let (Some(group), false) = (self.groups.get_mut(index), name.trim().is_empty()) {
            group.name = name.trim().to_owned();
        }
    }

    /// Deletes a group. Its places are not deleted: they show up outside any group.
    pub fn remove(&mut self, index: usize) {
        if index < self.groups.len() {
            self.groups.remove(index);
        }
    }

    /// Moves a group up (negative) or down by `delta` positions, stopping at
    /// the ends. Returns its new index.
    pub fn shift(&mut self, index: usize, delta: isize) -> usize {
        if index >= self.groups.len() {
            return index;
        }
        let target = index
            .saturating_add_signed(delta)
            .min(self.groups.len() - 1);
        let group = self.groups.remove(index);
        self.groups.insert(target, group);
        target
    }

    /// Folds or unfolds a group.
    pub fn set_collapsed(&mut self, index: usize, collapsed: bool) {
        if let Some(group) = self.groups.get_mut(index) {
            group.collapsed = collapsed;
        }
    }

    /// Puts the place `id` into the group at `group`, before the place
    /// `before` or at its end; with `group` `None`, takes it out of its group.
    /// It leaves whatever group it was in.
    pub fn assign(&mut self, id: &str, group: Option<usize>, before: Option<&str>) {
        if before == Some(id) {
            return;
        }
        for existing in &mut self.groups {
            existing.places.retain(|place| place != id);
        }
        if let Some(target) = group.and_then(|index| self.groups.get_mut(index)) {
            let at = before
                .and_then(|before| target.places.iter().position(|place| place == before))
                .unwrap_or(target.places.len());
            target.places.insert(at, id.to_owned());
        }
    }

    /// Moves the place `id` up or down by one within its group. Returns
    /// whether it moved.
    pub fn shift_place(&mut self, id: &str, down: bool) -> bool {
        let Some(group) = self
            .group_of(id)
            .and_then(|index| self.groups.get_mut(index))
        else {
            return false;
        };
        let Some(at) = group.places.iter().position(|place| place == id) else {
            return false;
        };
        let target = if down { at + 1 } else { at.wrapping_sub(1) };
        if target >= group.places.len() {
            return false;
        }
        group.places.swap(at, target);
        true
    }

    /// Forgets places that are no longer in the shared file. Returns whether
    /// anything was forgotten.
    ///
    /// Only call this with a list that was read successfully: an unreadable
    /// places file must not empty every group.
    pub fn forget_missing(&mut self, existing: &[Place]) -> bool {
        let known: HashSet<&str> = existing.iter().map(|place| place.id.as_str()).collect();
        let mut changed = false;
        for group in &mut self.groups {
            let before = group.places.len();
            group.places.retain(|id| known.contains(id.as_str()));
            changed |= group.places.len() != before;
        }
        changed
    }

    /// Arranges `places` for the panel.
    pub fn layout<'a>(&'a self, places: &'a [Place]) -> Layout<'a> {
        let grouped: HashSet<&str> = self
            .groups
            .iter()
            .flat_map(|group| group.places.iter().map(String::as_str))
            .collect();
        Layout {
            ungrouped: places
                .iter()
                .filter(|place| !grouped.contains(place.id.as_str()))
                .collect(),
            groups: self
                .groups
                .iter()
                .map(|group| {
                    let members = group
                        .places
                        .iter()
                        .filter_map(|id| places.iter().find(|place| place.id == *id))
                        .collect();
                    (group, members)
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(id: &str) -> Place {
        Place {
            id: id.to_owned(),
            title: id.to_uppercase(),
            href: format!("file:///{id}"),
            path: Some(format!("/{id}").into()),
            icon: None,
            hidden: false,
            system: false,
            only_in_app: None,
        }
    }

    fn groups(spec: &[(&str, &[&str])]) -> Groups {
        Groups {
            version: VERSION,
            groups: spec
                .iter()
                .map(|(name, places)| Group {
                    name: (*name).to_owned(),
                    collapsed: false,
                    places: places.iter().map(|id| (*id).to_owned()).collect(),
                })
                .collect(),
        }
    }

    fn shape(groups: &Groups) -> Vec<(String, Vec<String>)> {
        groups
            .groups
            .iter()
            .map(|group| (group.name.clone(), group.places.clone()))
            .collect()
    }

    fn expected(spec: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
        shape(&groups(spec))
    }

    #[test]
    fn groups_survive_being_written_and_read_back_with_order_and_collapsed_state() {
        let mut original = groups(&[("Work", &["1/0", "2/0"]), ("Média & co", &[])]);
        original.set_collapsed(1, true);

        let reread = Groups::parse(&original.to_toml());

        assert_eq!(reread, original);
        assert!(reread.groups[1].collapsed && !reread.groups[0].collapsed);
    }

    #[test]
    fn a_missing_or_damaged_file_means_no_groups() {
        assert_eq!(Groups::parse(""), Groups::default());
        assert_eq!(Groups::parse("groups = 3 [["), Groups::default());
    }

    #[test]
    fn a_place_listed_in_two_groups_stays_in_the_first() {
        let text = "[[groups]]\nname = \"A\"\nplaces = [\"x\", \"y\"]\n[[groups]]\nname = \"B\"\nplaces = [\"y\", \"z\"]\n";
        assert_eq!(
            shape(&Groups::parse(text)),
            expected(&[("A", &["x", "y"]), ("B", &["z"])])
        );
    }

    #[test]
    fn a_file_from_a_newer_version_is_recognised() {
        assert!(Groups::parse("version = 99").is_from_newer_version());
        assert!(!Groups::parse(&Groups::default().to_toml()).is_from_newer_version());
    }

    #[test]
    fn assigning_moves_a_place_between_groups_and_out_again() {
        let mut g = groups(&[("A", &["x", "y"]), ("B", &["z"])]);

        g.assign("x", Some(1), None);
        assert_eq!(shape(&g), expected(&[("A", &["y"]), ("B", &["z", "x"])]));
        g.assign("y", Some(1), Some("z"));
        assert_eq!(shape(&g), expected(&[("A", &[]), ("B", &["y", "z", "x"])]));
        g.assign("z", None, None);
        assert_eq!(shape(&g), expected(&[("A", &[]), ("B", &["y", "x"])]));
        assert_eq!(g.group_of("x"), Some(1));
        assert_eq!(g.group_of("z"), None);
        // A group that does not exist takes nothing, and the place is ungrouped.
        g.assign("x", Some(9), None);
        assert_eq!(g.group_of("x"), None);
    }

    #[test]
    fn a_place_can_be_reordered_within_its_group() {
        let mut g = groups(&[("A", &["x", "y", "z"])]);
        assert!(g.shift_place("x", true));
        assert_eq!(shape(&g), expected(&[("A", &["y", "x", "z"])]));
        assert!(g.shift_place("x", false));
        assert!(!g.shift_place("x", false), "already first");
        assert!(!g.shift_place("z", true), "already last");
        assert!(!g.shift_place("nowhere", true));
        assert_eq!(shape(&g), expected(&[("A", &["x", "y", "z"])]));
    }

    #[test]
    fn groups_are_added_renamed_reordered_and_deleted_without_losing_places() {
        let mut g = groups(&[("A", &["x"]), ("B", &["y"])]);

        assert_eq!(g.add("  C  "), 2);
        assert_eq!(g.add(""), 3);
        assert_eq!(g.add(" "), 4);
        assert_eq!(g.groups[3].name, "New Group");
        assert_eq!(g.groups[4].name, "New Group 2");
        g.rename(0, "Alpha");
        g.rename(1, "   ");
        assert_eq!(g.groups[0].name, "Alpha");
        assert_eq!(g.groups[1].name, "B", "an empty name is ignored");

        assert_eq!(g.shift(2, -5), 0, "stops at the top");
        assert_eq!(g.shift(0, 1), 1);
        assert_eq!(
            g.groups
                .iter()
                .map(|group| group.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "C", "B", "New Group", "New Group 2"]
        );

        g.remove(0);
        let places = [place("x"), place("y")];
        let layout = g.layout(&places);
        // Deleting the group did not delete its place: it is now ungrouped.
        assert_eq!(layout.ungrouped, [&places[0]]);
        assert_eq!(g.group_of("y"), Some(1));
    }

    #[test]
    fn the_layout_keeps_file_order_outside_groups_and_group_order_inside() {
        let places = [place("a"), place("b"), place("c"), place("d")];
        let g = groups(&[("Later", &["d", "gone", "b"]), ("Empty", &[])]);

        let layout = g.layout(&places);

        assert_eq!(layout.ungrouped, [&places[0], &places[2]]);
        assert_eq!(layout.groups[0].1, [&places[3], &places[1]]);
        assert!(layout.groups[1].1.is_empty());
    }

    #[test]
    fn places_gone_from_the_shared_file_are_forgotten_on_request() {
        let mut g = groups(&[("A", &["x", "gone"]), ("B", &["y"])]);
        assert!(g.forget_missing(&[place("x"), place("y")]));
        assert_eq!(shape(&g), expected(&[("A", &["x"]), ("B", &["y"])]));
        assert!(!g.forget_missing(&[place("x"), place("y")]));
    }
}
