# Codex Speak 架构

## 一句话

Codex Speak 把 Codex 的复杂输出变成适合听的中文导览，再用本地免费 TTS 在 macOS 或 Windows 上播放。

## 当前主路径

```text
Codex Skill
  -> 让 Codex 理解完整回复，并按儿童/成人模式生成可听导览和必要的可视化支架
Codex Plugin / MCP
  -> codex_speak_prepare 写入 side-channel
Codex Hook
  -> 回复结束后触发本地 CLI
Rust CLI
  -> Hook 主路径只消费 side-channel；缺失时播放“插件导览未到达”提示，不猜 final answer
Codex plugin install
  -> codex plugin add codex-speak@personal，把插件安装/启用到 Codex cache
本地 TTS
  -> 默认 Sherpa-ONNX + MeloTTS，经过播放队列后播放到系统播放器
Tauri 控制面板
  -> 控制总朗读、最终导览、过程提示、小伙伴显示、儿童模式、语速、音色和引擎
```

## 两种朗读时机

| 通道 | 触发 | 用途 |
| --- | --- | --- |
| 过程提示 | MCP `codex_speak_speak_text(background=true)` | 长任务中播放少量短句 |
| 最终导览 | MCP `codex_speak_prepare` + Hook | 回复结束后播放完整行动导览 |

不做默认逐字流式朗读。过程提示只负责“我正在做什么”；最终导览才负责“做了什么、结果如何、下一步怎么办”。

## 插件通道原则

当 side-channel 不可用时，责任不转移给本地规则。本地 Rust 不应该假装理解完整 final answer；它只播放一个明确的缺失提示，告诉用户没有收到插件准备好的朗读导览。

默认 Hook 不再读取普通 final answer，不再读取用户输入，也不再要求 Skill 在 Chat Session 中写 `**朗读导览**`。这能避免把代码说明、命令、长路径、测试清单、提示词片段或用户原始输入误读出来。

历史 HTML、隐藏注释和 Markdown `朗读导览` 解析能力保留给 `extract`、fixture、旧会话和排障样例；它们不再是 Hook 的默认产品路径。

缺失提示示例：

```text
我没有收到本地插件准备好的朗读导览。这次先不乱读屏幕内容。
请新开一个 Codex 会话，或者运行自检看看插件有没有加载。
```

## 播放队列

TTS 生成和系统播放器共享一把本地播放锁，避免多个 `codex-speak speak` 进程同时写 `last.wav` 或互相停止：

- Hook 最终导览使用排队策略：等当前朗读结束后完整播放。
- MCP 过程提示使用忙时跳过策略：如果最终导览或另一句提示正在播放，就跳过这句短提示。
- 手动试听和停止按钮仍可打断当前朗读。
- 停止按钮会释放播放锁，并让正在等待队列的旧任务退出，避免停止后又继续播旧内容。

## 朗读风格

朗读风格遵循 [朗读风格指南](speech-style-guide.md)：

- 儿童模式直接对孩子说话，不朗读“要生成儿童能听懂的内容”这类元说明。
- 儿童模式可以融入隐性教学：每次最多带一个自然贴合成果的小知识点，帮助孩子学会观察、测试、比较和排障。
- 儿童模式可以使用 [儿童可视化学习](visual-learning.md)：用 Mermaid、简单 HTML 或可选生成图片帮助孩子理解架构、流程和抽象概念；朗读只讲图意，不读图表源码。
- 是否教学、是否画图由 Codex 自动判断；默认克制，只有明显帮助理解或推进项目时才加入。
- 成人模式提供节省时间的简洁摘要，保留变更、验证、风险和下一步。

如果后续启用本地风格 RAG，向量数据库统一使用 [Qdrant](qdrant-style-rag.md)，只检索经过清洗验证的表达样例，不检索事实知识。

## 当前 macOS 验证状态

本机已验证：

- `doctor --json` 通过：CLI、Hook、Skill、Plugin、MCP、控制面板、Pet helper、MeloTTS 模型和播放器均可用。
- `codex plugin list` 显示 `codex-speak@personal installed, enabled`；插件 cache 已生成。
- `verify-codex --json` 通过：side-channel、Hook 消费、缺失导览提示、混合英文归一化和发音词典链路可用。
- `verify-controls --json` 通过：总朗读、最终导览、过程提示、小伙伴显示、儿童模式、语速、最大朗读长度、音色和 TTS 引擎可写入并恢复。
- 真实 `speak --text` 已播放成功，`last_spoken` 记录为清洗后的中文导览。

## 发布前不变门槛

- macOS 和 Windows 都要有真实安装/朗读/停止/自检证据。
- `cargo test`、控制面板构建、MCP stdio 检查和 release readiness 必须通过。
- 如果验证发现架构描述和实际行为不一致，先修架构文档，再决定修实现还是换方向。

## 朗读文本生产契约

1. Codex/Skill 负责理解任务，并生成“可听”的导览。
2. MCP 可用时，Codex 调用 `codex_speak_prepare` 写入 side-channel。
3. MCP 不可用时，Codex 不在 Chat Session 中补协议；Hook 播放缺失导览提示。
4. Hook/Rust 负责传输、选择、清洗和播放，不负责深度理解。
5. 没有结构化导览时，Hook 明示缺失，不能乱读。
