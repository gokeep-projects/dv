#!/usr/bin/env bash
set -euo pipefail
BIN_DIR="${HOME}/.local/bin"
PLUGIN_DIR="${HOME}/.local/share/devtool/plugins"

echo "=== DevTool Installer ==="
echo "[1/3] Building release..."
cargo build --release
echo "[2/3] Installing binary + plugins..."
mkdir -p "${BIN_DIR}" "${PLUGIN_DIR}"
cp target/release/devtool "${BIN_DIR}/"
cp target/release/libdevtool_plugin_*.so "${PLUGIN_DIR}/" 2>/dev/null || true
echo "[3/3] Generating shell completion..."
mkdir -p "${HOME}/.local/share/bash-completion/completions"
"${BIN_DIR}/devtool" completions bash > "${HOME}/.local/share/bash-completion/completions/devtool" 2>/dev/null || true
echo "Done! Binary: ${BIN_DIR}/devtool"
echo "      Plugins: ${PLUGIN_DIR}"
echo "For bash completion, add to ~/.bashrc:"
echo "  source ${HOME}/.local/share/bash-completion/completions/devtool"
