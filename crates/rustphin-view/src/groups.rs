//! Place groups: user-defined, collapsible sections of the places panel.
//!
//! Groups live in Rustphin's own file and refer to places by the identifier
//! KDE gives each bookmark; the shared places file knows nothing of them
//! (decision D-008). A place belongs to at most one group. Reading never
//! fails: a missing or damaged file yields no groups, and a damaged, newer
//! or unreadable file is kept rather than replaced ([`GroupsFile`]).
//!
//! The same file keeps the panel's separators (W-152): a hairline between
//! groups (an entry of the group list that is a separator), between the
//! places of a group (an entry of its places), or between the places outside
//! groups ([`LooseSeparator`], below the place it follows, as those places
//! keep the shared file's order). Each has an identifier of its own,
//! `separator-N`, which no KDE bookmark identifier can be.

use std::{collections::HashSet, io};

use rustphin_platform::places::Place;
use serde::{Deserialize, Serialize};

use crate::saved_file::{self, FileIssue};

/// The format version this build writes. Version 2 added separators
/// (W-152): an older Rustphin keeps such a file rather than losing them.
const VERSION: u32 = 2;

/// How every separator's identifier starts.
const SEPARATOR_PREFIX: &str = "separator-";

/// Whether `id` names a separator rather than a place.
pub fn is_separator(id: &str) -> bool {
    id.starts_with(SEPARATOR_PREFIX)
}

/// One group.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Group {
    /// The name shown as the section heading.
    pub name: String,
    /// Whether its places are folded away.
    pub collapsed: bool,
    /// Identifiers of its places, in the order shown; a separator's among
    /// them is drawn as a hairline between them.
    pub places: Vec<String>,
    /// Set when this entry is no group but a separator between groups:
    /// its identifier. It then has no name and no places.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
}

impl Group {
    /// Whether this entry is a group, not a separator between groups.
    pub fn is_group(&self) -> bool {
        self.separator.is_none()
    }

    /// How many places it holds, separators aside.
    pub fn place_count(&self) -> usize {
        self.places.iter().filter(|id| !is_separator(id)).count()
    }
}

/// A separator among the places outside groups.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LooseSeparator {
    /// Its identifier.
    pub id: String,
    /// The place it is drawn below; none: above them all. It goes when that
    /// place leaves the shared file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// Where a separator is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorAt {
    /// Among the places outside groups, at this position of
    /// [`Groups::separators`].
    Loose(usize),
    /// Between groups, at this position of the group list.
    Between(usize),
    /// In the group at the first position, at the second of its places.
    InGroup(usize, usize),
}

/// All groups, in the order shown.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Groups {
    version: u32,
    /// Whether the Devices section is folded away.
    pub devices_folded: bool,
    /// Whether the Remote section is folded away.
    pub remote_folded: bool,
    /// The separators among the places outside groups, in the order drawn
    /// where two follow the same place.
    pub separators: Vec<LooseSeparator>,
    /// The groups and the separators between them, top to bottom. Last:
    /// TOML writes tables after values.
    pub groups: Vec<Group>,
}

/// Whether the groups file may be written (W-140): not before it has been
/// read, so a change made meanwhile cannot replace it, and not when it
/// could not be read, is damaged or was written by a newer Rustphin, so it
/// is kept for the user as the saved tabs are.
#[derive(Debug, Default)]
pub struct GroupsFile {
    state: FileState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum FileState {
    #[default]
    Unread,
    Writable,
    Kept,
}

impl GroupsFile {
    /// Takes what reading the file gave (`None` when there is no file):
    /// the groups to show, if it held any that can be used, and why the
    /// file is kept, to tell the user.
    pub fn loaded(
        &mut self,
        saved: io::Result<Option<String>>,
    ) -> (Option<Groups>, Option<FileIssue>) {
        let decoded = saved_file::decode::<Groups>(saved, VERSION);
        self.state = if decoded.issue.is_some() {
            FileState::Kept
        } else {
            FileState::Writable
        };
        (decoded.value.map(Groups::tidied), decoded.issue)
    }

    /// Whether a change may be written now.
    pub fn may_save(&self) -> bool {
        self.state == FileState::Writable
    }
}

/// The places panel's content: the places outside any group, then each group
/// with its places.
#[derive(Debug, PartialEq, Eq)]
pub struct Layout<'a> {
    /// Places in no group, in the shared file's order.
    pub ungrouped: Vec<&'a Place>,
    /// Each entry of the group list with what it holds: a group's places
    /// that still exist and its separators; nothing for a separator
    /// between groups.
    pub groups: Vec<(&'a Group, Vec<Member<'a>>)>,
}

/// One line inside a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Member<'a> {
    /// A place.
    Place(&'a Place),
    /// A separator, by its identifier.
    Separator(&'a str),
}

impl Groups {
    /// Reads the groups file's text.
    pub fn parse(text: &str) -> Self {
        toml::from_str::<Self>(text).unwrap_or_default().tidied()
    }

    /// A place listed twice keeps its first group; a separator listed twice
    /// keeps its first position; a separator between groups holds nothing.
    fn tidied(mut self) -> Self {
        let mut seen = HashSet::new();
        self.separators
            .retain(|separator| is_separator(&separator.id) && seen.insert(separator.id.clone()));
        self.groups.retain_mut(|group| match &group.separator {
            Some(id) => {
                group.places.clear();
                group.name.clear();
                is_separator(id) && seen.insert(id.clone())
            }
            None => {
                group.places.retain(|id| seen.insert(id.clone()));
                true
            }
        });
        self
    }

    /// The text to store.
    pub fn to_toml(&self) -> String {
        let stored = Self {
            version: VERSION,
            devices_folded: self.devices_folded,
            remote_folded: self.remote_folded,
            separators: self.separators.clone(),
            groups: self.groups.clone(),
        };
        toml::to_string(&stored).unwrap_or_default()
    }

    /// Whether the file was written by a later version and must not be overwritten.
    pub fn is_from_newer_version(&self) -> bool {
        self.version > VERSION
    }

    /// The index of the group holding the place or separator `id`.
    pub fn group_of(&self, id: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.places.iter().any(|place| place == id))
    }

    /// A name for a new group that no group has yet.
    pub fn free_name(&self) -> String {
        crate::naming::free_name("New Group", |name| {
            self.groups
                .iter()
                .any(|group| group.is_group() && group.name == name)
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

    /// Renames a group. An empty name, or a separator, is ignored.
    pub fn rename(&mut self, index: usize, name: &str) {
        let group = self.groups.get_mut(index).filter(|group| group.is_group());
        if let (Some(group), false) = (group, name.trim().is_empty()) {
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

    /// Moves the group at `from` to `to`, taking that group's place, as a
    /// group dragged onto another's heading does. Returns where it went.
    pub fn move_group(&mut self, from: usize, to: usize) -> usize {
        if from >= self.groups.len() {
            return from;
        }
        let to = to.min(self.groups.len() - 1);
        let group = self.groups.remove(from);
        self.groups.insert(to, group);
        to
    }

    /// Folds or unfolds a group.
    pub fn set_collapsed(&mut self, index: usize, collapsed: bool) {
        if let Some(group) = self.groups.get_mut(index) {
            group.collapsed = collapsed;
        }
    }

    /// Puts the place `id` into the group at `group`, before the place
    /// `before` or at its end; with `group` `None`, takes it out of its group.
    /// It leaves whatever group it was in. A separator between groups takes
    /// nothing.
    pub fn assign(&mut self, id: &str, group: Option<usize>, before: Option<&str>) {
        if before == Some(id) {
            return;
        }
        for existing in &mut self.groups {
            existing.places.retain(|place| place != id);
        }
        let target = group
            .and_then(|index| self.groups.get_mut(index))
            .filter(|target| target.is_group());
        if let Some(target) = target {
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
    ///
    /// A separator below a place outside groups goes with that place.
    pub fn forget_missing(&mut self, existing: &[Place]) -> bool {
        let known: HashSet<&str> = existing.iter().map(|place| place.id.as_str()).collect();
        let mut changed = false;
        for group in &mut self.groups {
            let before = group.places.len();
            group
                .places
                .retain(|id| is_separator(id) || known.contains(id.as_str()));
            changed |= group.places.len() != before;
        }
        let before = self.separators.len();
        self.separators.retain(|separator| {
            separator
                .after
                .as_deref()
                .is_none_or(|after| known.contains(after))
        });
        changed | (self.separators.len() != before)
    }

    // ----- Separators (W-152) -----

    /// Where the separator `id` is.
    pub fn find_separator(&self, id: &str) -> Option<SeparatorAt> {
        if let Some(at) = self.separators.iter().position(|loose| loose.id == id) {
            return Some(SeparatorAt::Loose(at));
        }
        self.groups.iter().enumerate().find_map(|(index, group)| {
            if group.separator.as_deref() == Some(id) {
                return Some(SeparatorAt::Between(index));
            }
            group
                .places
                .iter()
                .position(|place| place == id)
                .map(|at| SeparatorAt::InGroup(index, at))
        })
    }

    /// An identifier no separator has yet.
    fn new_separator_id(&self) -> String {
        let number = |id: &str| {
            id.strip_prefix(SEPARATOR_PREFIX)
                .and_then(|number| number.parse::<u64>().ok())
        };
        let highest = self
            .separators
            .iter()
            .map(|loose| loose.id.as_str())
            .chain(
                self.groups
                    .iter()
                    .filter_map(|group| group.separator.as_deref()),
            )
            .chain(
                self.groups
                    .iter()
                    .flat_map(|group| group.places.iter().map(String::as_str)),
            )
            .filter_map(number)
            .max()
            .unwrap_or(0);
        format!("{SEPARATOR_PREFIX}{}", highest + 1)
    }

    /// Adds a separator between groups, above the entry at `index` (at the
    /// end past the last), and returns its identifier.
    pub fn add_separator_between(&mut self, index: usize) -> String {
        let id = self.new_separator_id();
        let separator = Group {
            separator: Some(id.clone()),
            ..Group::default()
        };
        self.groups.insert(index.min(self.groups.len()), separator);
        id
    }

    /// Adds a separator in the group at `group`, at position `at` of its
    /// places (at the end past the last); `None` for a separator between
    /// groups or no such group.
    pub fn add_separator_in(&mut self, group: usize, at: usize) -> Option<String> {
        let id = self.new_separator_id();
        let group = self
            .groups
            .get_mut(group)
            .filter(|group| group.is_group())?;
        group.places.insert(at.min(group.places.len()), id.clone());
        Some(id)
    }

    /// Adds a separator among the places outside groups, below the place
    /// `after` (none: above them all), drawn below any already there.
    pub fn add_loose_separator(&mut self, after: Option<&str>) -> String {
        let id = self.new_separator_id();
        self.place_loose(id.clone(), after.map(str::to_owned), None);
        id
    }

    /// Puts a loose separator below `after`, before the loose separator
    /// `before` when it is one there, else after those already there.
    fn place_loose(&mut self, id: String, after: Option<String>, before: Option<&str>) {
        let at = before
            .and_then(|before| {
                self.separators
                    .iter()
                    .position(|loose| loose.id == before && loose.after == after)
            })
            .or_else(|| {
                self.separators
                    .iter()
                    .rposition(|loose| loose.after == after)
                    .map(|at| at + 1)
            })
            .unwrap_or(self.separators.len());
        self.separators.insert(at, LooseSeparator { id, after });
    }

    /// Deletes the separator `id`. Returns whether there was one.
    pub fn remove_separator(&mut self, id: &str) -> bool {
        match self.find_separator(id) {
            Some(SeparatorAt::Loose(at)) => {
                self.separators.remove(at);
            }
            Some(SeparatorAt::Between(index)) => {
                self.groups.remove(index);
            }
            Some(SeparatorAt::InGroup(group, at)) => {
                self.groups[group].places.remove(at);
            }
            None => return false,
        }
        true
    }

    /// Moves the separator `id` among the places outside groups, below the
    /// place `after`, before the loose separator `before` when that is one
    /// there; it leaves wherever it was.
    pub fn move_separator_loose(&mut self, id: &str, after: Option<&str>, before: Option<&str>) {
        if before == Some(id) || !self.remove_separator(id) {
            return;
        }
        self.place_loose(id.to_owned(), after.map(str::to_owned), before);
    }

    /// Moves the separator `id` between groups, above the entry at `index`
    /// as counted before it moved; it leaves wherever it was. Returns where
    /// it went.
    pub fn move_separator_between(&mut self, id: &str, index: usize) -> Option<usize> {
        let at = self.find_separator(id)?;
        let index = match at {
            SeparatorAt::Between(from) if from == index => return Some(from),
            // Taken out above, it moves the entries below it up one.
            SeparatorAt::Between(from) if from < index => index - 1,
            _ => index,
        };
        self.remove_separator(id);
        let index = index.min(self.groups.len());
        self.groups.insert(
            index,
            Group {
                separator: Some(id.to_owned()),
                ..Group::default()
            },
        );
        Some(index)
    }

    /// Moves the separator `id` into the group at `group`, before the place
    /// or separator `before` or at its end; it leaves wherever it was.
    pub fn move_separator_in(&mut self, id: &str, group: usize, before: Option<&str>) {
        if before == Some(id)
            || !self.groups.get(group).is_some_and(Group::is_group)
            || self.find_separator(id).is_none()
        {
            return;
        }
        // Taking a separator between groups out moves the later groups up.
        let group = match self.find_separator(id) {
            Some(SeparatorAt::Between(index)) if index < group => group - 1,
            _ => group,
        };
        self.remove_separator(id);
        let target = &mut self.groups[group];
        let at = before
            .and_then(|before| target.places.iter().position(|place| place == before))
            .unwrap_or(target.places.len());
        target.places.insert(at, id.to_owned());
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
                        .filter_map(|id| {
                            if is_separator(id) {
                                return Some(Member::Separator(id));
                            }
                            places
                                .iter()
                                .find(|place| place.id == *id)
                                .map(Member::Place)
                        })
                        .collect();
                    (group, members)
                })
                .collect(),
        }
    }
}

#[cfg(test)]
// A test cannot go on without what it unwraps.
#[allow(clippy::unwrap_used)]
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
            devices_folded: false,
            remote_folded: false,
            separators: Vec::new(),
            groups: spec
                .iter()
                .map(|(name, places)| Group {
                    name: (*name).to_owned(),
                    collapsed: false,
                    places: places.iter().map(|id| (*id).to_owned()).collect(),
                    separator: None,
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
    fn a_dragged_group_takes_the_place_of_the_one_it_is_dropped_on() {
        let names = |g: &Groups| g.groups.iter().map(|g| g.name.clone()).collect::<Vec<_>>();
        let mut g = groups(&[("A", &["1"]), ("B", &[]), ("C", &["2"])]);
        assert_eq!(g.move_group(0, 2), 2);
        assert_eq!(names(&g), ["B", "C", "A"]);
        assert_eq!(g.move_group(2, 0), 0);
        assert_eq!(names(&g), ["A", "B", "C"]);
        // A group keeps its places on the way.
        assert_eq!(g.groups[0].places, ["1"]);
        // Past the end lands last; a group that is not there moves nothing.
        assert_eq!(g.move_group(0, 9), 2);
        assert_eq!(names(&g), ["B", "C", "A"]);
        assert_eq!(g.move_group(7, 0), 7);
        assert_eq!(names(&g), ["B", "C", "A"]);
    }

    #[test]
    fn the_folded_sections_are_kept_with_the_groups_and_older_files_still_load() {
        let mut groups = Groups::parse("version = 1\n");
        assert!(!groups.devices_folded && !groups.remote_folded);
        groups.devices_folded = true;
        groups.remote_folded = true;
        groups.add("Work");
        let again = Groups::parse(&groups.to_toml());
        assert!(again.devices_folded && again.remote_folded);
        assert_eq!(again.groups[0].name, "Work");
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
    fn the_file_is_not_written_before_it_has_been_read() {
        assert!(!GroupsFile::default().may_save());
    }

    #[test]
    fn a_missing_or_readable_file_may_be_replaced() {
        let mut missing = GroupsFile::default();
        let (groups, issue) = missing.loaded(Ok(None));
        assert!(groups.is_none() && issue.is_none());
        assert!(missing.may_save());

        let mut readable = GroupsFile::default();
        let text = "version = 1\n[[groups]]\nname = \"A\"\nplaces = [\"x\"]\n";
        let (groups, issue) = readable.loaded(Ok(Some(text.to_owned())));
        assert_eq!(
            groups.map(|groups| shape(&groups)),
            Some(expected(&[("A", &["x"])]))
        );
        assert!(issue.is_none());
        assert!(readable.may_save());
    }

    #[test]
    fn a_damaged_file_is_kept_and_shows_no_groups() {
        let mut file = GroupsFile::default();
        let (groups, issue) = file.loaded(Ok(Some("groups = 3 [[".to_owned())));
        assert!(groups.is_none());
        assert!(matches!(issue, Some(FileIssue::Damaged(_))));
        assert!(!file.may_save());
    }

    #[test]
    fn a_newer_file_is_shown_as_far_as_understood_and_kept() {
        let mut file = GroupsFile::default();
        let text = "version = 99\n[[groups]]\nname = \"A\"\nplaces = [\"x\", \"x\"]\n";
        let (groups, issue) = file.loaded(Ok(Some(text.to_owned())));
        assert_eq!(
            groups.map(|groups| shape(&groups)),
            Some(expected(&[("A", &["x"])]))
        );
        assert!(matches!(issue, Some(FileIssue::NewerVersion(99))));
        assert!(!file.may_save());
    }

    #[test]
    fn an_unreadable_file_is_kept() {
        let mut file = GroupsFile::default();
        let (groups, issue) = file.loaded(Err(io::ErrorKind::PermissionDenied.into()));
        assert!(groups.is_none());
        assert!(matches!(issue, Some(FileIssue::Read(_))));
        assert!(!file.may_save());
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
        assert_eq!(
            layout.groups[0].1,
            [Member::Place(&places[3]), Member::Place(&places[1])]
        );
        assert!(layout.groups[1].1.is_empty());
    }

    #[test]
    fn places_gone_from_the_shared_file_are_forgotten_on_request() {
        let mut g = groups(&[("A", &["x", "gone"]), ("B", &["y"])]);
        assert!(g.forget_missing(&[place("x"), place("y")]));
        assert_eq!(shape(&g), expected(&[("A", &["x"]), ("B", &["y"])]));
        assert!(!g.forget_missing(&[place("x"), place("y")]));
    }

    #[test]
    fn separators_of_every_kind_survive_being_written_and_read_back() {
        let mut g = groups(&[("A", &["x", "y"]), ("B", &[])]);
        let loose = g.add_loose_separator(Some("p"));
        let between = g.add_separator_between(1);
        let inside = g.add_separator_in(0, 1).unwrap();
        assert_eq!(
            [loose.as_str(), &between, &inside],
            ["separator-1", "separator-2", "separator-3"]
        );
        let text = g.to_toml();
        assert!(text.starts_with("version = 2"), "{text}");
        let again = Groups::parse(&text);
        assert_eq!(again, g);
        assert_eq!(again.find_separator(&loose), Some(SeparatorAt::Loose(0)));
        assert_eq!(
            again.find_separator(&between),
            Some(SeparatorAt::Between(1))
        );
        assert_eq!(
            again.find_separator(&inside),
            Some(SeparatorAt::InGroup(0, 1))
        );
        // A separator is no place: a group counts its places only.
        assert_eq!(again.groups[0].place_count(), 2);
        assert!(!again.groups[1].is_group());
    }

    #[test]
    fn a_separator_between_groups_takes_no_places_and_no_name() {
        let mut g = groups(&[("A", &[]), ("B", &[])]);
        g.add_separator_between(1);
        g.assign("x", Some(1), None);
        g.rename(1, "Named");
        assert!(g.groups[1].places.is_empty() && g.groups[1].name.is_empty());
        assert_eq!(g.group_of("x"), None);
        assert_eq!(g.add_separator_in(1, 0), None);
        // Even a hand-edited file cannot give it any.
        let text = "version = 2\n[[groups]]\nseparator = \"separator-1\"\nname = \"S\"\nplaces = [\"x\"]\n";
        let read = Groups::parse(text);
        assert!(read.groups[0].places.is_empty() && read.groups[0].name.is_empty());
    }

    #[test]
    fn a_separator_listed_twice_keeps_its_first_position_and_ids_stay_unique() {
        let text = "version = 2\n[[separators]]\nid = \"separator-4\"\n[[groups]]\nseparator = \"separator-4\"\n[[groups]]\nname = \"A\"\nplaces = [\"separator-4\", \"separator-9\"]\n";
        let mut g = Groups::parse(text);
        assert_eq!(g.separators.len(), 1);
        assert_eq!(g.groups.len(), 1, "the duplicate between groups went");
        assert_eq!(g.groups[0].places, ["separator-9"]);
        // A new one is numbered past the highest, even after one is deleted.
        assert!(g.remove_separator("separator-9"));
        assert!(!g.remove_separator("separator-9"));
        assert_eq!(g.add_loose_separator(None), "separator-5");
    }

    #[test]
    fn a_version_one_file_still_loads_and_a_loose_separator_goes_with_its_place() {
        let mut g = Groups::parse("version = 1\n[[groups]]\nname = \"A\"\nplaces = [\"x\"]\n");
        assert_eq!(g.groups[0].places, ["x"]);
        assert!(!g.is_from_newer_version());
        g.add_loose_separator(Some("x"));
        g.add_loose_separator(None);
        let inside = g.add_separator_in(0, 0).unwrap();
        assert!(g.forget_missing(&[place("y")]));
        // The one below x went with it; the one at the top and the one in
        // the group are no places and stay.
        assert_eq!(g.separators.len(), 1);
        assert_eq!(g.separators[0].after, None);
        assert_eq!(g.groups[0].places, [inside]);
    }

    #[test]
    fn moving_a_separator_between_kinds_keeps_one_copy_in_the_right_spot() {
        let mut g = groups(&[("A", &["x", "y"]), ("B", &["z"])]);
        let id = g.add_separator_between(0);
        // Into B, which moved up one when the separator left the list.
        g.move_separator_in(&id, 2, Some("z"));
        assert_eq!(g.groups.len(), 2);
        assert_eq!(g.groups[1].places, [id.clone(), "z".to_owned()]);
        // Out of the group, between A and B.
        assert_eq!(g.move_separator_between(&id, 1), Some(1));
        assert_eq!(g.groups[1].separator.as_deref(), Some(id.as_str()));
        assert_eq!(g.groups[2].places, ["z"]);
        // Down past B: counted before it moved, above the end.
        assert_eq!(g.move_separator_between(&id, 3), Some(2));
        // Out of the groups altogether.
        g.move_separator_loose(&id, Some("p"), None);
        assert_eq!(g.groups.len(), 2);
        assert_eq!(g.find_separator(&id), Some(SeparatorAt::Loose(0)));
    }
}
