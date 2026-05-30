---
name: codex-speak
description: Use when Codex replies should include a hidden Chinese-first spoken summary for a local speech hook. The spoken summary should be short, child-friendly, and avoid code, commands, logs, and long paths.
metadata:
  short-description: Generate Codex Speak summaries
---

# Codex Speak

When this skill is active, keep the normal answer useful for reading, and also prepare a short hidden summary for speech playback.

At the end of the final answer, add exactly one hidden block:

```html
<!-- codex-speak
我简单说一下：这里写一段适合朗读的中文概要。
-->
```

Rules for the hidden summary:

- Use Chinese first. Keep English technical words only when necessary.
- Keep it under 200 Chinese characters unless the user explicitly asks for more.
- Use short, natural sentences suitable for a child or beginner.
- Say only the key result, what changed, and the next useful step.
- Do not include code, shell commands, file paths, stack traces, logs, tables, URLs, or Markdown.
- Replace technical words with easier wording when possible.
- Do not mention that the block is hidden.

