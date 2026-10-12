//! The order of open tabs and which one is active, independent of what a tab holds.

/// A non-empty, ordered list of tabs with one active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tabs<T> {
    items: Vec<T>,
    active: usize,
}

impl<T> Tabs<T> {
    /// A list with a single, active tab.
    pub fn new(first: T) -> Self {
        Self {
            items: vec![first],
            active: 0,
        }
    }

    /// Builds a list from `items` with `active` selected. An out-of-range
    /// index selects the last tab; an empty list gives `None`.
    pub fn from_parts(items: Vec<T>, active: usize) -> Option<Self> {
        let last = items.len().checked_sub(1)?;
        Some(Self {
            items,
            active: active.min(last),
        })
    }

    /// Number of tabs; never zero.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Index of the active tab.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// The active tab.
    pub fn active(&self) -> &T {
        &self.items[self.active]
    }

    /// All tabs in order.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// The tab at `index`, for changing what it holds.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)
    }

    /// Makes the tab at `index` active, if there is one.
    pub fn activate(&mut self, index: usize) {
        if index < self.items.len() {
            self.active = index;
        }
    }

    /// Makes the next (`+1`) or previous (`-1`) tab active, wrapping around.
    pub fn cycle(&mut self, step: isize) {
        let len = self.items.len() as isize;
        self.active = (self.active as isize + step).rem_euclid(len) as usize;
    }

    /// Opens `tab` right after the active one and makes it active.
    pub fn open(&mut self, tab: T) {
        self.active += 1;
        self.items.insert(self.active, tab);
    }

    /// Closes the tab at `index` and returns it. The last remaining tab cannot
    /// be closed. Closing the active tab activates its right neighbour, or the
    /// left one if it was last.
    pub fn close(&mut self, index: usize) -> Option<T> {
        if self.items.len() == 1 || index >= self.items.len() {
            return None;
        }
        let closed = self.items.remove(index);
        if index < self.active || self.active == self.items.len() {
            self.active -= 1;
        }
        Some(closed)
    }

    /// Moves the tab at `from` so that it ends up at position `to`, keeping
    /// the same tab active.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        let len = self.items.len();
        if from >= len || to >= len || from == to {
            return;
        }
        let tab = self.items.remove(from);
        self.items.insert(to, tab);
        self.active = if self.active == from {
            to
        } else if from < self.active && to >= self.active {
            self.active - 1
        } else if from > self.active && to <= self.active {
            self.active + 1
        } else {
            self.active
        };
    }

    /// Moves the active tab one place left (`-1`) or right (`+1`), stopping at the ends.
    pub fn shift_active(&mut self, step: isize) {
        if let Some(to) = self.active.checked_add_signed(step) {
            self.move_tab(self.active, to);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn tabs(names: &[&'static str], active: usize) -> Tabs<&'static str> {
        Tabs::from_parts(names.to_vec(), active).expect("a non-empty list")
    }

    fn state(tabs: &Tabs<&'static str>) -> (Vec<&'static str>, &'static str) {
        (tabs.iter().copied().collect(), *tabs.active())
    }

    #[test]
    fn a_new_tab_opens_beside_the_active_one_and_becomes_active() {
        let mut list = tabs(&["a", "b", "c"], 0);
        list.open("new");
        assert_eq!(state(&list), (vec!["a", "new", "b", "c"], "new"));
    }

    #[test]
    fn closing_the_active_tab_activates_the_right_neighbour() {
        let mut list = tabs(&["a", "b", "c"], 1);
        assert_eq!(list.close(1), Some("b"));
        assert_eq!(state(&list), (vec!["a", "c"], "c"));
    }

    #[test]
    fn closing_the_last_active_tab_activates_the_left_neighbour() {
        let mut list = tabs(&["a", "b", "c"], 2);
        list.close(2);
        assert_eq!(state(&list), (vec!["a", "b"], "b"));
    }

    #[test]
    fn closing_another_tab_keeps_the_active_one() {
        let mut list = tabs(&["a", "b", "c"], 2);
        list.close(0);
        assert_eq!(state(&list), (vec!["b", "c"], "c"));
        list.close(0);
        assert_eq!(state(&list), (vec!["c"], "c"));
    }

    #[test]
    fn the_only_tab_and_missing_tabs_cannot_be_closed() {
        let mut list = tabs(&["a"], 0);
        assert_eq!(list.close(0), None);
        assert_eq!(tabs(&["a", "b"], 0).close(5), None);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn cycling_wraps_in_both_directions() {
        let mut list = tabs(&["a", "b", "c"], 2);
        list.cycle(1);
        assert_eq!(*list.active(), "a");
        list.cycle(-1);
        assert_eq!(*list.active(), "c");
    }

    #[test]
    fn moving_a_tab_keeps_the_same_tab_active() {
        let mut list = tabs(&["a", "b", "c", "d"], 1);
        list.move_tab(1, 3);
        assert_eq!(state(&list), (vec!["a", "c", "d", "b"], "b"));
        list.move_tab(0, 2);
        assert_eq!(state(&list), (vec!["c", "d", "a", "b"], "b"));
        list.move_tab(3, 0);
        assert_eq!(state(&list), (vec!["b", "c", "d", "a"], "b"));
        list.move_tab(2, 0);
        assert_eq!(state(&list), (vec!["d", "b", "c", "a"], "b"));
        list.move_tab(1, 9);
        assert_eq!(state(&list), (vec!["d", "b", "c", "a"], "b"));
    }

    #[test]
    fn shifting_the_active_tab_stops_at_the_ends() {
        let mut list = tabs(&["a", "b", "c"], 1);
        list.shift_active(-1);
        assert_eq!(state(&list), (vec!["b", "a", "c"], "b"));
        list.shift_active(-1);
        assert_eq!(state(&list), (vec!["b", "a", "c"], "b"));
        list.shift_active(1);
        list.shift_active(1);
        list.shift_active(1);
        assert_eq!(state(&list), (vec!["a", "c", "b"], "b"));
    }

    #[test]
    fn activating_and_building_handle_out_of_range_indices() {
        let mut list = tabs(&["a", "b"], 9);
        assert_eq!(*list.active(), "b");
        list.activate(0);
        assert_eq!(list.active_index(), 0);
        list.activate(7);
        assert_eq!(list.active_index(), 0);
        assert_eq!(Tabs::<&str>::from_parts(Vec::new(), 0), None);
    }
}
