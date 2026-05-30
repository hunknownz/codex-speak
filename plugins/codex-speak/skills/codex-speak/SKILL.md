---
name: codex-speak
description: Use when Codex should prepare speech-friendly replies for children or beginners through Codex Speak. Prefer the plugin side-channel when available; otherwise include the visible Codex Speak Protocol block.
metadata:
  short-description: Prepare Codex Speak narration
---

# Codex Speak

When this skill is active, make the final answer useful both for reading and for listening. The goal is not to read the whole answer aloud. The goal is to produce a natural spoken guide that helps a child or beginner keep using Codex.

## Preferred Path

If the `codex_speak_prepare` tool is available, call it near the end of the task with 3 to 5 Chinese guide items:

- `did`: what Codex just did.
- `code-summary` or `command-summary`: what code, commands, setup, or checks accomplished.
- `result`: whether the work passed or what changed.
- `next`: what the user can naturally ask next.
- `warning`: only when something is incomplete or risky.

The tool writes a local side-channel file. The hook can read that file after the reply completes, so the spoken guide does not need to be fully shown in the chat.

## Fallback Path

If the tool is not available, include a Codex Speak Protocol block:

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你改了朗读助手的规则。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：让程序先找到适合朗读的导览，而不是直接朗读整段技术回答。</p>
  <p class="codex-speak-result" data-role="result">我运行了测试，结果通过了。</p>
  <p class="codex-speak-next" data-role="next">接下来，你可以让我继续把这个协议接进插件通道。</p>
</aside>
```

This block should feel like a normal visible part of the answer, not hidden metadata. It is what the speech hook reads when the side-channel is unavailable.

## Optional Folded Display

If a visible guide would be too distracting, it may be wrapped in native HTML `details`:

```html
<details class="codex-speak-fold">
  <summary>朗读导览</summary>
  <aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
    <p class="codex-speak-did" data-role="did">我刚才帮你整理了方案。</p>
    <p class="codex-speak-next" data-role="next">下一步可以继续做插件控制面板。</p>
  </aside>
</details>
```

This is progressive enhancement. Some Codex renderers may show it folded, while others may show the raw or normal content. Do not rely on folding for correctness.

## Writing Rules

- Use Chinese first. Keep English technical words only when necessary.
- Keep it around 120 to 300 Chinese characters by default. Use up to 500 only for complex work.
- Do not read code verbatim. Explain what the code does and what problem it solves.
- Do not read raw shell commands. Explain the action, such as "我运行了测试" or "我安装了本地语音模型".
- Do not read long absolute paths. Say "项目里的配置文件" or a short filename only when useful.
- Do not read logs, stack traces, diffs, tables, URLs, or long hashes.
- Mention tests or verification in plain language, for example "我跑了测试，结果通过了".
- Avoid baby talk. Use a clear, warm older-sister explaining tone.

For very short conversational answers, you may skip the guide if the whole answer is already natural to hear.
