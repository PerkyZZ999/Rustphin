//! Desktop integration for Rustphin.
//!
//! Each capability is a trait with a real implementation and a fallback, chosen
//! at runtime by what the session offers. This crate never depends on the UI
//! toolkit; it is the only crate in the workspace allowed to use `unsafe`.

pub mod activation;
pub mod admin;
pub mod apps;
pub mod archive_tool;
pub mod autostart;
pub mod blur;
pub mod clipboard;
pub mod config;
pub mod devices;
pub mod file_manager;
pub mod font_warmup;
pub mod icons;
pub mod instance;
pub mod kdeglobals;
pub mod mime;
pub mod places;
pub mod process;
pub mod properties;
pub mod remote;
pub mod templates;
pub mod terminal;
pub mod thumbnails;
pub mod trash;
pub mod uri;
pub mod xdg;
