set shell := ["bash", "-euo", "pipefail", "-c"]

# ---------------------------------------------------------
# Fast developer workflow
# ---------------------------------------------------------

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

doctor: fmt clippy test deps features layers msrv
    @echo
    @echo "Rust-Toolchain: PASS"


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


miri:
    cargo +nightly miri test


fuzz target:
    cargo +nightly fuzz run "{{target}}"


deep: features-deep
    @echo
    @echo "Feature powerset validation complete."
    @echo "Run mutation, Miri and fuzz checks selectively:"
    @echo "  rust-tc mutants"
    @echo "  rust-tc miri"
    @echo "  rust-tc fuzz <target>"


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


# ---------------------------------------------------------
# Convenience
# ---------------------------------------------------------

clean-toolchain:
    rm -rf target/rust-toolchain


toolchain-help:
    @echo "Rust-Toolchain"
    @echo
    @echo "  rust-tc check         Fast compiler check"
    @echo "  rust-tc quick         Fast developer quality gate"
    @echo "  rust-tc doctor        Full local Rust-Toolchain validation"
    @echo "  rust-tc features-deep Deeper Cargo feature combinations"
    @echo "  rust-tc semver PKG    Public API compatibility"
    @echo "  rust-tc mutants       Mutation testing"
    @echo "  rust-tc miri          Undefined-behavior checking"
    @echo "  rust-tc fuzz TARGET   Targeted fuzzing"
    @echo "  rust-tc coverage      Optional local LCOV report"
    @echo
    @echo "Rustphin"
    @echo
    @echo "  rust-tc run [FOLDER]  Build and run the application"
    @echo "  rust-tc bench-listing 500,000-entry listing benchmark (R-101)"
    @echo "  rust-tc bench-palette 10,000-location palette benchmark (R-015)"
    @echo "  rust-tc layers        Crate layering check (part of doctor)"
    @echo "  rust-tc msrv          Build on Rust 1.95 (part of doctor)"
