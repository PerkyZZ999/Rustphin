//! Archives Rustphin does not open itself (RAR, ISO, CAB, …) are handed to
//! an installed tool (W-111f, D-032): Ark first, as Dolphin uses it, then
//! 7-Zip, then unrar. Ark shows its own progress, password and overwrite
//! questions; 7-Zip and unrar are told never to overwrite.

use std::{
    ffi::{OsStr, OsString},
    io,
    path::{Path, PathBuf},
    process::Command,
};

/// An installed extraction tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tool {
    /// KDE's Ark.
    Ark(PathBuf),
    /// 7-Zip (`7z`, `7za` or `7zz`).
    SevenZip(PathBuf),
    /// unrar.
    Unrar(PathBuf),
}

impl Tool {
    /// The tool's name for messages.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ark(_) => "Ark",
            Self::SevenZip(_) => "7-Zip",
            Self::Unrar(_) => "unrar",
        }
    }
}

/// The first tool found in the folders of `path` (a `PATH` value).
pub fn find(path: Option<&OsStr>) -> Option<Tool> {
    let dirs: Vec<PathBuf> = path.map(|path| std::env::split_paths(path).collect())?;
    let on_path = |program: &str| {
        dirs.iter()
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    };
    on_path("ark")
        .map(Tool::Ark)
        .or_else(|| {
            ["7z", "7zz", "7za"]
                .into_iter()
                .find_map(on_path)
                .map(Tool::SevenZip)
        })
        .or_else(|| on_path("unrar").map(Tool::Unrar))
}

/// The program and arguments that extract `archive` into `into`. With
/// `new_folder`, into that folder inside `into`; without, Ark decides as
/// "Extract here" does. 7-Zip and unrar always get a folder of their own,
/// since they cannot judge whether one is needed.
pub fn command(
    tool: &Tool,
    archive: &Path,
    into: &Path,
    new_folder: Option<&OsStr>,
) -> (PathBuf, Vec<OsString>) {
    let folder = |name: Option<&OsStr>| match name {
        Some(name) => into.join(name),
        None => into.join(stem_of(archive)),
    };
    match tool {
        Tool::Ark(program) => {
            let mut args: Vec<OsString> = vec!["--batch".into()];
            let destination = match new_folder {
                Some(name) => into.join(name),
                None => {
                    args.push("--autosubfolder".into());
                    into.to_path_buf()
                }
            };
            args.push("--destination".into());
            args.push(destination.into_os_string());
            args.push(archive.as_os_str().to_owned());
            (program.clone(), args)
        }
        Tool::SevenZip(program) => {
            let mut output = OsString::from("-o");
            output.push(folder(new_folder));
            (
                program.clone(),
                vec![
                    "x".into(),
                    "-aos".into(),
                    output,
                    "--".into(),
                    archive.as_os_str().to_owned(),
                ],
            )
        }
        Tool::Unrar(program) => {
            let mut destination = folder(new_folder).into_os_string();
            destination.push("/");
            (
                program.clone(),
                vec![
                    "x".into(),
                    "-o-".into(),
                    "--".into(),
                    archive.as_os_str().to_owned(),
                    destination,
                ],
            )
        }
    }
}

/// The archive's name without its last ending: the folder 7-Zip and unrar
/// extract into.
fn stem_of(archive: &Path) -> OsString {
    archive
        .file_stem()
        .map_or_else(|| OsString::from("extracted"), OsStr::to_owned)
}

/// Why an archive could not be handed over.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// Nothing that could open it is installed.
    #[error("no program that can open it is installed; install Ark, 7-Zip or unrar")]
    NoTool,
    /// The tool could not be started.
    #[error("{tool} could not be started: {source}")]
    Start {
        /// Which tool.
        tool: &'static str,
        /// Why.
        source: io::Error,
    },
    /// The tool ran and reported a failure.
    #[error("{tool} could not extract it (exit code {code})")]
    Failed {
        /// Which tool.
        tool: &'static str,
        /// Its exit code, or -1 when it was killed.
        code: i32,
    },
}

/// Extracts `archive` into `into` with the first installed tool, waiting
/// until it is done. Blocks: run it on a background thread.
pub fn extract(
    archive: &Path,
    into: &Path,
    new_folder: Option<&OsStr>,
) -> Result<&'static str, ToolError> {
    let tool = find(std::env::var_os("PATH").as_deref()).ok_or(ToolError::NoTool)?;
    let (program, args) = command(&tool, archive, into, new_folder);
    let status = Command::new(program)
        .args(args)
        .current_dir(into)
        .status()
        .map_err(|source| ToolError::Start {
            tool: tool.name(),
            source,
        })?;
    if status.success() {
        Ok(tool.name())
    } else {
        Err(ToolError::Failed {
            tool: tool.name(),
            code: status.code().unwrap_or(-1),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ark_is_preferred_then_7zip_then_unrar() {
        let bin = tempfile::tempdir().unwrap();
        let path = bin.path().as_os_str();
        assert_eq!(find(Some(path)), None);
        std::fs::write(bin.path().join("unrar"), "").unwrap();
        assert_eq!(
            find(Some(path)),
            Some(Tool::Unrar(bin.path().join("unrar")))
        );
        std::fs::write(bin.path().join("7za"), "").unwrap();
        assert_eq!(
            find(Some(path)),
            Some(Tool::SevenZip(bin.path().join("7za")))
        );
        std::fs::write(bin.path().join("ark"), "").unwrap();
        assert_eq!(find(Some(path)), Some(Tool::Ark(bin.path().join("ark"))));
        assert_eq!(find(None), None);
    }

    #[test]
    fn each_tool_is_told_where_and_never_to_overwrite() {
        let archive = Path::new("/home/someone/film.rar");
        let into = Path::new("/home/someone");
        let (_, ark) = command(&Tool::Ark("ark".into()), archive, into, None);
        assert_eq!(
            ark,
            [
                "--batch",
                "--autosubfolder",
                "--destination",
                "/home/someone",
                "/home/someone/film.rar"
            ]
        );
        let (_, ark) = command(
            &Tool::Ark("ark".into()),
            archive,
            into,
            Some(OsStr::new("film")),
        );
        assert_eq!(
            ark,
            [
                "--batch",
                "--destination",
                "/home/someone/film",
                "/home/someone/film.rar"
            ]
        );
        let (_, seven) = command(&Tool::SevenZip("7z".into()), archive, into, None);
        assert_eq!(
            seven,
            [
                "x",
                "-aos",
                "-o/home/someone/film",
                "--",
                "/home/someone/film.rar"
            ]
        );
        let (_, unrar) = command(&Tool::Unrar("unrar".into()), archive, into, None);
        assert_eq!(
            unrar,
            [
                "x",
                "-o-",
                "--",
                "/home/someone/film.rar",
                "/home/someone/film/"
            ]
        );
    }
}
