//! The window's tabs and split views, free of any toolkit (W-114e): which
//! tab Alt+number picks, how wide tabs are, what closed tabs are kept for
//! Ctrl+Shift+T, and copying or moving to the other view (Shift+F5/F6).

use std::path::{Path, PathBuf};

use rustphin_core::{display_name, ops::Request};

use crate::{metrics::size, session::SessionTab};

/// Whether the other view receives a copy or the entries themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    /// Shift+F5.
    Copy,
    /// Shift+F6.
    Move,
}

/// The operation that sends the active view's selection into the other
/// view's folder, as Dolphin's copy and move to the other view, or why
/// there is nothing to do.
pub fn transfer_to_other_view(
    transfer: Transfer,
    selected: Vec<PathBuf>,
    from: Option<&Path>,
    to: Option<&Path>,
) -> Result<Request, &'static str> {
    let Some(into) = to else {
        return Err("Split the view with F3 to copy or move to the other side.");
    };
    if selected.is_empty() {
        return Err("Select what to copy or move first.");
    }
    if from == Some(into) {
        return Err(
            "Both views show the same folder. Open another folder in the other view first.",
        );
    }
    let into = into.to_path_buf();
    Ok(match transfer {
        Transfer::Copy => Request::Copy {
            sources: selected,
            into,
        },
        Transfer::Move => Request::Move {
            sources: selected,
            into,
        },
    })
}

/// How many closed tabs Ctrl+Shift+T can bring back.
pub const CLOSED_TAB_LIMIT: usize = 6;

/// Records a closed tab, newest first, forgetting the oldest past the limit.
/// A tab with no saved location cannot be reopened and is not kept.
pub fn remember_closed_tab(closed: &mut Vec<SessionTab>, tab: SessionTab) {
    if tab.panes.is_empty() {
        return;
    }
    closed.insert(0, tab);
    closed.truncate(CLOSED_TAB_LIMIT);
}

/// The tab Alt+`number` activates, as in Dolphin: Alt+1 to Alt+9 pick that
/// tab if it exists, Alt+0 the last one.
pub fn tab_for_number(number: usize, tabs: usize) -> Option<usize> {
    match number {
        0 => tabs.checked_sub(1),
        n => (n <= tabs).then(|| n - 1),
    }
}

/// How wide each of `count` tabs is in a strip `available` pixels wide:
/// 160 while they fit beside the "+", shrinking evenly to 96, never less
/// (past that the strip scrolls).
pub fn tab_width(available: f32, count: usize) -> f32 {
    let room = (available - size::TAB_ADD).max(0.) / count.max(1) as f32;
    room.clamp(size::TAB_MIN, size::TAB)
}

/// A folder's label: its last component, or "/".
pub fn folder_label(path: &Path) -> String {
    match path.file_name() {
        Some(name) => display_name(name.as_encoded_bytes()).into_owned(),
        None => "/".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use crate::location::Location;

    use super::*;

    #[test]
    fn the_other_view_receives_the_selection_or_a_reason() {
        let a = vec![PathBuf::from("/left/a"), PathBuf::from("/left/b")];
        let left = Some(Path::new("/left"));
        let right = Some(Path::new("/right"));
        assert!(matches!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, right),
            Ok(Request::Copy { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(matches!(
            transfer_to_other_view(Transfer::Move, a.clone(), left, right),
            Ok(Request::Move { sources, into }) if sources == a && into == Path::new("/right")
        ));
        assert!(
            transfer_to_other_view(Transfer::Copy, a.clone(), left, None)
                .is_err_and(|reason| reason.contains("F3"))
        );
        assert!(
            transfer_to_other_view(Transfer::Copy, Vec::new(), left, right)
                .is_err_and(|reason| reason.contains("Select"))
        );
        assert!(
            transfer_to_other_view(Transfer::Move, a, left, left)
                .is_err_and(|reason| reason.contains("same folder"))
        );
    }

    #[test]
    fn closed_tabs_are_kept_newest_first_up_to_the_limit() {
        let tab = |path: &str| SessionTab {
            panes: vec![Location::Folder(PathBuf::from(path))],
            active_pane: 0,
        };
        let mut closed = Vec::new();
        for index in 0..CLOSED_TAB_LIMIT + 2 {
            remember_closed_tab(&mut closed, tab(&format!("/{index}")));
        }
        remember_closed_tab(&mut closed, SessionTab::default());
        assert_eq!(closed.len(), CLOSED_TAB_LIMIT);
        assert_eq!(closed[0], tab(&format!("/{}", CLOSED_TAB_LIMIT + 1)));
        assert_eq!(closed[CLOSED_TAB_LIMIT - 1], tab("/2"));
    }

    #[test]
    fn alt_numbers_pick_existing_tabs_and_alt_0_the_last() {
        assert_eq!(tab_for_number(1, 3), Some(0));
        assert_eq!(tab_for_number(3, 3), Some(2));
        assert_eq!(tab_for_number(4, 3), None);
        assert_eq!(tab_for_number(0, 3), Some(2));
        assert_eq!(tab_for_number(0, 0), None);
    }

    #[test]
    fn tabs_keep_their_width_shrink_evenly_then_stop_at_the_narrowest() {
        assert_eq!(tab_width(1000., 2), size::TAB);
        assert_eq!(tab_width(size::TAB_ADD + 300., 2), 150.);
        assert_eq!(tab_width(200., 9), size::TAB_MIN);
        assert_eq!(folder_label(Path::new("/srv/data")), "data");
        assert_eq!(folder_label(Path::new("/")), "/");
    }
}
