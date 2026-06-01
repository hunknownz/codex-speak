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
"$HOME/.codex/codex-speak/bin/codex-speak" doctor --json >"$SMOKE_DIR/doctor.json" || true
node - "$SMOKE_DIR/doctor.json" <<'NODE'
const fs = require("fs");
const [, , reportPath] = process.argv;
const report = JSON.parse(fs.readFileSync(reportPath, "utf8"));
const byId = new Map((report.checks || []).map((check) => [check.id, check]));
for (const id of [
  "codex_home",
  "config",
  "cli",
  "control_app",
  "pet_helper",
  "codex_notify",
  "plugin",
  "plugin_marketplace",
  "player"
]) {
  const check = byId.get(id);
  if (!check || check.status !== "ok") {
    throw new Error(`doctor core check failed: ${id}`);
  }
}
NODE
echo "macOS release smoke install OK"
