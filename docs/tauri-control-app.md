# Tauri 控制面板

## 定位

Tauri 控制面板是给普通用户点按钮用的小界面，不替代 Hook、Plugin 或 Rust CLI。

它负责：

- 开启或关闭自动朗读。
- 开启或关闭儿童模式。
- 调整语速。
- 选择声音档位。
- 切换朗读引擎。
- 安装当前朗读引擎需要的模型。
- 设置最大朗读字数。
- 停止当前朗读。
- 试听一句话。
- 运行自检。
- 显示桌面 Pet，并根据朗读状态切换动画。

## 为什么需要 Tauri

Plugin 目前不能稳定地在 Codex 聊天窗口里增加自定义按钮。MCP 很适合让 Codex 按自然语言控制本地能力，但普通用户仍然需要一个明确可见的小界面。

所以产品分工是：

```text
Codex Plugin / MCP
  -> Codex 可以主动写入 side-channel、改配置、停止朗读
Tauri App
  -> 用户可以点按钮改配置、停止朗读、试听声音，并看到桌面 Pet 状态
Hook
  -> 回复结束后自动触发朗读
Rust CLI
  -> 唯一的本地核心：配置、解析、TTS、播放、停止
```

## 当前实现

目录：

```text
apps/codex-speak-control/
  vite.config.js
  index.html
  src/
    main.js
    styles.css
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/main.rs
apps/codex-speak-pet-macos/
  CodexSpeakPet.swift
  assets/
    codex-agent.mov
    codex-agent-hit.png
    codex-agent-preview.png
    ASSET-NOTICE.txt
```

Tauri 后端不重新实现 TTS，也不直接改 Hook。它调用已安装的 CLI：

```text
~/.codex/codex-speak/bin/codex-speak
```

主要命令映射：

| UI 操作 | Rust CLI |
| --- | --- |
| 刷新状态 | `codex-speak status` |
| 自动朗读开关 | `codex-speak config set --enabled ...` |
| 儿童模式 | `codex-speak config set --child-mode ...` |
| 朗读引擎 | `codex-speak config set --provider ...` |
| 语速 | `codex-speak config set --speed ...` |
| 声音档位 | `codex-speak config set --voice-profile ...` |
| 最大朗读字数 | `codex-speak config set --max-read-chars ...` |
| 发音词典 | `codex-speak pronunciation list/set/remove/preview` |
| 安装模型 | `codex-speak models install` |
| 模型清单 | `codex-speak models list` |
| 试听 | `codex-speak speak --text ...` |
| 停止 | `codex-speak stop` |
| 自检 | `codex-speak doctor` / `codex-speak doctor --json` |
| 控制项验收 | `codex-speak verify-controls` |
| 支持包 | `codex-speak support-bundle` |
| Pet 状态 | `codex-speak pet-state` |

播放开始时，CLI 会把当前播放器子进程 PID 写到本地状态目录；停止按钮和 MCP `stop` 工具会优先结束这个子进程。这样 macOS 的 `afplay`/`say` 和 Windows 的 PowerShell `SoundPlayer` 都能被准确停止。

控制面板的健康状态读取 `codex-speak status`，其中会包含播放器可用性、发音词典是否能解析，以及 Skill、Hook wrapper、Plugin 和 MCP 脚本是否与当前 CLI 内置版本一致；人工排障或外部 QA 可以运行 `codex-speak doctor --json` 获取同一套结构化检查结果，运行 `codex-speak verify-controls` 确认控制项能写入并恢复，也可以点击“支持包”生成包含自检、状态、模型、发音词典摘要和最近日志的本地排障目录。

## 桌面 Pet

macOS 桌面 Pet 已经从 Tauri WebView 迁移为原生 AppKit helper：

```text
~/.codex/codex-speak/bin/codex-speak-pet-macos
```

控制面板启动时会拉起这个 helper。它不直接跑 TTS，也不理解 Codex 内容，而是读取统一的状态文件：

```text
~/.codex/codex-speak/state/pet-state.json
```

状态由核心链路写入：

| 写入方 | 状态 | 含义 |
| --- | --- | --- |
| MCP `codex_speak_prepare` | `ready` | Codex 已经准备好适合朗读的导览 |
| TTS 开始播放 | `speaking` | 正在朗读 |
| TTS 播放完成 | `done` | 本次朗读完成 |
| TTS 出错 | `error` | 模型、播放器或系统命令遇到问题 |
| 停止命令 | `idle` | 用户停止或回到待命 |

Pet 交互：

- 拖动 Pet 可以移动原生透明窗口。
- 朗读中点击 Pet 可以停止朗读。
- 双击 Pet 可以打开控制面板。

实现方式按 lil-agents 的核心路线落地：无边框透明原生窗口、沿 Dock 区域移动、`AVPlayerLayer` 播放 1080x1920 的 HEVC-with-alpha 透明 `.mov` 动画、`CVDisplayLink` 驱动位置更新、独立气泡窗口。点击命中优先采样窗口实际 alpha 像素；在 macOS 15 SDK 或系统不允许采样时，退回 `codex-agent-hit.png` 透明 mask，尽量让空白区域不拦截鼠标。

角色显示不再依赖 Tauri WebView、HTML、CSS、SVG 或 canvas。默认素材安装到：

```text
~/.codex/codex-speak/assets/pet/
  codex-agent.mov
  codex-agent-source-spritesheet.png
  codex-agent-hit.png
  codex-agent-preview.png
  ASSET-NOTICE.txt
```

`codex-agent.mov` 是真正显示的原创透明动画素材，规格与 lil-agents 参考项目一致：1080x1920、约 10 秒、HEVC with Alpha；`codex-agent-source-spritesheet.png` 是 6 帧二维行走素材源；`codex-agent-hit.png` 是点击命中的兜底 alpha mask；`codex-agent-preview.png` 是透明静帧预览；`ASSET-NOTICE.txt` 说明素材来源。后续换角色时，优先替换 sprite sheet 并运行 `scripts/build-pet-assets-from-spritesheet.swift`，helper 的透明浮窗和移动逻辑不用重写。

## 朗读引擎

控制面板提供 `provider` 下拉框，让用户直接试听不同本地 TTS 方案：

| Provider | 界面显示 | 定位 |
| --- | --- | --- |
| `sherpa_melo` | MeloTTS 中文女声 | 默认中文优先方案 |
| `sherpa_kokoro` | Kokoro | 更自然的备选方案，需要额外模型 |
| `sherpa_zipvoice` | ZipVoice | 实验性参考音频方案，需要额外模型和参考音频 |
| `piper` | Piper 轻量语音 | 低配置兜底，下载中文轻量模型后由 Sherpa-ONNX 运行 |
| `system` | 系统语音 | 无模型快速验证 |

状态区会显示当前 Provider 是否可用。如果用户选了一个还没安装模型的 Provider，试听会显示缺失原因；这样用户能明确知道“还没装模型”，而不是误以为这个声音不好听。

“安装模型”按钮会安装当前下拉框选中的 Provider。模型安装可能需要几十秒到几分钟，取决于包大小和网络速度。

CLI 也提供同一份模型清单：

```bash
codex-speak models list
```

这份清单来自 `src/model_catalog.rs`，包含 Provider 名称、语言优先级、体积提示、推荐状态和本机安装状态。控制面板和 CLI 复用同一份信息，避免 UI 里写死一套、命令行里再写一套。

## 声音档位

声音档位是 `voice_profile`，它和 `provider` 分开。Provider 决定用哪个朗读引擎，声音档位决定同一个引擎里尽量清楚、慢一点或快速预览。当前 MeloTTS 中文模型只有一个中文女声音色，所以档位先映射到本地 VITS 参数：

| 档位 | 目标 | 主要效果 |
| --- | --- | --- |
| `clear_bright` | 默认姐姐音色 | 清楚、明亮、速度适中 |
| `slow_clear` | 慢一点更清楚 | 降低语速，增加停顿 |
| `quick_preview` | 快速预览 | 语速更快，停顿更短 |

后续接入更多 Kokoro、ZipVoice 或 Piper 声音后，`voice_profile` 可以继续保留，底层再映射到真实声音模型。

## 开发命令

```bash
cd apps/codex-speak-control
npm ci
npm run dev
```

只检查 Rust 后端：

```bash
cargo check --manifest-path apps/codex-speak-control/src-tauri/Cargo.toml
```

正式分发时需要：

- macOS：签名、公证，最好提供 `.dmg` 或 `.pkg`。
- Windows：签名，降低 SmartScreen 和杀毒误报。
- 安装器：确保 Rust CLI、Hook、Skill、Plugin、模型和 Tauri App 版本一致。
