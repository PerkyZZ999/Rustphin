//! Rustphin's interface logic that does not depend on a toolkit (W-114b,
//! D-034): what the views show and decide, as plain data and functions, so
//! it is tested without a display and the Qt interface only draws it.
//!
//! Nothing here may depend on Qt or a display (`rust-tc layers`).

pub mod app_chooser;
pub mod archives;
pub mod colors;
pub mod commands;
pub mod dialogs;
pub mod drive_properties;
pub mod folder_views;
pub mod format;
pub mod groups;
pub mod history;
pub mod icons;
pub mod launch;
pub mod layout;
pub mod location;
pub mod menu;
pub mod metrics;
pub mod naming;
pub mod operation_board;
pub mod operations;
pub mod palette;
pub mod pane;
pub mod pane_state;
pub mod place_menu;
pub mod places_panel;
pub mod preferences;
pub mod properties;
pub mod questions;
pub mod remote;
pub mod saved_file;
pub mod selection_actions;
pub mod session;
pub mod settings;
pub mod settings_form;
pub mod tabs;
pub mod toolbar_items;
pub mod transfer;
pub mod window_size;
pub mod workspace;
