//! Headless core of the Rustphin file manager.
//!
//! Nothing in this crate depends on a UI toolkit, Wayland or D-Bus. Work runs on
//! worker threads owned by the crate and reaches consumers as immutable
//! [`Snapshot`]s over a channel.

mod details;
mod entry;
mod fs;
mod mode;
mod model;
pub mod ops;
mod recent;
mod rename;
mod search;
mod selection;
mod sort;

pub use entry::{CHUNK_LEN, Chunk, EntryKind, EntryStore, display_name};
pub use fs::{COPY_STEP, FileInfo, Fs, Metadata, RawEntry, RealFs, WatchGuard};
pub use mode::{
    combine_mode, execute_bit, has_bit, has_special, octal_mode, read_bit, symbolic_mode, with_bit,
    write_bit,
};
pub use model::{DirectoryModel, LoadError, ModelEvent, Snapshot, ViewOptions};
pub use recent::{RECENT_LOCATION_LIMIT, RecentKey, RecentLocations};
pub use rename::{CaseChange, NameProblem, Preview, PreviewRow, RenameRules};
pub use search::{MatchScore, SearchQuery};
pub use selection::Selection;
pub use sort::{SortKey, natural_cmp};
