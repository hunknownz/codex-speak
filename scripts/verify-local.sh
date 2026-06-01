#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="${NODE_BIN:-node}"

cd "$ROOT_DIR"

cargo fmt --check
cargo test
cargo build --release
cargo check --manifest-path apps/codex-speak-control/src-tauri/Cargo.toml
bash -n installers/install-macos.sh
bash -n installers/uninstall-macos.sh
bash -n scripts/package-macos-release.sh
bash -n scripts/smoke-install-macos-release.sh
bash -n scripts/sign-macos-release.sh

if command -v npm >/dev/null 2>&1; then
  (cd apps/codex-speak-control && npm run build:frontend)
fi

if command -v "$NODE_BIN" >/dev/null 2>&1; then
  "$NODE_BIN" --check scripts/check-release-package.mjs
  "$NODE_BIN" --check apps/codex-speak-control/vite.config.js
  "$NODE_BIN" --check apps/codex-speak-control/src/main.js
else
  echo "Skipping frontend syntax checks: node not found" >&2
fi

if [ "$(uname -s)" = "Darwin" ] && command -v swiftc >/dev/null 2>&1; then
  swiftc -O -framework AppKit -framework AVFoundation -o /tmp/codex-speak-pet-macos-check apps/codex-speak-pet-macos/CodexSpeakPet.swift
  swift scripts/generate-pet-assets.swift /tmp/codex-speak-pet-assets-check >/tmp/codex-speak-pet-assets-check.log
fi

echo "Codex Speak local verification passed."
