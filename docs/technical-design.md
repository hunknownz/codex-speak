# Codex 朗读助手技术文档

## 技术结论

推荐架构：

```text
Codex Skill
  -> 让回答自然包含朗读导览
Codex Speak Plugin / MCP
  -> 优先写入 side-channel
Codex Hook
  -> 回复结束后自动触发
speak-engine
  -> 消费 side-channel、提取 fallback 协议、清洗、配置、调度
本地 TTS
  -> MeloTTS / Kokoro / Piper / 系统兜底
播放器
  -> macOS afplay / Windows PowerShell 播放
installer
  -> 安装、升级、卸载、自检
```

第一阶段采用：

```text
Skill + Hook + 本地 TTS
```

Plugin 不作为 Hook 朗读的必需组件，但第一版已经可以提供 MCP 工具，用于状态查询、试听、停止朗读、开关配置、儿童模式、语速、声音档位和 side-channel 写入。

## 设计原则

- Codex 负责理解和表达，Hook 负责触发和执行。
- 不调用外部大模型做二次总结，避免费用、延迟和隐私问题。
- TTS 方案必须本地、免费、跨平台、低配置可运行。
- 中文优先，其次英文。
- Hook 逻辑保持薄，不把复杂改写都塞进脚本。
- 安装部署是产品体验的一部分，必须可自动化、可恢复、可升级。

## Skill、Hook、Plugin 分工

| 组件 | 是否第一阶段需要 | 职责 |
| --- | --- | --- |
| Skill | 是 | 让 Codex 每次自然生成适合朗读的中文导览 |
| Hook | 是 | Codex 回复结束后触发朗读流程 |
| 本地 TTS | 是 | 把朗读导览变成声音 |
| Plugin | 否，第二阶段 | 做设置界面、重读、暂停、声音选择 |

Skill 生成示例：

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你把朗读助手的规则改好了。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">现在它不会把代码、命令和长路径一个字一个字读出来，而是会说明这些内容解决了什么问题。</p>
  <p class="codex-speak-next" data-role="next">接下来，小朋友听完就能知道现在做到哪一步，也知道可以怎么继续问 Codex。</p>
</aside>
```

Hook 提取策略：

```text
优先读取并消费新鲜的 MCP side-channel latest.json
找不到 -> 读取 HTML 微格式协议 aside[data-codex-speak="guide"]
找不到 -> 读取旧版 Markdown 朗读导览
找不到 -> 读取旧版 codex-speak 调试块
找不到 -> 规则清洗最后一条回复
清洗失败 -> 系统朗读兜底
```

## 文本处理方案

### 第一层：Codex 生成朗读导览

由 Skill 约束 Codex：

- 用中文。
- 通常 120 到 300 字，复杂任务最多 500 字。
- 说明做了什么、结果是什么意思、下一步可以怎么继续。
- 不朗读代码、命令、日志、长路径，而是解释它们在解决什么问题。
- 技术词转成更容易听懂的说法。

导览使用 [Codex Speak Protocol v1](protocol-v1.md)。主路径是 MCP side-channel；HTML 微格式 `aside` 是 Plugin 不可用时的 fallback，`data-*` 供 Rust CLI 稳定解析，`class` 供未来 Plugin 渲染和校验。

当 Plugin MCP 工具可用时，优先让 Codex 调用 `codex_speak_prepare`，把相同结构的导览写入 `~/.codex/codex-speak/spool/latest.json`。Hook 触发后会读本地结构化内容，成功后移动为 `last-consumed.json`，Chat Session 里只需要保留自然的最终回答。

### 第二层：规则清洗兜底

规则清洗负责：

- 删除 Markdown 标记。
- 跳过代码块、diff、日志、表格。
- 压缩长链接和长路径。
- 替换常见技术词。
- 限制最大朗读字数。

示例词典：

```text
Hook -> 自动触发器
Plugin -> 插件
TTS -> 朗读工具
API -> 接口
config.toml -> 配置文件
terminal -> 命令窗口
```

## TTS 调研对比

| 方案 | 中文优先 | 跨平台 | 低配置 | 自然度 | 接入难度 | 建议定位 |
| --- | --- | --- | --- | --- | --- | --- |
| Sherpa-ONNX + MeloTTS zh_en | 强 | 强 | 中 | 中高 | 中 | 默认中文方案 |
| Sherpa-ONNX + Kokoro zh/multi-lang | 中高 | 强 | 中 | 高 | 中 | 高自然度备选 |
| Piper zh_CN | 中 | 强 | 强 | 中 | 低 | 低配兜底 |
| 系统朗读 | 中 | 强 | 强 | 低 | 低 | 最后兜底 |
| 直接 Python MeloTTS | 强 | 中 | 中低 | 中高 | 高 | 暂不推荐默认 |
| 直接 kokoro-onnx | 中高 | 中高 | 中 | 高 | 中 | 可实验，不做主入口 |

## 推荐 TTS 顺序

```text
默认：Sherpa-ONNX + MeloTTS zh_en
增强：Sherpa-ONNX + Kokoro 中文/多语言模型
低配兜底：Piper zh_CN
最后兜底：系统语音
```

原因：

- MeloTTS 中文模型明确适合中文和中英混读，符合中文优先。
- Sherpa-ONNX 适合作为统一推理底座，降低 Windows/macOS 适配成本。
- Kokoro 体积小、自然度潜力高，适合做更好听的备选。
- Piper 快、轻、稳定，适合低配机器。

## 跨平台实现

### macOS

```text
Hook -> speak-engine -> 本地 TTS 生成 wav -> afplay
```

系统兜底：

```text
say -v Tingting
```

发布要求：

- 开发阶段可以使用 shell 安装脚本。
- 面向普通用户分发时，建议提供签名并公证的 `.pkg` 或 `.app` 安装器。
- 安装器应写入用户目录，尽量避免管理员权限。
- 安装前备份 `~/.codex/config.toml`，卸载时可恢复。

### Windows

```text
Hook -> speak-engine -> 本地 TTS 生成 wav -> PowerShell 播放
```

系统兜底可调用 Windows SAPI 或 PowerShell 音频播放。

发布要求：

- 开发阶段可以使用 PowerShell 安装脚本。
- 面向普通用户分发时，建议提供签名的 MSIX、MSI 或 EXE 安装器。
- 模型和二进制文件安装到用户目录，例如 `%LOCALAPPDATA%\CodexSpeak` 或 `%USERPROFILE%\.codex\codex-speak`。
- 安装器需要处理 PowerShell 执行策略、路径空格、杀毒软件误报和 SmartScreen 提示。

## 安装器架构

安装器是第一阶段的核心交付物之一。

```text
bootstrap installer
  -> 检测系统和架构
  -> 检测 Codex 配置目录
  -> 备份原配置
  -> 安装 Skill
  -> 安装 Hook wrapper
  -> 安装 speak-engine
  -> 下载/校验 TTS 引擎和模型
  -> 可选安装 Plugin
  -> 可选安装 Tauri 控制面板
  -> 写入配置
  -> 运行 doctor 自检
  -> 播放测试音频
```

卸载流程：

```text
停止朗读进程
移除 Hook
移除 Skill
保留或删除模型，由用户选择
恢复 Codex 配置备份
运行 Codex 配置检查
```

升级流程：

```text
读取已安装版本
保留用户配置
更新程序文件
按需更新模型 manifest
迁移配置
运行 doctor 自检
```

## 安装包形态

| 阶段 | macOS | Windows | 目标 |
| --- | --- | --- | --- |
| 内测 | shell 脚本 | PowerShell 脚本 | 快速验证 |
| 公测 | 签名脚本 + 压缩包 | 签名脚本 + 压缩包 | 降低安装门槛 |
| 正式版 | 签名公证 `.pkg`/`.app` | 签名 MSIX/MSI/EXE | 面向普通用户 |

模型建议与程序分离：

- 安装器体积更小。
- 可以按需下载中文默认模型。
- 后续可以单独升级声音模型。
- 低配用户可以只下载 Piper 兜底模型。

## speak-engine 形态

第一阶段可以用脚本快速验证，但正式分发建议做成跨平台 CLI：

```text
codex-speak
  speak
  stop
  doctor
  status
  mcp
  config get
  config set
  install
  uninstall
```

## Tauri 控制面板

Tauri 控制面板是普通用户手动控制入口，不直接做 TTS 推理。它调用同一个 Rust CLI：

```text
Tauri UI -> Tauri backend -> codex-speak CLI -> config / stop / speak / doctor
```

当前第一版位于：

```text
apps/codex-speak-control
```

它提供：

- 自动朗读开关。
- 儿童模式开关。
- 语速滑块。
- 最大朗读字数滑块。
- 声音档位选择。
- 试听、停止、刷新、自检按钮。

这样可以把“普通用户点击配置”和“Codex 通过 MCP 改配置”统一到同一份 `config.toml`。

实现语言建议：

| 方案 | 优点 | 缺点 | 建议 |
| --- | --- | --- | --- |
| Shell + PowerShell | 快速、简单 | 跨平台维护分裂 | 原型期使用 |
| Node.js | 开发快 | 需要打包运行时 | 可做中期方案 |
| Go/Rust 单文件 CLI | 跨平台、部署干净 | 开发成本更高 | 正式版推荐 |
| Python | 生态丰富 | 依赖重，用户安装麻烦 | 不推荐做安装主链路 |

## 模型管理

使用 manifest 管理模型：

```toml
[[models]]
id = "melo-zh-en"
provider = "sherpa_onnx"
language = "zh,en"
size_mb = 163
url = "..."
sha256 = "..."
default = true

[[models]]
id = "piper-zh-cn-huayan"
provider = "piper"
language = "zh"
low_resource = true
url = "..."
sha256 = "..."
```

安装器必须校验下载文件，避免模型损坏或被替换。

## 配置示例

```toml
enabled = true
language = "zh"
child_mode = true
max_read_chars = 800

tts_provider = "sherpa_melo"
fallback_providers = ["sherpa_kokoro", "piper", "system"]

speed = 0.9
num_threads = 4
vits_noise_scale = 0.45
vits_noise_scale_w = 0.6
tts_silence_scale = 0.25
voice = "default"
skip_code_blocks = true
```

## 目录建议

```text
~/.codex/codex-speak/
  config.toml
  hooks/
    codex-speak-notify
  bin/
    speak-engine
  models/
    melo/
    kokoro/
    piper/
  logs/
    last-spoken.txt
    last-error.log
```

## 开发里程碑

### M1：可用原型

- Skill 生成 Codex Speak Protocol 朗读导览。
- Plugin/MCP 优先写入 side-channel。
- Hook 优先消费 side-channel，其次提取 HTML 微格式协议或旧版导览。
- 先用系统朗读播放。
- 提供 macOS shell 安装脚本和卸载脚本。

### M2：本地 TTS

- 接入 Sherpa-ONNX + MeloTTS。
- 增加 Kokoro/Piper 试听命令。
- 失败时自动兜底。
- 支持 Windows PowerShell 安装脚本。
- 增加 `doctor` 自检命令。

### M3：配置和控制

- 支持开关、语速、字数限制。
- 支持停止当前朗读。
- 记录最近一次朗读内容。
- 支持升级和配置迁移。

### M4：Plugin 或小型 UI

- 可视化开关。
- 选择声音。
- 手动重读。
- 查看日志和模型状态。
- 提供正式安装包和签名发布流程。

## 主要风险

- 中文多音字、英文技术词混读仍可能不自然。
- 不同 Windows 机器音频播放环境差异较大。
- 模型许可要单独核对，尤其是后续打包分发时。
- Skill 不是强制执行机制，仍需 Hook 规则清洗作为兜底。
- macOS Gatekeeper 和 Windows SmartScreen 会影响普通用户安装体验。
- 模型下载速度、校验失败和断点续传会影响首次安装体验。

## 参考资料

- Sherpa-ONNX TTS 预训练模型：https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/index.html
- Sherpa-ONNX Kokoro 文档：https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/kokoro.html
- Sherpa-ONNX VITS/MeloTTS 文档：https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/vits.html
- MeloTTS GitHub：https://github.com/myshell-ai/MeloTTS
- MeloTTS 中文模型：https://huggingface.co/myshell-ai/MeloTTS-Chinese
- Kokoro 中文 ONNX：https://huggingface.co/onnx-community/Kokoro-82M-v1.1-zh-ONNX
- Piper 新仓库：https://github.com/OHF-Voice/piper1-gpl
- Piper 中文声音示例：https://huggingface.co/rhasspy/piper-voices/blob/main/zh/zh_CN/huayan/x_low/MODEL_CARD
- Apple macOS 公证说明：https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- Microsoft MSIX 文档：https://learn.microsoft.com/en-us/windows/msix/
- Windows 应用签名选项：https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options
