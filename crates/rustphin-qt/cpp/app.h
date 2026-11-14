#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>

#include "rust/cxx.h"

// The application's windows (W-114c3). The shell owns the QML engine: it
// creates it for the first window and, when the last window closes, either
// deletes it (Rustphin keeps running, R-020) or quits.
//
// Registers the bundled fonts and the icon theme fallback, and how the last
// window's closing is handled. Call once, on the GUI thread, before the
// event loop.
void rustphin_shell_start(bool keep_running, const QString& home);
// Opens a window on each folder (one at home when the list is empty),
// giving the launch's activation token to the window. Safe from any thread.
void rustphin_shell_open(const QStringList& folders, const QString& activation);
// Closes every window and ends the process. Safe from any thread.
void rustphin_shell_quit();
// Runs `run` on the GUI thread, from its event loop. Safe from any thread.
void rustphin_post(rust::Fn<void()> run);
