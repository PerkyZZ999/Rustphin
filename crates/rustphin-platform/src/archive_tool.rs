//! Reading the archives Rustphin does not read itself (D-036): through
//! libarchive when it can, else through an installed 7-Zip or unrar
//! (encrypted RAR and 7Z, ARJ…). Either way the operation engine gets the
//! entries one by one and does the writing, so every format has the same
//! progress, conflict and password questions and the same Undo.
//!
//! A tool extracts into a private folder in Rustphin's cache, never into
//! the destination; the entries are read from there and the folder is
//! removed. A tool never asks anything itself: it gets the password the
//! engine asked for, or an empty one.

use std::{
    ffi::{OsStr, OsString},
    fs, io,
    ops::ControlFlow,
    os::unix::{
        ffi::{OsStrExt as _, OsStringExt as _},
        fs::MetadataExt as _,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use rustphin_core::{
    ReadSeek,
    archive::{ForeignReader, Header, Kind, ReadError, Visit},
};

use crate::libarchive::{self, Probe};

/// An installed extraction tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tool {
    /// 7-Zip (`7z`, `7zz` or `7za`).
    SevenZip(PathBuf),
    /// unrar.
    Unrar(PathBuf),
}

impl Tool {
    /// The tool's name for messages.
    pub fn name(&self) -> &'static str {
        match self {
            Self::SevenZip(_) => "7-Zip",
            Self::Unrar(_) => "unrar",
        }
    }
}

/// The first tool found in the folders of `path` (a `PATH` value):
/// 7-Zip, which reads nearly everything, then unrar.
pub fn find(path: Option<&OsStr>) -> Option<Tool> {
    let dirs: Vec<PathBuf> = path.map(|path| std::env::split_paths(path).collect())?;
    let on_path = |program: &str| {
        dirs.iter()
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    };
    ["7z", "7zz", "7za"]
        .into_iter()
        .find_map(on_path)
        .map(Tool::SevenZip)
        .or_else(|| on_path("unrar").map(Tool::Unrar))
}

/// The arguments that make `tool` extract `archive` into the empty folder
/// `into`, with `password` (an empty one when none was given, so the tool
/// never stops to ask) and without overwriting anything.
pub fn arguments(
    tool: &Tool,
    archive: &Path,
    into: &Path,
    password: Option<&str>,
) -> Vec<OsString> {
    match tool {
        Tool::SevenZip(_) => {
            let mut output = OsString::from("-o");
            output.push(into);
            vec![
                "x".into(),
                "-aos".into(),
                format!("-p{}", password.unwrap_or("")).into(),
                output,
                "--".into(),
                archive.as_os_str().to_owned(),
            ]
        }
        Tool::Unrar(_) => {
            let mut destination = into.as_os_str().to_owned();
            destination.push("/");
            vec![
                "x".into(),
                "-o-".into(),
                match password {
                    Some(password) => format!("-p{password}").into(),
                    // "-p-": do not ask for a password.
                    None => "-p-".into(),
                },
                "--".into(),
                archive.as_os_str().to_owned(),
                destination,
            ]
        }
    }
}

/// What reads an archive libarchive cannot.
#[derive(Debug, Clone)]
enum Fallback {
    /// The tool installed at the time, working in Rustphin's cache.
    Installed,
    /// This tool, if any, working in this folder.
    Given {
        tool: Option<Tool>,
        work: Option<PathBuf>,
    },
    /// Nothing: the engine works with administrator rights, and a tool
    /// would open the archive by its path with the user's own.
    Refused,
}

/// The engine's reader of the formats Rustphin does not read itself
/// (D-036): libarchive, on the content the engine opened through its `Fs`,
/// else the installed 7-Zip or unrar, which opens the archive by its path.
#[derive(Debug, Clone)]
pub struct ForeignArchives {
    fallback: Fallback,
}

impl ForeignArchives {
    /// libarchive, then the tool installed when an archive is read, for
    /// the engine that works with the user's own rights.
    pub fn installed() -> Self {
        Self {
            fallback: Fallback::Installed,
        }
    }

    /// libarchive, then `tool`, working in a folder of its own in `work`.
    pub fn with_tool(tool: Option<Tool>, work: Option<PathBuf>) -> Self {
        Self {
            fallback: Fallback::Given { tool, work },
        }
    }

    /// libarchive only, for the engine that works with administrator
    /// rights: a tool reads the archive by its path with the user's own
    /// rights, so what libarchive cannot read is refused, saying why.
    pub fn as_administrator() -> Self {
        Self {
            fallback: Fallback::Refused,
        }
    }
}

impl ForeignReader for ForeignArchives {
    fn read_entries(
        &self,
        path: &Path,
        mut source: Box<dyn ReadSeek>,
        password: Option<&str>,
        visit: Visit<'_>,
    ) -> Result<(), ReadError> {
        let name = path.file_name().map(OsStr::as_bytes).unwrap_or_default();
        let reason = match libarchive::probe(&mut *source)? {
            Probe::Readable => return libarchive::read_entries(name, source, password, visit),
            Probe::Unreadable(reason) => reason,
        };
        drop(source);
        let (tool, work) = match &self.fallback {
            Fallback::Installed => (
                find(std::env::var_os("PATH").as_deref()),
                crate::xdg::cache_home().map(|cache| cache.join("rustphin")),
            ),
            Fallback::Given { tool, work } => (tool.clone(), work.clone()),
            Fallback::Refused => {
                return Err(ReadError::Damaged(format!(
                    "{reason}, and 7-Zip cannot open it with administrator rights"
                )));
            }
        };
        let (Some(tool), Some(work)) = (tool, work) else {
            return Err(ReadError::Damaged(format!(
                "{reason}, and 7-Zip is not installed to open it"
            )));
        };
        read_with_tool(&tool, path, password, visit, &work)
    }
}

/// Removes its folder when dropped, whatever happened.
struct WorkFolder(PathBuf);

impl Drop for WorkFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A new, empty, private folder in `work` for one extraction.
fn work_folder(work: &Path) -> io::Result<WorkFolder> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fs::create_dir_all(work)?;
    loop {
        let folder = work.join(format!(
            ".extract-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&folder) {
            Ok(()) => {
                fs::set_permissions(&folder, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
                return Ok(WorkFolder(folder));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

/// Extracts `archive` with `tool` into a work folder, then hands what it
/// made to `visit`, folders before what they hold.
fn read_with_tool(
    tool: &Tool,
    archive: &Path,
    password: Option<&str>,
    visit: Visit<'_>,
    work: &Path,
) -> Result<(), ReadError> {
    let folder = work_folder(work)?;
    let program = match tool {
        Tool::SevenZip(program) | Tool::Unrar(program) => program,
    };
    let output = Command::new(program)
        .args(arguments(tool, archive, &folder.0, password))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            ReadError::Io(io::Error::new(
                error.kind(),
                format!("{} could not be started: {error}", tool.name()),
            ))
        })?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        // 7-Zip says "Wrong password"; unrar exits with 11 for one.
        let about_password = said.contains("password")
            || (matches!(tool, Tool::Unrar(_)) && output.status.code() == Some(11));
        return Err(if about_password {
            match password {
                None => ReadError::NeedsPassword,
                Some(_) => ReadError::WrongPassword,
            }
        } else {
            ReadError::Damaged(format!(
                "{} could not extract it (exit code {})",
                tool.name(),
                output.status.code().unwrap_or(-1)
            ))
        });
    }
    walk(&folder.0, Path::new(""), visit).map(drop)
}

/// Hands every entry under `root/relative` to `visit`, in name order.
fn walk(root: &Path, relative: &Path, visit: Visit<'_>) -> Result<ControlFlow<()>, ReadError> {
    let mut names: Vec<OsString> = fs::read_dir(root.join(relative))?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<_>>()?;
    names.sort();
    for name in names {
        let inside = relative.join(&name);
        let full = root.join(&inside);
        let metadata = fs::symlink_metadata(&full)?;
        let kind = if metadata.is_dir() {
            Kind::Directory
        } else if metadata.is_file() {
            Kind::File
        } else if metadata.file_type().is_symlink() {
            Kind::Link(fs::read_link(&full)?.into_os_string().into_vec())
        } else {
            Kind::Special
        };
        let header = Header {
            name: inside.as_os_str().as_bytes().to_vec(),
            kind: kind.clone(),
            mode: Some(metadata.mode() & 0o7777),
            modified: Some(metadata.mtime()),
            size: if kind == Kind::File {
                metadata.len()
            } else {
                0
            },
        };
        let flow = if kind == Kind::File {
            let mut content = fs::File::open(&full)?;
            visit(&header, &mut content)?
        } else {
            visit(&header, &mut io::empty())?
        };
        if flow.is_break() {
            return Ok(ControlFlow::Break(()));
        }
        if kind == Kind::Directory && walk(root, &inside, visit)?.is_break() {
            return Ok(ControlFlow::Break(()));
        }
    }
    Ok(ControlFlow::Continue(()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
