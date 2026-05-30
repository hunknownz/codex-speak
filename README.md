# Codex Speak

让 Codex 不只会写，也会讲。

## 一句话介绍

Codex Speak 是一个本地化、中文优先的 Codex 朗读助手：Codex 回复完成后，自动朗读一段由 Codex 自己生成的、儿童也能听懂的语音概要。

## 为什么做

Codex 的原始回复常常包含代码、命令、路径和技术词，直接朗读会很生硬。Codex Speak 的目标是把“给眼睛看的回答”变成“给耳朵听的回答”，并且尽量做到本地免费、保护隐私、跨平台、容易安装。

## 核心方案

```text
Codex Skill
  -> 生成适合朗读的中文概要
Codex Hook
  -> 回复结束后自动触发
speak-engine
  -> 提取、清洗、配置、调度
本地 TTS
  -> MeloTTS / Kokoro / Piper / 系统兜底
安装器
  -> macOS / Windows 自动部署
```

## 当前阶段

项目正在产品设计和原型准备阶段。

已完成：

- 需求文档
- 技术选型文档
- 商业价值分析
- 初始项目目录

下一步：

- 实现 `codex-speak` CLI 原型。
- 实现 Codex Skill 模板。
- 实现 Hook wrapper。
- 增加 macOS 安装脚本。
- 增加 Windows 安装脚本。

## 文档

- [需求文档](docs/requirements.md)
- [技术文档](docs/technical-design.md)
- [商业价值分析](docs/business-value.md)

## 项目结构

```text
codex-speak/
  README.md
  docs/
  bin/
  hooks/
  installers/
  skills/
  models/
```

