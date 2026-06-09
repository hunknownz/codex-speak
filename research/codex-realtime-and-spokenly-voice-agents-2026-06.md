# Codex Realtime 与 Spokenly Voice for Agents 调研

调研日期：2026-06-05  
调研对象：

- `codex.danielvaughan.com` 的 Codex CLI voice / Realtime V2 / WebRTC 文章。
- Spokenly 的 Voice for Agents MCP 文档。
- OpenAI Developers 官方 Realtime / WebRTC / Voice agents 文档，用于校验 OpenAI 相关能力边界。

## 结论摘要

这两个材料代表两条不同路线：

1. Codex Realtime / WebRTC：实时语音对话。重点是低延迟、双向音频、WebRTC transport、工具调用和会话事件。
2. Spokenly Voice for Agents：agent 需要用户输入时，通过 MCP 发起语音听写。重点是 ask-user，而不是持续播报。

对 Codex Speak 的直接判断：

- 不应把 Codex Speak 做成 “Realtime voice agent” 的替代品。Realtime 的优势是自然对话，Codex Speak 的优势应是可控、安静、中文、本地、基于状态的 speech guide。
- Spokenly 的 `ask_user_dictation` 模式值得借鉴到 `need_input` / `need_approval`，但它不是 Agent Narrator。
- OpenAI 官方 Realtime 文档已经把 voice agent 架构分成两类：speech-to-speech live audio 和 chained voice pipeline。Codex Speak 更适合后者：由我们掌控事件、摘要、播报策略和 TTS，而不是把所有交互放进实时音频会话。

## 来源可信度

| 来源 | 用法 | 可信度 |
| --- | --- | --- |
| OpenAI Developers Realtime / WebRTC / Voice agents | 官方能力边界、架构术语、transport 选择 | 高 |
| OpenAI Developers Codex changelog | Codex 官方产品变更线索 | 高 |
| Codex Knowledge Base by Daniel Vaughan | 结构化解读和版本线索 | 中，需要用官方文档或源码交叉确认 |
| Spokenly docs | Spokenly 产品行为和 MCP 工具说明 | 高，用于 Spokenly 自身能力 |

注意：Codex Knowledge Base 不是 OpenAI 官方文档。对外材料里不要把它的所有版本描述直接写成官方承诺。

## Codex Realtime / WebRTC 调研

### 文章主张

Daniel Vaughan 的文章把 Codex voice 分成两层：

- Voice transcription：push-to-talk，把语音转成 TUI composer 里的文本。
- Realtime voice sessions：通过 WebRTC 连接 Realtime API，实现双向音频、工具调用和工作进度播报。

文章还描述了一个 WebRTC flow：

```text
TUI / client
↓ thread/realtime/start + SDP
App Server
↓
OpenAI Realtime API
↓ answer SDP
client WebRTC peer connection
↓
audio response + tool call events + progress
```

这对 Codex Speak 的意义不是“我们也要做实时语音”，而是：平台正在把 voice input/output 和 agent loop 打通。基础语音能力会越来越像系统能力。

### 官方文档可确认的边界

OpenAI 官方 Realtime/WebRTC 文档确认：

- Realtime API 支持通过 WebRTC peer connection 连接 realtime models。
- 对浏览器或移动端直接采集/播放音频的场景，官方建议 WebRTC，而不是 WebSocket。
- WebRTC 可以走 unified interface，也可以由后端 mint ephemeral token 给前端连接。
- Realtime voice-agent session 会产生 model responses、tool calls、session events。
- Voice agents 有两种架构：speech-to-speech live audio，或 STT -> text reasoning -> TTS 的 chained pipeline。

这些官方点足以支撑一个战略判断：实时语音对话会平台化；Codex Speak 如果只做“有声音”，迟早被平台吸收。

### 对 Codex Speak 的影响

#### 平台压力

Codex 官方生态如果持续强化 Realtime，就会覆盖：

- 用户语音输入。
- Agent 语音回答。
- 音频设备配置。
- 实时会话里的工具调用。
- 部分过程进度播报。

所以 Codex Speak 不应把核心卖点放在 “Codex 能说话”。

#### 差异化机会

Realtime 的强项是对话自然，弱点是可控性、成本、隐私、噪声过滤、中文技术词细节和离线可用性。

Codex Speak 可以避开正面竞争：

```text
Realtime:
live conversation / low latency / speech-to-speech

Codex Speak:
state-aware guide / local TTS / Chinese / quiet policy / digest
```

#### 架构选择

OpenAI voice agents 文档里的 “chained voice pipeline” 更像 Codex Speak 当前路线：

```text
Agent event
↓
State/digest policy
↓
Speech-safe text
↓
Local or selected TTS
↓
Playback queue
```

这比 speech-to-speech 更适合我们的核心目标，因为我们需要决定：

- 什么不说。
- 什么时候说。
- 用什么详细度说。
- 是否需要儿童/学习模式。
- 是否应本地离线。

### 风险和注意点

- Realtime session 的 audio loop 可能带来 self-interruption、echo loop、成本和上下文膨胀问题。第三方文章提到这些风险；官方文档也强调需要管理 session lifecycle、cost、safety identifier。
- 如果 Codex Speak 未来接 Realtime，应作为可选 provider，而不是替代当前 hook/side-channel。
- 不能把第三方文章里的版本号和功能描述直接写进 README，除非用 OpenAI changelog 或源码确认。

## Spokenly Voice for Agents 调研

### 产品定位

Spokenly Voice for Agents 是一个 MCP 语音听写集成。文档明确说明它连接 Claude Code、Codex、Cursor，让这些工具在需要澄清时向用户发起语音问题。

核心不是：

```text
Agent 持续播报状态
```

而是：

```text
Agent 需要输入
↓
调用 MCP ask_user_dictation
↓
用户说答案
↓
转写文本返回 agent
```

### 工作方式

文档描述的流程：

```text
Spokenly 本地 MCP server
http://localhost:51089
↓
AI tool 连接 MCP
↓
获得 ask_user_dictation 工具
↓
agent 调用工具并传 questions: string[]
↓
Spokenly 开始录音
↓
用户说话并按 Enter
↓
转写文本返回 agent
```

它还支持可选的 “Speak Questions Aloud”：在开始录音前把 agent 的问题朗读出来。该功能默认关闭，使用 cloud TTS，并消耗 credits。

### 对 Codex Speak 的启发

Spokenly 给我们一个很清晰的未来功能模型：

```text
need_input / need_approval event
↓
Codex Speak 播报问题
↓
用户用语音或按钮回答
↓
答案回到 agent
```

这个能力比“全程语音输入”更适合 Codex Speak，因为触发点明确、噪声少、价值高。

建议未来如果做语音输入，不要先做泛用 dictation，而是先做：

- `need_approval`：批准、拒绝、选择第几个选项。
- `need_clarification`：回答 agent 的澄清问题。
- `resume_prompt`：离屏回来后用一句话继续任务。

### 和 Codex Speak 的差异

| 维度 | Spokenly | Codex Speak |
| --- | --- | --- |
| 核心方向 | 用户语音回答 agent 问题 | agent 状态/结果朗读导览 |
| 触发方式 | agent 主动调用 MCP 工具 | hook / side-channel / future event bridge |
| 输出内容 | 问题可选朗读 | 状态、结果、下一步、digest |
| 隐私 | 语音转写产品，朗读问题用 cloud TTS | 目标是本地 TTS 优先 |
| 壁垒 | dictation UX、系统集成 | 状态理解、过滤策略、中文 speech guide |

## 产品路线建议

### 短期：不要追 Realtime

短期应继续强化：

- side-channel final guide。
- hook 触发可靠性。
- 本地 TTS。
- `quiet/brief/guide/learning`。
- `done_unseen`、`need_approval`、`failed`。
- digest 和 spoken offset 去重。

原因：Realtime 会把产品拉到低延迟音频、设备管理、回声/打断、成本、安全标识等复杂系统里，短期会稀释我们的核心差异化。

### 中期：做 ask-user 的窄场景

参考 Spokenly，不做全局语音输入，先做事件触发型：

```text
Codex 需要确认
↓
Codex Speak 朗读确认问题
↓
用户说“批准”或“拒绝”
↓
回传到 Codex / UI / hook bridge
```

这条路线和 Codex Speak 的定位一致：不是替代键盘，而是让离屏工作流不中断。

### 长期：Realtime 作为可选通道

如果未来要接 Realtime，建议只作为 optional high-interactivity mode：

- code review voice session。
- CI triage voice session。
- accessibility mode。
- pairing/teaching mode。

但默认主体验仍应是 quiet notification layer。

## 对当前文档/产品表述的修正

可以写：

```text
Codex Speak follows a controlled speech-guide pipeline rather than a full realtime voice-agent loop.
```

不要写：

```text
Codex Speak competes directly with OpenAI Realtime.
```

可以写：

```text
Spokenly validates voice answers for agent clarification, which is a strong future use case for Codex Speak's need-input events.
```

不要写：

```text
Spokenly is an Agent Narrator competitor.
```

## 可落地需求清单

### P1 / P2 可考虑

- 为 guide event 增加 `need_input`、`need_approval` 类型。
- 为 spoken guide 增加 `question` role 或 `action_required` role。
- 为播放队列增加 `requires_user_response` 元数据。
- 设计“朗读问题后等待用户操作”的状态机，即使第一版只等按钮/键盘。

### P3 / P4 可考虑

- 本地 STT provider：Whisper / Parakeet / 系统 dictation。
- Spoken answer schema：`approve`、`reject`、`choose_option`、`free_text`。
- Realtime provider adapter：只在高互动模式启用。
- Safety/cost guard：实时语音必须有时长、token、上下文镜像上限。

## 参考链接

- [Codex Knowledge Base: Voice-Driven Development in Codex CLI](https://codex.danielvaughan.com/2026/04/17/codex-cli-voice-realtime-webrtc-push-to-talk/)
- [Spokenly: Voice for Agents MCP](https://spokenly.app/docs/macos/voice-for-agents)
- [OpenAI Developers: Realtime API with WebRTC](https://developers.openai.com/api/docs/guides/realtime-webrtc)
- [OpenAI Developers: Realtime and audio](https://developers.openai.com/api/docs/guides/realtime)
- [OpenAI Developers: Voice agents](https://developers.openai.com/api/docs/guides/voice-agents)
- [OpenAI Developers: Codex changelog](https://developers.openai.com/codex/changelog)
