# W-114a spike: QtBridge for Rust 0.4 (throwaway)

The same list as `../qt-cxx`, through QtBridge. QtBridge finds Qt through
`qmake` on the PATH, which here is Qt 5's; the build uses a shim:

    mkdir -p shim && ln -sf /usr/bin/qmake6 shim/qmake
    PATH=$PWD/shim:$PATH cargo build --release
    QT_FORCE_STDERR_LOGGING=1 target/release/qtbridgespike <folder> [--scroll]

Without `QT_FORCE_STDERR_LOGGING=1`, QML errors go to the journal and the
program exits silently. Not part of the workspace; do not depend on it.
