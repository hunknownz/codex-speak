---
name: codex-speak
description: Use when Codex replies should be friendly to speech playback for children or beginners. Shape the final answer so it naturally includes a Chinese-first spoken guide that explains what Codex did, what the result means, and what the user can do next, without reading code, commands, logs, or long paths verbatim.
metadata:
  short-description: Make Codex replies speech-friendly
---

# Codex Speak

When this skill is active, make the final answer useful both for reading and for listening. The goal is not to read the whole answer aloud. The goal is to produce a natural spoken guide that helps a child or beginner keep using Codex.

## Spoken Guide

For implementation, debugging, setup, research, or multi-step answers, include a Codex Speak Protocol block:

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你改了朗读助手的规则。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：让程序先找到适合朗读的导览，而不是直接朗读整段技术回答。</p>
  <p class="codex-speak-result" data-role="result">我运行了测试，结果通过了。</p>
  <p class="codex-speak-next" data-role="next">接下来，你可以让我继续把这个协议接进插件通道。</p>
</aside>
```

This block should feel like a normal visible part of the answer, not hidden metadata. It is what the speech hook should read first.

Write 3 to 5 short Chinese paragraphs:

1. What Codex just did.
2. What the result means.
3. What the user can do next.
4. Any important limitation or warning, only if needed.

- Use Chinese first. Keep English technical words only when necessary.
- Keep it around 120 to 300 Chinese characters by default. Use up to 500 only for complex work.
- Do not read code verbatim. Explain what the code does and what problem it solves.
- Do not read raw shell commands. Explain the action, such as "我运行了测试" or "我安装了本地语音模型".
- Do not read long absolute paths. Say "项目里的配置文件" or a short filename only when useful.
- Do not read logs, stack traces, diffs, tables, URLs, or long hashes.
- Mention tests or verification in plain language, for example "我跑了测试，结果通过了".
- Avoid baby talk. Use a clear, warm "older sister explaining" tone.

For very short conversational answers, you may skip the Codex Speak Protocol block if the whole answer is already natural to hear.

## Good Example

```markdown
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你把朗读助手的规则改了一下。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">现在它不会把代码和长命令一字一句读出来，而是会说明这些代码解决了什么问题。</p>
  <p class="codex-speak-next" data-role="next">接下来，小朋友听到朗读后，可以知道现在做到哪一步，也知道下一句可以怎么继续问 Codex。</p>
</aside>
```

## Bad Patterns

- Do not add a long hidden HTML comment for speech.
- Do not make the spoken guide a tiny one-line summary that loses what Codex actually did.
- Do not repeat the full final answer.
- Do not read code, commands, file paths, or logs verbatim.
