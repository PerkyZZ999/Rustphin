//! Headless core of the Rustphin file manager.
//!
//! Nothing in this crate depends on a UI toolkit, Wayland or D-Bus. Work runs on
//! worker threads owned by the crate and reaches consumers as immutable
//! [`Snapshot`]s over a channel.

mod details;
mod entry;
mod fs;
mod model;
pub mod ops;
mod selection;
mod sort;

pub use entry::{CHUNK_LEN, Chunk, EntryKind, EntryStore, display_name};
pub use fs::{COPY_STEP, FileInfo, Fs, Metadata, RawEntry, RealFs, WatchGuard};
pub use model::{DirectoryModel, LoadError, ModelEvent, Snapshot, ViewOptions};
pub use selection::Selection;
pub use sort::{SortKey, natural_cmp};
