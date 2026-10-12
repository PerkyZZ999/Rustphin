//! Per-entry details that are loaded after the listing, only when needed.

use crate::Metadata;

/// Size and modification time for every entry of a listing, by entry index.
#[derive(Debug, Default)]
pub(crate) struct Details {
    entries: Vec<Option<Metadata>>,
}

impl Details {
    pub(crate) fn new(entries: Vec<Option<Metadata>>) -> Self {
        Self { entries }
    }

    /// The details of one entry, or `None` if they could not be read.
    pub(crate) fn get(&self, index: u32) -> Option<Metadata> {
        self.entries.get(index as usize).copied().flatten()
    }
}
