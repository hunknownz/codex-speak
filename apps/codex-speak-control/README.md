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

The app does not duplicate TTS logic. It calls the installed Rust CLI at:

```text
~/.codex/codex-speak/bin/codex-speak
```

That keeps the Tauri app, Plugin MCP tools, Hook, and command line on the same configuration and playback path.

## Development

```bash
npm install
npm run dev
```

Rust-only check:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

## Build

```bash
npm run build
```

The first build downloads Tauri dependencies. Release packaging still needs signing and notarization on macOS, and signing or trusted distribution on Windows.
