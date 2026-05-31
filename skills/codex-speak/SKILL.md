---
name: codex-speak
description: Use when Codex replies should be friendly to speech playback for children or beginners. Prefer the Codex Speak MCP side-channel when available, so the spoken guide does not need to appear in the chat.
metadata:
  short-description: Make Codex replies speech-friendly
---

# Codex Speak

When this skill is active, make the final answer useful both for reading and for listening. The goal is not to read the whole answer aloud. The goal is to produce a natural spoken guide that helps a child or beginner keep using Codex.

## Preferred Path: MCP Side-Channel

If the `codex_speak_prepare` tool is available, call it near the end of the task with 3 to 5 Chinese guide items.

Use these roles:

- `did`: what Codex just did.
- `code-summary`: what code changed or what problem the code solved.
- `command-summary`: what commands, installation, or verification steps did.
- `result`: whether the work passed or what changed.
- `next`: what the user can naturally ask next.
- `warning`: only when something is incomplete or risky.

The tool writes `~/.codex/codex-speak/spool/latest.json`. The Hook reads and consumes that file after the reply completes, so the full spoken guide does not need to be shown in the chat.

When the side-channel succeeds:

- Keep the final answer natural and concise.
- Do not include the full HTML protocol block.
- Mention only the important visible result for the user.

## Fallback Path: Folded HTML Protocol

If `codex_speak_prepare` is not available, include a folded Codex Speak Protocol block at the end of substantial implementation, debugging, setup, research, or multi-step answers:

```html
<details class="codex-speak-fold">
  <summary>朗读导览</summary>
  <aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
    <p class="codex-speak-did" data-role="did">我刚才帮你改了朗读助手的规则。</p>
    <p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：让程序先找到适合朗读的导览，而不是直接朗读整段技术回答。</p>
    <p class="codex-speak-result" data-role="result">我运行了测试，结果通过了。</p>
    <p class="codex-speak-next" data-role="next">接下来，你可以让我继续把这个协议接进插件通道。</p>
  </aside>
</details>
```

The folded HTML block is only a fallback. Some Codex renderers may fold it, while others may show it as normal content. Do not rely on folding for correctness.

For very short conversational answers, skip the guide if the whole answer is already natural to hear.

## Writing Rules

- Use Chinese first. Keep English technical words only when necessary.
- Keep it around 120 to 300 Chinese characters by default. Use up to 500 only for complex work.
- Do not read code verbatim. Explain what the code does and what problem it solves.
- Do not read raw shell commands. Explain the action, such as "我运行了测试" or "我安装了本地语音模型".
- Do not read long absolute paths. Say "项目里的配置文件" or a short filename only when useful.
- Do not read logs, stack traces, diffs, tables, URLs, or long hashes.
- Mention tests or verification in plain language, for example "我跑了测试，结果通过了".
- Avoid baby talk. Use a clear, warm older-sister explaining tone.

## Bad Patterns

- Do not add a long hidden HTML comment for speech.
- Do not make the spoken guide a tiny one-line summary that loses what Codex actually did.
- Do not repeat the full final answer.
- Do not read code, commands, file paths, or logs verbatim.
