#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="${NODE_BIN:-node}"
SWIFT_MODULE_CACHE="${SWIFT_MODULE_CACHE:-${TMPDIR:-/tmp}/codex-speak-swift-module-cache}"
CONTROL_APP_DIR="$ROOT_DIR/apps/codex-speak-control"

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
bash -n scripts/manual-qa-macos.sh

if command -v npm >/dev/null 2>&1; then
  (cd "$CONTROL_APP_DIR" && npm run build:frontend)
else
  echo "Skipping frontend build: npm not found. CI still runs npm ci and the frontend build." >&2
fi

if command -v "$NODE_BIN" >/dev/null 2>&1; then
  "$NODE_BIN" --check scripts/check-release-package.mjs
  "$NODE_BIN" --check scripts/check-manual-qa-report.mjs
  "$NODE_BIN" --check scripts/check-release-manifest.mjs
  "$NODE_BIN" --check scripts/write-release-manifest.mjs
  "$NODE_BIN" --check scripts/prepare-qa-handoff.mjs
  "$NODE_BIN" --check scripts/check-release-readiness.mjs
  "$NODE_BIN" --check apps/codex-speak-control/vite.config.js
  "$NODE_BIN" --check apps/codex-speak-control/src/main.js
  "$NODE_BIN" scripts/prepare-qa-handoff.mjs \
    --platform windows \
    --allow-missing \
    --ci-artifacts-json tests/fixtures/ci-artifacts.json \
    --output-dir "${TMPDIR:-/tmp}/codex-speak-qa-handoff-fixture-check" \
    >/tmp/codex-speak-qa-handoff-fixture-check.log
else
  echo "Skipping frontend syntax checks: node not found" >&2
fi

if [ "$(uname -s)" = "Darwin" ] && command -v swiftc >/dev/null 2>&1; then
  mkdir -p "$SWIFT_MODULE_CACHE"
  swiftc -module-cache-path "$SWIFT_MODULE_CACHE" -O -framework AppKit -framework AVFoundation -o /tmp/codex-speak-pet-macos-check apps/codex-speak-pet-macos/CodexSpeakPet.swift
  swift -module-cache-path "$SWIFT_MODULE_CACHE" scripts/generate-pet-assets.swift /tmp/codex-speak-pet-assets-check >/tmp/codex-speak-pet-assets-check.log
  swift -module-cache-path "$SWIFT_MODULE_CACHE" scripts/build-pet-assets-from-spritesheet.swift \
    apps/codex-speak-pet-macos/assets/codex-agent-source-spritesheet.png \
    /tmp/codex-speak-pet-spritesheet-assets-check \
    >/tmp/codex-speak-pet-spritesheet-assets-check.log
fi

echo "Codex Speak local verification passed."
