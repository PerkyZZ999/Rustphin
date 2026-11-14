//! Work handed to the Qt thread from any other: the core's operations,
//! the clipboard and timers report here, and what they report is applied
//! on the thread that owns the interface (D-009).

use std::{
    collections::VecDeque,
    sync::{Mutex, PoisonError},
};

type Task = Box<dyn FnOnce() + Send>;

static TASKS: Mutex<VecDeque<Task>> = Mutex::new(VecDeque::new());

/// Runs `task` on the Qt thread, after what was handed over before it.
/// Call only once the application object exists.
pub fn on_qt(task: impl FnOnce() + Send + 'static) {
    TASKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_back(Box::new(task));
    crate::shell::qobject::rustphin_post(run_posted);
}

/// Runs, on the Qt thread, every task handed over so far.
fn run_posted() {
    loop {
        let task = TASKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front();
        match task {
            Some(task) => task(),
            None => break,
        }
    }
}

/// Runs `task` on the Qt thread after `delay`.
pub fn after(delay: std::time::Duration, task: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        on_qt(task);
    });
}
