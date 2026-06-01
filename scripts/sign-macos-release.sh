#!/usr/bin/env bash
set -euo pipefail

PACKAGE_DIR="${1:-}"
if [ -z "$PACKAGE_DIR" ] || [ ! -d "$PACKAGE_DIR" ]; then
  echo "Usage: sign-macos-release.sh <package-dir>" >&2
  exit 1
fi

IDENTITY="${MACOS_CODESIGN_IDENTITY:-}"
if [ -z "$IDENTITY" ]; then
  echo "Skipping macOS signing: MACOS_CODESIGN_IDENTITY is not set."
  exit 0
fi

APP="$PACKAGE_DIR/apps/Codex Speak.app"
CLI="$PACKAGE_DIR/bin/codex-speak"
PET_HELPER="$PACKAGE_DIR/bin/codex-speak-pet-macos"

codesign_file() {
  local target="$1"
  if [ ! -e "$target" ]; then
    echo "Missing sign target: $target" >&2
    exit 1
  fi
  codesign --force --timestamp --options runtime --sign "$IDENTITY" "$target"
  codesign --verify --strict --verbose=2 "$target"
}

codesign_file "$CLI"
codesign_file "$PET_HELPER"
codesign --force --deep --timestamp --options runtime --sign "$IDENTITY" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] && [ -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]; then
  tmp_dir="$(mktemp -d)"
  zip_path="$tmp_dir/codex-speak-app.zip"
  ditto -c -k --keepParent "$APP" "$zip_path"
  xcrun notarytool submit "$zip_path" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --wait
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"
  rm -rf "$tmp_dir"
else
  echo "Skipping macOS notarization: APPLE_ID, APPLE_TEAM_ID, or APPLE_APP_SPECIFIC_PASSWORD is not set."
fi
