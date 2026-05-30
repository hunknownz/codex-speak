#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo is required to build this development installer."
  echo "Install Rust from https://rustup.rs, then run this script again."
  exit 1
fi

cd "$ROOT_DIR"
cargo build --release
"$ROOT_DIR/target/release/codex-speak" install "$@"

