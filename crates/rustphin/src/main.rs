//! Rustphin, a file manager for KDE Plasma on Wayland.

use std::{env, path::PathBuf};

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("RUSTPHIN_LOG").unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();

    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    // The first argument is a folder to open; without one the previous
    // session's tabs come back.
    let start = env::args_os().nth(1).map(PathBuf::from).map(|folder| {
        if folder.is_relative() {
            env::current_dir()
                .map(|dir| dir.join(&folder))
                .unwrap_or(folder)
        } else {
            folder
        }
    });

    rustphin_ui::run(rustphin_ui::Options {
        start,
        home,
        force_opaque: env::var_os("RUSTPHIN_OPAQUE").is_some_and(|value| value != "0"),
    });
}
