//! Prints the type, description and icon file the platform layer resolves for
//! each file name given on the command line. A diagnostic for icon themes.
//!
//! Usage: `icon_lookup [--size N] <name>...` (a name ending in `/` is a folder).

use rustphin_platform::{icons, mime::MimeDb, xdg};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut size = 16;
    if args.first().map(String::as_str) == Some("--size") && args.len() >= 2 {
        size = args[1].parse().unwrap_or(size);
        args.drain(..2);
    }
    let data_dirs = xdg::data_dirs();
    let theme_name = xdg::config_home()
        .and_then(|config| icons::system_theme_name(&config))
        .unwrap_or_else(|| "breeze".to_owned());
    let theme = icons::IconTheme::load(&theme_name, &data_dirs);
    let mimes = MimeDb::load(&data_dirs);
    println!("icon theme: {theme_name}, size {size}");
    for name in args {
        let is_dir = name.ends_with('/');
        let mime = mimes.mime_type(name.trim_end_matches('/').as_bytes(), is_dir);
        let icon = theme.find(&mimes.icon_names(mime), size);
        println!(
            "{name:<22} {mime:<34} {:<28} {}",
            mimes.description(mime),
            icon.map_or_else(|| "(no icon)".to_owned(), |path| path.display().to_string())
        );
    }
}
