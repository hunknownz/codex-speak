# Codex Speak Plugin

Codex Speak Plugin adds Codex-facing controls and tools on top of the Rust CLI.

## What It Can Do

- Expose a `codex-speak` skill for speech-friendly final answers.
- Provide MCP tools for status, preview, stop, test speech, enable/disable, child mode, speed, voice profiles, TTS provider switching, and side-channel preparation.
- Let Codex write a spoken guide to `~/.codex/codex-speak/spool/latest.json` so the hook can read that guide without relying only on visible chat text.

## What It Cannot Reliably Do Yet

The current Codex plugin surface does not provide a documented API for rewriting or hiding already-rendered chat messages. For hiding or folding protocol content, use these two approaches:

- Preferred: `codex_speak_prepare` side-channel, so the full spoken guide does not have to appear in chat.
- Progressive enhancement: wrap the visible protocol block in native HTML `details`, which may render as folded when the Codex renderer allows it.

## MCP Tools

- `codex_speak_status`
- `codex_speak_prepare`
- `codex_speak_extract`
- `codex_speak_speak_text`
- `codex_speak_stop`
- `codex_speak_set_enabled`
- `codex_speak_update_config`
- `codex_speak_set_child_mode`
- `codex_speak_set_speed`
- `codex_speak_set_voice_profile`
- `codex_speak_set_provider`

The plugin expects the Rust CLI to be installed at `~/.codex/codex-speak/bin/codex-speak` or available on `PATH`.
