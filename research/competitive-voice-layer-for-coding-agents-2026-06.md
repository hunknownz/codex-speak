# Coding Agent 语音层竞品调研

调研日期：2026-06-05  
调研对象：面向 Codex / Claude Code / Cursor / Gemini CLI / OpenCode 等编码 Agent 的朗读、语音输入、远程控制和状态通知产品。

## 关键结论

Codex Speak 所在方向已经有明显竞品和参考项目，不是空白赛道。过去几个月出现了多种“让 Coding Agent 开口/可远程控制”的产品：Sled、Codexini、Heard、AgentVibes、Aftertone、TalkToCursor、Happy、CodeVibe、CC Pocket、Paseo、Untether、VoiceMode、Spokenly、Voxlert 等。

但这些项目的重心并不完全相同。目前市场可分成五类：

1. 手机远程控制 Agent：解决“不在电脑旁时 Agent 停住”的问题。
2. 双向语音编码：解决“用户用语音给 Agent 下指令”的问题。
3. Agent 朗读/通知层：解决“Agent 做完、卡住、报错时告诉我”的问题。
4. 多 Agent 编排和状态面板：解决“同时跑多个 Agent 时谁在做什么”的问题。
5. 普通 TTS/MCP 语音工具：解决“让模型调用一个发声工具”的问题。

对 Codex Speak 最重要的判断是：真正有价值的竞争点不是 TTS 引擎本身，而是 Agent 状态理解、噪声过滤、语音稿生成、中文/儿童友好、本地隐私、安装链路和听觉场景设计。

Codex Speak 目前的差异化不应定位为“给 Codex 加 TTS”，而应定位为：

```text
把 Codex 的技术输出转换成适合中文听众理解的本地朗读导览。
```

如果未来继续扩展，可以升级为：

```text
面向 Coding Agent 的中文本地 Voice Notification / Speech Guide Layer。
```

## 我们当前产品的位置

从仓库现状看，Codex Speak 已经不是一个简单 Hook 播放器，而是有清晰产品边界：

- Codex Skill / MCP side-channel 生成结构化朗读导览。
- Hook 在回复结束后消费导览。
- 本地 TTS 优先，中文优先，有多 Provider。
- 控制面板可切换自动朗读、儿童模式、语速、音色、TTS 引擎。
- macOS Pet 和控制台展示状态。
- 自检、安装器、支持包、发音词典、英文技术词归一化已经成体系。

这意味着我们和大多数“读 Agent 原文”的工具相比，优势在“听觉内容产品化”：不是把屏幕文字转声音，而是让 Codex 产出一份专门给耳朵听的导览。

## 竞品矩阵

| 项目 | 类型 | 支持对象 | 语音方向 | 与 Codex Speak 的关系 | 关键观察 |
| --- | --- | --- | --- | --- | --- |
| Sled | 手机远程 + 语音 | Claude Code、Codex、Gemini CLI | 输入 + 输出 + 通知 | 场景竞品 | 强调 AirPods、手机、远离桌面；但官网当前会跳转，需关注产品形态变化 |
| Codexini | 语音桥 + 多 Agent 调度 | Claude Code、Codex、OpenClaw、Hermes | 输入 + 短摘要回调 | 高相关竞品 | 定位很接近“不要读原始日志，听短摘要” |
| Heard / herdr | Agent 状态/通知/多 Agent | Claude Code、Codex 等 | 中间状态 + 完成/失败/需输入 | 高相关参考 | 强调过滤、verbosity、swarm mode；“不要 audio soup”是核心洞察 |
| AgentVibes | TTS/MCP + 个性化声音 | Claude Code、Warp、OpenClaw 等，搜索结果显示也覆盖 Codex 相关版本 | 输出为主 | 直接功能竞品 | 侧重音色、人格、音乐、跨平台 TTS provider |
| Aftertone | 本地短摘要 TTS | Cursor、Claude Code；Codex/OpenCode roadmap | 输出 | 高相关参考 | 只读 `<spoken_summary>`，tag-only 默认安静，和我们的 side-channel 思路高度一致 |
| TalkToCursor | Cursor 语音界面 | Cursor | 输入 + 输出 + 自动循环 | 相邻竞品 | 更像 Cursor + ElevenLabs + Wispr Flow |
| Happy | 手机/Web 客户端 | Claude Code、Codex | 实时语音 + 推送 | 场景竞品 | 远程控制、E2E、多端切换，GitHub 热度高 |
| CodeVibe | 手机控制/审批 | Claude Code、Gemini CLI、Codex CLI、Antigravity CLI | 语音输入 + 推送 | 场景竞品 | 商业化明确，按 active sessions / messages 收费 |
| CC Pocket | 手机控制 | Codex、Claude Code | 进度/审批为主 | 场景参考 | 自托管 Bridge，强调代码留在本机 |
| Paseo | 多 Agent 编排 | Claude Code、Codex、OpenCode、Copilot、Pi 等 | 本地语音输入/输出 | 平台级竞品 | 目标是 Agent workspace / orchestrator，不只是语音 |
| Untether | Telegram 桥 | Claude Code、Codex、OpenCode、Pi、Gemini CLI、Amp | 语音输入 + 进度流 | 远程场景参考 | Telegram 是低门槛 remote UX |
| VoiceMode | MCP 双向语音 | Claude Code 和 MCP Agent | 语音对话 | 相邻竞品 | 核心是“自然语音对话”，不专注最终朗读导览 |
| Spokenly Voice for Agents | Dictation MCP | Claude Code、Codex、Cursor | Agent 提问 + 用户语音回答，可朗读问题 | 相邻竞品 | 把 voice 作为 ask-user 工具，不是 Agent 叙事层 |
| Voxlert | 本地通知 + 角色化声音 | Claude Code、Cursor、Codex | 完成/错误/需输入通知 | 高相关参考 | 每个 session 不同声音，多 Agent 可辨识 |
| Warp CLI Agent support | 终端平台增强 | Claude Code、Codex、OpenCode 等 | 通知/远程控制/工具带 | 平台压力 | 大平台会把通知和 remote control 变成基础能力 |

## 重点项目观察

### 本轮补查：原脚注项目是否已经逐项看完

2026-06-05 已把同事初步调研中列出的 TalkToCursor、Sled、Heard、Codex Voice Hooks、V1GPT、VoiceTerm、WIRED 全部补查。能 clone 的开源项目已作为 submodule 拉到 `_research/competitors/`，并单独形成核查文档：

- `research/footnote-source-check-2026-06-05.md`
- `research/open-source-reference-projects.md`
- `research/deep-dive-open-source-voice-agent-projects-2026-06.md`

补查后的口径：

- Heard 是当前最接近 Codex Speak 的直接竞品，核心能力是 narrator judgment、verbosity、digest、swarm mode、去重。
- V1GPT 是 Codex companion/avatar 方向，值得参考 Codex session watcher、spoken summary、approval notice、mood/cadence，但不建议短期追 3D avatar。
- Codex Voice Hooks 证明“完成时响一下”已经非常低门槛，不能成为定位核心。
- VoiceTerm 和 TalkToCursor 都是 hands-free voice coding 参考，主要解决用户语音输入，不是 Codex 最终结果朗读。
- WIRED 的价值是市场背景：Cursor、OpenAI、Anthropic 都在争 coding agent platform，基础通知和远程控制会越来越平台化。

### Sled

Sled 的原始定位是“Run your coding agent from your phone, with voice”，支持 Claude Code、OpenAI Codex、Gemini CLI，主打手机远程、语音输入、语音输出、通知、AirPods、Tailscale/ngrok、本地运行。搜索缓存和第三方页面仍能看到其完整介绍，但 `sled.layercode.com` 当前打开会跳到 Toyo，所以需要持续观察它是否已经转型或并入 Layercode/Toyo 产品线。

对我们的启发：

- AirPods / 离开桌面的场景是真需求。
- “Agent 每 10-60 分钟需要用户输入，否则停住”是很好传播的痛点表述。
- 纯语音输出不够，远程审批和移动端回控会变成强需求。

参考：[Sled 搜索缓存](https://sled.layercode.com/)、[Product Hunt: Sled](https://www.producthunt.com/products/sled-voice-interface-for-claude-code)、[ProductCool: Sled](https://www.productcool.com/product/sled)

### Codexini

Codexini 自称是 “quiet voice bridge”，帮助用户 steer Claude Code、Codex、OpenClaw、Hermes，同时避免读 raw logs。它的页面直接强调 “Only important moments return” 和 “callback is short and speech-safe: no raw code, no secrets, no terminal sludge”。这与我们“不要直接朗读 Codex 原始回复”的判断高度一致。

对我们的启发：

- 竞品已经开始明确占位“短摘要回调”。
- 它是 macOS first，并有免费时长限制，说明该方向有商业化尝试。
- 它定位更偏 voice-first orchestration；我们可以避免正面撞“语音调度”，先守住“中文本地朗读导览”。

参考：[Codexini](https://codexini.com/)

### Heard / herdr

Heard 在 Reddit 上传播的核心洞察是：TTS for agents 不是 voice problem，而是 filtering problem。搜索结果显示它有 Claude Code + Codex adapter、`heard run` 泛化包装、4 个 verbosity profiles、swarm mode，多 Agent 同时运行时后台 Agent 只在失败时打断，避免 audio soup。

Herdr 文档显示其更偏多 Agent 终端状态管理：自动检测 Claude Code、Codex、OpenCode 等状态，并把 Idle / Working / Blocked roll up 到 pane、tab、workspace。它强调用户不该手动轮询每个终端。

源码补查后需要把 Heard 和 Herdr 分开看：

- Heard 是语音旁白产品。它在 `verbosity.py` 中把事件分为 speak/drop/digest，在 `spoken.py` 中做 per-session hash 和 transcript offset，在 `multi_agent.py` 中做 solo/swarm/pinned 路由。
- Herdr 是状态面板/通知产品。它的关键启发是 `done_unseen`：完成但用户还没看过，才值得通知。

对我们的启发：

- “决定什么不说”比“怎么合成声音”更关键。
- 多 Agent 场景下需要优先级：主 Agent 可多说，后台 Agent 只在失败/阻塞/完成时说。
- 状态层应至少抽象为：working、blocked、need input、done、failed。
- Codex Speak 应补 digest/offset/去重，并把状态 facts 与语音 phrasing 分层。

参考：[Reddit: I gave my coding agents a voice](https://www.reddit.com/r/SideProject/comments/1szzugt/i_gave_my_coding_agents_a_voice_oss/)、[Heard GitHub](https://github.com/heardlabs/heard)、[herdr Agents docs](https://herdr.dev/docs/agents/)

### Aftertone

Aftertone 的产品约束很克制：本地运行、Cursor 和 Claude Code 可用、Codex/OpenCode 在 roadmap；默认 tag-only，如果没有 `<spoken_summary>` 就保持安静。它只读短线：“what landed, blockers, next step”，不读完整墙文本。

对我们的启发：

- 我们的 MCP side-channel 比 `<spoken_summary>` 更结构化，但理念一致：让 Agent 主动提供 speech-safe 摘要。
- 默认安静是好设计。没有专门朗读稿时，不应猜测朗读原文。
- “what landed / blockers / next step” 可作为最终导览的最低信息集。

参考：[Reddit: Aftertone](https://www.reddit.com/r/vibecoding/comments/1ti236b/a_completely_local_tts_for_cursor_and_claude_code/)

### AgentVibes

AgentVibes 是更成熟的 TTS 工具体系，侧重声音、人格、Provider、TUI、MCP，自带 Piper、Soprano、macOS Say、Windows SAPI 等路线。搜索结果显示它近期版本围绕多 LLM 身份、远程 receiver、verbosity、无重叠播放、markdown 清洗等做了大量打磨。

对我们的启发：

- 用户会在意音色、速度、可玩性和“这个 Agent 是谁在说话”。
- 但我们不应该一开始卷 900+ voices；中文可懂、低噪声、安装稳定更重要。
- “无重叠播放”和队列策略需要长期重视。

参考：[AgentVibes GitHub](https://github.com/paulpreibisch/AgentVibes)

### TalkToCursor

TalkToCursor 针对 Cursor，使用 MCP、ElevenLabs、Wispr Flow 等实现 TTS、语音命令、自动听写循环。它解决的是 Cursor hands-free loop，不是 Codex 朗读导览。

源码补查确认它的实现是：

- MCP server 暴露 `speak` 和 `listen`。
- `speak` 用 ElevenLabs 排队播放，播放结束写 `tts-complete.json`。
- `listen` 写 `listen-signal.json`，后台脚本等 TTS 完成后触发 Wispr Flow。
- auto-submit 通过 macOS Accessibility 轮询 Cursor 文本框，检测听写文本后延迟按 Enter。
- silence detector 是 RMS 状态机，不是复杂 VAD。

对我们的启发：

- “语音输入 + 语音输出 + 自动循环”很容易成为用户对 voice coding 的默认想象。
- Codex Speak 若暂不做语音输入，需要明确：我们是 speech guide / notification layer，不是 dictation layer。
- 如果未来做 ask-user，必须知道 TTS playback 已完成，否则 agent 自己的朗读会被听写工具误收进去。

参考：[TalkToCursor](https://talktocursor.com/)、[TalkToCursor GitHub](https://github.com/MindSyncTech/talk-to-cursor)

### V1GPT

V1GPT 是 Codex CLI 和 Codex Mac App 的 3D voice avatar。Reddit 帖和源码都显示它包含：Three.js/Tauri 头像、Codex session watcher、spoken summaries、thinking audio、approval notice、mood changes、cursor tracking、TTS server 和 websocket audio/status。

源码补查重点：

- watcher 轮询 `~/.codex/sessions` JSONL，启动时 prime 到 EOF 避免重播旧内容。
- 识别 task start、function call、function output、commentary、final answer、task complete。
- 对 escalated approval 做一次性语音提醒。
- spoken summary prompt 要求 JSON-only、短句、先说结论，不复制 markdown/path/command/URL/code。
- cadence 有 `off`、`once`、`15s`、`30s`、`always`。

对我们的启发：

- Pet/avatar 可以作为状态可感知层，但不应成为当前主线。
- Codex JSONL watcher 可作为 hook 之外的事件来源，但 schema/轮询/来源识别会增加维护成本。
- speech cadence 值得引入，独立于 verbosity 控制中途提示频率。

参考：[Reddit: V1GPT](https://www.reddit.com/r/codex/comments/1s0avrn/i_gave_codex_a_3d_avatar_v1gpt_is_now_my_voice/)、[V1GPT GitHub](https://github.com/intarm/V1GPT)

### Codex Voice Hooks

Codex Voice Hooks / Codex CLI Hooks 是极轻量路线：把 Codex hook 映射到预录人声或提示音。

源码补查重点：

- 当前覆盖 `SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PermissionRequest`、`PostToolUse`、`Stop`、`PreCompact`、`PostCompact`。
- macOS 用 `afplay`，Linux 尝试 `paplay/aplay/ffplay/mpg123`，Windows 用 `winsound`。
- `SessionStart` 会向 stdout 注入日期、git branch、cwd；其他事件只播放音频。
- 支持 per-hook disable 和 JSONL log，但没有摘要、过滤、状态判断。

对我们的启发：

- 只做 hook 声音没有壁垒。
- Codex Speak 要把 hook 当触发器，而不是产品主体。
- PreCompact/PostCompact 可能适合未来做上下文压缩提示。

参考：[Reddit: Codex Voice Hooks](https://www.reddit.com/r/OpenAI/comments/1rgw0w1/i_gave_codex_cli_a_voice_so_it_tells_me_when_its/)、[Codex CLI Hooks GitHub](https://github.com/shanraisshan/codex-cli-hooks)

### VoiceTerm

VoiceTerm 是 voice-first terminal overlay for Codex and Claude。它的强项是用户语音输入，不是 Agent 结果朗读。

源码/文档补查重点：

- Whisper 默认本地运行。
- 支持 `hey codex` / `hey claude` wake phrase，再用 `send` / `submit` 发送。
- send mode 分 `auto` 和 `insert`，可运行时切换。
- CLI 忙时会 queue transcript，等 ready 后再发。
- HUD 有 mic meter、latency、history、notification history、prompt-safe overlay。

对我们的启发：

- 如果未来做语音输入，必须保留 insert-only 审核模式。
- 免手场景需要完整 loop：什么时候听、什么时候发、忙时怎么办。
- 但短期 Codex Speak 应继续把“Agent 说给用户听”作为主线。

参考：[Reddit: VoiceTerm](https://www.reddit.com/r/ClaudeCode/comments/1rll38z/voiceterm_handsfree_voice_coding_for_ai_clis_mac/)、[VoiceTerm GitHub](https://github.com/jguida941/voiceterm)

### Happy、CodeVibe、CC Pocket、Paseo、Untether

这组产品共同说明“从手机控制 Coding Agent”正在变成大方向。

- Happy：Codex 和 Claude Code 的移动/Web 客户端，E2E，加推送，GitHub 热度很高。
- CodeVibe：商业化清晰，支持 Claude/Gemini/Codex/Antigravity，主打审批、diff、实时工具使用、推送、E2E，定价为月费/年费。
- CC Pocket：自托管 Bridge，Codex/Claude，从手机启动、恢复、监控、审批。
- Paseo：更大平台，支持桌面/Web/移动/CLI，多 Agent、worktree preview、diff、PR、merge，并有本地语音栈。
- Untether：Telegram 桥，支持多 Agent，进度 streaming、resume、voice input、error hints。

对我们的启发：

- 移动端和远程审批会逐渐从加分项变成标配。
- Codex Speak 的短期价值仍在本地朗读质量；长期可以成为这些 remote/orchestrator 的“中文语音输出组件”。
- 如果要商业化，CodeVibe 的免费/Pro/Max 分层值得参考，但我们的短期收费点更适合“安装包、企业离线、本地中文、教育版”。

参考：[Happy GitHub](https://github.com/slopus/happy)、[CodeVibe](https://quantiya.ai/codevibe/)、[CC Pocket](https://k9i-0.github.io/ccpocket/install/)、[Paseo](https://paseo.sh/)、[Untether GitHub](https://github.com/littlebearapps/untether)

### VoiceMode、Spokenly、普通语音输入工具

VoiceMode 和 Spokenly 代表另一条路线：让 Agent 可以和用户自然语音对话，或在需要澄清时用 MCP 发起语音提问。Spokenly 的 MCP 工具让 Claude Code、Codex、Cursor 调用 `ask_user_dictation`，用户用语音回答；它也可把 Agent 的问题先朗读出来。

对我们的启发：

- ask-user 场景是语音最自然的高价值切入点之一。
- 但这不是 Codex Speak 当前主线；可作为后续 P4 功能：当 Codex 需要用户确认时，播报问题并等待语音或按钮输入。

参考：[VoiceMode GitHub](https://github.com/mbailey/voicemode)、[Spokenly Voice for Agents](https://spokenly.app/docs/macos/voice-for-agents)

### Warp 和平台级压力

Warp 已经把第三方 CLI Agent 支持做成平台能力：自动检测 Claude Code、Codex、OpenCode 等，并提供 rich input editor、agent notifications、remote control、inline code review 等。它说明基础通知、远程控制、工具栏增强未来会被平台吸收。

对我们的启发：

- 只做“通知我完成了”很容易被大平台吃掉。
- 我们要保留的壁垒应是：中文听觉内容、儿童/初学者解释、本地模型和发音词典、Codex Skill/MCP 协议、可迁移到其他 Agent 的 speech-safe guide。

参考：[Warp third-party CLI agents](https://docs.warp.dev/agent-platform/cli-agents/overview/)

## 产品机会

### 机会 1：智能语音摘要，而不是朗读原文

直接读 Codex 的最终回复会遇到代码、路径、日志、命令参数、英文缩写、表格和长列表。Codex Speak 已经通过 side-channel 和结构化 role 避开这个坑，应继续强化。

建议最低导览结构：

- 做了什么。
- 为什么这么做。
- 改了哪里或检查了什么。
- 结果如何。
- 是否需要用户动作。
- 下一步建议。

### 机会 2：事件级播报

除了最终导览，长任务期间需要少量事件级提示。推荐状态：

```text
started
working
found_issue
running_tests
blocked
need_approval
failed
succeeded
```

默认只播：

- 开始处理较长任务。
- 进入测试/构建等用户关心阶段。
- 阻塞、失败、需要确认。
- 最终完成。

不播：

- 每次读文件。
- 每次编辑文件。
- 原始命令输出。
- 未清洗的推理碎片。

### 机会 3：中文优先和儿童/初学者友好

大多数竞品面向英文开发者，语音素材、文案和默认使用场景都偏英文。Codex Speak 的中文技术词归一化、本地发音词典、儿童模式是非常清楚的差异化。

建议把这点更前置：

```text
不是让 Codex 说话，而是让 Codex 用中文把刚才的技术工作讲清楚。
```

### 机会 4：本地隐私和离线可控

不少竞品使用 ElevenLabs、Layercode、OpenAI Speech 或托管 relay。用户在代码场景对隐私敏感。Codex Speak 的本地 TTS、默认不上传、支持包脱敏、自检可解释，是强优势。

建议传播时强调：

- 默认本地 TTS。
- 不把代码/对话发给云端 TTS。
- 支持企业离线包。
- 用户可控制 Provider。

### 机会 5：多 Agent 可辨识通知

多 Agent 已经成为趋势。短期 Codex Speak 可以先不做 orchestrator，但可以预留“session identity / agent label / voice profile”的协议字段。

未来可支持：

- 每个 Agent 不同提示音或音色。
- 后台 Agent 只播异常。
- 当前活跃 Agent 播完整导览。
- 多 Agent 汇总成一句：“前端完成，后端需要确认，测试失败在认证模块。”

### 机会 6：AirPods 和离屏工作流

“戴耳机离开屏幕，Agent 有事叫我”是最容易传播的场景。我们不一定先做手机端，也可以先把桌面端做成 AirPods 友好：

- 低延迟短提示。
- 队列不重叠。
- Stop 快捷控制。
- 失败/确认提示更明显。
- 最终导览控制在 10-25 秒。

## 差异化定位建议

不建议主打：

```text
给 Codex 加 TTS
```

这个表述会立刻进入红海：TalkToCursor、AgentVibes、Aftertone、Voxlert、系统 say 脚本都能讲类似故事。

建议主打：

```text
Codex Speak 是本地中文朗读导览层，把 Codex 的技术回复变成听得懂的简短说明。
```

进一步扩展版：

```text
面向 Coding Agent 的 speech-safe notification layer：只在重要时刻，用适合听的语言告诉你发生了什么。
```

面向教育/家庭：

```text
让孩子和初学者听懂 Codex 刚才做了什么。
```

面向开发者：

```text
不用盯终端，也不用听原始日志；Codex 只在关键节点讲重点。
```

## MVP / 下一步建议

结合当前项目阶段，建议不要马上扩张到手机端或双向语音。优先把我们已经接近完成的差异点打透：

1. 完成 P2 外部真机 QA，让安装和自检可靠。
2. 把最终导览的默认时长压到可听范围，目标 10-25 秒。
3. 增加“简短/标准/学习”三档朗读详细度，而不是只靠儿童模式。
4. 增加事件级提示策略文档和默认规则：哪些说，哪些不说。
5. 为多 Agent 预留协议字段：`agent_id`、`agent_label`、`session_id`、`priority`。
6. 做一页“为什么不直接朗读 Codex 原文”的对外介绍。
7. 做中文技术词和项目名发音词典的演示，这是英文竞品不容易复制的展示点。
8. 增加 spoken offset/hash 去重，避免 pause/resume、hook retry、新装后重复播报。
9. 增加 digest 队列和 cadence：低价值过程先攒着，超过阈值或任务完成时再说一句。

## 风险

- 大平台会吞掉基础通知、远程控制和语音输入。
- 竞品已经开始占“短摘要、过滤、swarm mode”的叙事。
- 如果最终导览太长，用户会关掉；如果太频繁，用户会烦。
- 如果安装失败或模型下载慢，用户不会进入体验核心。
- 儿童模式需要谨慎，不应暗示替代老师/家长判断。

## 后续调研方向

- 深挖 AgentVibes、Aftertone、Heard 的实现方式：Hook、MCP、CLI wrapper、队列和过滤策略。
- 跟踪 Codexini 是否真实活跃、定价、GitHub release、用户反馈。
- 跟踪 Sled 是否已转型为 Toyo 或仍维护开源仓库。
- 研究 Herdr / Paseo 的 Agent 状态检测模型，判断 Codex Speak 是否需要抽象独立状态层。
- 调研中文 TTS 在技术词、英文混读、儿童语气上的最佳参数和模型。
- 收集真实用户在离屏场景的音频忍耐度：单次最长几秒、多久提示一次会烦、哪些事件必须说。

## 参考来源

- [Sled 搜索缓存/官网入口](https://sled.layercode.com/)
- [Product Hunt: Sled](https://www.producthunt.com/products/sled-voice-interface-for-claude-code)
- [Codexini](https://codexini.com/)
- [Reddit: I gave my coding agents a voice](https://www.reddit.com/r/SideProject/comments/1szzugt/i_gave_my_coding_agents_a_voice_oss/)
- [Herdr agents docs](https://herdr.dev/docs/agents/)
- [Reddit: Aftertone](https://www.reddit.com/r/vibecoding/comments/1ti236b/a_completely_local_tts_for_cursor_and_claude_code/)
- [AgentVibes GitHub](https://github.com/paulpreibisch/AgentVibes)
- [TalkToCursor](https://talktocursor.com/)
- [Happy GitHub](https://github.com/slopus/happy)
- [CodeVibe](https://quantiya.ai/codevibe/)
- [CC Pocket](https://k9i-0.github.io/ccpocket/install/)
- [Paseo](https://paseo.sh/)
- [Untether GitHub](https://github.com/littlebearapps/untether)
- [VoiceMode GitHub](https://github.com/mbailey/voicemode)
- [Spokenly Voice for Agents](https://spokenly.app/docs/macos/voice-for-agents)
- [Voxlert Reddit](https://www.reddit.com/r/LocalLLM/comments/1rqbji0/fully_local_voice_notifications_for_ai_coding/)
- [Warp third-party CLI agents](https://docs.warp.dev/agent-platform/cli-agents/overview/)
