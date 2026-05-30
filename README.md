# Codex Speak

让 Codex 不只会写，也会讲。

## 一句话介绍

Codex Speak 是一个本地化、中文优先的 Codex 朗读助手：Codex 回复完成后，优先朗读回答中自然生成的“朗读导览”，让小朋友听懂 Codex 刚才做了什么、结果是什么、下一步怎么继续。

## 为什么做

Codex 的原始回复常常包含代码、命令、路径和技术词，直接朗读会很生硬。Codex Speak 的目标是把“给眼睛看的回答”变成“给耳朵听的回答”，并且尽量做到本地免费、保护隐私、跨平台、容易安装。

## 核心方案

```text
Codex Skill
  -> 让最终回答自然包含朗读导览
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

项目已经进入 Rust CLI 原型阶段，macOS 端到端链路已跑通。

已完成：

- 需求文档
- 技术选型文档
- 商业价值分析
- 初始项目目录
- Rust CLI
- Codex Skill 模板
- Codex Hook wrapper
- macOS 安装/卸载脚本
- Sherpa-ONNX + MeloTTS 中文模型接入

下一步：

- 完善 Windows 安装脚本。
- 增加 GitHub Actions 跨平台构建。
- 增加模型 manifest 和校验。
- 增加 Plugin 或图形化设置界面。

## 开发安装

macOS:

```bash
./installers/install-macos.sh
```

如果已经下载过 TTS 工具和模型，只想更新 CLI/Hook/Skill：

```bash
./installers/install-macos.sh --skip-tts-download
```

卸载：

```bash
./installers/uninstall-macos.sh
```

## CLI

```bash
codex-speak extract
codex-speak speak
codex-speak stop
codex-speak doctor
codex-speak install
codex-speak uninstall
```

测试朗读：

```bash
~/.codex/codex-speak/bin/codex-speak speak --text "你好，这是 Codex Speak 的中文本地朗读测试。"
```

默认中文声音使用 MeloTTS 的中英混读女声。当前模型只有一个中文音色，所以第一版通过略微放慢语速、减少 VITS 随机噪声、增加推理线程来优化“口齿清晰、声音明亮、像姐姐在讲”的效果；后续会通过 Kokoro 或 ZipVoice 增加更多可选音色。

自检：

```bash
~/.codex/codex-speak/bin/codex-speak doctor
```

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
  src/
  tests/
```
