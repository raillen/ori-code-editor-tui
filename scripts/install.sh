#!/usr/bin/env bash
# Instala o binário `oride` em ~/.local/bin (ou PREFIX).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="${PREFIX}/bin"

echo "==> building oride (release)"
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p oride

mkdir -p "$BIN_DIR"
install -m 755 "$ROOT/target/release/oride" "$BIN_DIR/oride"

echo "==> installed: $BIN_DIR/oride"
if ! command -v oride >/dev/null 2>&1; then
  echo "    adicione ao PATH: export PATH=\"$BIN_DIR:\$PATH\""
fi
oride --version 2>/dev/null || "$BIN_DIR/oride" --version
