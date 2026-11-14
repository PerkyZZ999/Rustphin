#pragma once

#include <QtCore/QStringList>
#include <QtGui/QGuiApplication>

// Blur behind every top-level window through KWin; true when KWin's blur
// effect is available.
bool spike_blur();
// Puts local paths on the clipboard as Dolphin does (text/uri-list, and
// KDE's cut mark when cutting).
void spike_copy(const QStringList& paths, bool cut);
// The local paths on the clipboard, if any.
QStringList spike_paste();
// Keeps the application running after its last window closes.
void spike_keep_running();
