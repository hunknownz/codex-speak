#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_TARGET="$HOME/.codex/codex-speak/apps/Codex Speak.app"
PET_HELPER_SOURCE="$ROOT_DIR/apps/codex-speak-pet-macos/CodexSpeakPet.swift"
PET_HELPER_TARGET="$HOME/.codex/codex-speak/bin/codex-speak-pet-macos"
PET_ASSETS_TARGET="$HOME/.codex/codex-speak/assets/pet"
SKIP_CONTROL_APP=0
SKIP_TTS_DOWNLOAD=0
CODEX_SPEAK_ARGS=()

for arg in "$@"; do
  case "$arg" in
    --skip-control-app)
      SKIP_CONTROL_APP=1
      ;;
    --skip-tts-download)
      SKIP_TTS_DOWNLOAD=1
      CODEX_SPEAK_ARGS+=("$arg")
      ;;
    *)
      CODEX_SPEAK_ARGS+=("$arg")
      ;;
  esac
done

print_next_steps() {
  local cli="$HOME/.codex/codex-speak/bin/codex-speak"
  local include_control_app=0
  if [ "$SKIP_CONTROL_APP" -eq 0 ] && [ -e "$APP_TARGET" ]; then
    include_control_app=1
  fi
  echo
  echo "Next steps:"
  echo "  1. Run self-check:"
  echo "     $cli doctor"
  if [ "$include_control_app" -eq 1 ]; then
    echo "  2. Open the control app:"
    echo "     $cli app open"
    echo "  3. If you need help, create a support bundle:"
  else
    echo "  2. If you need help, create a support bundle:"
  fi
  echo "     $cli support-bundle"
  if [ "$SKIP_TTS_DOWNLOAD" -eq 1 ]; then
    if [ "$include_control_app" -eq 1 ]; then
      echo "  4. Install the default local Chinese voice model when ready:"
    else
      echo "  3. Install the default local Chinese voice model when ready:"
    fi
    echo "     $cli models install --provider sherpa_melo"
  fi
}

copy_app() {
  local source="$1"
  mkdir -p "$(dirname "$APP_TARGET")"
  rm -rf "$APP_TARGET"
  ditto "$source" "$APP_TARGET"
  echo "Codex Speak control app installed at $APP_TARGET"
}

copy_pet_assets() {
  local source="$1"
  rm -rf "$PET_ASSETS_TARGET"
  mkdir -p "$PET_ASSETS_TARGET"
  ditto "$source" "$PET_ASSETS_TARGET"
  echo "Codex Speak pet assets installed at $PET_ASSETS_TARGET"
}

install_prebuilt_helper() {
  local source="$1"
  mkdir -p "$(dirname "$PET_HELPER_TARGET")"
  cp "$source" "$PET_HELPER_TARGET"
  chmod +x "$PET_HELPER_TARGET"
  echo "Codex Speak native pet helper installed at $PET_HELPER_TARGET"
}

cd "$ROOT_DIR"

CLI_SOURCE=""
if [ -x "$ROOT_DIR/bin/codex-speak" ]; then
  CLI_SOURCE="$ROOT_DIR/bin/codex-speak"
elif [ -x "$ROOT_DIR/target/release/codex-speak" ]; then
  CLI_SOURCE="$ROOT_DIR/target/release/codex-speak"
elif command -v cargo >/dev/null 2>&1; then
  cargo build --release
  CLI_SOURCE="$ROOT_DIR/target/release/codex-speak"
else
  echo "Codex Speak CLI was not found."
  echo "Use a release package that contains bin/codex-speak, or install Rust from https://rustup.rs and run this script from the source checkout."
  exit 1
fi

"$CLI_SOURCE" install --no-summary "${CODEX_SPEAK_ARGS[@]}"

if [ "$SKIP_CONTROL_APP" -eq 1 ]; then
  echo "Skipping Codex Speak control app and native pet helper install."
  print_next_steps
  exit 0
fi

if [ -x "$ROOT_DIR/bin/codex-speak-pet-macos" ]; then
  install_prebuilt_helper "$ROOT_DIR/bin/codex-speak-pet-macos"
elif [ -x "$ROOT_DIR/target/release/codex-speak-pet-macos" ]; then
  install_prebuilt_helper "$ROOT_DIR/target/release/codex-speak-pet-macos"
elif [ -f "$PET_HELPER_SOURCE" ] && command -v swiftc >/dev/null 2>&1; then
  swiftc -O -framework AppKit -framework AVFoundation -o "$PET_HELPER_TARGET" "$PET_HELPER_SOURCE"
  echo "Codex Speak native pet helper installed at $PET_HELPER_TARGET"
else
  echo "Codex Speak native pet helper was not installed because no prebuilt helper or swiftc source build is available."
fi

if [ -d "$ROOT_DIR/assets/pet" ]; then
  copy_pet_assets "$ROOT_DIR/assets/pet"
elif [ -d "$ROOT_DIR/apps/codex-speak-pet-macos/assets" ]; then
  copy_pet_assets "$ROOT_DIR/apps/codex-speak-pet-macos/assets"
else
  echo "Codex Speak pet assets were not installed because no assets/pet directory was found."
fi

if [ -d "$ROOT_DIR/apps/Codex Speak.app" ]; then
  copy_app "$ROOT_DIR/apps/Codex Speak.app"
  print_next_steps
  exit 0
fi

APP_SOURCE="$ROOT_DIR/apps/codex-speak-control/src-tauri/target/release/bundle/macos/Codex Speak.app"
if [ -d "$APP_SOURCE" ]; then
  copy_app "$APP_SOURCE"
  print_next_steps
  exit 0
fi

if [ -f "$ROOT_DIR/apps/codex-speak-control/package.json" ] && command -v npm >/dev/null 2>&1; then
  echo "Building Codex Speak control app..."
  (
    cd "$ROOT_DIR/apps/codex-speak-control"
    npm install
    npm run build -- --bundles app
  )
fi

if [ -d "$APP_SOURCE" ]; then
  copy_app "$APP_SOURCE"
else
  echo "Codex Speak control app was not installed because no prebuilt app or npm build path is available."
fi
print_next_steps
