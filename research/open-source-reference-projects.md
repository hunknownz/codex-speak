# 开源参考项目索引

更新时间：2026-06-05

本页记录已经作为 git submodule 拉到 `_research/competitors/` 下的开源参考项目。它们不作为 Codex Speak 的依赖，只用于调研产品形态、集成方式、Hook/MCP/队列/状态管理等实现思路。

## Submodule 清单

| 项目 | 本地路径 | 上游 | 类型 | 优先级 | 重点看什么 |
| --- | --- | --- | --- | --- | --- |
| Sled | `_research/competitors/sled` | `https://github.com/layercodedev/sled.git` | 手机 Web UI + 双向语音 + ACP wrapper | 高 | ACP 如何接 Claude/Codex/Gemini；手机远程访问；voice API 边界 |
| Aftertone | `_research/competitors/aftertone` | `https://github.com/omarelkhal/aftertone.git` | 本地短摘要 TTS | 最高 | `<spoken_summary>`、tag-only、Codex/Claude/Cursor hook、daemon/队列 |
| AgentVibes | `_research/competitors/agentvibes` | `https://github.com/paulpreibisch/AgentVibes.git` | 多 Provider TTS + MCP + TUI | 高 | TTS provider 管理、跨平台播放、voice picker、无重叠播放、远程 receiver |
| VoiceMode | `_research/competitors/voicemode` | `https://github.com/mbailey/voicemode.git` | MCP 双向自然语音 | 中 | MCP converse 工具、本地 Whisper/Kokoro、低延迟语音对话 |
| Untether | `_research/competitors/untether` | `https://github.com/littlebearapps/untether.git` | Telegram 远程控制 Agent | 中高 | 进度 streaming、权限按钮、错误提示、跨环境 resume、多引擎抽象 |
| Happy | `_research/competitors/happy` | `https://github.com/slopus/happy.git` | Codex/Claude 手机/Web 客户端 | 中高 | CLI wrapper、移动端同步、推送、E2E、多端切换 |
| Herdr | `_research/competitors/herdr` | `https://github.com/ogulcancelik/herdr.git` | 终端内多 Agent 状态管理 | 高 | blocked/working/done/idle 状态检测、rollup、集成状态报告 |
| Paseo | `_research/competitors/paseo` | `https://github.com/getpaseo/paseo.git` | 多 Agent 桌面/Web/移动/CLI 编排平台 | 中高 | daemon/client 架构、跨端 remote、voice control、worktree/PR 流程 |
| Heard | `_research/competitors/heard` | `https://github.com/heardlabs/heard.git` | Coding Agent 智能旁白 | 最高 | verbosity、digest、multi-agent swarm、Codex/Claude hooks、去重/offset |
| V1GPT | `_research/competitors/v1gpt` | `https://github.com/intarm/V1GPT.git` | Codex 3D avatar + TTS | 高 | Codex session JSONL watcher、spoken summary、mood、approval notice、cadence |
| Codex CLI Hooks | `_research/competitors/codex-cli-hooks` | `https://github.com/shanraisshan/codex-cli-hooks.git` | Codex hook 预录提示音 | 中 | Codex 8 个 hook 面、跨平台播放、完成/权限/compact 事件 |
| VoiceTerm | `_research/competitors/voiceterm` | `https://github.com/jguida941/voiceterm.git` | Codex/Claude 语音输入终端 | 中 | wake phrase、voice send modes、HUD、防遮挡、忙时 transcript queue |
| TalkToCursor | `_research/competitors/talk-to-cursor` | `https://github.com/MindSyncTech/talk-to-cursor.git` | Cursor TTS MCP + Wispr loop | 中 | MCP speak/listen、ElevenLabs 队列、auto-submit、silence detector |

## 第一轮阅读建议

## 深读状态

已完成一轮源码深读，详见：

- `research/deep-dive-open-source-voice-agent-projects-2026-06.md`
- `research/footnote-source-check-2026-06-05.md`

阅读结论摘要：

- Aftertone 最接近 Codex Speak 的当前主线：专门语音稿、tag-only、默认安静、Hook 后处理。
- Herdr 最适合参考状态模型：`working`、`blocked`、`done`、`idle`，尤其是 `done` 表示“完成但用户还没看过”。
- AgentVibes 最适合作为 TTS 工程清单：provider、队列、静音、per-LLM voice、远程播放。
- Untether/Paseo 适合参考结构化 action/timeline；它们证明应先把 Agent 事件归一化，再决定怎么显示或朗读。
- Happy/Paseo/Untether 的手机远程方向很强，但短期不应冲散 Codex Speak 的“本地中文导览”定位。
- VoiceMode 适合未来语音确认/语音问答，不是最终朗读导览的直接模板。
- Heard 是当前最接近“Agent Narrator”的直接竞品：它的重点不是 TTS，而是判断什么该说、什么应该积累成 digest。
- V1GPT 验证了 Codex Desktop/CLI session watcher + avatar/pet 的可行性，但它偏 companion/视觉陪伴，短期不应成为 Codex Speak 主线。
- Codex CLI Hooks 证明基础 hook 通知非常容易做，也最容易被平台吸收；我们的壁垒必须在 speech-safe 内容层。
- VoiceTerm 和 TalkToCursor 都在做 hands-free loop，但主要解决语音输入，不是最终朗读导览。

### 1. 先读 Aftertone

原因：它和 Codex Speak 的“不要读整段回复，只读专门语音稿”最接近。

优先文件：

- `_research/competitors/aftertone/README.md`
- `_research/competitors/aftertone/docs/adapters/codex.md`
- `_research/competitors/aftertone/docs/adapters/claude.md`
- `_research/competitors/aftertone/.cursor/rules/spoken-summary.mdc`
- `_research/competitors/aftertone/scripts/`

要回答的问题：

- Codex hook 是怎么安装和启用的？
- `<spoken_summary>` 怎么约束 Agent 输出？
- 没有 tag 时如何保持安静？
- 它的 daemon 如何接收、排队、播放？
- Windows hook latency 如何处理？

### 2. 再读 Herdr

原因：我们未来如果做过程提示或多 Agent 通知，需要 Agent 状态抽象。Herdr 的 blocked/working/done/idle 很值得参考。

优先文件：

- `_research/competitors/herdr/README.md`
- `_research/competitors/herdr/docs/`
- `_research/competitors/herdr/src/`
- `_research/competitors/herdr/SKILL.md`

要回答的问题：

- 它如何用 process detection、terminal output heuristics、integration state reports 判断状态？
- Codex integration 暴露了哪些信息？
- 状态如何 roll up 到 workspace/tab/pane？
- 哪些状态适合转成语音事件？

### 3. 读 AgentVibes

原因：它在 TTS provider、跨平台、TUI 配置、远程播放上有大量工程细节，可以反向检查 Codex Speak 的 P3/P4 风险。

优先文件：

- `_research/competitors/agentvibes/README.md`
- `_research/competitors/agentvibes/docs/quick-start.md`
- `_research/competitors/agentvibes/docs/mcp-setup.md`
- `_research/competitors/agentvibes/docs/voice-library.md`
- `_research/competitors/agentvibes/mcp-server/`

要回答的问题：

- 它如何管理 Piper / Soprano / macOS Say / Windows SAPI？
- 如何避免多段 TTS 重叠？
- per-LLM voice routing 怎么设计？
- SSH remote receiver 对我们未来远程机器播放有什么启发？

### 4. 读 Sled

原因：Sled 是手机 + voice + Codex/Claude/Gemini 的轻量实现，尤其值得看 ACP 和本地 Web UI 方案。

优先文件：

- `_research/competitors/sled/README.md`
- `_research/competitors/sled/app/`
- `_research/competitors/sled/package.json`

要回答的问题：

- ACP adapter 如何包装 Codex？
- 语音输入/输出如何接入浏览器和 Layercode？
- 本地运行和远程 tunnel 的安全提示怎么写？
- 手机端 UX 如何设计为“Agent 有事叫我”？

### 5. 读 Paseo / Happy / Untether

原因：它们更像远程 Agent 控制平台，不是 Codex Speak 的短期主战场，但能帮助判断未来是不是该接手机、Telegram、daemon 或 relay。

优先文件：

- `_research/competitors/paseo/packages/server/`
- `_research/competitors/paseo/packages/cli/`
- `_research/competitors/happy/packages/happy-cli/`
- `_research/competitors/happy/packages/happy-agent/`
- `_research/competitors/untether/src/`
- `_research/competitors/untether/docs/`

要回答的问题：

- agent session 是怎么建模的？
- 远程控制和本地 Agent 之间如何通信？
- 如何处理 approval、diff、progress streaming？
- 语音输入在这些产品里是核心能力还是附加能力？

### 6. 最后读 VoiceMode

原因：它是双向语音对话，不是当前主线。但它的 MCP 工具和本地 Whisper/Kokoro 组合可用于未来“Codex 需要用户语音确认”的功能。

优先文件：

- `_research/competitors/voicemode/README.md`
- `_research/competitors/voicemode/docs/guides/kokoro-setup.md`
- `_research/competitors/voicemode/docs/guides/whisper-setup.md`
- `_research/competitors/voicemode/src/`

要回答的问题：

- MCP converse 如何设计权限？
- 本地 STT/TTS 服务如何安装和切换？
- 智能静音检测是否值得引入？

## 对 Codex Speak 的直接行动项

1. 把 Aftertone 的 tag-only 思路对照我们的 side-channel：确认“没有导览就不读原文”的默认策略继续保持。
2. 把 Herdr 的状态集合映射到 Codex Speak 事件级播报：`working`、`blocked`、`done`、`failed`、`need_approval`。
3. 对照 AgentVibes 梳理 P3 的 TTS 工程清单：provider、voice picker、preview、queue、stop、remote playback。
4. 对照 Sled 写一版 AirPods/离屏场景的产品页文案。
5. 暂不追手机端，但记录 Paseo/Happy/Untether 的 session/remote architecture，避免未来重构困难。

## Submodule 使用

拉取：

```bash
git submodule update --init --recursive
```

更新某个参考项目：

```bash
git submodule update --remote _research/competitors/aftertone
```

查看当前固定版本：

```bash
git submodule status --recursive
```
