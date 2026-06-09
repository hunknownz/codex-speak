# 初步调研脚注逐项核查

核查日期：2026-06-05  
目标：把同事初步调研中列出的项目、帖子和文章逐项打开，并尽量阅读对应开源项目源码。能 clone 的项目已作为 git submodule 放在 `_research/competitors/`。

## 总结

这次补查后，原判断需要加强而不是推翻：赛道已经有人在做，但还没有统一标准。真正直接对标 Codex Speak 的不是普通 TTS，也不是语音输入，而是 Agent Narrator / Speech Guide：理解 agent 状态、过滤噪声、用适合听的短句告诉用户关键进展。

最值得持续跟踪的直接竞品是 Heard、Aftertone、Sled、V1GPT、AgentVibes。TalkToCursor 和 VoiceTerm 更像 voice coding / hands-free input 参考。Codex Voice Hooks 是低成本 hook 通知参考，能说明“只做完成提示”壁垒很低。

## 逐项核查

| 原脚注 | 已查对象 | 开源状态 | 本地路径 | 关键结论 |
| --- | --- | --- | --- | --- |
| TalkToCursor | 官网、README、MCP server、auto-submit、silence detector | 开源 | `_research/competitors/talk-to-cursor` | Cursor + ElevenLabs + Wispr Flow 的双向循环。它靠 MCP `speak/listen` 和 macOS Accessibility 自动提交，核心是 voice loop，不是 Codex 状态旁白。 |
| Sled / Toyo | 官网入口、Sled 源码、ACP wrapper、sentence detector、Durable Object | 开源 | `_research/competitors/sled` | 手机远程 + ACP wrapper + voice 是强场景竞品。它证明 AirPods/离屏是真需求，但架构比 Codex Speak 重很多。 |
| Heard Reddit | Reddit 入口、Heard 源码、Codex adapter、verbosity、spoken dedup、multi-agent | 开源 | `_research/competitors/heard` | 当前最接近“Agent Narrator”。核心不是朗读原文，而是 co-pilot/companion 两种模式、digest、swarm routing、去重和 offset。 |
| Codex Voice Hooks | Reddit 贴、Codex CLI Hooks 源码 | 开源 | `_research/competitors/codex-cli-hooks` | 覆盖 Codex 8 个 hook 的预录声音包，适合参考 hook surface 和跨平台播放；产品壁垒低，不处理语义过滤。 |
| V1GPT Reddit | Reddit 贴、V1GPT 源码、session watcher、FastAPI TTS、Tauri/Three.js 前端 | 开源 | `_research/competitors/v1gpt` | Codex-first avatar companion。值得借鉴 spoken summary、approval notice、mood/cadence，不建议短期追 3D avatar。 |
| VoiceTerm Reddit | Reddit 贴、VoiceTerm 源码/文档 | 开源 | `_research/competitors/voiceterm` | Voice-first terminal overlay，解决用户对 Codex/Claude 的语音输入、wake phrase、send/submit、HUD 和 transcript queue。不是结果朗读竞品。 |
| WIRED | Cursor agent 新闻 | 非代码项目 | 无 | 市场背景：Cursor、OpenAI、Anthropic 正在争夺 coding agent 心智，多 agent / agent platform 会让“通知层”变成刚需。 |

## 两个用户特别点名 Reddit 链接

### V1GPT

Reddit 帖明确说 V1GPT 是 Codex CLI 和 Codex Mac App 的 voice avatar layer，包含 3D avatar、local/configurable TTS、spoken summaries、lip sync、mood changes、cursor tracking、thinking audio、approval notice。源码进一步确认：

- `server/scripts/codex-session-watch.sh` 和 `codex_session_watcher.py` 轮询 `~/.codex/sessions`，默认只接受 `cli` / `vscode` 来源，启动时 prime 到 EOF，避免安装后重播旧内容。
- 看到 `task_started`、`response_item:function_call`、`function_call_output`、`event_msg:agent_message`、`final_answer`、`task_complete` 后，转成状态、mood、tool_use 和 spoken summary。
- 对 `sandbox_permissions=require_escalated` 有一次性 approval notice。
- `spoken-summary-prompt.md` 明确要求“for listening”，不要复制 markdown、路径、命令、URL、代码。
- `spoken-style.toml` 有 `max_sentences`、`max_chars`、drop path-like/command-like/offer/question sentences 的规则。

对 Codex Speak 的判断：V1GPT 证明 Codex session watcher 可行，也证明 Pet/avatar 可作为情绪化状态展示；但它更像 companion。Codex Speak 的短期优势仍应是本地中文 speech-safe guide，而不是视觉形象。

### Heard

Heard 源码确认它不是“读日志”工具，而是有一整套 narrator brain：

- README 把它定位成“coding agent 有声音”，且明确对应 Wispr Flow 这类输入工具：用户说给 agent 的部分由 Wispr 处理，agent 说给用户的部分由 Heard 处理。
- `verbosity.py` 把事件分成 `speak`、`drop`、`digest`，失败、权限问题、长任务可以穿透安静档。
- `agent_state.py` 做 Layer 2 agent state：只记录事实和 hints，不让 LLM 直接猜状态。
- `multi_agent.py` 有 solo/swarm/pinned 模式，主 session 正常说，后台 session 的 routine event 推迟或丢弃，失败/问题/用户意图才打断。
- `spoken.py` 用 per-session hash 和 transcript offset 避免重播旧内容。

对 Codex Speak 的判断：Heard 是最值得对标的竞品。它已经把“决定什么不说”做成核心产品能力。我们要在中文、本地隐私、Codex Skill side-channel、儿童/初学者解释上形成差异，而不是只做 hook 声音。

## 原脚注外但本轮顺手确认的关键项目

- Aftertone：tag-only `<spoken_summary>` 和“没有专门语音稿就不读”与 Codex Speak 当前 side-channel 方向高度一致。
- Herdr：`done_unseen`、blocked/working/idle 状态模型对事件级播报非常重要。
- AgentVibes：provider、queue、mute、per-LLM voice、远程播放是 TTS 工程清单参考。
- Paseo / Happy / Untether：远程、多端、多 agent 平台会逐渐吃掉基础通知，但也会需要更好的语音输出组件。

## 对 Codex Speak 的更新结论

1. “给 Codex 加 TTS”已经是红海表述。
2. “把 Codex 技术回复转换成适合中文听众理解的本地朗读导览”仍有清晰差异化。
3. 事件级播报必须围绕状态转移和优先级，不能按日志行朗读。
4. 下一阶段最该补的是 verbosity/digest、per-session 开关、spoken offset/去重、long-running heartbeat、need approval/failure 的高优先级打断。
5. Pet/avatar 可以继续保留，但它应服务状态可感知，不应吞掉 speech guide 主线。

## 参考链接

- [TalkToCursor](https://talktocursor.com/)
- [TalkToCursor GitHub](https://github.com/MindSyncTech/talk-to-cursor)
- [Sled](https://sled.layercode.com/)
- [Sled GitHub](https://github.com/layercodedev/sled)
- [Reddit: V1GPT](https://www.reddit.com/r/codex/comments/1s0avrn/i_gave_codex_a_3d_avatar_v1gpt_is_now_my_voice/)
- [V1GPT GitHub](https://github.com/intarm/V1GPT)
- [Reddit: Heard](https://www.reddit.com/r/SideProject/comments/1szzugt/i_gave_my_coding_agents_a_voice_oss/)
- [Heard GitHub](https://github.com/heardlabs/heard)
- [Reddit: Codex Voice Hooks](https://www.reddit.com/r/OpenAI/comments/1rgw0w1/i_gave_codex_cli_a_voice_so_it_tells_me_when_its/)
- [Codex CLI Hooks GitHub](https://github.com/shanraisshan/codex-cli-hooks)
- [Reddit: VoiceTerm](https://www.reddit.com/r/ClaudeCode/comments/1rll38z/voiceterm_handsfree_voice_coding_for_ai_clis_mac/)
- [VoiceTerm GitHub](https://github.com/jguida941/voiceterm)
- [WIRED: Cursor agent experience](https://www.wired.com/story/cusor-launches-coding-agent-openai-anthropic/)
