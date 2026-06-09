#!/usr/bin/env bash
set -euo pipefail

ARCHIVE="${1:-dist/codex-speak-macos.tar.gz}"
SHA_FILE="${ARCHIVE}.sha256"
SMOKE_DIR="${TMPDIR:-/tmp}/codex-speak-release-smoke"

rm -rf "$SMOKE_DIR"
mkdir -p "$SMOKE_DIR"

if [ ! -f "$SHA_FILE" ]; then
  echo "Missing checksum file: $SHA_FILE" >&2
  exit 1
fi
(
  cd "$(dirname "$ARCHIVE")"
  shasum -a 256 -c "$(basename "$SHA_FILE")"
) >"$SMOKE_DIR/archive-sha256.txt"

tar -xzf "$ARCHIVE" -C "$SMOKE_DIR"

"$SMOKE_DIR/codex-speak-macos/bin/codex-speak" verify-package --package-dir "$SMOKE_DIR/codex-speak-macos" >"$SMOKE_DIR/verify-package.txt"
node "$SMOKE_DIR/codex-speak-macos/scripts/check-release-manifest.mjs" "$SMOKE_DIR/codex-speak-macos" >"$SMOKE_DIR/check-release-manifest.txt"
"$SMOKE_DIR/codex-speak-macos/installers/install-macos.sh" --skip-tts-download

test -x "$HOME/.codex/codex-speak/bin/codex-speak"
test -x "$HOME/.codex/codex-speak/bin/codex-speak-pet-macos"
test -d "$HOME/.codex/codex-speak/apps/Codex Speak.app"
test -f "$HOME/.codex/codex-speak/assets/pet/codex-agent.mov"
test -f "$HOME/.codex/codex-speak/assets/pet/codex-agent-source-spritesheet.png"
test -f "$HOME/.codex/codex-speak/release-manifest.json"

"$HOME/.codex/codex-speak/bin/codex-speak" models list >"$SMOKE_DIR/models.json"
"$HOME/.codex/codex-speak/bin/codex-speak" verify-install --allow-missing-models >"$SMOKE_DIR/verify-install.txt"
"$HOME/.codex/codex-speak/bin/codex-speak" verify-codex >"$SMOKE_DIR/verify-codex.txt"
"$HOME/.codex/codex-speak/bin/codex-speak" verify-controls >"$SMOKE_DIR/verify-controls.txt"
"$SMOKE_DIR/codex-speak-macos/scripts/manual-qa-macos.sh" \
  --cli-path "$HOME/.codex/codex-speak/bin/codex-speak" \
  --output-dir "$SMOKE_DIR/manual-qa" \
  --allow-missing-models \
  --non-interactive \
  >"$SMOKE_DIR/manual-qa.txt"
node "$SMOKE_DIR/codex-speak-macos/scripts/check-manual-qa-report.mjs" \
  "$SMOKE_DIR/manual-qa" \
  --allow-non-interactive \
  >>"$SMOKE_DIR/manual-qa.txt"
"$HOME/.codex/codex-speak/bin/codex-speak" support-bundle --output "$SMOKE_DIR/support"
test -f "$SMOKE_DIR/support/doctor.json"
test -f "$SMOKE_DIR/support/status.json"
test -f "$SMOKE_DIR/support/models.json"
test -f "$SMOKE_DIR/support/pronunciation-dictionary.json"
test -f "$SMOKE_DIR/support/support-bundle-metadata.json"
test -f "$SMOKE_DIR/support/release-manifest.json"
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
  "codex_stop_hook",
  "codex_stop_hook_script",
  "agents_hint",
  "queue",
  "codex_skill",
  "player"
]) {
  const check = byId.get(id);
  if (!check || check.status !== "ok") {
    throw new Error(`doctor core check failed: ${id}`);
  }
}
NODE
echo "macOS release smoke install OK"
