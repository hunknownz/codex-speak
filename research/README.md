# Research

这个目录用于沉淀 Codex Speak 相关的市场、竞品、参考项目和产品机会调研。

约定：

- `competitive-*.md`：竞品和参考项目分析。
- `notes-*.md`：较轻量的原始调研笔记。
- `decision-*.md`：调研后形成的产品/技术决策记录。
- `strategic-*.md`：面向定位、商业化、路线图和投资人视角的战略分析。

## `research/` 与 `_research/` 的关系

`research/` 是整理后的研究报告区。这里的文档应该是人可以直接阅读、引用和维护的结论材料，用来支持产品、架构和路线图决策。

`_research/` 是原料仓。这里存放原始参考项目、git submodule、压缩包、探针脚本、抓取到的 schema、实验输出和其他外部代码材料。它可以比较杂，也可以包含很大的 vendor/source 文件；默认不把这里的内容当成项目主文档。

约定：

- 先在 `_research/` 里放原始证据和可复现实验。
- 再在 `research/` 里写对应的总结文档。
- `research/` 文档需要链接到 `_research/` 的关键证据路径，但不要把大段原始输出复制进来。
- 搜索和阅读时优先看 `research/`；只有要核对证据、重跑探针或读竞品源码时，才进入 `_research/`。
- 宽泛搜索默认排除 `_research/competitors/**`、vendor、node_modules 等高噪声目录，避免把大段外部源码灌进上下文。

例子：

```text
_research/app-server-realtime-probe/
  -> 原始 app-server realtime 探针脚本、schema 和实验输出

research/app-server-realtime-probe-2026-06-05.md
  -> 对上述探针的整理版结论、风险和下一步
```

当前开源竞品和参考项目使用 git submodule 管理在 `_research/competitors/` 下，索引见 [open-source-reference-projects.md](open-source-reference-projects.md)。

重点文档：

- [competitive-voice-layer-for-coding-agents-2026-06.md](competitive-voice-layer-for-coding-agents-2026-06.md)：竞品与产品机会总览。
- [strategic-competitive-matrix-2026-06.md](strategic-competitive-matrix-2026-06.md)：投资人/战略规划视角的竞品矩阵。
- [codex-realtime-and-spokenly-voice-agents-2026-06.md](codex-realtime-and-spokenly-voice-agents-2026-06.md)：Codex Realtime/WebRTC 与 Spokenly MCP 专题调研。
- [app-server-realtime-probe-2026-06-05.md](app-server-realtime-probe-2026-06-05.md)：本机 Codex app-server realtime 接入探针记录。
- [deep-dive-open-source-voice-agent-projects-2026-06.md](deep-dive-open-source-voice-agent-projects-2026-06.md)：开源项目源码深读。
- [footnote-source-check-2026-06-05.md](footnote-source-check-2026-06-05.md)：初步调研脚注逐项核查。
