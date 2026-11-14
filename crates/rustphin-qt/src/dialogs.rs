//! The Compress to… and batch rename dialogs' objects (W-114f): the
//! state is `rustphin_view::dialogs`; the entries come from the pane as
//! paths, never through QML.

// cxx-qt generates the unsafe FFI glue for the objects; the bridge file
// holds declarations only (AGENTS.md, D-034).
#[allow(unsafe_code)]
#[path = "dialogs_bridge.rs"]
mod bridge;

pub use bridge::qobject;

use std::{collections::HashSet, ffi::OsString, pin::Pin};

use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use rustphin_core::CaseChange;
use rustphin_view::dialogs::{BatchRename, Compress, Mode};

use crate::{folder::handed, hub};

/// The Rust side of `CompressModel`.
#[derive(Default)]
pub struct CompressModelRust {
    title: QString,
    name: QString,
    format: i32,
    level: i32,
    note: QString,
    ready: bool,
    open: bool,
    state: Option<Compress>,
}

impl qobject::CompressModel {
    fn start(mut self: Pin<&mut Self>) {
        let Some(given) = handed::take() else {
            return;
        };
        let entries: Vec<(&[u8], bool)> = given
            .sources
            .iter()
            .map(|path| {
                (
                    path.file_name()
                        .map(|name| name.as_encoded_bytes())
                        .unwrap_or_default(),
                    path.is_dir(),
                )
            })
            .collect();
        let base = rustphin_view::archives::compress_base(&entries);
        let taken: HashSet<Vec<u8>> = given.taken.into_iter().collect();
        let state = Compress::new(given.sources, given.folder, taken, base);
        self.as_mut().set_title(QString::from(&state.title()));
        self.as_mut().set_name(QString::from(&state.name));
        self.as_mut().rust_mut().state = Some(state);
        self.as_mut().show();
        self.set_open(true);
    }

    fn show(mut self: Pin<&mut Self>) {
        let Some(state) = self.rust().state.clone() else {
            return;
        };
        let (note, ready) = state.note();
        self.as_mut().set_note(QString::from(&note));
        self.as_mut().set_ready(ready);
        self.as_mut()
            .set_format(i32::try_from(state.format).unwrap_or(0));
        self.set_level(i32::try_from(state.level).unwrap_or(1));
    }

    fn type_name(mut self: Pin<&mut Self>, name: &QString) {
        if let Some(state) = self.as_mut().rust_mut().state.as_mut() {
            state.name = name.to_string();
        }
        self.show();
    }

    fn choose_format(mut self: Pin<&mut Self>, index: i32) {
        if let (Some(state), Ok(index)) = (
            self.as_mut().rust_mut().state.as_mut(),
            usize::try_from(index),
        ) {
            state.format = index.min(rustphin_view::dialogs::FORMATS.len() - 1);
        }
        self.show();
    }

    fn choose_level(mut self: Pin<&mut Self>, index: i32) {
        if let (Some(state), Ok(index)) = (
            self.as_mut().rust_mut().state.as_mut(),
            usize::try_from(index),
        ) {
            state.level = index.min(rustphin_view::dialogs::LEVELS.len() - 1);
        }
        self.show();
    }

    fn accept(mut self: Pin<&mut Self>) -> bool {
        let Some(request) = self.rust().state.as_ref().and_then(Compress::request) else {
            return false;
        };
        hub::start(request);
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
        true
    }

    fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
    }
}

/// The Rust side of `BatchModel`.
#[derive(Default)]
pub struct BatchModelRust {
    view: QString,
    mode: i32,
    case_change: i32,
    open: bool,
    state: Option<BatchRename>,
}

impl qobject::BatchModel {
    fn start(mut self: Pin<&mut Self>) {
        let Some(given) = handed::take() else {
            return;
        };
        let selected: Vec<OsString> = given
            .sources
            .iter()
            .filter_map(|path| path.file_name().map(ToOwned::to_owned))
            .collect();
        self.as_mut().rust_mut().state =
            Some(BatchRename::new(given.folder, selected, given.staying));
        self.as_mut().show();
        self.set_open(true);
    }

    fn show(mut self: Pin<&mut Self>) {
        let Some(state) = self.rust().state.as_ref() else {
            return;
        };
        let json = serde_json::to_string(&state.view()).unwrap_or_default();
        let mode = match state.mode {
            Mode::Replace => 0,
            Mode::Pattern => 1,
            Mode::Case => 2,
        };
        let case = match state.case {
            CaseChange::Lower => 0,
            CaseChange::Upper => 1,
            CaseChange::Words => 2,
        };
        self.as_mut().set_view(QString::from(&json));
        self.as_mut().set_mode(mode);
        self.set_case_change(case);
    }

    fn edit(mut self: Pin<&mut Self>, field: &QString, value: &QString) {
        let value = value.to_string();
        if let Some(state) = self.as_mut().rust_mut().state.as_mut() {
            match field.to_string().as_str() {
                "find" => state.find = value,
                "replace" => state.replace = value,
                "regex" => state.regex = value == "true",
                "pattern" => state.pattern = value,
                "start" => state.start = value,
                "step" => state.step = value,
                _ => {}
            }
        }
        self.show();
    }

    fn choose_mode(mut self: Pin<&mut Self>, mode: i32) {
        if let Some(state) = self.as_mut().rust_mut().state.as_mut() {
            state.mode = match mode {
                1 => Mode::Pattern,
                2 => Mode::Case,
                _ => Mode::Replace,
            };
        }
        self.show();
    }

    fn choose_case(mut self: Pin<&mut Self>, change: i32) {
        if let Some(state) = self.as_mut().rust_mut().state.as_mut() {
            state.case = match change {
                1 => CaseChange::Upper,
                2 => CaseChange::Words,
                _ => CaseChange::Lower,
            };
        }
        self.show();
    }

    fn accept(mut self: Pin<&mut Self>) -> bool {
        let Some(request) = self.rust().state.as_ref().and_then(BatchRename::request) else {
            return false;
        };
        hub::start(request);
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
        true
    }

    fn cancel(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().state = None;
        self.set_open(false);
    }
}
