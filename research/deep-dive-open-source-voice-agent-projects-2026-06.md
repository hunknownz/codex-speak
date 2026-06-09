# 开源项目深读：Coding Agent 语音层

调研日期：2026-06-05  
调研范围：已拉取到 `_research/competitors/` 的开源参考项目。  
调研目标：判断哪些机制可复用到 Codex Speak，哪些方向应避免追随，哪些能力需要提前留接口。

## 本轮结论

这一轮源码阅读后，可以把市场拆成两层：

1. 语音工程层：TTS/STT provider、队列、播放中断、音色、VAD、远程播放、调试音频。
2. Agent 语义层：状态检测、事件归一化、最终摘要、权限请求、session 身份、多 agent 通知。

Codex Speak 最该守住第二层。第一层是体验底座，必须稳定，但不是壁垒。Aftertone、Herdr、Untether、Paseo 都从不同角度证明：用户真正需要的不是“把所有输出念出来”，而是“在合适时机听到合适粒度的状态和结论”。

对当前项目的最重要建议：

- 继续坚持 side-channel/final guide，不要回退到朗读原始回复。
- 默认安静：没有专门语音稿时不应猜测读什么。
- 抽象一个事件摘要层，未来可同时服务 UI、Pet、notification、TTS。
- 语音播报优先级应来自 agent 状态转移，而不是来自日志行。
- 远程和手机场景先不急做，但 `session_id`、`agent_label`、`priority`、`seen/heard` 这些字段应尽早进入内部模型。

## 固定版本

| 项目 | 本地路径 | 本轮读取 commit |
| --- | --- | --- |
| Sled | `_research/competitors/sled` | `f5a3746627e9de3b1b796e7e4c5a98bcd1de10ad` |
| Aftertone | `_research/competitors/aftertone` | `d886cd76cc662ab26a12c6837da18c7fe95fe047` |
| Herdr | `_research/competitors/herdr` | `acfbb7c85a44d28432512772e4070428df60cfd6` |
| AgentVibes | `_research/competitors/agentvibes` | `d082b1371d9709c016f91fd4514bb297bcac5b34` |
| VoiceMode | `_research/competitors/voicemode` | `e1564f2476c0404ce06d7a226c6ff941ae3916db` |
| Untether | `_research/competitors/untether` | `4009531622df7d77217d0e12ccde6751d90454e7` |
| Happy | `_research/competitors/happy` | `e10e51979143f4ffa9ead16c210af72c9864ac80` |
| Paseo | `_research/competitors/paseo` | `f30f2170232e442cdfdcd9de7fd3fdf9095fb7e9` |
| Heard | `_research/competitors/heard` | `15436fafc019eb13780a12945f622ab381cd3cc4` |
| V1GPT | `_research/competitors/v1gpt` | `6fd07bf23cf0e60b65b16a01caed5a82657791e5` |
| Codex CLI Hooks | `_research/competitors/codex-cli-hooks` | `854e02abee6e2d9745b74f8456a5368416b0fb96` |
| VoiceTerm | `_research/competitors/voiceterm` | `d5d7668a839b2f1dc9d4d18eafc253dbee80bd4c` |
| TalkToCursor | `_research/competitors/talk-to-cursor` | `c1810de8066922f16f02e484cf349b28b7f1fda0` |

## 逐项目深读

### Aftertone

相关度：最高。

读过的关键文件：

- `_research/competitors/aftertone/docs/adapters/codex.md`
- `_research/competitors/aftertone/.cursor/rules/spoken-summary.mdc`
- `_research/competitors/aftertone/py/aftertone/prepare.py`
- `_research/competitors/aftertone/py/aftertone/extract.py`
- `_research/competitors/aftertone/py/aftertone/hook_run.py`
- `_research/competitors/aftertone/py/aftertone/sessions.py`
- `_research/competitors/aftertone/py/aftertone/spoken_tag.py`
- `_research/competitors/aftertone/py/tts_daemon.py`

核心机制：

- Codex 结束播报依赖 `~/.codex/hooks.json` 的 `Stop` hook，而不是依赖 MCP。原因很现实：Agent 可能不调用 MCP，但 hook 会在回复结束时触发。
- 默认 `summary_mode = "tag_only"`，并且 `only_speak_spoken_summary = true`。没有 `<spoken_summary>` 就输出空 JSON，不朗读。
- Codex hook 会读取 `last_assistant_message` 或 `transcript_path`。对于 transcript flush 的时序问题，`prepare.py` 有短延迟重试。
- `spoken_tag.py` 取最后一个闭合的 `</spoken_summary>`，并回找最近的 open tag，避免把中途提到的标签样例误读。
- `sessions.py` 支持 per-session allowlist/denylist/all。它用 pending 文件解决“打开语音时还不知道当前 session id”的问题：下一次 hook 触发时注册当前 session。
- `tts_daemon.py` 常驻加载模型，提供 `/say`，支持队列/打断模式，并记录 latency。

对 Codex Speak 的启发：

- 我们当前的 side-channel 比 tag 更结构化，应继续作为主路径。
- `missing_guide_policy` 默认保持安静是正确方向。
- 如果遇到 Codex side-channel/hook 时序问题，应加入短 retry，而不是读原始 assistant 文本兜底。
- Per-session 开关值得做，尤其适合用户只想让当前任务说话。
- 文案规范可借鉴：语音稿只说状态、意义、下一步，不读路径、代码、Markdown。

### Sled

相关度：高，偏 remote/phone/ACP。

读过的关键文件：

- `_research/competitors/sled/server-client/src/acp-ws-proxy.ts`
- `_research/competitors/sled/app/src/agentProtocolSession.ts`
- `_research/competitors/sled/app/src/agentBridge.ts`
- `_research/competitors/sled/app/src/sentenceDetector.ts`
- `_research/competitors/sled/app/src/durableObject.ts`

核心机制：

- Sled 不是 hook 播放器，而是本地 Web/手机 remote wrapper。它启动 `claude-code-acp`、`codex-acp` 或 Gemini experimental ACP，再通过 JSON-RPC/WebSocket 代理。
- ACP session 显式处理 `initialize`、`session/new`、`session/prompt`、`session/cancel`、`session/request_permission`。
- Durable Object 维护历史和 TTS heard marker，避免重连后重复播报。
- 流式 TTS 需要 sentence detector。代码要处理 code fence、URL、路径、小数点、缩写，这说明“流式读 assistant 文本”复杂且脆弱。
- 语音开启后，音频和 agent 对话会进 Layercode API。它把代码/历史本地化和语音云服务之间的隐私边界写得比较清楚。

对 Codex Speak 的启发：

- ACP 能拿到更丰富的权限/状态，但会把产品从轻量 hook/MCP 拉到 wrapper 平台，短期不应切换。
- 如果未来做手机模式，需要 `heard`/`seen` 标记，避免重复通知。
- 我们用最终导览而非流式断句，是更稳的中文体验路径。

### Herdr

相关度：高，偏状态/通知。

读过的关键文件：

- `_research/competitors/herdr/src/detect/mod.rs`
- `_research/competitors/herdr/src/detect/agents/codex.rs`
- `_research/competitors/herdr/src/integration/assets/codex/herdr-agent-state.sh`
- `_research/competitors/herdr/src/app/agents.rs`
- `_research/competitors/herdr/src/server/notifications.rs`
- `_research/competitors/herdr/src/sound.rs`

核心机制：

- 公开状态是 `idle`、`working`、`blocked`、`done`、`unknown`。
- `done` 不是普通 idle，而是“完成了但用户还没看过”。用户看过 pane 后才降为 idle。
- Codex 状态检测非常克制：强阻塞提示优先，其次 working，再检测 prompt idle，弱阻塞只在没有 idle prompt 时生效。
- transcript viewer 会被跳过，避免用户翻历史时误判当前 agent 状态。
- Codex hook 只上报 session identity，不再用 hook 上报状态。原因是旧 hook 状态容易过期，屏幕状态和最终 hook 更可靠。
- 通知只在有意义的状态转换时触发，声音分 Done 和 Request。

对 Codex Speak 的启发：

- 事件级播报应引入 `done-but-unseen`，否则完成通知会重复或消失得太快。
- 状态播报只在 transition 上发声，不按轮询结果发声。
- 可借鉴状态集合：`working`、`blocked`、`need_approval`、`done`、`failed`、`idle`。
- 如果未来做屏幕/terminal 状态检测，要特别防 stale state。

### Heard

相关度：最高，当前最接近 Agent Narrator。

读过的关键文件：

- `_research/competitors/heard/README.md`
- `_research/competitors/heard/heard/adapters/codex.py`
- `_research/competitors/heard/heard/verbosity.py`
- `_research/competitors/heard/heard/spoken.py`
- `_research/competitors/heard/heard/agent_state.py`
- `_research/competitors/heard/heard/multi_agent.py`
- `_research/competitors/heard/heard/tts/managed.py`
- `_research/competitors/heard/heard/hook.py`

核心机制：

- README 的核心定位是“agent 有声音”，但强调不是 transcription，而是 narrator：根据上下文、工具、最近活动和决策点判断什么该说。
- 有 co-pilot 和 companion 两种模式：前者适合用户看着屏幕，短提示即可；后者适合离屏，给更完整 brief，并用一句话把用户带回行动。
- Codex adapter 写 `~/.codex/hooks.json`，接 `Stop`、`PreToolUse`、`PostToolUse`。hook dispatcher 同时兼容 Claude Code 的更多事件。
- `verbosity.py` 把事件分类为 `speak`、`drop`、`digest`。失败、权限、长任务可穿透安静档；routine 工具事件会被丢弃或攒成 digest。
- `spoken.py` 用 per-session hash 和 transcript byte offset 避免重播旧内容。第一次遇到 transcript 会从 EOF 开始，避免安装后把历史全念出来。
- `agent_state.py` 是明确的 Layer 2 Agent State：只记录事实和 hints，不让 LLM 在状态层自由发挥。字段包括 repo/cwd、current tool、touched files、error count、response shape、salience。
- `multi_agent.py` 有 solo/swarm/pinned 模式。Swarm 中主 session 正常说，后台 routine 事件延后或丢弃，失败、问题、prompt intent 才打断，并且可为不同 repo/session 分配不同 voice。

对 Codex Speak 的启发：

- Heard 已经把“决定什么不说”做成产品核心，是直接竞品。
- Codex Speak 需要补 `digest` 概念：低价值过程事件不立即朗读，但可在长任务或完成时合成一句进展。
- 建议把状态层和 phrasing 层分开：本地先产出 deterministic facts/hints，LLM 或 side-channel 只负责把它说顺。
- 需要补 spoken offset / 去重，尤其是 pause/resume、hook 重试、session 切换后避免重复播放。
- Multi-agent 早晚会来，应尽早预留 `agent_id`、`agent_label`、`focus/pinned`、`priority`。

### AgentVibes

相关度：高，偏 TTS 工程。

读过的关键文件：

- `_research/competitors/agentvibes/docs/architecture/diagrams.md`
- `_research/competitors/agentvibes/mcp-server/server.py`
- `_research/competitors/agentvibes/.claude/hooks/play-tts.sh`
- `_research/competitors/agentvibes/.claude/hooks/tts-queue.sh`
- `_research/competitors/agentvibes/.claude/hooks/tts-queue-worker.sh`
- `_research/competitors/agentvibes/.claude/hooks/provider-manager.sh`
- `_research/competitors/agentvibes/src/services/verbosity-service.js`
- `_research/competitors/agentvibes/.codex/hooks/init-agentvibes.sh`

核心机制：

- 它的强项是 provider/router/queue：Piper、macOS Say、Windows SAPI、Windows Piper、SSH remote、Soprano 等统一进 `play-tts.sh`。
- Provider 配置按 project/global fallback 查找，并支持 provider 切换后的 voice migration。
- `tts-queue.sh` 和 worker 用用户私有临时目录、lock、PID 文件、顺序播放和 idle timeout。
- `play-tts.sh` 有 global/project mute，以及 per-LLM voice/pretext/engine routing。
- Verbosity 分 `minimal`、`low`、`medium`、`high`、`custom`，控制 prompt-submit / response-complete 是否说，以及说多长。
- Codex hook 目前主要是初始化，真正发声更多通过 AGENTS/MCP 工具协议触发。

对 Codex Speak 的启发：

- 语音工程清单要补齐：队列、停止、打断、provider fallback、project/global mute、per-session/per-agent voice。
- Verbosity 是必须产品化的，不只是 debug 开关。推荐我们用更贴近用户的命名：`quiet`、`brief`、`guide`、`learning`。
- 不建议追 AgentVibes 的“人格/音乐/大量音色”路线；那会稀释我们中文导览和本地稳定性的优势。

### VoiceMode

相关度：中，偏语音输入和双向对话。

读过的关键文件：

- `_research/competitors/voicemode/voice_mode/tools/converse.py`
- `_research/competitors/voicemode/voice_mode/core.py`
- `_research/competitors/voicemode/voice_mode/providers.py`
- `_research/competitors/voicemode/docs/reference/converse-parameters.md`

核心机制：

- MCP server 提供 `converse` 工具，让 agent 说一句话，然后可选择等待用户语音回答。
- STT/TTS 统一使用 OpenAI-compatible endpoint，能接 OpenAI、Kokoro、Whisper 等本地服务。
- 有 voice-first provider selection：按用户偏好的 voice 找到支持它的健康 endpoint。
- 支持 WebRTC VAD、最短/最长录音、蓝牙 chime 前后静音、debug 音频保存、STT 失败后手动恢复。
- TTS 支持 streaming，记录 TTFA 等指标。

对 Codex Speak 的启发：

- 未来“Codex 需要确认时用语音问用户”可参考 `converse`，但这不是当前朗读主线。
- Provider discovery 可以参考，但我们目前本地模型/系统 TTS 的路径更简单。
- 蓝牙/AirPods 场景要考虑前导静音和尾部静音，否则提示音和第一句话容易被截断。

### VoiceTerm

相关度：中，偏语音输入和终端 overlay。

读过的关键文件：

- `_research/competitors/voiceterm/README.md`
- `_research/competitors/voiceterm/guides/USAGE.md`
- `_research/competitors/voiceterm/dev/adr/0011-voice-send-modes.md`
- `_research/competitors/voiceterm/rust/src/`

核心机制：

- VoiceTerm 是 Codex / Claude Code 的 voice-first terminal overlay。Whisper 默认本地运行，把用户语音转文字并注入现有 CLI PTY。
- 支持 push-to-talk、auto voice、wake phrase：`hey codex` / `hey claude` 后说 prompt，再说 `send` / `submit`。
- send mode 有 `auto` 和 `insert` 两种，且可运行时切换。`auto` 立即按 Enter，`insert` 只插入文字，让用户编辑后再发。
- 如果 CLI 正忙，transcript queue 会等 CLI ready 后再发送。
- HUD 做了很多实际工程：prompt-safe overlay、防遮挡、mic meter、latency badge、history、notification history、theme/settings。
- 支持 voice macros、voice navigation、screenshot prompts。它解决的是“用户不用打字”，不是“Agent 说什么”。

对 Codex Speak 的启发：

- 如果未来做语音输入，`auto/insert` 是很好的安全模式边界：危险或长 prompt 应默认 insert。
- 离屏体验不只是 TTS，还需要“什么时候继续听用户说话”的循环控制；但这应是后续能力。
- HUD 防遮挡和 transcript queue 对桌面端状态面板有参考价值。

### TalkToCursor

相关度：中，偏 Cursor voice loop。

读过的关键文件：

- `_research/competitors/talk-to-cursor/README.md`
- `_research/competitors/talk-to-cursor/src/index.ts`
- `_research/competitors/talk-to-cursor/src/config.ts`
- `_research/competitors/talk-to-cursor/scripts/auto-submit.py`
- `_research/competitors/talk-to-cursor/scripts/silence_detector.py`

核心机制：

- MCP server 暴露 `speak` 和 `listen` 两个工具。`speak` 调 ElevenLabs 并排队播放，避免音频重叠；播放完成后写 `tts-complete.json`。
- `listen` 写 `listen-signal.json`，后台 Python 脚本收到后触发 Wispr Flow 热键，等 TTS 完成再开始听，避免自己说话被识别成用户输入。
- auto-submit 通过 macOS Accessibility 轮询 Cursor 当前文本框，检测到听写新增文本后延迟按 Enter。
- `silence_detector.py` 是简单 RMS 状态机：IDLE -> SPEECH -> TRAILING_SILENCE -> DONE。
- Cursor rules 决定 agent 什么时候调用 `speak`，因此它依赖 agent 自觉调用 MCP，而不是 hook 强制最终播放。

对 Codex Speak 的启发：

- `speak/listen` 文件信号非常简单，但足以串起 hands-free loop。
- 播放完成信号对自动继续听非常关键；如果未来做语音确认，也要知道 TTS 是否真的结束。
- 它再次说明 MCP 发声工具容易被 agent 忘记调用，Codex Speak 的 hook/final guide 路线更稳。

### Codex CLI Hooks

相关度：中，偏基础 hook 通知。

读过的关键文件：

- `_research/competitors/codex-cli-hooks/README.md`
- `_research/competitors/codex-cli-hooks/CLAUDE.md`
- `_research/competitors/codex-cli-hooks/.codex/hooks/scripts/hooks.py`
- `_research/competitors/codex-cli-hooks/install/hooks-mac.json`

核心机制：

- 支持 Codex CLI 8 个 hook：`SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PermissionRequest`、`PostToolUse`、`Stop`、`PreCompact`、`PostCompact`。
- 一个 Python handler 根据 `--hook <EventName>` 查 `HOOK_SOUND_MAP`，播放对应预录音频。
- macOS 用 `afplay`，Linux 尝试 `paplay/aplay/ffplay/mpg123`，Windows 用 `winsound` 播 WAV。
- `SessionStart` 特殊：会向 stdout 输出日期、git branch、cwd 等上下文，进入 Codex context；其他事件只播声音。
- 有 config local override、disable per-hook、JSONL log，但不做语义摘要或状态过滤。

对 Codex Speak 的启发：

- 基础完成通知门槛很低，不能作为长期壁垒。
- 这个项目是 Codex hook surface 的版本索引，提醒我们文档里要持续核对 Codex hooks 版本。
- PreCompact/PostCompact 值得关注：上下文压缩前后可能适合做一次“我正在压缩上下文，稍后继续”的极短提示。

### V1GPT

相关度：高，偏 Codex companion / pet / avatar。

读过的关键文件：

- `_research/competitors/v1gpt/README.md`
- `_research/competitors/v1gpt/server/README.md`
- `_research/competitors/v1gpt/server/config/spoken-summary-prompt.md`
- `_research/competitors/v1gpt/server/config/spoken-style.toml`
- `_research/competitors/v1gpt/server/src/codex_voice/codex_session_watcher.py`
- `_research/competitors/v1gpt/server/src/codex_voice/spoken_text.py`
- `_research/competitors/v1gpt/server/src/codex_voice/server.py`
- `_research/competitors/v1gpt/src/ws-client.ts`
- `_research/competitors/v1gpt/src/audio-player.ts`

核心机制：

- Tauri + Three.js avatar 前端，FastAPI TTS server，Codex session watcher，目标是 Codex CLI 和 Codex Mac App 的 voice avatar layer。
- watcher 轮询 `~/.codex/sessions` JSONL，默认只接受 `cli` / `vscode`，启动时 prime 到 EOF，避免重播旧 session。
- 识别 `task_started`、function call、function output、commentary、final answer、task complete，并广播 `thinking`、`tool_use`、`idle`、mood。
- 对 `sandbox_permissions=require_escalated` 这类需要 approval 的命令会播一次提醒。
- `spoken-summary-prompt.md` 要求 JSON-only、1-N 短句、先说结论，不复制 markdown/path/command/URL/code。
- `spoken-style.toml` 有 max sentences/chars，并过滤 offer/question/path-like/command-like 句子。
- server 提供 `/speak`、`/alert`、`/status`、`/stop`、`/voice`、`/mute`、`/speech-cadence-mode`、`/ws/status`、`/ws/audio`。cadence 有 `off`、`once`、`15s`、`30s`、`always`。
- 前端通过 WebSocket 接收 status/audio，audio player 用 Web Audio analyser 驱动 lip sync/waveform。

对 Codex Speak 的启发：

- Codex session JSONL watcher 是 hook 之外的可选事件来源，可用于 Desktop 场景，但轮询和 schema 兼容成本更高。
- speech cadence 很适合引入 Codex Speak：可以独立于 verbosity 控制中途播报频率。
- Approval notice、thinking loop、mood 都适合映射到 Pet，但视觉 companion 不应盖过 speech-safe guide。
- 用 Codex 自己二次总结 final answer 会带来递归、成本和延迟；我们的 side-channel 直接产语音稿更干净。

### Untether

相关度：中高，偏远程进度和多引擎抽象。

读过的关键文件：

- `_research/competitors/untether/src/untether/runners/codex.py`
- `_research/competitors/untether/src/untether/progress.py`
- `_research/competitors/untether/src/untether/telegram/bridge.py`
- `_research/competitors/untether/src/untether/markdown.py`
- `_research/competitors/untether/docs/how-to/voice-notes.md`
- `_research/competitors/untether/docs/how-to/interactive-approval.md`
- `_research/competitors/untether/docs/how-to/verbose-progress.md`

核心机制：

- Codex runner 使用 `codex exec --json --skip-git-repo-check --color=never`，把 JSONL 事件转成统一的 `UntetherEvent`。
- Codex item 映射很完整：command、tool、web_search、file_change、todo、reasoning、error、final answer。
- `ProgressTracker` 维护 action set、phase、ok、first_seen、last_update 和 wall-clock，用于进度渲染。
- Markdown renderer 有 compact/verbose 两种视图。长任务超过 60 秒会追加 elapsed tail，比如“做了多久、在做什么”。
- Telegram approval 对 Claude Code 有中途按钮；Codex 当前是 pre-run approval policy。
- Voice notes 是输入能力：Telegram 语音转文字后作为普通 prompt 运行。

对 Codex Speak 的启发：

- 我们应做统一事件摘要层：先把 Codex/工具事件变成标准 action，再决定 UI 显示或 TTS 播报。
- Compact/verbose 可以映射到 `brief/guide/learning` 朗读档位。
- 长任务 heartbeat 很适合语音化，但必须限频，例如 60 秒以上才说一次，并且只说“仍在跑测试/构建”这种高价值状态。

### Happy

相关度：中高，偏手机/Web/E2E/语音会话。

读过的关键文件：

- `_research/competitors/happy/docs/voice-architecture.md`
- `_research/competitors/happy/docs/protocol.md`
- `_research/competitors/happy/docs/encryption.md`
- `_research/competitors/happy/packages/happy-cli/src/index.ts`

核心机制：

- CLI wrapper：用户运行 `happy` 或 `happy codex`，由 wrapper 负责本地/远程切换和 daemon/session 管理。
- Protocol 区分 persistent update 和 ephemeral activity。session、message、metadata、agentState 都是同步原语。
- 大部分内容端到端加密，server 存 opaque blob。push token 和 session id 是少量非内容元数据。
- Voice architecture 中，语音 agent 可向当前 session 发送消息、处理权限请求。当前 session 由单一 `currentSessionId` 控制。
- 语音事件分 `sendContext` 和 `sendPrompt`：上下文注入不触发回复，权限/ready 这种事件触发 voice agent 回答；说话中会 queue，回到 idle 后 batch flush。

对 Codex Speak 的启发：

- 如果未来做实时语音 assistant，区分“静默上下文注入”和“触发朗读/回答”的通道很重要。
- 远程 sync 一旦做，就要认真设计 E2E 和最小协议面；否则隐私会拖累定位。
- 短期可借鉴 activity/presence 模型，不需要复制整套移动端。

### Paseo

相关度：中高，偏 agent OS / daemon / voice-first 平台。

读过的关键文件：

- `_research/competitors/paseo/docs/architecture.md`
- `_research/competitors/paseo/docs/agent-lifecycle.md`
- `_research/competitors/paseo/public-docs/voice.md`
- `_research/competitors/paseo/packages/server/src/server/agent/providers/codex-app-server-agent.ts`
- `_research/competitors/paseo/packages/server/src/server/agent/agent-sdk-types.ts`
- `_research/competitors/paseo/packages/server/src/server/agent/tts-manager.ts`
- `_research/competitors/paseo/packages/server/src/server/agent/stt-manager.ts`

核心机制：

- Paseo 是本地 daemon + WebSocket client 架构，管理 Claude、Codex、Copilot、OpenCode、Pi 等 agent。
- Agent lifecycle 是 `initializing -> idle <-> running -> error/closed`，并持久化到 `$PASEO_HOME/agents/...`。
- Agent 可以创建 subagent。子 agent 与 parent 绑定，parent archive 时级联 archive。
- Codex provider 包装的是 Codex AppServer，支持 streaming、session persistence、MCP servers、tool invocations、reasoning stream、rewind 等能力。
- ToolCallDetail 被归一化成 shell/read/edit/write/search/fetch/worktree 等类型。
- TTSManager 按 260 字符切段，预取 2 段，发给客户端播放，并等待 playback confirmation。
- Voice 支持 local-first：本地 Parakeet STT、Kokoro TTS，也可切到 OpenAI。Voice orchestration 使用隐藏 agent session 和 MCP bridge。

对 Codex Speak 的启发：

- 如果未来要支持多 agent，必须把 `agent_id`、parent/child、lifecycle、timeline 分开建模。
- TTS 播放如果跨设备，应有 playback confirmation；否则“以为说完了”和“用户实际听到”会不一致。
- 本地 speech 模型下载目录和 provider 配置要清晰，否则安装支持成本会很高。

## 横向模式

### 1. 触发机制

| 机制 | 代表项目 | 优点 | 风险 |
| --- | --- | --- | --- |
| Stop hook | Aftertone, Codex Speak | 结束时可靠触发，不依赖 agent 主动调用工具 | 只有最终时刻，过程状态弱 |
| MCP tool | VoiceMode, AgentVibes | Agent 可主动说话/问用户 | Agent 可能忘记调用，且会污染任务上下文 |
| ACP/AppServer wrapper | Sled, Paseo | 状态、权限、streaming 最完整 | 产品会变重，安装和兼容性成本高 |
| Terminal/screen detection | Herdr | 可覆盖原生 CLI，不侵入 agent | heuristic 易碎，需处理 stale state |
| Codex session JSONL watcher | V1GPT | 可覆盖 CLI/Desktop session flow，能看到过程事件 | 轮询、schema 变化和来源识别成本高 |
| 预录 hook sound | Codex CLI Hooks | 安装简单，hook surface 清楚 | 无语义层，容易吵，壁垒低 |
| Remote daemon/bridge | Happy, Untether, Paseo | 适合手机和多端 | 协议、隐私、安全、同步成本高 |

Codex Speak 短期应保持 Hook + MCP side-channel。中期可以补 optional event bridge，但不应马上变成 full wrapper。

### 2. 内容选择

最优路径是“专门语音稿”：

- Aftertone 用 `<spoken_summary>`。
- Codex Speak 用 side-channel guide items。
- Untether/Paseo 用结构化 action/timeline 再渲染。
- Heard 用 facts/hints + verbosity/digest 来决定说、不说或稍后总结。
- V1GPT 用 spoken summary prompt/style rules 把最终回答改写成适合听的短句。

最不推荐路径是“读原始输出”：

- 代码、路径、日志、表格、Markdown 对耳朵都很差。
- 流式断句需要大量边界处理，Sled 的 sentence detector 说明这个坑很深。

### 3. 状态模型

建议 Codex Speak 内部采用：

```text
idle
working
long_running
need_approval
blocked
failed
done_unseen
done_seen
```

其中 `done_unseen` 是 Herdr 给我们的关键启发。离屏/AirPods 场景下，“完成但用户没感知”才是需要通知的状态。

### 4. 播放策略

从 AgentVibes、Aftertone、Paseo 可以抽出几条工程规则：

- 支持 queue 和 interrupt 两种模式。
- 高优先级事件可打断低优先级导览。
- completion guide 不应和 next event 重叠。
- 蓝牙设备需要 leading/trailing silence。
- 长文本要切段并限制每段长度。
- 最好能知道 playback completed，至少本地要有 job id 和日志。
- pause/resume 或新装后要有 offset/去重，避免旧 transcript 重播。
- 中途播报要有 cadence，不能只靠 verbosity。

### 5. Verbosity

竞品都在做 verbosity，但命名和语义不统一。建议 Codex Speak 采用面向用户的档位：

| 档位 | 行为 |
| --- | --- |
| `quiet` | 只播失败、阻塞、需要用户确认 |
| `brief` | 完成时一句话，失败/阻塞详细一点 |
| `guide` | 当前默认：做了什么、结果、下一步 |
| `learning` | 给初学者/儿童更多解释，但仍不读代码 |

### 6. 远程/多 Agent

Happy、Untether、Paseo 的共同信号是：手机远程和多 agent 会变成高频场景。但这不是 Codex Speak 当前最小闭环。现在需要做的是留数据模型：

- `session_id`
- `agent_id`
- `agent_label`
- `parent_agent_id`
- `priority`
- `event_kind`
- `seen_at`
- `spoken_at`
- `source_surface`

## Roadmap 建议

### 近期

1. 固化“没有 guide 就不读”的默认策略，并在 README 中解释原因。
2. 增加 `brief/guide/learning/quiet` 朗读档位。
3. 增加 per-session enable/disable，参考 Aftertone pending session registration。
4. 抽出内部 speech event/action 模型，为 UI/Pet/TTS 共用。
5. 增加 `done_unseen` 或 `spoken_at` 去重，避免完成提示重复。
6. 梳理 TTS 队列/打断/停止策略，补 job id 和日志。
7. 为 AirPods/蓝牙增加可配置 leading/trailing silence。
8. 增加 spoken offset/hash 去重，避免 hook retry、resume、新装后重复朗读。
9. 增加 digest 队列：低价值事件不立即说，但可在长任务/完成时合成一句。

### 中期

1. 做事件级播报：long-running、need_approval、failed、done。
2. 把 final guide 与事件播报统一走 priority scheduler。
3. 增加 agent/session label，支持多 Codex session 分辨。
4. 可选接入 Codex JSONL/AppServer/ACP 事件流，但保持 hook/MCP 轻量路径。
5. 研究语音确认：当 Codex 需要用户确认时，播报问题并允许语音/按钮回复。
6. 为 Pet 增加 mood/tool_mood/status 映射，但不做 3D avatar 主线。

### 暂缓

1. 手机 App/Telegram/relay：市场上已经很多，且会显著增加安全和支持成本。
2. 大量人格/音乐/音效：会分散“专业中文导览”的定位。
3. 流式朗读 assistant 原文：复杂度高，中文听感不稳定，和我们的核心差异化冲突。

## 参考链接

- [Sled](https://github.com/layercodedev/sled)
- [Aftertone](https://github.com/omarelkhal/aftertone)
- [Herdr](https://github.com/ogulcancelik/herdr)
- [AgentVibes](https://github.com/paulpreibisch/AgentVibes)
- [VoiceMode](https://github.com/mbailey/voicemode)
- [Untether](https://github.com/littlebearapps/untether)
- [Happy](https://github.com/slopus/happy)
- [Paseo](https://github.com/getpaseo/paseo)
- [Heard](https://github.com/heardlabs/heard)
- [V1GPT](https://github.com/intarm/V1GPT)
- [Codex CLI Hooks](https://github.com/shanraisshan/codex-cli-hooks)
- [VoiceTerm](https://github.com/jguida941/voiceterm)
- [TalkToCursor](https://github.com/MindSyncTech/talk-to-cursor)
