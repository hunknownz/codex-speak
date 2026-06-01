# Codex Speak

让 Codex 不只会写，也会讲。

## 一句话介绍

Codex Speak 是一个本地化、中文优先的 Codex 朗读助手：Codex 回复完成后，优先朗读回答中自然生成的“朗读导览”，让小朋友听懂 Codex 刚才做了什么、结果是什么、下一步怎么继续。

## 为什么做

Codex 的原始回复常常包含代码、命令、路径和技术词，直接朗读会很生硬。Codex Speak 的目标是把“给眼睛看的回答”变成“给耳朵听的回答”，并且尽量做到本地免费、保护隐私、跨平台、容易安装。

## 核心方案

```text
Codex Skill
  -> 让 Codex 生成儿童/初学者友好的朗读导览
Codex Speak Plugin / MCP
  -> 写入 side-channel，也可触发少量后台进度朗读
Codex Hook
  -> 回复结束后自动触发最终导览朗读
speak-engine
  -> 消费 side-channel、提取 fallback 协议、清洗兜底、配置、调度
本地 TTS
  -> MeloTTS / Kokoro / ZipVoice / Piper / 系统兜底
Tauri App / Native Desktop Pet
  -> 点按钮调配置，macOS 原生透明小伙伴展示朗读状态
安装器
  -> macOS / Windows 自动部署
```

## 当前阶段

项目已经进入产品化收尾阶段：macOS 端到端链路已跑通，GitHub Actions 已覆盖 macOS/Windows release 包构建和安装烟测；正式公开前主要还剩外部真机交互验收和真实签名/公证验证。

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
- Codex Speak Protocol v1：MCP side-channel 主路径，HTML 微格式 fallback
- Codex Speak Plugin 第一版：Skill、MCP 工具、side-channel 写入和消费链路、非阻塞进度朗读
- Tauri 控制面板第一版：自动朗读、儿童模式、语速、声音档位、TTS 引擎切换、试听、停止、自检
- 桌面 Pet 第一版：macOS 原生透明浮窗，按 lil-agents 的 `NSWindow + AVPlayerLayer + 1080x1920 HEVC-with-alpha .mov + CVDisplayLink` 方式显示角色，沿 Dock 区域行走，支持待命/待朗读/朗读中/完成/错误状态、拖动、点击停止、双击打开控制面板
- 原创 Pet 透明动画素材：由 `scripts/generate-pet-assets.swift` 生成，不再依赖 lil-agents 参考角色素材

下一步：

- 真机交互验收：已补 macOS/Windows QA 收集脚本，下一步是在外部机器上跑交互验收。
- 配置真实签名 secrets，验证 macOS codesign/notarization 和 Windows signtool。
- P2 通过后开始 P3：系统化优化语音清晰度、自然度和延迟。

发布构建：

- CI：`.github/workflows/ci.yml`
- 手动或 tag 发布构建：`.github/workflows/release.yml`。打 `v*` tag 时会上传 macOS/Windows release 包和 `.sha256` 校验文件到 GitHub Release；workflow 会先解包并执行一次跳过模型下载的安装烟测。

## 安装

release 包安装时，下载并解压 `codex-speak-macos.tar.gz` 或 `codex-speak-windows.zip`，进入解压后的目录运行对应安装脚本。release 包自带 CLI、控制面板和 Pet helper，普通用户不需要安装 Rust 或 Node.js。

macOS:

```bash
./installers/install-macos.sh
```

如果已经下载过 TTS 工具和模型，只想更新 CLI/Hook/Skill：

```bash
./installers/install-macos.sh --skip-tts-download
```

如果暂时不安装控制面板 App 和桌面 Pet：

```bash
./installers/install-macos.sh --skip-control-app
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
codex-speak doctor --json
codex-speak verify-install --allow-missing-models
codex-speak status
codex-speak support-bundle
codex-speak pet-state
codex-speak app open
codex-speak app path
codex-speak models list
codex-speak mcp
codex-speak config get
codex-speak config set --child-mode true --speed 0.9
codex-speak config set --provider sherpa_melo
codex-speak models install --provider piper
codex-speak models install --all
codex-speak install
codex-speak uninstall
```

测试朗读：

```bash
~/.codex/codex-speak/bin/codex-speak speak --text "你好，这是 Codex Speak 的中文本地朗读测试。"
```

默认中文声音使用 MeloTTS 的中英混读女声。控制面板和 CLI 现在都可以切换朗读引擎：

| Provider | 定位 | 说明 |
| --- | --- | --- |
| `sherpa_melo` | 默认中文方案 | 中文优先，中英混读，当前安装器默认下载 |
| `sherpa_kokoro` | 高自然度备选 | 可通过 `models install` 下载 Kokoro 模型 |
| `sherpa_zipvoice` | 参考音频克隆/实验 | 可通过 `models install` 下载 ZipVoice 模型和 vocoder |
| `piper` | 低配兜底 | 可通过 `models install` 下载 Piper 中文轻量模型，底层仍用 Sherpa-ONNX 运行 |
| `system` | 系统兜底 | macOS/Windows 系统自带，质量较低但最容易验证 |

如果正在试听某个额外 Provider，而模型还没有安装，Codex Speak 会直接报出缺失原因，不会偷偷切回默认声音；只有默认 `sherpa_melo` 失败时才会按配置兜底到系统语音。

自检：

```bash
~/.codex/codex-speak/bin/codex-speak doctor
```

需要给别人排查问题时，可以输出机器可读结果：

```bash
~/.codex/codex-speak/bin/codex-speak doctor --json
```

如果是刚装完、想快速确认安装链路是否可交付，可以运行：

```bash
~/.codex/codex-speak/bin/codex-speak verify-install
```

release 包烟测或跳过模型下载的安装，可以允许模型项暂时缺失，但仍然要求 CLI、Hook、Plugin、控制面板和播放器这些核心项通过：

```bash
~/.codex/codex-speak/bin/codex-speak verify-install --allow-missing-models
```

也可以生成一个本地支持包：

```bash
~/.codex/codex-speak/bin/codex-speak support-bundle
```

支持包会包含 `release-manifest.json`，用于确认外部用户正在运行哪个 release 包构建。

本地开发验证：

```bash
./scripts/verify-local.sh
```

发布 readiness 检查：

```bash
node scripts/check-release-readiness.mjs
```

## 文档

- [需求文档](docs/requirements.md)
- [技术文档](docs/technical-design.md)
- [商业价值分析](docs/business-value.md)
- [产品完成计划](docs/product-completion-plan.md)
- [安装与分发](docs/installation.md)
- [发布 QA](docs/release-qa.md)
- [签名与公证](docs/signing.md)
- [Codex Speak Protocol v1](docs/protocol-v1.md)
- [Plugin 设计](docs/plugin-design.md)
- [Tauri 控制面板](docs/tauri-control-app.md)

## Plugin

第一版插件在 [plugins/codex-speak](/Users/sherry/Documents/啦啦啦%202/codex-speak/plugins/codex-speak)。

它提供：

- Codex Speak Skill。
- MCP 工具：状态、提取预览、停止、试听、后台进度朗读、开关、儿童模式、语速、声音档位、side-channel 写入。
- `codex_speak_prepare`：让 Codex 把儿童友好的朗读导览写到本地 `spool/latest.json`，Hook 会优先朗读这段内容，成功读取后移动为 `last-consumed.json`。
- `codex_speak_speak_text background=true`：让 Codex 在长任务中播放一句简短进度提示，并马上继续工作。
- 安装器会把插件复制到个人插件目录，并写入个人 marketplace，方便 Codex 发现和启用。

当前 Codex Plugin 规范没有稳定的“隐藏或折叠已渲染 Chat 消息”能力。需要减少可见协议内容时，优先使用 MCP side-channel；可折叠 HTML `details` 只作为渲染器支持时的渐进增强。

## Tauri 控制面板

第一版控制面板在 [apps/codex-speak-control](/Users/sherry/Documents/啦啦啦%202/codex-speak/apps/codex-speak-control)。

它提供普通用户需要点的控件：

- 自动朗读开关。
- 儿童模式开关。
- 语速滑块。
- 最大朗读字数滑块。
- 声音档位选择。
- 朗读引擎选择。
- 当前朗读引擎模型安装。
- 试听、停止、刷新、自检。
- macOS 原生透明桌面 Pet：显示朗读状态，朗读中点击可停止，双击可打开控制面板。

开发运行：

```bash
cd apps/codex-speak-control
npm install
npm run dev
```

## 项目结构

```text
codex-speak/
  apps/
  README.md
  docs/
  bin/
  hooks/
  installers/
  plugins/
  skills/
  models/
  src/
  tests/
```
