# Codex Speak Plugin

Codex Speak Plugin adds Codex-facing controls and tools on top of the Rust CLI.

## What It Can Do

- Expose a `codex-speak` skill for speech-friendly final answers.
- Provide MCP tools for status, preview, stop, test speech, background progress speech, enable/disable, child mode, speed, voice profiles, TTS provider switching, model installation, and side-channel preparation.
- Let Codex write a spoken guide to `~/.codex/codex-speak/spool/latest.json` so the hook can read that guide without relying only on visible chat text.
- Let Codex say a short non-blocking progress prompt with `codex_speak_speak_text` and `background: true` during long tasks.
- Keep progress prompts and final guides separate: progress speech is sparse and non-authoritative; the final guide is prepared through `codex_speak_prepare` and played after the Codex reply completes.

## What It Cannot Reliably Do Yet

The current Codex plugin surface does not provide a documented API for rewriting or hiding already-rendered chat messages. Codex Speak therefore avoids putting custom protocol content into normal chat messages:

- Preferred: `codex_speak_prepare` side-channel, so the full spoken guide does not have to appear in chat.
- Compatibility: the Rust CLI can still parse old HTML or Markdown fallback blocks from previous sessions and QA fixtures.
- If MCP is unavailable, the Skill keeps the visible reply natural and speech-friendly, and the Hook cleans the final reply as a last resort.
- The plugin does not stream every generated chat token into TTS. Continuous token-level speech is not the default product behavior because it can read unfinished, uncleaned, or later-corrected content aloud.

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
- `codex_speak_install_model`
- `codex_speak_list_pronunciation`
- `codex_speak_set_pronunciation`
- `codex_speak_remove_pronunciation`

The plugin expects the Rust CLI to be installed at `~/.codex/codex-speak/bin/codex-speak` or available on `PATH`.

## Installation

The product installer copies this plugin into the user's personal plugin marketplace:

```text
~/plugins/codex-speak
~/.agents/plugins/marketplace.json
```

The marketplace entry uses a local source path:

```text
./plugins/codex-speak
```

During product installation, `.mcp.json` is generated for the current platform and points directly at the installed Rust CLI in `~/.codex/codex-speak/bin`. The wrapper scripts stay in the plugin for debugging and compatibility.

That keeps Plugin, Skill, MCP config, Hook, and the installed Rust CLI on the same local version.
