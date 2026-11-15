//! Which entries of a listing are selected.

use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::Snapshot;

/// The next revision handed out; shared so independent selections never
/// share one after a change.
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// The selected entries of one listing, with the anchor and focus that
/// keyboard and range selection work from.
///
/// Entries are tracked by their id within a listing, so the selection survives
/// re-sorting and filtering. It is emptied when the directory is read again.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    listing: u64,
    bits: Vec<u64>,
    count: usize,
    /// Where a range selection starts.
    anchor: Option<u32>,
    /// The entry keyboard movement continues from.
    focus: Option<u32>,
    /// See [`Self::revision`].
    revision: u64,
}

impl Selection {
    /// Identifies this state of the selection: it changes whenever the
    /// selected entries may have changed, so equal revisions mean the same
    /// entries are selected. Lets views cache what they derive from it, such
    /// as the size of the selected files.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn changed(&mut self) {
        self.revision = NEXT_REVISION.fetch_add(1, Ordering::Relaxed);
    }

    /// Number of selected entries.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Brings the selection in line with `snapshot`: emptied if the directory
    /// was read again, otherwise reduced to the entries that are still shown.
    pub fn sync(&mut self, snapshot: &Snapshot) {
        if self.listing != snapshot.listing() {
            *self = Self {
                listing: snapshot.listing(),
                ..Self::default()
            };
            self.changed();
            return;
        }
        if self.count == 0 {
            return;
        }
        self.changed();
        let mut shown = vec![0u64; self.bits.len()];
        for row in 0..snapshot.len() {
            let id = snapshot.entry_id(row) as usize;
            if let Some(word) = shown.get_mut(id / 64) {
                *word |= 1 << (id % 64);
            }
        }
        for (word, shown) in self.bits.iter_mut().zip(shown) {
            *word &= shown;
        }
        self.count = self
            .bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum();
        let keep = |id: Option<u32>| id.filter(|&id| snapshot.row_of_entry(id).is_some());
        self.anchor = keep(self.anchor);
        self.focus = keep(self.focus);
    }

    /// Moves the selection from `old` to `new`, a later view of the same
    /// directory. If the directory was read again in between, entries are
    /// matched by name: the ones still present stay selected and the rest drop
    /// out. Otherwise this is [`Self::sync`].
    pub fn carry_over(&mut self, old: &Snapshot, new: &Snapshot) {
        if old.listing() == new.listing() || self.listing != old.listing() {
            self.sync(new);
            return;
        }
        let name_of = |id: Option<u32>| {
            let row = old.row_of_entry(id?)?;
            Some(old.name(row))
        };
        let (anchor, focus) = (name_of(self.anchor), name_of(self.focus));
        let selected: HashSet<&[u8]> = self
            .rows(old)
            .into_iter()
            .map(|row| old.name(row))
            .collect();

        let mut carried = Self {
            listing: new.listing(),
            ..Self::default()
        };
        if !selected.is_empty() {
            for row in 0..new.len() {
                if selected.contains(new.name(row)) {
                    carried.set(new.entry_id(row), true);
                }
            }
        }
        let id_of = |name: Option<&[u8]>| Some(new.entry_id(new.row_of(name?)?));
        carried.anchor = id_of(anchor);
        carried.focus = id_of(focus);
        *self = carried;
        self.changed();
    }

    /// Whether the entry shown in `row` is selected.
    pub fn contains(&self, snapshot: &Snapshot, row: usize) -> bool {
        self.listing == snapshot.listing() && self.has(snapshot.entry_id(row))
    }

    /// The row keyboard movement continues from, if its entry is shown.
    pub fn focus_row(&self, snapshot: &Snapshot) -> Option<usize> {
        self.row_in(snapshot, self.focus)
    }

    /// The selected rows of `snapshot`, in display order.
    pub fn rows(&self, snapshot: &Snapshot) -> Vec<usize> {
        if self.listing != snapshot.listing() {
            return Vec::new();
        }
        (0..snapshot.len())
            .filter(|&row| self.has(snapshot.entry_id(row)))
            .collect()
    }

    /// Deselects everything.
    pub fn clear(&mut self) {
        self.changed();
        self.bits.clear();
        self.count = 0;
        self.anchor = None;
        self.focus = None;
    }

    /// Selects only the entry in `row` (a plain click or arrow key).
    pub fn select_only(&mut self, snapshot: &Snapshot, row: usize) {
        let Some(id) = self.adopt(snapshot, row) else {
            return;
        };
        self.bits.clear();
        self.count = 0;
        self.set(id, true);
        self.anchor = Some(id);
        self.focus = Some(id);
    }

    /// Puts the focus and the anchor on the entry in `row` and leaves what
    /// is selected as it is (the keyboard reaching a list with no focused
    /// entry).
    pub fn focus_only(&mut self, snapshot: &Snapshot, row: usize) {
        let Some(id) = self.adopt(snapshot, row) else {
            return;
        };
        self.changed();
        self.anchor = Some(id);
        self.focus = Some(id);
    }

    /// Adds the entry in `row` to the selection or removes it (Ctrl+click).
    pub fn toggle(&mut self, snapshot: &Snapshot, row: usize) {
        let Some(id) = self.adopt(snapshot, row) else {
            return;
        };
        self.set(id, !self.has(id));
        self.anchor = Some(id);
        self.focus = Some(id);
    }

    /// Selects exactly the rows from the anchor to `row` (Shift+click or
    /// Shift+arrow). Without an anchor, `row` becomes it.
    pub fn extend_to(&mut self, snapshot: &Snapshot, row: usize) {
        let Some(id) = self.adopt(snapshot, row) else {
            return;
        };
        let anchor_row = self.row_in(snapshot, self.anchor).unwrap_or(row);
        self.anchor = Some(snapshot.entry_id(anchor_row));
        self.focus = Some(id);
        self.bits.clear();
        self.count = 0;
        for row in anchor_row.min(row)..=anchor_row.max(row) {
            self.set(snapshot.entry_id(row), true);
        }
    }

    /// Selects the given rows (a rubber band). With `add`, they join the
    /// current selection; otherwise they replace it. Rows outside the list are
    /// ignored. The anchor and focus move to the last row given.
    pub fn select_rows(
        &mut self,
        snapshot: &Snapshot,
        rows: impl IntoIterator<Item = usize>,
        add: bool,
    ) {
        if self.listing != snapshot.listing() || !add {
            self.clear();
            self.listing = snapshot.listing();
        }
        for row in rows.into_iter().filter(|&row| row < snapshot.len()) {
            let id = snapshot.entry_id(row);
            self.set(id, true);
            self.anchor = Some(id);
            self.focus = Some(id);
        }
    }

    /// Selects every shown entry.
    pub fn select_all(&mut self, snapshot: &Snapshot) {
        self.replace(snapshot, |_| true);
    }

    /// Selects every shown entry that was not selected, and deselects the rest.
    pub fn invert(&mut self, snapshot: &Snapshot) {
        let before = self.clone();
        self.replace(snapshot, |id| !before.has(id));
    }

    fn replace(&mut self, snapshot: &Snapshot, selected: impl Fn(u32) -> bool) {
        let same_listing = self.listing == snapshot.listing();
        let (anchor, focus) = (self.anchor, self.focus);
        self.clear();
        self.listing = snapshot.listing();
        for row in 0..snapshot.len() {
            let id = snapshot.entry_id(row);
            if selected(id) {
                self.set(id, true);
            }
        }
        if same_listing {
            (self.anchor, self.focus) = (anchor, focus);
        }
    }

    /// Makes this selection belong to `snapshot`'s listing and returns the id
    /// of the entry in `row`, or `None` if there is no such row.
    fn adopt(&mut self, snapshot: &Snapshot, row: usize) -> Option<u32> {
        if row >= snapshot.len() {
            return None;
        }
        if self.listing != snapshot.listing() {
            self.clear();
            self.listing = snapshot.listing();
        }
        Some(snapshot.entry_id(row))
    }

    fn row_in(&self, snapshot: &Snapshot, id: Option<u32>) -> Option<usize> {
        if self.listing != snapshot.listing() {
            return None;
        }
        snapshot.row_of_entry(id?)
    }

    fn has(&self, id: u32) -> bool {
        let id = id as usize;
        self.bits
            .get(id / 64)
            .is_some_and(|word| word & (1 << (id % 64)) != 0)
    }

    fn set(&mut self, id: u32, selected: bool) {
        if self.has(id) == selected {
            return;
        }
        self.changed();
        let id = id as usize;
        if self.bits.len() <= id / 64 {
            self.bits.resize(id / 64 + 1, 0);
        }
        self.bits[id / 64] ^= 1 << (id % 64);
        if selected {
            self.count += 1;
        } else {
            self.count -= 1;
        }
    }
}
