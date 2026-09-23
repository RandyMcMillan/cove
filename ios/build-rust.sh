#!/bin/bash
# Build the Rust library for the current Xcode platform.
# This script is called from a Run Script build phase in the Cove target.
#
# Usage: ./build-rust.sh [--force]
#   --force  Skip staleness checks and rebuild unconditionally

set -euo pipefail

FORCE=false
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=true ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="${SCRIPT_DIR}/../rust"
XCFRAMEWORK_PATH="${SCRIPT_DIR}/CoveCore/Sources/cove_core_ffi.xcframework"

NEEDS_REBUILD=false
if [ "$FORCE" == "true" ]; then
    echo "--force set — rebuilding unconditionally"
    NEEDS_REBUILD=true
else
    # Check if we need to rebuild: compare newest Rust source against xcframework
    NEWEST_RUST=$(find "${RUST_DIR}/src" "${RUST_DIR}/crates" -name '*.rs' -newer "${XCFRAMEWORK_PATH}/Info.plist" 2>/dev/null | head -1 || true)

    # Also rebuild if the xcframework is missing a required slice
    if [ -n "$NEWEST_RUST" ]; then
        echo "Rust sources newer than xcframework — rebuilding"
        NEEDS_REBUILD=true
    elif [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64" ] || [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64-simulator" ]; then
        echo "xcframework missing required slices — rebuilding"
        NEEDS_REBUILD=true
    fi
fi

if [ "$NEEDS_REBUILD" != "true" ]; then
    echo "xcframework is up to date"
    exit 0
fi

cd "$RUST_DIR"

# Use all available CPU cores for Cargo builds. `nproc` is Linux-only; on macOS
# `sysctl -n hw.ncpu` gives the logical core count.
if command -v nproc >/dev/null 2>&1; then
    export CARGO_BUILD_JOBS=$(nproc)
elif command -v sysctl >/dev/null 2>&1; then
    export CARGO_BUILD_JOBS=$(sysctl -n hw.ncpu)
fi

# Build both device and simulator slices
# Use debug for Debug builds, release for Release builds
if [ "${CONFIGURATION:-Debug}" == "Release" ]; then
    echo "Building Rust library (release) with ${CARGO_BUILD_JOBS:-default} jobs..."
    just build-ios-release
else
    echo "Building Rust library (debug) with ${CARGO_BUILD_JOBS:-default} jobs..."
    just build-ios-debug-device
fi

echo "Rust library build complete"
