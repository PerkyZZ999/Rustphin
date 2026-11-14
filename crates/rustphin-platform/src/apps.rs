//! Applications registered for a file type, and starting them.
//!
//! Associations come from the desktop's `mimeapps.list` files and from
//! `MimeType` in application `.desktop` files. A command is started directly:
//! nothing is passed through a shell, and a malformed desktop file is skipped
//! rather than run. Terminal applications are left out of the list; the
//! default opener still receives an ordinary file.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs, io,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use thiserror::Error;

use crate::uri;

/// Largest desktop file that is read. Anything bigger is treated as unreadable.
const MAX_DESKTOP_BYTES: u64 = 1_048_576;
/// How many desktop files are indexed. The rest are ignored.
const MAX_DESKTOP_FILES: usize = 5_000;
/// How many parent types are followed for one lookup.
const MAX_PARENTS: usize = 32;

/// An application that can open a file type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Application {
    /// Desktop-file id, such as `org.kde.kate.desktop`.
    pub id: String,
    /// The application's name, as shown in a menu.
    pub name: String,
    /// Its `Icon` key: an icon name from the theme, or an absolute path.
    pub icon: Option<String>,
}

/// What activating a file should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchKind {
    /// Hand the file to the default application. No confirmation.
    Document,
    /// The file is a program. Ask before executing it.
    Program,
    /// The file is an application shortcut. Ask before starting it.
    Desktop {
        /// The shortcut's name.
        name: String,
    },
    /// The file could not be classified. Nothing was started.
    Unreadable(String),
}

/// Why a file could not be opened or run.
#[derive(Debug, Error)]
pub enum LaunchError {
    /// The program could not be started.
    #[error("could not start {program}: {source}")]
    Start {
        /// The program that was asked for.
        program: String,
        /// The operating system's reason.
        #[source]
        source: io::Error,
    },
    /// A desktop file named no usable command.
    #[error("“{0}” has no command to run")]
    NoCommand(String),
    /// The desktop file or the association could not be used.
    #[error("{0}")]
    Message(String),
}

/// Applications indexed from desktop files and mimeapps lists.
#[derive(Debug, Default)]
pub struct Associations {
    apps: HashMap<String, Application>,
    /// MIME type to desktop ids that list it, in scan order.
    by_mime: HashMap<String, Vec<String>>,
    defaults: HashMap<String, Vec<String>>,
    added: HashMap<String, Vec<String>>,
    removed: HashMap<String, HashSet<String>>,
    /// Child MIME type to its parent types.
    parents: HashMap<String, Vec<String>>,
}

impl Associations {
    /// Reads application desktop files and mimeapps lists. Missing directories
    /// are skipped. A desktop file that is not valid UTF-8, is too large, or
    /// does not describe an application is ignored.
    pub fn load(config_dirs: &[PathBuf], data_dirs: &[PathBuf]) -> Self {
        let mut index = Self::default();
        let mut seen = HashSet::new();
        for (id, path) in desktop_files(data_dirs) {
            if !seen.insert(id.clone()) {
                continue;
            }
            if let Some(parsed) = read_desktop(&path) {
                for mime in &parsed.mimes {
                    index
                        .by_mime
                        .entry(mime.clone())
                        .or_default()
                        .push(id.clone());
                }
                index.apps.insert(
                    id.clone(),
                    Application {
                        id,
                        name: parsed.name,
                        icon: parsed.icon,
                    },
                );
            }
        }
        for path in mimeapps_files(config_dirs, data_dirs) {
            if let Some(text) = read_text(&path) {
                merge_mimeapps(&text, &mut index);
            }
        }
        for dir in data_dirs {
            if let Some(text) = read_text(&dir.join("mime").join("subclasses")) {
                merge_subclasses(&text, &mut index.parents);
            }
        }
        index
    }

    /// Every indexed application, in no particular order.
    pub fn all(&self) -> impl Iterator<Item = &Application> {
        self.apps.values()
    }

    /// Applications for `mime`, the default first, then added associations,
    /// then other applications that list the type, then the same for parent
    /// types. Removed associations are left out.
    pub fn for_type(&self, mime: &str) -> Vec<Application> {
        let mut ids = Vec::new();
        let mut seen_types = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(mime.to_owned());
        while let Some(mime) = queue.pop_front() {
            if !seen_types.insert(mime.clone()) || seen_types.len() > MAX_PARENTS {
                continue;
            }
            ids.extend(self.ids_for(&mime));
            if let Some(parents) = self.parents.get(&mime) {
                queue.extend(parents.iter().cloned());
            }
        }
        let mut seen = HashSet::new();
        ids.retain(|id| seen.insert(id.clone()));
        ids.into_iter()
            .filter_map(|id| self.apps.get(&id).cloned())
            .collect()
    }

    /// The application the desktop opens `mime` with by default, if a
    /// mimeapps list names one that is installed and not removed.
    pub fn default_for(&self, mime: &str) -> Option<&Application> {
        let removed = self.removed.get(mime);
        self.defaults
            .get(mime)?
            .iter()
            .filter(|id| removed.is_none_or(|removed| !removed.contains(*id)))
            .find_map(|id| self.apps.get(id))
    }

    /// The installed application with desktop-file id `id`.
    pub fn get(&self, id: &str) -> Option<&Application> {
        self.apps.get(id)
    }

    fn ids_for(&self, mime: &str) -> Vec<String> {
        let mut ids = Vec::new();
        if let Some(default) = self.defaults.get(mime) {
            ids.extend(default.iter().cloned());
        }
        if let Some(added) = self.added.get(mime) {
            ids.extend(added.iter().cloned());
        }
        if let Some(listed) = self.by_mime.get(mime) {
            ids.extend(listed.iter().cloned());
        }
        if let Some(removed) = self.removed.get(mime) {
            ids.retain(|id| !removed.contains(id));
        }
        ids
    }
}

/// Makes `id` the default application for `mime` in the user's
/// `mimeapps.list` under `config_home`, as KDE's Default Applications page
/// does. Every other line is kept; the previous defaults stay behind it.
/// Reads and writes files: call from a background thread.
pub fn set_default(config_home: &Path, mime: &str, id: &str) -> io::Result<()> {
    let path = config_home.join("mimeapps.list");
    let text = crate::config::read(&path)?.unwrap_or_default();
    // The list is the desktop's, readable as other applications made it;
    // keep its permissions (a new one gets the usual 0644).
    let mode = fs::metadata(&path).map_or(0o644, |meta| meta.permissions().mode() & 0o7777);
    crate::config::write_atomic(&path, with_default(&text, mime, id).as_bytes())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(mode))
}

/// `text`, a mimeapps list, with `id` first among the defaults for `mime`.
fn with_default(text: &str, mime: &str, id: &str) -> String {
    const GROUP: &str = "[Default Applications]";
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_group = false;
    let mut group_end = None;
    let mut found = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_group && group_end.is_none() {
                group_end = Some(index);
            }
            in_group = trimmed == GROUP;
        } else if in_group
            && let Some((key, _)) = trimmed.split_once('=')
            && key.trim() == mime
        {
            found = Some(index);
            break;
        }
    }
    match found {
        Some(index) => {
            let others = lines[index]
                .split_once('=')
                .map(|(_, ids)| ids.to_owned())
                .unwrap_or_default();
            let mut ids = vec![id.to_owned()];
            ids.extend(
                others
                    .split(';')
                    .map(str::trim)
                    .filter(|other| !other.is_empty() && *other != id)
                    .map(str::to_owned),
            );
            lines[index] = format!("{mime}={};", ids.join(";"));
        }
        None => {
            let entry = format!("{mime}={id};");
            let group = lines.iter().position(|line| line.trim() == GROUP);
            match group {
                Some(start) => {
                    // After the group's last entry, before any blank lines
                    // that separate it from the next group.
                    let mut at = group_end.unwrap_or(lines.len());
                    while at > start + 1 && lines[at - 1].trim().is_empty() {
                        at -= 1;
                    }
                    lines.insert(at, entry);
                }
                None => {
                    if lines.last().is_some_and(|line| !line.trim().is_empty()) {
                        lines.push(String::new());
                    }
                    lines.push(GROUP.to_owned());
                    lines.push(entry);
                }
            }
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Whether activating `path` opens a document, runs a program, or starts a
/// shortcut. Does not start anything.
pub fn launch_kind(path: &Path) -> LaunchKind {
    let shown = path.display().to_string();
    if is_desktop_file(path) {
        return match read_desktop(path) {
            Some(parsed) => LaunchKind::Desktop { name: parsed.name },
            None => LaunchKind::Unreadable(format!(
                "Could not read {shown} as an application shortcut."
            )),
        };
    }
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 => {
            LaunchKind::Program
        }
        Ok(_) => LaunchKind::Document,
        Err(error) => LaunchKind::Unreadable(format!("Could not open {shown}: {error}.")),
    }
}

/// Opens `path` with the desktop's default application.
pub fn open_with_default(path: &Path) -> Result<(), LaunchError> {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    spawn(command, "xdg-open")
}

/// Executes `path` itself. The caller has already confirmed that with the user.
pub fn run_program(path: &Path) -> Result<(), LaunchError> {
    let mut command = Command::new(path);
    if let Some(dir) = path.parent()
        && !dir.as_os_str().is_empty()
    {
        command.current_dir(dir);
    }
    spawn(command, &path.display().to_string())
}

/// Starts the application described by a `.desktop` file. The caller has
/// already confirmed that with the user.
pub fn run_desktop(path: &Path) -> Result<(), LaunchError> {
    let Some(parsed) = read_desktop(path) else {
        return Err(LaunchError::Message(format!(
            "Could not read {} as an application shortcut.",
            path.display()
        )));
    };
    let lines = command_lines(&parsed.exec, &[], &parsed.name, Some(path))
        .ok_or_else(|| LaunchError::NoCommand(parsed.name.clone()))?;
    for argv in lines {
        spawn_argv(&argv)?;
    }
    Ok(())
}

/// Opens `path` with the application whose desktop-file id is `id`.
pub fn open_with(id: &str, path: &Path) -> Result<(), LaunchError> {
    if !valid_id(id) {
        return Err(LaunchError::Message(format!(
            "“{id}” is not an application."
        )));
    }
    let Some(desktop) = find_desktop(id, &crate::xdg::data_dirs()) else {
        return Err(LaunchError::Message(format!(
            "Could not find the application “{id}”."
        )));
    };
    let Some(parsed) = read_desktop(&desktop) else {
        return Err(LaunchError::Message(format!(
            "Could not read the application “{id}”."
        )));
    };
    let files = [path.to_owned()];
    let lines = command_lines(&parsed.exec, &files, &parsed.name, Some(&desktop))
        .ok_or(LaunchError::NoCommand(parsed.name))?;
    for argv in lines {
        spawn_argv(&argv)?;
    }
    Ok(())
}

/// The commands a desktop `Exec` line becomes for `files`.
///
/// Field codes are replaced here. `%f` and `%u` produce one command per file;
/// `%F` and `%U` produce one command with every file. An unquoted `;` is an
/// ordinary character, not a shell separator. `None` when the line is not a
/// usable command.
pub fn command_lines(
    exec: &str,
    files: &[PathBuf],
    name: &str,
    desktop: Option<&Path>,
) -> Option<Vec<Vec<std::ffi::OsString>>> {
    let tokens = split_exec(exec)?;
    let (per_file, list) = field_codes(exec);
    if per_file && !list && !files.is_empty() {
        return Some(
            files
                .iter()
                .map(|file| {
                    expand(
                        &tokens,
                        Some(file),
                        std::slice::from_ref(file),
                        name,
                        desktop,
                    )
                })
                .filter(|argv| !argv.is_empty())
                .collect(),
        )
        .filter(|lines: &Vec<_>| !lines.is_empty());
    }
    let argv = expand(
        &tokens,
        files.first().map(PathBuf::as_path),
        files,
        name,
        desktop,
    );
    (!argv.is_empty()).then(|| vec![argv])
}

struct ParsedDesktop {
    name: String,
    exec: String,
    mimes: Vec<String>,
    icon: Option<String>,
}

fn read_desktop(path: &Path) -> Option<ParsedDesktop> {
    parse_desktop(&read_text(path)?)
}

fn read_text(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_DESKTOP_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn parse_desktop(text: &str) -> Option<ParsedDesktop> {
    let mut in_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut mimes = Vec::new();
    let mut icon = None;
    let mut hidden = false;
    let mut no_display = false;
    let mut terminal = false;
    let mut is_application = false;
    let mut try_exec = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(group) = line.strip_prefix('[') {
            in_entry = group.strip_suffix(']') == Some("Desktop Entry");
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.contains('[') {
            continue;
        }
        match key {
            "Type" => is_application = value == "Application",
            "Name" => name = clean_name(value),
            "Exec" => exec = Some(value.to_owned()),
            "Icon" => icon = Some(value.trim().to_owned()).filter(|icon| !icon.is_empty()),
            "MimeType" => {
                mimes.extend(
                    value
                        .split(';')
                        .filter(|mime| mime.contains('/') && !mime.contains([' ', '\t']))
                        .map(str::to_owned),
                );
            }
            "Hidden" => hidden = value == "true",
            "NoDisplay" => no_display = value == "true",
            "Terminal" => terminal = value == "true",
            "TryExec" => try_exec = Some(value.to_owned()),
            _ => {}
        }
    }
    if !is_application || hidden || no_display || terminal {
        return None;
    }
    if try_exec
        .as_deref()
        .is_some_and(|program| !try_exec_exists(program))
    {
        return None;
    }
    let name = name?;
    let exec = exec.filter(|exec| !exec.is_empty())?;
    Some(ParsedDesktop {
        name,
        exec,
        mimes,
        icon,
    })
}

/// True when the file name ends in `.desktop`, even if the rest is not UTF-8.
fn is_desktop_file(path: &Path) -> bool {
    let Some(name) = path.file_name() else {
        return false;
    };
    let bytes = name.as_bytes();
    let suffix = b".desktop";
    bytes.len() >= suffix.len() && bytes[bytes.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

fn clean_name(value: &str) -> Option<String> {
    let name = value.trim();
    if name.is_empty() || name.chars().any(char::is_control) {
        None
    } else {
        Some(name.to_owned())
    }
}

fn try_exec_exists(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

fn valid_id(id: &str) -> bool {
    id.ends_with(".desktop")
        && !id.is_empty()
        && !id.contains(['/', '\\'])
        && !id.split('-').any(|part| part == "..")
}

fn desktop_files(data_dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for dir in data_dirs {
        walk_applications(
            &dir.join("applications"),
            &dir.join("applications"),
            &mut found,
        );
    }
    found
}

fn walk_applications(root: &Path, dir: &Path, found: &mut Vec<(String, PathBuf)>) {
    if found.len() >= MAX_DESKTOP_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        if found.len() >= MAX_DESKTOP_FILES {
            return;
        }
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            if fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
                push_desktop(root, &path, found);
            }
            continue;
        }
        if kind.is_dir() {
            walk_applications(root, &path, found);
        } else if kind.is_file() {
            push_desktop(root, &path, found);
        }
    }
}

fn push_desktop(root: &Path, path: &Path, found: &mut Vec<(String, PathBuf)>) {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return;
    };
    if !name.ends_with(".desktop") {
        return;
    }
    let id = path
        .strip_prefix(root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('/', "-"))
        .unwrap_or_else(|| name.to_owned());
    if valid_id(&id) {
        found.push((id, path.to_owned()));
    }
}

fn find_desktop(id: &str, data_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in data_dirs {
        let direct = dir.join("applications").join(id);
        if direct.is_file() {
            return Some(direct);
        }
        if let Some(nested) = nested_desktop(&dir.join("applications"), id) {
            return Some(nested);
        }
    }
    None
}

/// `kde-org.kde.kate.desktop` is `applications/kde/org.kde.kate.desktop`.
fn nested_desktop(applications: &Path, id: &str) -> Option<PathBuf> {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() < 2 {
        return None;
    }
    let mut acc = applications.to_owned();
    for index in 0..parts.len() - 1 {
        acc.push(parts[index]);
        if !acc.is_dir() {
            return None;
        }
        let file = acc.join(parts[index + 1..].join("-"));
        if file.is_file() {
            return Some(file);
        }
    }
    None
}

fn mimeapps_files(config_dirs: &[PathBuf], data_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in config_dirs {
        files.push(dir.join("mimeapps.list"));
    }
    for dir in data_dirs {
        files.push(dir.join("applications").join("mimeapps.list"));
        files.push(dir.join("applications").join("defaults.list"));
    }
    files
}

fn merge_mimeapps(text: &str, index: &mut Associations) {
    let mut section = "";
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(group) = line.strip_prefix('[') {
            section = group.strip_suffix(']').unwrap_or("");
            continue;
        }
        let Some((mime, ids)) = line.split_once('=') else {
            continue;
        };
        if !mime.contains('/') {
            continue;
        }
        let ids: Vec<String> = ids
            .split(';')
            .filter(|id| valid_id(id))
            .map(str::to_owned)
            .collect();
        if ids.is_empty() {
            continue;
        }
        match section {
            "Default Applications" => {
                index.defaults.entry(mime.to_owned()).or_insert(ids);
            }
            "Added Associations" => {
                index.added.entry(mime.to_owned()).or_default().extend(ids);
            }
            "Removed Associations" => {
                index
                    .removed
                    .entry(mime.to_owned())
                    .or_default()
                    .extend(ids);
            }
            _ => {}
        }
    }
}

fn merge_subclasses(text: &str, parents: &mut HashMap<String, Vec<String>>) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let (Some(child), Some(parent)) = (fields.next(), fields.next()) else {
            continue;
        };
        if child.contains('/') && parent.contains('/') {
            parents
                .entry(child.to_owned())
                .or_default()
                .push(parent.to_owned());
        }
    }
}

/// `(one command per file, one command for the whole list)`.
fn field_codes(exec: &str) -> (bool, bool) {
    let mut chars = exec.chars().peekable();
    let mut per_file = false;
    let mut list = false;
    while let Some(char) = chars.next() {
        if char != '%' {
            continue;
        }
        match chars.next() {
            Some('f' | 'u') => per_file = true,
            Some('F' | 'U') => list = true,
            _ => {}
        }
    }
    (per_file, list)
}

pub(crate) fn split_exec(exec: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = exec.chars().peekable();
    let mut quote = None;
    let mut any = false;
    while let Some(char) = chars.next() {
        match (quote, char) {
            (None, ' ' | '\t') => {
                if any {
                    tokens.push(std::mem::take(&mut current));
                    any = false;
                }
            }
            (None, '"' | '\'') => {
                quote = Some(char);
                any = true;
            }
            (Some(opened), char) if char == opened => quote = None,
            (_, '\\') => {
                let escaped = chars.next()?;
                if escaped.is_control() {
                    return None;
                }
                current.push(escaped);
                any = true;
            }
            (_, char) if char.is_control() => return None,
            (_, char) => {
                current.push(char);
                any = true;
            }
        }
    }
    if quote.is_some() {
        return None;
    }
    if any {
        tokens.push(current);
    }
    (!tokens.is_empty()).then_some(tokens)
}

fn expand(
    tokens: &[String],
    file: Option<&Path>,
    files: &[PathBuf],
    name: &str,
    desktop: Option<&Path>,
) -> Vec<std::ffi::OsString> {
    let mut argv = Vec::new();
    for token in tokens {
        match token.as_str() {
            "%f" => argv.extend(file.map(|file| file.as_os_str().to_owned())),
            "%u" => argv.extend(file.map(|file| uri::canonical_file_uri(file).into())),
            "%F" => argv.extend(files.iter().map(|file| file.as_os_str().to_owned())),
            "%U" => argv.extend(
                files
                    .iter()
                    .map(|file| uri::canonical_file_uri(file).into()),
            ),
            "%i" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => {}
            "%c" => argv.push(name.into()),
            "%k" => {
                if let Some(desktop) = desktop {
                    argv.push(desktop.as_os_str().to_owned());
                }
            }
            _ => argv.push(expand_embedded(token, file, name, desktop)),
        }
    }
    argv
}

fn expand_embedded(
    token: &str,
    file: Option<&Path>,
    name: &str,
    desktop: Option<&Path>,
) -> std::ffi::OsString {
    if !token.contains('%') {
        return token.into();
    }
    let mut out = std::ffi::OsString::new();
    let mut rest = token;
    while let Some(index) = rest.find('%') {
        out.push(&rest[..index]);
        let code = rest[index + 1..].chars().next();
        let bytes = code.map_or(1, char::len_utf8) + 1;
        match code {
            Some('%') => out.push("%"),
            Some('f') => {
                if let Some(file) = file {
                    out.push(file);
                }
            }
            Some('u') => {
                if let Some(file) = file {
                    out.push(uri::canonical_file_uri(file));
                }
            }
            Some('c') => out.push(name),
            Some('k') => {
                if let Some(desktop) = desktop {
                    out.push(desktop);
                }
            }
            Some(_) => out.push(&rest[index..index + bytes]),
            None => out.push("%"),
        }
        rest = rest.get(index + bytes..).unwrap_or("");
    }
    out.push(rest);
    out
}

pub(crate) fn spawn(mut command: Command, program: &str) -> Result<(), LaunchError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command.spawn().map_err(|source| LaunchError::Start {
        program: program.to_owned(),
        source,
    })?;
    // The child outlives this call. Waiting here would block until it exits,
    // and not waiting would leave a zombie, so a short-lived thread reaps it.
    let _ = std::thread::Builder::new()
        .name("rustphin-launch".to_owned())
        .spawn(move || {
            let _ = child.wait();
        });
    Ok(())
}

fn spawn_argv(argv: &[std::ffi::OsString]) -> Result<(), LaunchError> {
    let Some((program, args)) = argv.split_first() else {
        return Err(LaunchError::NoCommand(String::new()));
    };
    if program.is_empty() {
        return Err(LaunchError::NoCommand(String::new()));
    }
    let mut command = Command::new(program);
    command.args(args);
    spawn(command, &program.to_string_lossy())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    };

    use super::*;

    fn desktop(name: &str, exec: &str, mime: &str) -> String {
        format!("[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nMimeType={mime};\n")
    }

    fn write(dir: &Path, name: &str, text: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
    }

    #[test]
    fn the_default_application_leads_and_a_removed_one_is_absent() {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config");
        let data = root.path().join("data");
        write(
            &data.join("applications"),
            "writer.desktop",
            &desktop("Writer", "writer %f", "text/plain"),
        );
        write(
            &data.join("applications"),
            "editor.desktop",
            &desktop("Editor", "editor %f", "text/plain"),
        );
        write(
            &config,
            "mimeapps.list",
            "[Default Applications]\ntext/plain=writer.desktop;\n\
             [Removed Associations]\ntext/plain=editor.desktop;\n",
        );
        let index = Associations::load(&[config], &[data]);
        let apps = index.for_type("text/plain");
        assert_eq!(
            apps,
            vec![Application {
                id: "writer.desktop".into(),
                name: "Writer".into(),
                icon: None,
            }]
        );
    }

    #[test]
    fn an_application_keeps_its_icon() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        let with_icon = format!(
            "{}Icon=org.example.viewer\n",
            desktop("Viewer", "viewer %f", "image/png")
        );
        write(&data.join("applications"), "viewer.desktop", &with_icon);
        write(
            &data.join("applications"),
            "plain.desktop",
            &format!("{}Icon=\n", desktop("Plain", "plain %f", "image/png")),
        );
        let index = Associations::load(&[], &[data]);
        let icons: HashMap<_, _> = index
            .for_type("image/png")
            .into_iter()
            .map(|app| (app.name, app.icon))
            .collect();
        assert_eq!(icons["Viewer"].as_deref(), Some("org.example.viewer"));
        assert_eq!(icons["Plain"], None);
    }

    #[test]
    fn a_parent_type_supplies_an_application_the_child_lacks() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        write(
            &data.join("applications"),
            "editor.desktop",
            &desktop("Editor", "editor %f", "text/plain"),
        );
        write(&data.join("mime"), "subclasses", "text/x-rust text/plain\n");
        let index = Associations::load(&[], &[data]);
        assert_eq!(index.for_type("text/x-rust")[0].name, "Editor");
    }

    #[test]
    fn a_malformed_desktop_file_is_skipped() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        write(
            &data.join("applications"),
            "broken.desktop",
            "not a desktop\n",
        );
        write(
            &data.join("applications"),
            "binary.desktop",
            &String::from_utf8(vec![0xff, 0xfe, b'[', b'D']).unwrap_or_default(),
        );
        fs::write(
            data.join("applications").join("bad.desktop"),
            [0xff, 0xfe, 0xfd],
        )
        .unwrap();
        write(
            &data.join("applications"),
            "ok.desktop",
            &desktop("Ok", "ok %f", "text/plain"),
        );
        let index = Associations::load(&[], &[data]);
        assert_eq!(index.for_type("text/plain").len(), 1);
    }

    #[test]
    fn escaping_a_control_character_does_not_make_an_exec_line_valid() {
        assert_eq!(command_lines("editor \\\0", &[], "Editor", None), None);
        assert_eq!(command_lines("editor \\\n", &[], "Editor", None), None);
        assert_eq!(
            command_lines("editor \"\\\u{7f}\"", &[], "Editor", None),
            None
        );
    }

    #[test]
    fn a_quoted_exec_line_keeps_spaces_and_does_not_use_a_shell() {
        let lines = command_lines(
            r#"editor "two words" %f; touch /tmp/nope"#,
            &[PathBuf::from("/tmp/note.txt")],
            "Editor",
            None,
        )
        .unwrap();
        assert_eq!(lines.len(), 1);
        // The semicolon stays in an argument. It is not a shell separator,
        // so `touch` is not a command of its own.
        assert_eq!(
            lines[0],
            [
                "editor",
                "two words",
                "/tmp/note.txt;",
                "touch",
                "/tmp/nope"
            ]
            .map(std::ffi::OsString::from)
        );
    }

    #[test]
    fn a_single_file_code_starts_one_command_per_file() {
        let lines = command_lines(
            "editor %f",
            &[PathBuf::from("/a"), PathBuf::from("/b")],
            "Editor",
            None,
        )
        .unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1][1], std::ffi::OsString::from("/b"));
    }

    fn os(args: &[&[u8]]) -> Vec<std::ffi::OsString> {
        args.iter()
            .map(|arg| std::ffi::OsString::from_vec(arg.to_vec()))
            .collect()
    }

    #[test]
    fn a_list_code_starts_one_command_with_every_file_as_bytes() {
        let files = [
            PathBuf::from(std::ffi::OsString::from_vec(b"/srv/caf\xe9.txt".to_vec())),
            PathBuf::from("/srv/b c.txt"),
        ];

        let paths = command_lines("viewer %F", &files, "Viewer", None).unwrap();
        let uris = command_lines("viewer %U", &files, "Viewer", None).unwrap();

        assert_eq!(
            paths,
            [os(&[b"viewer", b"/srv/caf\xe9.txt", b"/srv/b c.txt"])]
        );
        assert_eq!(
            uris,
            [os(&[
                b"viewer",
                b"file:///srv/caf%E9.txt",
                b"file:///srv/b%20c.txt"
            ])]
        );
    }

    #[test]
    fn a_single_uri_code_starts_one_command_per_file_with_its_uri() {
        let files = [
            PathBuf::from(std::ffi::OsString::from_vec(b"/srv/caf\xe9".to_vec())),
            PathBuf::from("/srv/100%"),
        ];

        let lines = command_lines("player --open=%u", &files, "Player", None).unwrap();

        assert_eq!(
            lines,
            [
                os(&[b"player", b"--open=file:///srv/caf%E9"]),
                os(&[b"player", b"--open=file:///srv/100%25"]),
            ]
        );
    }

    #[test]
    fn name_desktop_file_icon_and_percent_codes_are_filled_in() {
        let desktop = Path::new("/usr/share/applications/viewer.desktop");

        let lines = command_lines(
            "viewer %i --title %c --from=%k %k 100%% %% %f",
            &[PathBuf::from("/a")],
            "Image Viewer",
            Some(desktop),
        )
        .unwrap();
        let without_desktop = command_lines("viewer %k --from=%k", &[], "Viewer", None).unwrap();

        assert_eq!(
            lines,
            [os(&[
                b"viewer",
                b"--title",
                b"Image Viewer",
                b"--from=/usr/share/applications/viewer.desktop",
                b"/usr/share/applications/viewer.desktop",
                b"100%",
                b"%",
                b"/a"
            ])]
        );
        assert_eq!(without_desktop, [os(&[b"viewer", b"--from="])]);
    }

    #[test]
    fn without_files_a_file_code_is_left_out() {
        let lines = command_lines("editor %f --new", &[], "Editor", None).unwrap();

        assert_eq!(lines, [os(&[b"editor", b"--new"])]);
    }

    #[test]
    fn an_unclosed_quote_is_not_a_command() {
        assert!(command_lines("editor \"unterminated", &[], "Editor", None).is_none());
    }

    #[test]
    fn an_executable_a_shortcut_and_a_document_are_told_apart() {
        let root = tempfile::tempdir().unwrap();
        let document = root.path().join("note.txt");
        fs::write(&document, b"hello").unwrap();
        assert_eq!(launch_kind(&document), LaunchKind::Document);

        let program = root.path().join("tool");
        fs::write(&program, b"#!/bin/sh\n").unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(launch_kind(&program), LaunchKind::Program);

        let shortcut = root.path().join("Demo.desktop");
        fs::write(&shortcut, desktop("Demo", "true", "")).unwrap();
        assert_eq!(
            launch_kind(&shortcut),
            LaunchKind::Desktop {
                name: "Demo".into()
            }
        );

        let broken = root.path().join("broken.desktop");
        fs::write(&broken, b"nope").unwrap();
        assert!(matches!(launch_kind(&broken), LaunchKind::Unreadable(_)));

        let mut raw = vec![0xff];
        raw.extend(b".desktop");
        let odd = root.path().join(OsString::from_vec(raw));
        fs::write(&odd, b"nope").unwrap();
        assert!(matches!(launch_kind(&odd), LaunchKind::Unreadable(_)));
    }

    #[test]
    fn a_program_is_started_without_waiting_for_it() {
        let program = ["/usr/bin/true", "/bin/true"]
            .into_iter()
            .map(Path::new)
            .find(|path| path.is_file())
            .expect("a true program");
        run_program(program).unwrap();
    }

    #[test]
    fn making_an_application_the_default_puts_it_first_and_keeps_the_rest() {
        let text = "[Added Associations]\ntext/plain=org.kde.kate.desktop;\n\n[Default Applications]\ninode/directory=org.kde.dolphin.desktop;other.desktop;\ntext/plain=org.kde.kate.desktop;\n";
        assert_eq!(
            with_default(text, "inode/directory", "rustphin.desktop"),
            "[Added Associations]\ntext/plain=org.kde.kate.desktop;\n\n[Default Applications]\ninode/directory=rustphin.desktop;org.kde.dolphin.desktop;other.desktop;\ntext/plain=org.kde.kate.desktop;\n"
        );
        // Asking twice changes nothing more.
        let once = with_default(text, "inode/directory", "rustphin.desktop");
        assert_eq!(
            with_default(&once, "inode/directory", "rustphin.desktop"),
            once
        );
    }

    #[test]
    fn a_missing_default_line_or_group_is_added() {
        let text = "[Default Applications]\ntext/plain=org.kde.kate.desktop;\n\n[Added Associations]\nimage/png=gwenview.desktop;\n";
        assert_eq!(
            with_default(text, "inode/directory", "rustphin.desktop"),
            "[Default Applications]\ntext/plain=org.kde.kate.desktop;\ninode/directory=rustphin.desktop;\n\n[Added Associations]\nimage/png=gwenview.desktop;\n"
        );
        assert_eq!(
            with_default("", "inode/directory", "rustphin.desktop"),
            "[Default Applications]\ninode/directory=rustphin.desktop;\n"
        );
        assert_eq!(
            with_default(
                "[Added Associations]\nimage/png=a.desktop;",
                "inode/directory",
                "r.desktop"
            ),
            "[Added Associations]\nimage/png=a.desktop;\n\n[Default Applications]\ninode/directory=r.desktop;\n"
        );
    }

    #[test]
    fn the_default_is_the_first_installed_application_not_removed() {
        let mut index = Associations::default();
        for id in ["dolphin.desktop", "rustphin.desktop"] {
            index.apps.insert(
                id.to_owned(),
                Application {
                    id: id.to_owned(),
                    name: id.to_owned(),
                    icon: None,
                },
            );
        }
        index.defaults.insert(
            "inode/directory".to_owned(),
            vec![
                "gone.desktop".to_owned(),
                "dolphin.desktop".to_owned(),
                "rustphin.desktop".to_owned(),
            ],
        );
        let default = |index: &Associations| {
            index
                .default_for("inode/directory")
                .map(|app| app.id.clone())
        };
        assert_eq!(default(&index).as_deref(), Some("dolphin.desktop"));
        index.removed.insert(
            "inode/directory".to_owned(),
            ["dolphin.desktop".to_owned()].into(),
        );
        assert_eq!(default(&index).as_deref(), Some("rustphin.desktop"));
        assert!(index.default_for("text/plain").is_none());
    }

    #[test]
    fn setting_the_default_writes_the_users_list_and_reads_back() {
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("mimeapps.list"),
            "[Default Applications]\ninode/directory=dolphin.desktop;\n",
        )
        .unwrap();
        // Unlike the 0o644 a new file gets, whatever the umask.
        fs::set_permissions(
            home.path().join("mimeapps.list"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();

        set_default(home.path(), "inode/directory", "rustphin.desktop").unwrap();

        // The desktop's file keeps the permissions it had.
        let mode = fs::metadata(home.path().join("mimeapps.list"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o640);
        let mut index = Associations::default();
        merge_mimeapps(
            &std::fs::read_to_string(home.path().join("mimeapps.list")).unwrap(),
            &mut index,
        );
        assert_eq!(
            index.defaults["inode/directory"],
            ["rustphin.desktop", "dolphin.desktop"]
        );
    }
}
