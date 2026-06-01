#!/usr/bin/env bash
set -euo pipefail

CLI="$HOME/.codex/codex-speak/bin/codex-speak"
APP="$HOME/.codex/codex-speak/apps/Codex Speak.app"
PET_HELPER="$HOME/.codex/codex-speak/bin/codex-speak-pet-macos"
PET_ASSETS="$HOME/.codex/codex-speak/assets/pet"
if [ -x "$CLI" ]; then
  pkill -x codex-speak-pet-macos 2>/dev/null || true
  "$CLI" uninstall "$@"
  rm -rf "$APP"
  rm -rf "$PET_ASSETS"
  rm -f "$PET_HELPER" "$HOME/.codex/codex-speak/state/pet-helper.pid"
else
  echo "Codex Speak CLI not found at $CLI"
  exit 1
fi
