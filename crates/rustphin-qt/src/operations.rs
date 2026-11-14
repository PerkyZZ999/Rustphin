//! File operations as a window shows them (W-114f): a QML singleton that
//! follows the hub, which outlives the windows.

// cxx-qt generates the unsafe FFI glue for the object; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "operations_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{path::PathBuf, pin::Pin};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use jiff::tz::TimeZone;
use rustphin_view::{
    operations::fraction,
    questions::{self, Answer},
};

use crate::hub;

/// The Rust side of `Operations`.
pub struct OperationsRust {
    progress_visible: bool,
    progress_label: QString,
    progress_detail: QString,
    progress_icon: QString,
    progress_fraction: f64,
    running_count: i32,
    question: QString,
    notice: QString,
    notice_error: bool,
    notice_undo: bool,
    problems: QString,
    can_paste: bool,
    revision: i32,
    admin: bool,
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
            question: QString::default(),
            notice: QString::default(),
            notice_error: false,
            notice_undo: false,
            problems: QString::default(),
            can_paste: false,
            revision: 0,
            admin: false,
            admin_pending: false,
            admin_installed: rustphin_platform::admin::installed(),
            zone: TimeZone::system(),
            watching: false,
        }
    }
}

/// The problems behind a notice as JSON: the operation, then each entry
/// left out and why.
fn problems_json(problems: &rustphin_view::operations::Problems) -> String {
    let entries: Vec<serde_json::Value> = problems
        .entries
        .iter()
        .map(|skipped| {
            serde_json::json!({
                "path": skipped.path.display().to_string(),
                "reason": skipped.reason,
            })
        })
        .collect();
    serde_json::json!({ "title": problems.title, "entries": entries }).to_string()
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
        let state = hub::with(|hub| {
            let shown = hub.running.iter().find(|running| running.is_visible());
            let progress = shown.map(|running| {
                (
                    running.label.clone(),
                    running.detail(),
                    running.icon.clone(),
                    fraction(&running.progress).map_or(-1., f64::from),
                )
            });
            let question = hub.questions.front().map(|question| {
                let verb = question.asker().map_or("change", |id| hub.verb_of(id));
                let shown = questions::show(question, verb, &zone);
                serde_json::to_string(&shown).unwrap_or_default()
            });
            (
                progress,
                hub.running.len(),
                question,
                hub.notice.clone(),
                hub.problems.as_ref().map(problems_json),
                hub.has_clip(),
            )
        });
        let (progress, running, question, notice, problems, can_paste) = state;
        let (label, detail, icon, part) = progress.unwrap_or_default();
        self.as_mut().set_progress_visible(!label.is_empty());
        self.as_mut().set_progress_label(QString::from(&label));
        self.as_mut().set_progress_detail(QString::from(&detail));
        self.as_mut().set_progress_icon(QString::from(&icon));
        self.as_mut().set_progress_fraction(part);
        self.as_mut()
            .set_running_count(i32::try_from(running).unwrap_or(i32::MAX));
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
        self.as_mut().set_admin(crate::admin::active());
        self.as_mut().set_admin_pending(crate::admin::pending());
        let next = self.revision().wrapping_add(1);
        self.as_mut().set_revision(next);
    }

    fn cancel(self: Pin<&mut Self>) {
        hub::cancel(None);
    }

    fn undo(self: Pin<&mut Self>) {
        hub::undo();
    }

    fn answer(self: Pin<&mut Self>, answer: i32, apply_to_all: bool, password: &QString) {
        let Some(answer) = Answer::from_number(answer) else {
            return;
        };
        let password = Some(password.to_string()).filter(|password| !password.is_empty());
        hub::answer(answer, apply_to_all, password);
    }

    fn toggle_admin(self: Pin<&mut Self>) {
        crate::admin::toggle();
    }

    fn dismiss_notice(self: Pin<&mut Self>) {
        hub::dismiss_notice();
    }

    fn drop_files(self: Pin<&mut Self>, uris: &QString, into: &QString, action: i32) {
        let sources = rustphin_platform::uri::paths_from_uri_list(uris.to_string().as_bytes());
        hub::drop_files(
            sources,
            PathBuf::from(into.to_string()),
            crate::folder::drop_action(action),
        );
    }
}
