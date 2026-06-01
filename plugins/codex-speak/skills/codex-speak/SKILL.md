---
name: codex-speak
description: Use when Codex should prepare speech-friendly replies for children or beginners through Codex Speak. Prefer the plugin side-channel when available; otherwise include the folded visible Codex Speak Protocol block.
metadata:
  short-description: Prepare Codex Speak narration
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

For long-running tasks, you may also call `codex_speak_speak_text` with `background: true` for a short one-sentence progress prompt at a natural milestone. Use it sparingly: it is for "I am running the tests now" or "the build passed and I am checking packaging", not for reading every intermediate thought. Keep the final guide in `codex_speak_prepare`.

When the side-channel succeeds:

- Keep the final answer natural and concise.
- Do not include the full HTML protocol block.
- Mention only the important visible result for the user.

## Configuration Tools

When the user asks to control speech settings in natural language, use the available MCP tools:

- Use `codex_speak_set_child_mode` for "打开儿童模式" or "关闭儿童模式".
- Use `codex_speak_set_speed` for "慢一点读" or "快一点读".
- Use `codex_speak_set_voice_profile` for "声音清楚明亮", "慢一点更清楚", or "快速预览".
- Use `codex_speak_set_provider` for "换一个朗读引擎", "试试 Kokoro", "切到 Piper", or "用系统语音".
- Use `codex_speak_install_model` for "安装这个声音", "下载 Kokoro 模型", or "把当前朗读引擎补全".
- Use `codex_speak_update_config` when several settings should change together.
- Use `codex_speak_stop` when the user asks to stop speech.
- Use `codex_speak_speak_text` when the user asks to try or preview a voice. For task progress speech, set `background: true` so Codex can keep working immediately.

Available providers are `sherpa_melo`, `sherpa_kokoro`, `sherpa_zipvoice`, `piper`, and `system`. Extra local models may be required for everything except `system` and the default installed MeloTTS path.

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
- Rewrite common English abbreviations into Chinese explanations when they are meant for speech, for example `MCP` as "插件通道", `JSON` as "数据格式", `CLI` as "命令行工具", and `API` as "接口".
- Rewrite file names, command flags, and code identifiers into their purpose when they are meant for speech, for example `README.md` as "说明文件", `--provider sherpa_melo` as "选择默认中文朗读引擎的命令参数", and `codex_speak_prepare` as "准备朗读导览的插件工具".
- If an uppercase English acronym is not important to the next action, explain the meaning or say "一个英文缩写" instead of leaving it for the TTS voice to spell letter by letter.
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
