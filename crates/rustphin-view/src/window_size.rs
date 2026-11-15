//! The size a new window opens at (W-160): the last size the user gave a
//! window, and whether it was maximised, kept in Rustphin's state folder
//! as `window.toml`, as Dolphin keeps its window's size.
//!
//! A maximised or full-screen window keeps the size it had before, so
//! leaving the maximised state later goes back to it. A window is never
//! restored full screen or minimised.

use serde::{Deserialize, Serialize};

use crate::saved_file::{self, FileIssue};

/// The format version this build writes.
const VERSION: u32 = 1;

/// The smallest window, as Main.qml allows.
pub const MIN_WIDTH: u32 = 480;
/// The smallest window's height.
pub const MIN_HEIGHT: u32 = 320;
/// Larger than any screen: a saved size above it is not believed.
const MAX_SIDE: u32 = 16_384;

/// A window's size when nothing was saved.
pub const DEFAULT: WindowSize = WindowSize {
    width: 1000,
    height: 700,
    maximised: false,
};

/// What a window looks like on screen, as Qt's visibility says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// An ordinary window, of the size the user gave it.
    Windowed,
    /// Maximised.
    Maximised,
    /// Full screen, minimised or hidden: nothing to remember.
    Other,
}

impl Shown {
    /// Qt's `QWindow::Visibility` number.
    pub fn from_visibility(number: i32) -> Self {
        match number {
            2 => Self::Windowed,
            4 => Self::Maximised,
            _ => Self::Other,
        }
    }
}

/// The size a new window opens at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    /// Width in pixels, when not maximised.
    pub width: u32,
    /// Height in pixels, when not maximised.
    pub height: u32,
    /// Whether the window opens maximised.
    pub maximised: bool,
}

impl Default for WindowSize {
    fn default() -> Self {
        DEFAULT
    }
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct File {
    version: u32,
    width: u32,
    height: u32,
    maximised: bool,
}

impl WindowSize {
    /// Reads the saved file, as [`crate::saved_file`] keeps it: a file that
    /// could not be read, is damaged or is newer comes with the issue that
    /// keeps it from being written over this run. A size out of reason is
    /// brought within it.
    pub fn read(saved: std::io::Result<Option<String>>) -> (Self, Option<FileIssue>) {
        let decoded = saved_file::decode::<File>(saved, VERSION);
        let size = decoded.value.map_or(DEFAULT, |file| {
            Self {
                width: file.width,
                height: file.height,
                maximised: file.maximised,
            }
            .fitted()
        });
        (size, decoded.issue)
    }

    /// The size as TOML text.
    pub fn to_toml(&self) -> String {
        let file = File {
            version: VERSION,
            width: self.width,
            height: self.height,
            maximised: self.maximised,
        };
        toml::to_string(&file).unwrap_or_default()
    }

    /// Within the smallest window and any screen; an unset side takes the
    /// default.
    fn fitted(self) -> Self {
        let side = |value: u32, min: u32, default: u32| match value {
            0 => default,
            value if value > MAX_SIDE => default,
            value => value.max(min),
        };
        Self {
            width: side(self.width, MIN_WIDTH, DEFAULT.width),
            height: side(self.height, MIN_HEIGHT, DEFAULT.height),
            maximised: self.maximised,
        }
    }

    /// What to remember after a window became `width` by `height`, shown
    /// as `shown`: a windowed size, or only that it is maximised.
    pub fn after(self, width: i32, height: i32, shown: Shown) -> Self {
        match shown {
            Shown::Windowed => {
                let side = |value: i32| u32::try_from(value).unwrap_or(0);
                Self {
                    width: side(width),
                    height: side(height),
                    maximised: false,
                }
                .fitted()
            }
            Shown::Maximised => Self {
                maximised: true,
                ..self
            },
            Shown::Other => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> (WindowSize, Option<FileIssue>) {
        WindowSize::read(Ok(Some(text.to_owned())))
    }

    #[test]
    fn a_saved_size_is_read_back() {
        let size = WindowSize {
            width: 1400,
            height: 900,
            maximised: true,
        };
        let (back, issue) = read(&size.to_toml());
        assert_eq!(back, size);
        assert!(issue.is_none());
    }

    #[test]
    fn without_a_file_windows_open_at_the_default_size() {
        let (size, issue) = WindowSize::read(Ok(None));
        assert_eq!(size, DEFAULT);
        assert!(issue.is_none());
    }

    #[test]
    fn a_size_out_of_reason_is_brought_within_it() {
        let (small, _) = read("version = 1\nwidth = 10\nheight = 20\n");
        assert_eq!((small.width, small.height), (MIN_WIDTH, MIN_HEIGHT));
        let (huge, _) = read("version = 1\nwidth = 900000\nheight = 0\n");
        assert_eq!((huge.width, huge.height), (DEFAULT.width, DEFAULT.height));
    }

    #[test]
    fn a_damaged_or_newer_file_is_kept() {
        let (size, issue) = read("width = \"wide\"\n");
        assert_eq!(size, DEFAULT);
        assert!(matches!(issue, Some(FileIssue::Damaged(_))));
        let (size, issue) = read("version = 2\nwidth = 1200\nheight = 800\n");
        assert_eq!(size.width, 1200);
        assert!(matches!(issue, Some(FileIssue::NewerVersion(2))));
    }

    #[test]
    fn a_resized_window_remembers_its_size_and_that_it_is_not_maximised() {
        let was = WindowSize {
            maximised: true,
            ..DEFAULT
        };
        assert_eq!(
            was.after(1300, 850, Shown::Windowed),
            WindowSize {
                width: 1300,
                height: 850,
                maximised: false
            }
        );
    }

    #[test]
    fn maximising_keeps_the_size_to_go_back_to() {
        let was = WindowSize {
            width: 1300,
            height: 850,
            maximised: false,
        };
        let now = was.after(2560, 1400, Shown::Maximised);
        assert_eq!(
            now,
            WindowSize {
                maximised: true,
                ..was
            }
        );
    }

    #[test]
    fn full_screen_and_minimised_windows_change_nothing() {
        let was = WindowSize {
            width: 1300,
            height: 850,
            maximised: false,
        };
        assert_eq!(was.after(2560, 1440, Shown::Other), was);
        assert_eq!(Shown::from_visibility(5), Shown::Other);
        assert_eq!(Shown::from_visibility(3), Shown::Other);
        assert_eq!(Shown::from_visibility(2), Shown::Windowed);
        assert_eq!(Shown::from_visibility(4), Shown::Maximised);
    }
}
