#!/usr/bin/env bash
set -euo pipefail

ARCHIVE="${1:-dist/codex-speak-macos.tar.gz}"
SMOKE_DIR="${TMPDIR:-/tmp}/codex-speak-release-smoke"

rm -rf "$SMOKE_DIR"
mkdir -p "$SMOKE_DIR"
tar -xzf "$ARCHIVE" -C "$SMOKE_DIR"

"$SMOKE_DIR/codex-speak-macos/installers/install-macos.sh" --skip-tts-download

test -x "$HOME/.codex/codex-speak/bin/codex-speak"
test -x "$HOME/.codex/codex-speak/bin/codex-speak-pet-macos"
test -d "$HOME/.codex/codex-speak/apps/Codex Speak.app"
test -f "$HOME/.codex/codex-speak/assets/pet/codex-agent.mov"

"$HOME/.codex/codex-speak/bin/codex-speak" models list >"$SMOKE_DIR/models.json"
echo "macOS release smoke install OK"
