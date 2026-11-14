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

doctor: fmt clippy test deps features layers msrv fuzz-check
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
# or D-Bus, and the platform layer must not depend on the UI toolkit.
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
    forbid rustphin-core 'gpui|wayland|zbus'
    forbid rustphin-platform 'gpui'
    forbid rustphin-view 'gpui|cxx-qt'
    exit "$status"


# Minimum supported Rust version (requirement R-109).
msrv:
    cargo +1.95.0 check --workspace --all-targets --all-features


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

# Build the optional pinned-toolkit profiler; launch only in the isolated UI harness.
build-diagnostics:
    cargo build --release --package rustphin --features diagnostics


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
    @echo "  rust-tc msrv          Compiler check on Rust 1.95 (part of doctor)"
