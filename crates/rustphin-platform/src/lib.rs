//! Desktop integration for Rustphin.
//!
//! Each capability is a trait with a real implementation and a fallback, chosen
//! at runtime by what the session offers. This crate never depends on the UI
//! toolkit; it is the only crate in the workspace allowed to use `unsafe`.

pub mod blur;
pub mod config;
pub mod icons;
pub mod mime;
pub mod trash;
pub mod xdg;
