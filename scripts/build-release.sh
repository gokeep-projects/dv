#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build/release"
VERSION="${VERSION:-$(grep '^version' "$PROJECT_ROOT/crates/cli/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')}"

TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    "x86_64-pc-windows-msvc"
)

PLUGIN_NAMES=(
    "devtool_plugin_json"
    "devtool_plugin_crypto"
    "devtool_plugin_terminal"
    "devtool_plugin_log_search"
    "devtool_plugin_service_status"
    "devtool_plugin_middleware"
    "devtool_plugin_script_runner"
)

echo "=== DevTool Build Script ==="
echo "Version: $VERSION"
echo "Targets: ${TARGETS[*]}"
echo ""

for target in "${TARGETS[@]}"; do
    echo "--- Building for $target ---"

    # Check if target is available
    if ! rustup target list --installed | grep -q "$target"; then
        echo "  Installing target $target..."
        rustup target add "$target"
    fi

    # Build in release mode
    cargo build --release --target "$target" 2>&1 | tail -1

    # Package
    ARCHIVE_DIR="$BUILD_DIR/devtool-${VERSION}-${target}"
    mkdir -p "$ARCHIVE_DIR/plugins"

    # Copy binary
    if [[ "$target" == *"windows"* ]]; then
        cp "target/$target/release/devtool.exe" "$ARCHIVE_DIR/" 2>/dev/null || true
        # Copy plugin DLLs
        for plugin in "${PLUGIN_NAMES[@]}"; do
            cp "target/$target/release/${plugin}.dll" "$ARCHIVE_DIR/plugins/" 2>/dev/null || true
        done
    else
        cp "target/$target/release/devtool" "$ARCHIVE_DIR/" 2>/dev/null || true
        # Copy plugin shared libraries
        for plugin in "${PLUGIN_NAMES[@]}"; do
            cp "target/$target/release/lib${plugin}.so" "$ARCHIVE_DIR/plugins/" 2>/dev/null || true
            cp "target/$target/release/lib${plugin}.dylib" "$ARCHIVE_DIR/plugins/" 2>/dev/null || true
        done
    fi

    # Create archive
    echo "  Creating archive..."
    cd "$BUILD_DIR"
    if [[ "$target" == *"windows"* ]]; then
        zip -rq "devtool-${VERSION}-${target}.zip" "devtool-${VERSION}-${target}"
    else
        tar -czf "devtool-${VERSION}-${target}.tar.gz" "devtool-${VERSION}-${target}"
    fi
    cd "$PROJECT_ROOT"

    echo "  ✓ $target done"
    echo ""
done

echo "=== Build Complete ==="
echo "Artifacts in: $BUILD_DIR"
ls -lh "$BUILD_DIR"/*.{tar.gz,zip} 2>/dev/null || ls -lh "$BUILD_DIR/"
