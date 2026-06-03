# Codex Speak 架构

## 一句话

Codex Speak 把 Codex 的复杂输出变成适合听的中文导览，再用本地免费 TTS 在 macOS 或 Windows 上播放。

## 当前主路径

```text
Codex Skill
  -> 让 Codex 理解完整回复，生成儿童/初学者友好的导览
Codex Plugin / MCP
  -> codex_speak_prepare 写入 side-channel
Codex Hook
  -> 回复结束后触发本地 CLI
Rust CLI
  -> 优先消费 side-channel，兜底生成导览式摘要
本地 TTS
  -> 默认 Sherpa-ONNX + MeloTTS，播放到系统播放器
Tauri 控制面板
  -> 控制总朗读、最终导览、过程提示、儿童模式、语速、音色和引擎
```

## 两种朗读时机

| 通道 | 触发 | 用途 |
| --- | --- | --- |
| 过程提示 | MCP `codex_speak_speak_text(background=true)` | 长任务中播放少量短句 |
| 最终导览 | MCP `codex_speak_prepare` + Hook | 回复结束后播放完整行动导览 |

不做默认逐字流式朗读。过程提示只负责“我正在做什么”；最终导览才负责“做了什么、结果如何、下一步怎么办”。

## 兜底原则

当 side-channel 不可用时，Hook 不应该朗读整段最终回复。它会先清洗最终可见回复，再压缩成 3 到 4 句导览式内容。这个兜底没有 MCP 主路径那么理解充分，但仍必须遵守“听得懂、能行动”的原则。

兜底导览必须做到：

- 保留有用中文上下文，不能因为同一行里有长路径或代码片段就整行丢掉。
- 长路径替换成“项目里的文件”这类可听表达。
- 代码片段、命令参数、日志和英文技术词转成作用或中文解释。
- 中文导览里的常见开发短语要转成自然说法，例如 `cargo test` 读成“测试命令”，`build failed because timeout` 读成“构建失败，因为超时”。
- 纯英文文本尽量保留给英文朗读路径，不强行中文化。
- 不能把控制台回复、代码说明、测试方式和长列表原样整段朗读。

## 当前 macOS 验证状态

本机已验证：

- `doctor --json` 通过：CLI、Hook、Skill、Plugin、MCP、控制面板、Pet helper、MeloTTS 模型和播放器均可用。
- `verify-codex --json` 通过：side-channel、Hook 消费、短导览兜底、混合英文归一化和发音词典链路可用。
- `verify-controls --json` 通过：总朗读、最终导览、过程提示、儿童模式、语速、最大朗读长度、音色和 TTS 引擎可写入并恢复。
- 真实 `speak --text` 已播放成功，`last_spoken` 记录为清洗后的中文导览。

## 发布前不变门槛

- macOS 和 Windows 都要有真实安装/朗读/停止/自检证据。
- `cargo test`、控制面板构建、MCP stdio 检查和 release readiness 必须通过。
- 如果验证发现架构描述和实际行为不一致，先修架构文档，再决定修实现还是换方向。
