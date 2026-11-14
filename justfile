set shell := ["bash", "-euo", "pipefail", "-c"]

# ---------------------------------------------------------
# Fast developer workflow
# ---------------------------------------------------------

# Build the locked application without opening a window or reading user state.
# Select an installed compiler with RUSTUP_TOOLCHAIN when validating R-109.
build:
    cargo build --locked --release --package rustphin


fmt:
    cargo fmt --all -- --check


check:
    cargo check --workspace --all-targets --all-features


clippy:
    cargo clippy \
        --workspace \
        --all-targets \
        --all-features \
        -- \
        -D warnings


test:
    cargo nextest run \
        --workspace \
        --all-features

    cargo test \
        --doc \
        --workspace \
        --all-features


# Fast developer validation.
quick: fmt check clippy test


# Alias developers/agents are expected to use most often.
doctor-check: quick


# ---------------------------------------------------------
# Dependency verification
# ---------------------------------------------------------

deps:
    cargo deny check

    cargo shear --deny-warnings


# ---------------------------------------------------------
# Cargo feature validation
# ---------------------------------------------------------

features:
    cargo hack check \
        --workspace \
        --each-feature \
        --no-dev-deps


features-deep:
    cargo hack check \
        --workspace \
        --feature-powerset \
        --depth 2 \
        --no-dev-deps


# ---------------------------------------------------------
# Comprehensive LOCAL Rust-Toolchain
# ---------------------------------------------------------
#
# NOTE:
# Plain cargo check is intentionally omitted here because
# Clippy already performs compilation/checking as part of its
# analysis.
#
# `rust-tc check` remains useful during normal development because
# it provides the fastest compiler-only feedback path.
#

doctor: fmt qml-format clippy test qml-lint deps features layers msrv fuzz-check
    @echo
    @echo "Rust-Toolchain: PASS"


# R-109: one entry point for the full gate and both headless benchmarks.
# Keep benchmarks sequential and propagate their performance failures.
validate: doctor bench-listing bench-palette


# Nextest does not currently execute doctests, so this is a
# distinct/non-duplicated test category.
doctest:
    cargo test \
        --doc \
        --workspace \
        --all-features


# Optional local coverage. Not part of `rust-tc doctor`.
coverage:
    mkdir -p target/rust-toolchain
    cargo llvm-cov \
        --lcov \
        --output-path target/rust-toolchain/lcov.info \
        nextest \
        --workspace \
        --all-features


# ---------------------------------------------------------
# SemVer validation
# ---------------------------------------------------------
#
# Intended primarily for library/public API crates.
#

semver package baseline="origin/main":
    cargo semver-checks \
        --package "{{package}}" \
        --baseline-rev "{{baseline}}"


# ---------------------------------------------------------
# Deep verification
# ---------------------------------------------------------

mutants:
    cargo mutants


miri *args:
    cargo +nightly miri test {{args}}


# Bounded, repeatable parser campaigns; mutable discoveries stay untracked.
fuzz target seconds="30":
    @if ! [[ "{{seconds}}" =~ ^[1-9][0-9]*$ ]]; then echo "seconds must be a positive integer" >&2; exit 2; fi
    mkdir -p "fuzz/corpus/{{target}}"
    cp fuzz/seeds/{{target}}/* "fuzz/corpus/{{target}}/"
    cargo +nightly fuzz run "{{target}}" -- -max_total_time={{seconds}} -max_len=4096 -timeout=5 -rss_limit_mb=2048 -seed=7501

# The harness has its own workspace so libFuzzer never ships in the app.
fuzz-check:
    cargo fmt --manifest-path fuzz/Cargo.toml --all -- --check
    cargo clippy --manifest-path fuzz/Cargo.toml --bins -- -D warnings
    cargo deny --manifest-path fuzz/Cargo.toml check --config deny.toml


deep: features-deep
    @echo
    @echo "Feature powerset validation complete."
    @echo "Run mutation, Miri and fuzz checks selectively:"
    @echo "  rust-tc mutants"
    @echo "  rust-tc miri"
    @echo "  rust-tc fuzz <target> [seconds]"


# ---------------------------------------------------------
# Rustphin project checks
# ---------------------------------------------------------

# Layering (decision D-002): the core must not depend on a UI toolkit, Wayland
# or D-Bus; the platform layer and the interface logic must not depend on
# Qt; nothing depends on GPUI, which the Qt interface replaced (W-114i).
layers:
    #!/usr/bin/env bash
    set -euo pipefail
    status=0
    forbid() {
        local found
        found="$(cargo tree --package "$1" --edges normal --prefix none | grep -E "^($2)" | sort -u || true)"
        if [[ -n "$found" ]]; then
            echo "layering violation: $1 must not depend on:"
            echo "$found"
            status=1
        fi
    }
    forbid rustphin-core 'gpui|wayland|zbus|cxx'
    forbid rustphin-platform 'gpui|cxx'
    forbid rustphin-view 'gpui|cxx'
    forbid rustphin 'gpui'
    exit "$status"


# QML formatting (W-116): every file as qmlformat writes it, with the
# settings in crates/rustphin/qml/.qmlformat.ini.
qml-format:
    #!/usr/bin/env bash
    set -euo pipefail
    status=0
    for file in crates/rustphin/qml/*.qml; do
        if ! /usr/lib/qt6/bin/qmlformat "$file" | cmp -s - "$file"; then
            echo "not formatted: $file (run: /usr/lib/qt6/bin/qmlformat -i $file)"
            status=1
        fi
    done
    exit "$status"


# QML lint (W-116): qmllint with the maintainer's BestPractices plugin, every
# category a warning and no warning allowed. The plugin is built from
# ../QtQmlBestPractices into target/ against the
# installed Qt. The module's generated type information is copied without
# the opaque stand-ins qmltyperegistrar writes for cxx-qt's QString,
# QStringList, QMap and plain types, which would hide Qt's own types, and
# the QML files are linked in, so qmllint sees the module as Qt loads it.
qml-lint:
    #!/usr/bin/env bash
    set -euo pipefail
    plugin=target/qmllint-plugin
    if [[ ! -f "$plugin/libBestPracticesPlugin.so" ]]; then
        cmake -S ../QtQmlBestPractices -B "$plugin" -DCMAKE_BUILD_TYPE=Release >/dev/null
        cmake --build "$plugin" -j"$(nproc)" >/dev/null
    fi
    cargo build --quiet --package rustphin
    generated="$(find target/debug/build/rustphin -name plugin.qmltypes -printf '%T@ %h\n' | sort -n | tail -1 | cut -d' ' -f2)"
    module=target/qmllint-module/Rustphin
    rm -rf target/qmllint-module
    mkdir -p "$module"
    sed '/isTypeOpaque: true/d; s/"QMap_QString_QVariant"/"QVariantMap"/g' "$generated/plugin.qmltypes" > "$module/plugin.qmltypes"
    { cat "$generated/qmldir"; echo "depends QtQuick"; echo "depends QtQml"; } > "$module/qmldir"
    ln -s "$PWD/crates/rustphin/qml" "$module/qml"
    ln -s "$PWD/crates/rustphin/icons" "$module/icons"
    categories=(prefer-typed-properties layout-child-geometry versioned-imports no-state-in-delegates imperative-completed-assignment native-style-customization prefer-interaction-signals redundant-window-import)
    args=()
    for category in "${categories[@]}"; do
        args+=("--Plugin.BestPractices.$category" warning)
    done
    # JSON output: in text mode qmllint 6.12 counts some plugin warnings it
    # does not print.
    /usr/lib/qt6/bin/qmllint --json target/qmllint-module/report.json -I target/qmllint-module -P "$PWD/$plugin" "${args[@]}" crates/rustphin/qml/*.qml || true
    python3 - target/qmllint-module/report.json <<'REPORT'
    import json, sys
    found = 0
    for entry in json.load(open(sys.argv[1]))["files"]:
        for warning in entry["warnings"]:
            if warning["type"] in ("warning", "critical"):
                found += 1
                print(f'{entry["filename"]}:{warning.get("line", 0)}:{warning.get("column", 0)}: {warning["message"]} [{warning["id"]}]')
    sys.exit(1 if found else 0)
    REPORT


# Minimum supported Rust version (requirement R-109).
msrv:
    cargo +1.100.0 check --workspace --all-targets --all-features


# Listing benchmark on 500,000 entries; fails when requirement R-101's
# thresholds are missed.
bench-listing:
    cargo run --release --quiet --package rustphin-core --example bench_listing -- target/bench/listing-500k


# Fuzzy ranking on 10,000 recent locations; fails above R-015's 50 ms limit.
bench-palette:
    cargo run --release --quiet --package rustphin-core --example bench_palette


# Run the application. Pass a folder to open, e.g. `rust-tc run ~/Documents`.
run *args:
    cargo run --release --package rustphin -- {{args}}

# ---------------------------------------------------------
# Convenience
# ---------------------------------------------------------

clean-toolchain:
    rm -rf target/rust-toolchain


toolchain-help:
    @echo "Rust-Toolchain"
    @echo
    @echo "  rust-tc build         Build the locked release application"
    @echo "  rust-tc check         Fast compiler check"
    @echo "  rust-tc quick         Fast developer quality gate"
    @echo "  rust-tc doctor        Full local Rust-Toolchain validation"
    @echo "  rust-tc validate      Full gate and both headless benchmarks"
    @echo "  rust-tc features-deep Deeper Cargo feature combinations"
    @echo "  rust-tc semver PKG    Public API compatibility"
    @echo "  rust-tc mutants       Mutation testing"
    @echo "  rust-tc miri          Undefined-behavior checking"
    @echo "  rust-tc fuzz TARGET [SECONDS]  Bounded parser fuzzing (default 30s)"
    @echo "  rust-tc coverage      Optional local LCOV report"
    @echo
    @echo "Rustphin"
    @echo
    @echo "  rust-tc run [FOLDER]  Build and run the application"
    @echo "  rust-tc bench-listing 500,000-entry listing benchmark (R-101)"
    @echo "  rust-tc bench-palette 10,000-location palette benchmark (R-015)"
    @echo "  rust-tc layers        Crate layering check (part of doctor)"
    @echo "  rust-tc qml-lint      qmllint with the BestPractices plugin (part of doctor)"
    @echo "  rust-tc qml-format    qmlformat check of the QML (part of doctor)"
    @echo "  rust-tc msrv          Compiler check on Rust 1.100 (part of doctor)"
