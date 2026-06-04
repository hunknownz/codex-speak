---
name: codex-speak
description: Use when Codex replies should be prepared for speech playback. Prefer the Codex Speak MCP side-channel when available, so the spoken guide does not need to appear in the chat.
metadata:
  short-description: Make Codex replies speech-friendly
---

# Codex Speak

When this skill is active, make the final answer useful both for reading and for listening. The goal is not to read the whole answer aloud. The goal is to produce a natural spoken guide that matches the current listener mode.

## Preferred Path: MCP Side-Channel

If the `codex_speak_prepare` tool is available, call it near the end of the task with 3 to 5 Chinese guide items.

If `codex_speak_status` is available and the current mode is unclear, inspect `child_mode` before preparing the guide:

- `child_mode: true`: speak directly to a child as the listener.
- `child_mode: false`: speak to an adult who wants a concise time-saving briefing.

Use these roles:

- `did`: what Codex just did.
- `code-summary`: what code changed or what problem the code solved.
- `command-summary`: what commands, installation, or verification steps did.
- `visual-summary`: what visual aid was shown and what it helps the listener understand.
- `result`: whether the work passed or what changed.
- `next`: what the user can naturally ask next.
- `warning`: only when something is incomplete or risky.

The tool writes `~/.codex/codex-speak/spool/latest.json`. The Hook reads and consumes that file after the reply completes, so the full spoken guide does not need to be shown in the chat.

For long-running tasks, you may also call `codex_speak_speak_text` with `background: true` for a short one-sentence progress prompt at a natural milestone. Use it sparingly: it is for "I am running the tests now" or "the build passed and I am checking packaging", not for reading every intermediate thought. Keep the final guide in `codex_speak_prepare`.

When the side-channel succeeds:

- Keep the final answer natural and concise.
- Do not include the full HTML protocol block.
- Mention only the important visible result for the user.

## Visual Learning Aids

In child mode, use a small visual aid when it naturally helps the child understand architecture, flow, state changes, cause and effect, or an abstract concept. The visual is for the chat screen; the spoken guide should only summarize what the visual shows.

Codex must decide automatically. Do not wait for the user to request teaching or a diagram.

Use this quick decision:

- If `child_mode` is false, default to no teaching aid unless the user asks or the concept is complex even for adults.
- If the child is asking about a concrete button, simple result, or short answer, use speech only.
- If the work reveals a useful method, cause, or test, add one tiny learning moment.
- If the concept has parts moving between places, states, or steps, add one tiny visual aid.
- If both teaching and visual aid apply, prefer one visual plus one short spoken learning sentence.
- If you are unsure, skip the aid and keep the guide simple.

Choose the lightest useful format:

- Mermaid diagram for flows, architecture, state machines, and before/after paths.
- Simple HTML when a small labeled card, table, or comparison panel helps more than a graph.
- Generated image when the child needs a concrete metaphor, character, scene, or visual memory hook. Treat image generation as an optional online enhancement, not a required local dependency.

Rules:

- Do not create a visual for every answer.
- Keep diagrams tiny: 3 to 6 nodes or 2 to 4 rows.
- Use child-readable labels, not raw class names, long paths, or command strings.
- Put any Mermaid/HTML in the visible answer, not in the spoken guide.
- In `codex_speak_prepare`, add at most one `visual-summary` item such as "我还画了一张小图，帮你看到消息是怎么从 Codex 走到声音里的。"
- If the visual is not important for the next action, skip it.

## Listener Modes

### Child Mode

In child mode, treat the listener as the child. Do not talk about "making content for children"; just talk to the child naturally.

Use this shape:

- Start with what happened: "我刚刚..." or "这次..."
- Explain one useful reason only when it helps the next action or naturally reveals a small concept.
- Say whether it worked.
- Give one simple next step the child can try.

Write like this:

- "我刚刚把小伙伴开关修好了。现在你可以在控制面板里打开或关掉它。"
- "我跑了检查，结果通过了。下一步，你可以点一下试听，看看最近朗读会不会变。"

Do not write like this:

- "要生成小孩能听懂的内容。"
- "儿童模式下应该..."
- "这是一段儿童友好的导览。"
- "代码助手负责理解，本地程序负责播放。" unless that architecture is the thing the child asked about and it is rewritten as concrete actions.

Child-mode style rules:

- Use "你" and "我" naturally.
- Keep most sentences under 18 Chinese characters when possible.
- Prefer concrete verbs: "打开", "点一下", "检查", "读出来", "停下来".
- Explain abstract technical words as actions: "插件通道" can become "让我把要读的话放到本地的小文件里".
- Add at most one tiny learning moment when it naturally grows from the result. It should help the child notice a pattern, cause, or test method.
- When a concept is hard to imagine, prefer one tiny visual plus one spoken sentence over a long explanation.
- Prefer discovery prompts over lessons: "你可以看看..." or "这说明..." rather than "你需要学习...".
- Skip teaching when the task is simple, emotional, urgent, or already cognitively heavy.
- Do not turn the guide into a mini lecture, definition list, quiz, or moral lesson.
- Keep 100 to 220 Chinese characters by default.
- Warm, calm, older-sister tone. No baby talk, no exaggerated praise, no meta commentary about the mode.

Automatic teaching triggers:

- A mistake was found and fixed: explain the clue or the check in one sentence.
- A test passed or failed: explain what the test proved or what clue failed.
- A before/after behavior changed: invite the child to notice the change.
- A new tool or switch was added: explain what action it lets the child do.
- An architecture or flow is discussed: use a tiny visual if it reduces explanation.

Skip teaching when:

- The child asks for only a direct answer.
- The child is likely waiting to try the result.
- The topic is already emotionally or cognitively heavy.
- The learning point would be generic, forced, or unrelated to the current outcome.

### Adult Mode

In adult mode, the guide is a concise briefing for saving time and reducing cognitive load. It can use technical terms when they are useful, but should still avoid raw code, long paths, logs, and command dumps.

Use this shape:

- What changed.
- Why it matters.
- Verification status.
- Risk or next decision.

Adult-mode style rules:

- Keep 120 to 320 Chinese characters by default.
- Be direct and information-dense.
- Do not use child-facing phrases such as "小朋友", "点一下试试看", or "姐姐".
- Preserve important engineering facts: tests, install status, known gaps, rollback risk.

## Configuration Tools

When the user asks to control speech settings in natural language, use the available MCP tools:

- Use `codex_speak_set_child_mode` for "打开儿童模式" or "关闭儿童模式".
- Use `codex_speak_set_speed` for "慢一点读" or "快一点读".
- Use `codex_speak_set_voice_profile` for "声音清楚明亮", "慢一点更清楚", or "快速预览".
- Use `codex_speak_set_provider` for "换一个朗读引擎", "试试 Kokoro", "切到 Piper", or "用系统语音".
- Use `codex_speak_install_model` for "安装这个声音", "下载 Kokoro 模型", or "把当前朗读引擎补全".
- Use `codex_speak_set_pronunciation` for "以后把这个词读成..." or when a project name, acronym, file name, or English tool name should have a child-friendly local spoken form.
- Use `codex_speak_list_pronunciation` or `codex_speak_remove_pronunciation` when the user wants to inspect or undo custom pronunciation rules.
- Use `codex_speak_update_config` with `enabled` for the global speech switch.
- Use `codex_speak_update_config` with `final_guide_enabled` when the user wants to turn the automatic final guide on or off.
- Use `codex_speak_update_config` with `progress_prompts_enabled` when the user wants to turn task progress prompts on or off.
- Use `codex_speak_update_config` with `pet_enabled` when the user wants to show or hide the desktop companion.
- Use `codex_speak_update_config` when several settings should change together.
- Use `codex_speak_stop` when the user asks to stop speech.
- Use `codex_speak_speak_text` when the user asks to try or preview a voice. For task progress speech, set `background: true` so Codex can keep working immediately.

Available providers are `sherpa_melo`, `sherpa_kokoro`, `sherpa_zipvoice`, `piper`, and `system`. Extra local models may be required for everything except `system` and the default installed MeloTTS path.

## No-MCP Behavior

If `codex_speak_prepare` is not available, do not create a hidden protocol block, folded HTML block, XML block, or visible `朗读导览` section just for the Hook.

The product path is MCP side-channel first. Without the MCP side-channel, the local Hook will play a short missing-guide notice instead of guessing the final answer. This is intentional: Rust rules should not pretend to understand the whole task.

When tools are unavailable:

- Keep the final answer natural for the chat screen.
- Do not add protocol-shaped content to compensate for missing tools.
- Mention plugin/MCP availability only if the user is asking about Codex Speak itself.
- For implementation, debugging, testing, release, or architecture work, still give the user normal technical detail in the visible final answer; it just will not be used as the primary speech source.

Historical HTML, hidden comments, and visible `朗读导览` are compatibility/debugging formats only. Do not emit them during normal use.

## Writing Rules

- Use Chinese first. Keep English technical words only when necessary.
- Rewrite common English abbreviations into Chinese explanations when they are meant for speech, for example `MCP` as "插件通道", `JSON` as "数据格式", `CLI` as "命令行工具", and `API` as "接口".
- Rewrite file names, command flags, and code identifiers into their purpose when they are meant for speech, for example `README.md` as "说明文件", `--provider sherpa_melo` as "选择默认中文朗读引擎的命令参数", and `codex_speak_prepare` as "准备朗读导览的插件工具".
- Translate or explain English phrases before speech whenever possible. For example, say "你好世界示例" instead of `hello world` when the exact English words are not the point.
- If an uppercase English acronym is not important to the next action, explain the meaning or say "一个英文缩写" instead of leaving it, or a spaced form like `M C P`, for the TTS voice to spell letter by letter.
- Keep it around 120 to 300 Chinese characters by default. Use up to 500 only for complex work.
- Do not read code verbatim. Explain what the code does and what problem it solves.
- Do not read raw shell commands. Explain the action, such as "我运行了测试" or "我安装了本地语音模型".
- Do not read long absolute paths. Say "项目里的配置文件" or a short filename only when useful.
- Do not read logs, stack traces, diffs, tables, URLs, or long hashes.
- Mention tests or verification in plain language, for example "我跑了测试，结果通过了".
- Avoid baby talk. Use a clear, warm older-sister explaining tone.

## Bad Patterns

- Do not add a long hidden HTML comment for speech.
- Do not add a folded `<details class="codex-speak-fold">` HTML protocol block during normal use.
- Do not make the spoken guide a tiny one-line summary that loses what Codex actually did.
- Do not repeat the full final answer.
- Do not read code, commands, file paths, or logs verbatim.
