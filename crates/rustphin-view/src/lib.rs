//! Rustphin's interface logic that does not depend on a toolkit (W-114b,
//! D-034): what the views show and decide, as plain data and functions, so
//! the GPUI interface and the Qt one being built (W-114c) share it and it
//! is tested without a display.
//!
//! Nothing here may depend on GPUI, Qt or a display (`rust-tc layers`).

pub mod archives;
pub mod folder_views;
pub mod format;
pub mod groups;
pub mod launch;
pub mod layout;
pub mod location;
pub mod metrics;
pub mod naming;
pub mod session;
pub mod settings;
pub mod tabs;
pub mod toolbar_items;
