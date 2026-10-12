//! Rustphin, a file manager for KDE Plasma on Wayland.

use std::{env, path::PathBuf};

use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("RUSTPHIN_LOG").unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();

    // The first argument is the folder to open; default to the home directory.
    let start = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("/"));
    let start = if start.is_relative() {
        env::current_dir()
            .map(|dir| dir.join(&start))
            .unwrap_or(start)
    } else {
        start
    };

    rustphin_ui::run(rustphin_ui::Options {
        start,
        force_opaque: env::var_os("RUSTPHIN_OPAQUE").is_some_and(|value| value != "0"),
    });
}
