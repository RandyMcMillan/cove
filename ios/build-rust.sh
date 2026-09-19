#!/bin/bash
# Build the Rust library for the current Xcode platform.
# This script is called from a Run Script build phase in the Cove target.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_DIR="${SCRIPT_DIR}/../rust"
XCFRAMEWORK_PATH="${SCRIPT_DIR}/CoveCore/Sources/cove_core_ffi.xcframework"

# Check if we need to rebuild: compare newest Rust source against xcframework
NEWEST_RUST=$(find "${RUST_DIR}/src" "${RUST_DIR}/crates" -name '*.rs' -newer "${XCFRAMEWORK_PATH}/Info.plist" 2>/dev/null | head -1 || true)

# Also rebuild if the xcframework is missing a required slice
NEEDS_REBUILD=false
if [ -n "$NEWEST_RUST" ]; then
    echo "Rust sources newer than xcframework — rebuilding"
    NEEDS_REBUILD=true
elif [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64" ] || [ ! -d "${XCFRAMEWORK_PATH}/ios-arm64-simulator" ]; then
    echo "xcframework missing required slices — rebuilding"
    NEEDS_REBUILD=true
fi

if [ "$NEEDS_REBUILD" != "true" ]; then
    echo "xcframework is up to date"
    exit 0
fi

cd "$RUST_DIR"

# Build both device and simulator slices
# Use debug for Debug builds, release for Release builds
if [ "${CONFIGURATION:-Debug}" == "Release" ]; then
    echo "Building Rust library (release)..."
    just build-ios-release
else
    echo "Building Rust library (debug)..."
    just build-ios-debug-device
fi

echo "Rust library build complete"
