# Codex Speak Control

Small Tauri control app for Codex Speak.

## What It Controls

- Automatic speech on or off.
- Child mode on or off.
- Voice profile:
  - `clear_bright`
  - `slow_clear`
  - `quick_preview`
- Speech speed.
- Maximum spoken characters.
- Stop current playback.
- Play a local sample sentence.
- Run `doctor` checks.
- Launch the native macOS desktop pet that reflects the current speech state.

The app does not duplicate TTS logic. It calls the installed Rust CLI at:

```text
~/.codex/codex-speak/bin/codex-speak
```

That keeps the Tauri app, Plugin MCP tools, Hook, and command line on the same configuration and playback path.

## Desktop Pet

On macOS the desktop pet is not a Tauri WebView. The control app launches the native helper:

```text
~/.codex/codex-speak/bin/codex-speak-pet-macos
```

The helper follows the same display model as lil-agents: a borderless AppKit `NSWindow`, clear background, status-bar window level, `AVPlayerLayer` for a 1080x1920 HEVC-with-alpha `.mov` character animation, display-link updates, a separate native speech bubble, and visible-pixel hit testing with an alpha-mask fallback.

The helper reads the shared state file written by the Rust core:

```text
~/.codex/codex-speak/state/pet-state.json
```

The pet stays intentionally small: it shows idle, ready, speaking, done, and error states; it can be dragged; clicking it while speaking stops playback; double-clicking opens the control window.

Pet assets are installed at:

```text
~/.codex/codex-speak/assets/pet/
```

The default asset is an original 2D walking companion built from `codex-agent-source-spritesheet.png` with `scripts/build-pet-assets-from-spritesheet.swift`. To redesign the pet, replace the sprite sheet and regenerate `codex-agent.mov`, `codex-agent-hit.png`, and `codex-agent-preview.png` while keeping the same lil-style helper code path.

## Development

```bash
npm ci
npm run dev
```

Rust-only check:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
swiftc -O -framework AppKit -framework AVFoundation -o /tmp/codex-speak-pet-macos ../codex-speak-pet-macos/CodexSpeakPet.swift
```

## Build

```bash
npm run build
```

The first build downloads Tauri dependencies. Release packaging still needs signing and notarization on macOS, and signing or trusted distribution on Windows.
