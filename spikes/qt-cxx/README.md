# W-114a spike: Qt Quick through cxx-qt (throwaway)

A 500,000-row details list over `rustphin-core`, with frame timing, blur
through KDE's `KWindowEffects`, drag-out and the clipboard. Results are in
`docs/project/QT_MIGRATION_PLAN.md` ("Spike results") and VALIDATION.md.

    QMAKE=/usr/bin/qmake6 cargo build --release
    target/release/qt-cxx-spike <folder> [--scroll]
    target/release/qt-cxx-spike --background   # no window, for memory

F5 opens the folder again; the column buttons sort. Measurements go to
stderr. Not part of the workspace; do not depend on it.
