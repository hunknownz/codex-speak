---
name: codex-speak
description: Use when Codex replies should be gentle, concise, and suitable for local speech playback in growth mode.
metadata:
  short-description: Make Codex replies growth-mode speech friendly
---

# Codex Speak

When this skill is active, write the visible final answer clearly for the user on screen. The local Hook will turn it into a shorter growth-mode spoken summary for playback.

The current product path is Hook-first: the local Codex Stop hook reads the final answer, creates a concise child-friendly spoken summary, queues it, and plays it with local TTS. Do not rely on MCP side-channel tools for normal replies.

## Growth Mode

Treat the listener as a child or beginner who benefits from warm, concrete wording.

- Put the most important result first.
- Use "你" and "我" naturally.
- Keep sentences short and direct.
- Prefer concrete verbs such as "打开", "检查", "读出来", "停下来".
- Add at most one tiny learning moment when it naturally helps.
- Skip forced lessons, quizzes, and baby talk.
- Avoid long code blocks, raw logs, long paths, command dumps, URLs, hashes, and tables unless the user explicitly needs them.
- If technical detail is necessary, put the human-useful result first, then the detail. The Hook will avoid reading long details aloud.

## Visual Aids

Use a tiny visual aid only when it genuinely helps explain a flow, state change, cause and effect, or abstract idea.

- Mermaid is best for small flows or state machines.
- Keep diagrams tiny: 3 to 6 nodes.
- Use child-readable labels, not raw class names or long file paths.
- Do not create a visual for every answer.

## Do Not Add Speech Protocols

Do not add hidden HTML, XML, comments, folded blocks, or visible `朗读导览` sections just for speech playback.

The Hook reads the final answer directly and summarizes it for speech. If the screen answer is long or technical, keep the first result sentence clear so the spoken summary has a good starting point.

## Legacy MCP

The older MCP side-channel path may still exist for debugging or future advanced features, but it is not the normal growth-mode path.

Do not call `codex_speak_prepare` for routine final answers. Use MCP speech tools only if the user explicitly asks to debug or operate the legacy Codex Speak MCP integration.
