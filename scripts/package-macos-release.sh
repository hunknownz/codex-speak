#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE="$ROOT_DIR/dist/codex-speak-macos"
APP_SOURCE="$ROOT_DIR/apps/codex-speak-control/src-tauri/target/release/bundle/macos/Codex Speak.app"

cd "$ROOT_DIR"

rm -rf "$PACKAGE" "$ROOT_DIR/dist/codex-speak-macos.tar.gz" "$ROOT_DIR/dist/codex-speak-macos.tar.gz.sha256"
mkdir -p "$PACKAGE/bin" "$PACKAGE/apps" "$PACKAGE/assets/pet" "$PACKAGE/installers" "$PACKAGE/scripts" "$PACKAGE/docs"

cp "$ROOT_DIR/target/release/codex-speak" "$PACKAGE/bin/codex-speak"
cp "$ROOT_DIR/target/release/codex-speak-pet-macos" "$PACKAGE/bin/codex-speak-pet-macos"
ditto "$APP_SOURCE" "$PACKAGE/apps/Codex Speak.app"
ditto "$ROOT_DIR/apps/codex-speak-pet-macos/assets" "$PACKAGE/assets/pet"
cp "$ROOT_DIR/installers/install-macos.sh" "$ROOT_DIR/installers/uninstall-macos.sh" "$PACKAGE/installers/"
cp "$ROOT_DIR/scripts/manual-qa-macos.sh" "$ROOT_DIR/scripts/check-manual-qa-report.mjs" "$PACKAGE/scripts/"
chmod +x \
  "$PACKAGE/bin/codex-speak" \
  "$PACKAGE/bin/codex-speak-pet-macos" \
  "$PACKAGE/installers/install-macos.sh" \
  "$PACKAGE/installers/uninstall-macos.sh" \
  "$PACKAGE/scripts/manual-qa-macos.sh" \
  "$PACKAGE/scripts/check-manual-qa-report.mjs"

cp "$ROOT_DIR/README.md" "$PACKAGE/README.md"
cp \
  "$ROOT_DIR/docs/installation.md" \
  "$ROOT_DIR/docs/release-qa.md" \
  "$ROOT_DIR/docs/requirements.md" \
  "$ROOT_DIR/docs/technical-design.md" \
  "$ROOT_DIR/docs/tauri-control-app.md" \
  "$PACKAGE/docs/"

"$ROOT_DIR/scripts/sign-macos-release.sh" "$PACKAGE"
node "$ROOT_DIR/scripts/write-release-manifest.mjs" macos "$PACKAGE"
node "$ROOT_DIR/scripts/check-release-package.mjs" macos "$PACKAGE"

tar -czf "$ROOT_DIR/dist/codex-speak-macos.tar.gz" -C "$ROOT_DIR/dist" codex-speak-macos
(cd "$ROOT_DIR/dist" && shasum -a 256 codex-speak-macos.tar.gz > codex-speak-macos.tar.gz.sha256)

echo "Created $ROOT_DIR/dist/codex-speak-macos.tar.gz"
