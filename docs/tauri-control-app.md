# Tauri 控制面板

## 定位

Tauri 控制面板是给普通用户点按钮用的小界面，不替代 Hook、Plugin 或 Rust CLI。

它负责：

- 开启或关闭自动朗读。
- 开启或关闭儿童模式。
- 调整语速。
- 选择声音档位。
- 设置最大朗读字数。
- 停止当前朗读。
- 试听一句话。
- 运行自检。

## 为什么需要 Tauri

Plugin 目前不能稳定地在 Codex 聊天窗口里增加自定义按钮。MCP 很适合让 Codex 按自然语言控制本地能力，但普通用户仍然需要一个明确可见的小界面。

所以产品分工是：

```text
Codex Plugin / MCP
  -> Codex 可以主动写入 side-channel、改配置、停止朗读
Tauri App
  -> 用户可以点按钮改配置、停止朗读、试听声音
Hook
  -> 回复结束后自动触发朗读
Rust CLI
  -> 唯一的本地核心：配置、解析、TTS、播放、停止
```

## 当前实现

目录：

```text
apps/codex-speak-control/
  src/
    index.html
    main.js
    styles.css
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/main.rs
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
| 语速 | `codex-speak config set --speed ...` |
| 声音档位 | `codex-speak config set --voice-profile ...` |
| 最大朗读字数 | `codex-speak config set --max-read-chars ...` |
| 试听 | `codex-speak speak --text ...` |
| 停止 | `codex-speak stop` |
| 自检 | `codex-speak doctor` |

## 声音档位

第一版不做真正多音色切换，因为当前 MeloTTS 中文模型只有一个中文女声音色。声音档位先映射到本地 VITS 参数：

| 档位 | 目标 | 主要效果 |
| --- | --- | --- |
| `clear_bright` | 默认姐姐音色 | 清楚、明亮、速度适中 |
| `slow_clear` | 慢一点更清楚 | 降低语速，增加停顿 |
| `quick_preview` | 快速预览 | 语速更快，停顿更短 |

后续接入 Kokoro 或其他中文模型后，`voice_profile` 可以继续保留，底层再映射到真实声音模型。

## 开发命令

```bash
cd apps/codex-speak-control
npm install
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
