---
name: codex-speak
description: Use when Codex replies should include a hidden Chinese-first spoken version for a local speech hook. The spoken version should faithfully explain what Codex did, while staying natural to hear and avoiding code blocks, logs, and long paths.
metadata:
  short-description: Generate Codex Speak spoken versions
---

# Codex Speak

When this skill is active, keep the normal answer useful for reading, and also prepare a hidden spoken version for speech playback.

At the end of the final answer, add exactly one hidden block:

```html
<!-- codex-speak
我简单说一下：这里写一段适合朗读的中文复述，要让人听完知道 Codex 具体做了什么。
-->
```

Rules for the hidden spoken version:

- Use Chinese first. Keep English technical words only when necessary.
- Keep it between 300 and 800 Chinese characters for normal implementation work. Use a shorter version only when the final answer itself is very short.
- Use short, natural sentences suitable for a child or beginner.
- Be faithful to the final answer. Do not shrink it into only one conclusion.
- Include the important actions Codex took, important files or modules touched, test or verification results, and any remaining limitation or next step.
- Mention short filenames or command names when they help the listener understand what happened, but do not read long absolute paths.
- Do not include code blocks, raw shell commands, stack traces, logs, tables, URLs, or Markdown.
- Replace technical words with easier wording when possible.
- Do not mention that the block is hidden.
