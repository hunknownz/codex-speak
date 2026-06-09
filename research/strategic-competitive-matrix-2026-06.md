# Codex Speak 战略竞品矩阵

调研日期：2026-06-05  
视角：投资人看项目 + 产品负责人做战略规划  
目标：判断 Codex Speak 应该进入哪个市场叙事、避开什么红海、把壁垒建在哪里。

## 结论先行

Codex Speak 不应被定义为 “AI Voice for Codex”。这个说法会把项目拉进两个已经拥挤的市场：

1. Voice Input：让用户用语音给 agent 下指令。
2. Basic TTS：把 agent 输出转成声音。

更好的定位是：

```text
The Voice & Notification Layer for Coding Agents
```

更贴合当前中文、本地、导览能力的版本是：

```text
面向 Coding Agent 的本地中文语音导览与状态通知层。
```

真正值得押注的不是 TTS，而是：

```text
Agent Events
↓
State Machine
↓
Summarization / Digest
↓
Speech-safe Voice
```

这也是 Heard、Aftertone、Herdr、Paseo、V1GPT 等项目共同指向的空白：用户不想听日志，用户想知道 Agent 是否在工作、是否卡住、是否失败、是否需要自己回来。

## 分类框架

这份矩阵把竞品拆成四类能力：

| 能力 | 定义 | 对 Codex Speak 的战略意义 |
| --- | --- | --- |
| Voice Input | 用户说话，agent 接收 prompt/approval | 市场很拥挤，短期不应作为主线 |
| Voice Output | agent 或工具把内容朗读出来 | 必须有，但单独没有壁垒 |
| Agent Remote | 手机、网页、Telegram 等远程控制 agent | 未来机会大，但会显著增加协议/安全/支持成本 |
| Agent Monitoring | 理解 agent 状态、聚合事件、过滤噪声、通知用户 | 最接近 Codex Speak 应该建立壁垒的地方 |

Codex Speak 当前主要在 Voice Output + Agent Monitoring 的交界处。未来可以向 Agent Remote 延伸，但不应过早变成远程控制平台。

## 战略竞品矩阵

说明：

- `✅`：已经明确具备。
- `⚠️`：有部分能力、基础能力，或依赖 agent 自觉调用。
- `❌`：没有看到明确能力。
- `?`：未能可靠确认。
- “Codex Speak 建议定位”表示战略目标，不代表当前版本已经全部完成。

| 产品 | Voice Input | Voice Output | Agent 状态播报 | 手机远程 | 多 Agent | 智能摘要 / Digest | 开源 | 商业化 | 战略判断 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Sled | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ | ✅ | 暂不明确 | 手机远程 + voice 的直接场景竞品，架构偏 wrapper/ACP |
| AICHE Voice Code | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | Pro 订阅 | Voice Input 红海代表，强在 pause-aware auto-send 和语音审批 |
| Spokenly MCP | ✅ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | SaaS / Pro | ask-user dictation 工具，适合澄清/审批，不是旁白层 |
| Codex Realtime / 官方生态 | ✅ | ✅ | ⚠️ | ❌ | ⚠️ | ⚠️ | ❌ | OpenAI 生态 | 最大平台压力：双向语音、进度流、后台 agent 会逐渐平台化 |
| Cursor Voice / Cursor 生态 | ✅ | ⚠️ | ❌ | ❌ | ⚠️ | ❌ | ❌ | Cursor 订阅 | 输入效率与 IDE 平台压力，不是独立 narrator |
| Claude Voice Mode | ✅ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | Claude 订阅 | 主要是 voice input；Linux 支持对 VoiceTerm/AICHE 有影响 |
| TalkToCursor | ✅ | ✅ | ⚠️ | ❌ | ❌ | ❌ | ✅ | npm / ElevenLabs 成本 | Cursor + MCP speak/listen + Wispr loop，偏 hands-free loop |
| VoiceTerm | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | OSS / Homebrew | 本地 Whisper voice input terminal，适合参考 wake/send/HUD |
| Heard | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ✅ | 托管 TTS/可能商业化 | 当前最直接 Agent Narrator 竞品 |
| Aftertone | ❌ | ✅ | ⚠️ | ❌ | ❌ | ✅ | ✅ | OSS | tag-only、本地短摘要，是 Codex Speak side-channel 的强参考 |
| Herdr | ❌ | ⚠️ | ✅ | ❌ | ✅ | ❌ | ✅ | OSS | 状态管理参考，不是 TTS 主体 |
| V1GPT | ❌ | ✅ | ⚠️ | ❌ | ❌ | ✅ | ✅ | OSS | Codex companion/avatar，参考 session watcher 和 cadence |
| AgentVibes | ❌ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | OSS / services | TTS provider/queue/voice routing 工程参考 |
| Paseo | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ✅ | 平台化 | Agent OS / orchestrator，长期平台级竞品 |
| Happy | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | 未明确 | 移动/Web 远程控制，参考 E2E 和 session sync |
| Untether | ✅ | ❌ | ✅ | ✅ | ⚠️ | ⚠️ | ✅ | OSS | Telegram remote，参考 progress/action/timeline |
| Codex Speak 建议定位 | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | Freemium / 企业离线 | 不做 dictation 红海，做本地中文 speech-safe notification layer |

## 关键修正

### 1. “Voice Agent” 不是一个同质市场

用户给出的矩阵把 Codex Realtime、Spokenly 放在 Voice Agent 里是合理的，但需要细分：

- Spokenly 是 agent ask-user dictation：agent 问，用户说。
- Codex Realtime 是官方双向 voice conversation：用户和 agent 实时对话。
- Heard / Codex Speak 更像 Agent Narrator：agent 工作时，系统判断哪些状态值得说。

这三类用户体验看起来都叫 “voice”，但产品壁垒完全不同。

### 2. AICHE 很强，但它不是 Codex Speak 的直接竞品

AICHE 的页面明确说它是 “Voice for the Agent Loop”，支持 Claude Code、Cursor、Codex、Antigravity；核心是 pause-aware auto-send、code-adjacent recognition、自定义词汇、语音审批。

它解决的是：

```text
人说
↓
Prompt 自动发送
↓
Agent 干活
```

Codex Speak 解决的是：

```text
Agent 干活
↓
状态/结果被理解
↓
用户听到重点
```

所以 AICHE 更像未来语音输入模块的竞品/参考，不是当前朗读导览的正面对手。

### 3. Spokenly 是 ask-user 工具，不是持续旁白层

Spokenly MCP 让 Claude Code、Codex、Cursor 连接本地 MCP server，并调用 `ask_user_dictation`。它可以在 agent 需要澄清时让用户用语音回答，也可以可选朗读 agent 的问题。

这对 Codex Speak 的启发是：未来做 “need_approval / need_input” 时，可以把 spoken question + voice answer 做成高价值功能。但它不是 Agent 状态管理，也不是智能摘要播报。

### 4. Codex 官方生态是最大平台压力，但要谨慎引用

用户给出的 “Codex Realtime” 来源是第三方 Codex Knowledge Base，不是 OpenAI 官方文档。它提到 Codex CLI 从 push-to-talk 到 Realtime/WebRTC、双向音频、后台进度流等方向。

官方可确认的是：OpenAI Developers 文档已有 Realtime/audio、voice agents、WebRTC、Codex changelog、background/subagent 等相关导航和更新。第三方博客可以作为趋势参考，但不应在对外材料里写成“官方已发布某某完整功能”，除非逐条在 OpenAI 官方 changelog 或代码里确认。

战略含义依然成立：基础语音输入、基础语音输出、后台状态流迟早会被平台吸收。Codex Speak 的壁垒必须比“有声音”更深。

## 定位地图

### 维度 1：用户说 vs Agent 说

```text
Voice Input
用户说给 Agent

AICHE / VoiceTerm / Cursor Voice / Claude Voice Mode
```

```text
Voice Output
Agent 说给用户

TalkToCursor / AgentVibes / Aftertone / Heard / Codex Speak
```

### 维度 2：朗读原文 vs 理解状态

```text
低壁垒
Agent 输出
↓
原样朗读
```

```text
高壁垒
Agent Events
↓
State Machine
↓
Digest / Summary
↓
Speech-safe Voice
```

Codex Speak 应该站在第二条路径。

## 市场空白

当前更明显的空白不是“有没有语音”，而是：

```text
Agent 状态管理 + 语音通知
```

具体包括：

- 任务开始后多久还在工作。
- 是否进入 long-running 阶段。
- 是否需要 approval。
- 是否 blocked。
- 是否失败，以及失败集中在哪里。
- 是否完成但用户还没听到/看到。
- 多个 agent 同时跑时，谁需要注意。
- 哪些 routine event 应该 drop，哪些应该进入 digest。

Heard 已经在做一部分，Herdr/Paseo 在状态侧做一部分，但还没有一个产品在“本地、中文、Codex-first、speech-safe guide”上形成心智。

## Codex Speak 功能路线建议

### V1：事件级通知

目标：先从“不要盯终端”切入。

支持：

```text
Codex
Claude Code
Gemini CLI
```

播报：

```text
任务开始
任务完成
测试失败
需要审批
任务阻塞
```

明确不播：

```text
读文件
写文件
原始命令输出
长日志
未清洗 Markdown
```

### V2：Agent Intelligence

目标：从通知器升级为 narrator。

示例：

```text
Agent 原始事件：
Reading file A
Reading file B
Reading file C
```

用户听到：

```text
正在分析项目结构，重点在认证和路由模块。
```

需要补的能力：

- action/event normalization。
- verbosity：`quiet`、`brief`、`guide`、`learning`。
- digest queue。
- spoken offset/hash 去重。
- cadence：`off`、`once`、`30s`、`60s`、`important-only`。

### V3：多 Agent 通知层

目标：从单 Agent narrator 升级为 notification OS。

示例：

```text
前端任务完成。
后端 Agent 需要确认数据库迁移。
测试 Agent 在认证模块失败。
```

需要补的能力：

- `agent_id`
- `agent_label`
- `session_id`
- `parent_agent_id`
- `priority`
- `focus/pinned`
- `spoken_at`
- `seen_at`

## 投资人视角：壁垒在哪里

### 低壁垒

```text
Hook
↓
TTS
```

这类项目两周内可复制。Codex CLI Hooks 已经证明：完成提示、权限提示、预录声音都很容易做。

### 中等壁垒

```text
MCP speak tool
↓
Provider queue
↓
Voice picker
```

AgentVibes、TalkToCursor 已经覆盖很多。这里需要做，但不宜作为主卖点。

### 高壁垒

```text
Agent Events
↓
State Machine
↓
Policy / Verbosity
↓
Digest / Summary
↓
Local Speech-safe Voice
```

壁垒来自：

- 对 agent 生命周期的理解。
- 对“什么不该说”的策略积累。
- 中文技术词、项目名、英文混读的发音质量。
- 本地隐私与企业离线安装。
- 多 agent 优先级和去重。
- 用户在离屏/AirPods 场景里的真实打磨。

## 商业化判断

Codex Speak 更适合从开源 + Freemium 起步，而不是一上来做全 SaaS。

可商业化点：

- Pro provider pack：更好的本地模型、更多音色、企业部署包。
- Team policy：统一 quiet/brief/guide 档位、敏感词、上传禁用。
- 企业离线包：不上传代码、不依赖云 TTS。
- 教育/儿童版：更强解释、更安全语言、更稳定中文发音。
- Remote add-on：手机通知、远程 approval、跨设备 playback confirmation。

不建议短期收费点：

- 单纯 TTS。
- 单纯完成提示。
- 大量角色音色。
- 纯语音输入。

## 推荐对外表述

不推荐：

```text
AI Voice for Codex
```

推荐：

```text
Codex Speak is the local speech guide and notification layer for coding agents.
```

中文：

```text
Codex Speak 是面向 Coding Agent 的本地中文朗读导览与状态通知层。
```

更产品化：

```text
不用盯终端，也不用听日志。Codex Speak 只在关键节点，用中文告诉你 Agent 做到哪了。
```

## 参考来源

- [Sled](https://sled.layercode.com/)
- [Sled GitHub](https://github.com/layercodedev/sled)
- [AICHE Voice Code](https://aiche.app/features/voice-code-for-ai-agents)
- [Spokenly Voice for Agents](https://spokenly.app/docs/macos/voice-for-agents)
- [Codex Knowledge Base: Voice-Driven Development in Codex CLI](https://codex.danielvaughan.com/2026/04/17/codex-cli-voice-realtime-webrtc-push-to-talk/)
- [OpenAI Developers: Codex changelog](https://developers.openai.com/codex/changelog)
- [TalkToCursor](https://talktocursor.com/)
- [TalkToCursor GitHub](https://github.com/MindSyncTech/talk-to-cursor)
- [VoiceTerm GitHub](https://github.com/jguida941/voiceterm)
- [Heard GitHub](https://github.com/heardlabs/heard)
- [Aftertone GitHub](https://github.com/omarelkhal/aftertone)
- [Herdr GitHub](https://github.com/ogulcancelik/herdr)
- [V1GPT GitHub](https://github.com/intarm/V1GPT)
- [AgentVibes GitHub](https://github.com/paulpreibisch/AgentVibes)
- [Paseo GitHub](https://github.com/getpaseo/paseo)
- [Happy GitHub](https://github.com/slopus/happy)
- [Untether GitHub](https://github.com/littlebearapps/untether)
