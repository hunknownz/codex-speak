# Legacy Plugin Product Design

## 状态

这份文档保存旧的 Codex Speak Plugin 产品设计。该设计已经不再作为成长模式 MVP 的默认产品形态；当前默认产品形态是：

```text
Tauri App / installer
  -> codex-speak install
  -> Codex Stop Hook
  -> session-aware final extraction
  -> file queue
  -> local TTS
```

Plugin 相关代码和包不再进入默认安装路径。MCP side-channel 仍可作为 legacy/debug/future optional 能力保留，详见 [Legacy MCP Side-Channel](legacy-mcp-side-channel.md)。

## 旧设计目标

旧设计把 Codex Speak Plugin 当成 Codex 侧能力包：

```text
Codex Plugin
  -> Skill
  -> MCP config
  -> codex_speak_prepare
  -> codex_speak_speak_text
  -> side-channel spool/latest.json
```

当时希望解决的问题：

- 让 Codex 显式生成一份和屏幕 final 分离的朗读导览。
- 避免在可见回答里塞 HTML、隐藏协议块或单独的“朗读导览”章节。
- 让 Codex 可以主动查询状态、切换配置、设置发音词典、播放进度提示。
- 通过 plugin marketplace 分发 Skill 和 MCP 配置。

## 旧安装链路

旧链路包括：

```text
codex-speak install
  -> 复制 plugin 源目录到 ~/plugins/codex-speak
  -> 写 ~/.agents/plugins/marketplace.json
  -> 生成 plugin .mcp.json
  -> 写 ~/.codex/config.toml 的 [mcp_servers.codex_speak]
  -> 尝试 codex plugin add codex-speak@personal
```

安装成功后，期望 Codex 新 thread 自动拥有：

- `codex-speak` skill。
- `codex_speak_prepare` 等 MCP 工具。
- 每轮 final 前由 AGENTS/Skill 引导调用 `codex_speak_prepare`。

## 为什么停止作为主产品形态

成长模式 MVP 后来改成“直接朗读可见 final”。在这个目标下，Plugin 主路径带来的成本大于收益：

- Codex agent 可能忘记调用 MCP。
- 旧 session 不会热加载新增 MCP tools。
- MCP 启动慢或握手失败可能卡住 session。
- 全局 `spool/latest.json` 单槽不适合多 session 并发。
- Plugin scoped MCP 和 global MCP 同时存在时容易混淆。
- 半卸载后残留 plugin/global MCP 仍可能影响新 session。
- 竞品调研显示，Codex 语音层更常见的是 installer 写 `~/.codex/hooks.json`，不是把 Codex plugin 当主安装入口。

因此新决策是：

```text
默认产品入口：Tauri App + local installer
默认 Codex 接入：Stop Hook + AGENTS hint
默认朗读来源：final answer
Plugin：不默认安装，不作为 doctor required，不作为用户可见主卖点
```

## 保留的历史价值

虽然默认产品不再使用 Plugin，这个设计仍有参考价值：

- 未来需要“屏幕内容和朗读内容分离”时，可以重新引入 optional side-channel。
- 未来需要 Codex 主动改配置、设置发音词典、发进度提示或 ask-user 语音交互时，可以用 MCP 工具。
- 未来如果 Codex Plugin hooks/installer 机制足够稳定，可以重新评估是否提供一个可选 plugin 包。

重新启用时的要求：

- 不能把 MCP 设置为 Codex session 的 required dependency。
- 不能使用单槽 `latest.json` 作为并发主通道。
- 必须 session-aware，并且有超时、去重和残留清理。
- 必须是 optional，不影响 Hook-first 主路径。
