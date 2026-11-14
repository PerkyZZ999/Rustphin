#pragma once

// Whether KWin's blur effect can blur behind Rustphin's windows.
bool rustphin_blur_available();
// Asks KWin to blur behind every top-level window of the application.
void rustphin_blur_windows();
