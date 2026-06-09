# Legacy MCP Side-Channel

## 定位

这份文档记录 Codex Speak 曾经采用的主链路：

```text
AGENTS / Skill
  -> Codex 调用 codex_speak_prepare
  -> MCP 写入 ~/.codex/codex-speak/spool/latest.json
  -> Hook 在回复结束后读取并消费 latest.json
  -> 本地 TTS 播放
```

这条路径现在是 legacy/debug/future optional。成长模式 MVP 的默认主路径已经改为 Stop Hook 直接读取 final answer，并通过文件队列播放。

## 当时为什么设计 side-channel

最初目标是把“给屏幕看的回答”和“给耳朵听的导览”分开：

- 屏幕 final 可以保留代码、命令、路径、测试清单和技术细节。
- 朗读导览可以由 Codex 单独生成，更短、更口语化。
- Hook 不需要理解完整 final，只读取模型已经准备好的结构化内容。
- 可见回答里不需要出现 `朗读导览` 章节、HTML aside、隐藏注释或协议块。

因此 MCP 工具 `codex_speak_prepare` 接收结构化 items：

```json
{
  "items": [
    { "role": "did", "text": "我整理了主链路。" },
    { "role": "result", "text": "现在默认用 Stop Hook 朗读最终回答。" },
    { "role": "next", "text": "下一步要做并发真机验证。" }
  ],
  "audience": "child",
  "style": "growth"
}
```

MCP 合成中文导览并写入：

```text
~/.codex/codex-speak/spool/latest.json
```

这里的 `spool` 是“等待被后续进程取走的临时任务目录”。旧架构里它是一个单槽 side-channel，`latest.json` 表示“下一次 Hook 应该播放的最新导览”。

## 旧链路职责

| 组件 | 职责 |
| --- | --- |
| AGENTS/Skill | 提醒 Codex 每轮 final 前调用 `codex_speak_prepare` |
| MCP server | 接收结构化 items、查询配置、按儿童/成人模式合成导览 |
| `spool/latest.json` | 存放下一次 final hook 要播放的导览 |
| Hook | 识别 final phase，读取 fresh latest，播放后消费 |
| session activity 检测 | 防止 commentary 阶段提前消费 latest |
| missing guide policy | 没有 latest 时决定静默、短提示或诊断提示 |

## 已修问题

### MCP stdio framing

真实 Codex MCP 使用 line-delimited JSON-RPC，旧服务只支持 `Content-Length` framing，导致真实 Codex 客户端握手卡住。后来 `src/mcp.rs` 同时支持 header framing 和 line framing，`scripts/check-mcp-stdio.mjs` 覆盖两种协议。

### commentary 误触发和提前消费

Codex Desktop 的中间 commentary 也可能触发旧 notify/turn-ended。旧逻辑先读 `latest.json` 再判断 phase，导致 commentary hook 提前消费导览，真正 final 到来时已经没有内容。

修复方式：

- `src/session.rs` 增加 latest assistant activity 检测。
- `resolve_text_with_options` 先判断最近 assistant 活动是不是 final。
- 非 final phase 不消费 side-channel。
- 空文本时 `src/main.rs` early exit。
- 默认 `missing_guide_policy = "silent"`。

### 全局 MCP 和插件 MCP 冲突

一度同时存在全局 `[mcp_servers.codex_speak]` 和插件 scoped MCP，导致到底哪个 server 生效、哪个版本启动变得混乱。后来手动禁用插件 scoped MCP，以全局 MCP 为主。

### missing guide 噪声

其他 chat session 没有触发 skill 或没有调用 MCP 时，Hook 会播放“没有收到插件导览”的提示。后来默认改成 silent，避免正常聊天被缺失提示打扰。

## 踩坑

### 120 秒启动超时

如果全局 MCP 配置带 `required = true`，而 MCP server 握手失败，Codex session 初始化会等待并最终失败。用户体感就是新 session 卡住或不动。

Hook-first MVP 避免把朗读能力放在 session 初始化必需依赖上。

### agent 忘调用

Skill/AGENTS 是提示，不是强制执行机制。即使 MCP 工具可用，Codex 仍可能因为上下文、任务压力、工具选择、旧 session 未加载工具或模型策略而没有调用 `codex_speak_prepare`。

成长模式如果依赖“每轮 final 前必须调用 MCP”，就会出现时好时坏。Hook-first 直接读 final，降低对 agent 工具调用行为的依赖。

### 全局 latest 单槽

`spool/latest.json` 只有一个槽。多个 chat session 并发时，A 写入、B 触发、commentary 提前消费、stale latest 等问题都需要额外判断。

文件队列按 job 文件和 session/turn id 管理，更适合并发。

### 半卸载残留

用户删除 plugin 或部分文件后，`~/.codex/config.toml` 里的全局 MCP 可能仍被 Codex 加载。结果是 session 仍尝试启动旧 MCP，甚至继续卡住。

新 installer/uninstaller 要移除 Codex Speak 自己写入的 legacy MCP block，并把残留作为 doctor warning。

### Queue 等锁不是文件队列

旧 `PlaybackPolicy::Queue` 是播放锁策略，最多可能为了等当前播放结束而等待很久。它不能作为 Hook 等待队列，否则 Hook 会阻塞 Codex session。

新主路径让 Hook 只写 job 文件并立即退出；worker 才负责等待和播放。

## 为什么 MVP 改 Hook-first

成长模式的关键产品判断是：可见 final 本身就应该适合孩子读和听。既然屏幕内容和朗读内容默认不分离，主路径可以直接朗读 final：

- 不需要 MCP 初始化。
- 不需要 agent 每轮记得调用工具。
- 不需要全局 `latest.json` 单槽。
- 不需要缺失导览提示。
- Hook 可以 session-aware，避免串台。
- 文件队列可以自然处理多个 session 同时结束。

MCP 的价值没有消失，只是不适合做成长模式 MVP 的默认依赖。

## 何时可能重新启用 MCP

未来如果出现这些需求，可以把 MCP 作为 optional side-channel 拉回：

- 屏幕 final 必须非常技术化，但朗读必须显著简化。
- 需要 Codex 在执行中发少量进度提示。
- 需要让 Codex 主动控制本地配置、发音词典、停止播放或模型安装。
- 需要更高级的 ask-user、视觉内容摘要、面向不同听众的多版本朗读。

重新启用时建议避免单槽 `latest.json`，改为 session-aware side-channel job，并且不能把 MCP 设为 Codex session 的 required dependency。

## 保留代码

暂不删除：

- `src/mcp.rs`
- `src/side_channel.rs`
- `codex_speak_prepare`
- `codex_speak_speak_text`
- 旧 HTML/hidden/visible guide 解析 fixture

这些代码继续服务于历史验证、手动排障、发音词典和未来高级能力。主路径文档和 installer 不再把它们描述成 required。
