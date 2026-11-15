//! File operations as a window shows them (W-114f): a QML singleton that
//! follows the hub, which outlives the windows.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "operations_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{pin::Pin, time::Instant};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use jiff::tz::TimeZone;
use rustphin_core::ops::OpId;
use rustphin_view::{operations::Problems, questions::Answer};

use crate::hub;

/// The Rust side of `Operations`.
pub struct OperationsRust {
    progress_visible: bool,
    progress_label: QString,
    progress_detail: QString,
    progress_icon: QString,
    progress_fraction: f64,
    running_count: i32,
    /// The transfer dialog's cards as JSON (`transfer::Card`).
    transfers: QString,
    question: QString,
    notice: QString,
    notice_error: bool,
    notice_undo: bool,
    problems: QString,
    can_paste: bool,
    revision: i32,
    admin_pending: bool,
    admin_installed: bool,
    zone: TimeZone,
    watching: bool,
}

impl Default for OperationsRust {
    fn default() -> Self {
        Self {
            progress_visible: false,
            progress_label: QString::default(),
            progress_detail: QString::default(),
            progress_icon: QString::default(),
            progress_fraction: -1.,
            running_count: 0,
            transfers: QString::default(),
            question: QString::default(),
            notice: QString::default(),
            notice_error: false,
            notice_undo: false,
            problems: QString::default(),
            can_paste: false,
            revision: 0,
            admin_pending: false,
            admin_installed: rustphin_platform::admin::installed(),
            zone: TimeZone::system(),
            watching: false,
        }
    }
}

impl qobject::Operations {
    fn watch(mut self: Pin<&mut Self>) {
        if self.rust().watching {
            return;
        }
        self.as_mut().rust_mut().watching = true;
        hub::watch(self.qt_thread());
        self.refresh();
    }

    pub(crate) fn refresh(mut self: Pin<&mut Self>) {
        let zone = self.rust().zone.clone();
        let now = Instant::now();
        let state = hub::with(|hub| {
            let board = &hub.board;
            let progress = board.progress(now).map(|line| {
                (
                    line.label,
                    line.detail,
                    line.icon,
                    line.fraction.map_or(-1., f64::from),
                )
            });
            let question = board
                .question(&zone)
                .map(|shown| serde_json::to_string(&shown).unwrap_or_default());
            (
                progress,
                board.running_count(),
                serde_json::to_string(&board.transfers(now)).unwrap_or_default(),
                question,
                board.notice().cloned(),
                board.problems().map(Problems::json),
                board.can_paste(),
            )
        });
        let (progress, running, transfers, question, notice, problems, can_paste) = state;
        let (label, detail, icon, part) = progress.unwrap_or_default();
        self.as_mut().set_progress_visible(!label.is_empty());
        self.as_mut().set_progress_label(QString::from(&label));
        self.as_mut().set_progress_detail(QString::from(&detail));
        self.as_mut().set_progress_icon(QString::from(&icon));
        self.as_mut().set_progress_fraction(part);
        self.as_mut()
            .set_running_count(i32::try_from(running).unwrap_or(i32::MAX));
        self.as_mut().set_transfers(QString::from(&transfers));
        self.as_mut()
            .set_question(QString::from(&question.unwrap_or_default()));
        self.as_mut().set_notice(QString::from(
            notice.as_ref().map_or("", |notice| notice.text.as_str()),
        ));
        self.as_mut()
            .set_notice_error(notice.as_ref().is_some_and(|notice| notice.is_error));
        self.as_mut()
            .set_notice_undo(notice.as_ref().is_some_and(|notice| notice.offers_undo));
        self.as_mut()
            .set_problems(QString::from(&problems.unwrap_or_default()));
        self.as_mut().set_can_paste(can_paste);
        self.as_mut().set_admin_pending(crate::admin::pending());
        let next = self.revision().wrapping_add(1);
        self.as_mut().set_revision(next);
    }

    fn cancel(self: Pin<&mut Self>) {
        hub::cancel(None);
    }

    fn cancel_operation(self: Pin<&mut Self>, id: &QString) {
        if let Ok(number) = id.to_string().parse() {
            hub::cancel(Some(OpId::from_u64(number)));
        }
    }

    fn undo(self: Pin<&mut Self>, window: i32) {
        hub::undo(crate::admin::active(window));
    }

    fn new_window(&self) -> i32 {
        crate::admin::new_window()
    }

    fn admin_in(&self, window: i32) -> bool {
        crate::admin::active(window)
    }

    fn leave_admin(&self, window: i32) {
        crate::admin::leave(window);
    }

    fn answer(self: Pin<&mut Self>, answer: i32, apply_to_all: bool, password: &QString) {
        let Some(answer) = Answer::from_number(answer) else {
            return;
        };
        let password = Some(password.to_string()).filter(|password| !password.is_empty());
        hub::answer(answer, apply_to_all, password);
    }

    fn toggle_admin(self: Pin<&mut Self>, window: i32) {
        crate::admin::toggle(window);
    }

    fn dismiss_notice(self: Pin<&mut Self>) {
        hub::dismiss_notice();
    }

    fn drop_files(self: Pin<&mut Self>, uris: &QString, into: &QString, action: i32, window: i32) {
        let Some(into) = crate::qml_path::from_qml(into) else {
            return;
        };
        let sources = rustphin_platform::uri::paths_from_uri_list(uris.to_string().as_bytes());
        hub::drop_files(
            sources,
            into,
            crate::folder::drop_action(action),
            crate::admin::active(window),
        );
    }
}
