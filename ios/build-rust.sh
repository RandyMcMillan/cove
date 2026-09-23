#!/bin/bash
# Build the Rust library for the current Xcode platform.
# This script is called from a Run Script build phase in the Cove target.
#
# Usage: ./build-rust.sh [--force] [--verbose]
#   --force   Skip staleness checks and rebuild unconditionally
#   --verbose Pass -v to xtask so cargo build output is shown

set -euo pipefail

# Xcode's external-build-tool target captures both stdout and stderr, but the
# build log surfaces stderr most reliably. Merge stdout into stderr so every
# status line shows up while the build is running.
exec 1>&2

echo "[build-rust.sh] Starting Rust build check..."
echo "[build-rust.sh] CONFIGURATION=${CONFIGURATION:-Debug} PLATFORM_NAME=${PLATFORM_NAME:-unknown}"

# Ensure cargo and just are discoverable. Do not hardcode $HOME/.cargo/bin;
# prefer the caller's PATH and fall back to common install locations.
ensure_on_path() {
    if command -v cargo >/dev/null 2>&1 && command -v just >/dev/null 2>&1; then
        return
    fi
    for dir in "$HOME/.cargo/bin" "/usr/local/bin" "/opt/homebrew/bin" "/opt/local/bin"; do
        if [ -d "$dir" ]; then
            export PATH="$dir:$PATH"
        fi
    done
}
ensure_on_path

echo "[build-rust.sh] cargo=$(command -v cargo) just=$(command -v just)"

if [ "${BUILD_RUST_VERBOSE:-}" == "1" ]; then
    set -x
fi

FORCE=false
VERBOSE=false
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=true ;;
        --verbose) VERBOSE=true ;;
    esac
done

if [ "$VERBOSE" == "true" ]; then
    BUILD_RUST_VERBOSE=1
    export BUILD_RUST_VERBOSE
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="${SCRIPT_DIR}/../rust"
XCFRAMEWORK_PATH="${SCRIPT_DIR}/CoveCore/Sources/cove_core_ffi.xcframework"

NEEDS_REBUILD=false
if [ "$FORCE" == "true" ]; then
    echo "[build-rust.sh] --force set — rebuilding unconditionally"
    NEEDS_REBUILD=true
else
    # Check if we need to rebuild: compare newest Rust source against xcframework
    NEWEST_RUST=$(find "${RUST_DIR}/src" "${RUST_DIR}/crates" -name '*.rs' -newer "${XCFRAMEWORK_PATH}/Info.plist" 2>/dev/null | head -1 || true)

    # Also rebuild if the xcframework is missing a required slice
    if [ -n "$NEWEST_RUST" ]; then
        echo "[build-rust.sh] Rust sources newer than xcframework — rebuilding"
        NEEDS_REBUILD=true
    elif [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64" ] || [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64-simulator" ]; then
        echo "[build-rust.sh] xcframework missing required slices — rebuilding"
        NEEDS_REBUILD=true
    fi
fi

if [ "$NEEDS_REBUILD" != "true" ]; then
    echo "[build-rust.sh] xcframework is up to date — nothing to do"
    exit 0
fi

cd "$RUST_DIR"

echo "[build-rust.sh] Working directory: $(pwd)"

# Use all available CPU cores for Cargo builds. `nproc` is Linux-only; on macOS
# `sysctl -n hw.ncpu` gives the logical core count.
if command -v nproc >/dev/null 2>&1; then
    export CARGO_BUILD_JOBS=$(nproc)
elif command -v sysctl >/dev/null 2>&1; then
    export CARGO_BUILD_JOBS=$(sysctl -n hw.ncpu)
fi

# Build both device and simulator slices
# Use debug for Debug builds, release for Release builds
XTASK_FLAGS=""
if [ "${BUILD_RUST_VERBOSE:-}" == "1" ]; then
    XTASK_FLAGS="-v"
fi

if [ "${CONFIGURATION:-Debug}" == "Release" ]; then
    echo "[build-rust.sh] Building Rust library (release) with ${CARGO_BUILD_JOBS:-default} jobs..."
    echo "[build-rust.sh] $ just build-ios-release ${XTASK_FLAGS}"
    just build-ios-release ${XTASK_FLAGS}
else
    echo "[build-rust.sh] Building Rust library (debug) with ${CARGO_BUILD_JOBS:-default} jobs..."
    echo "[build-rust.sh] $ just build-ios-debug-device ${XTASK_FLAGS}"
    just build-ios-debug-device ${XTASK_FLAGS}
fi

echo "[build-rust.sh] Rust library build complete"
