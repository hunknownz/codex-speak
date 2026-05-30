#!/usr/bin/env bash
set -euo pipefail

CLI="$HOME/.codex/codex-speak/bin/codex-speak"
if [ -x "$CLI" ]; then
  "$CLI" uninstall "$@"
else
  echo "Codex Speak CLI not found at $CLI"
  exit 1
fi

