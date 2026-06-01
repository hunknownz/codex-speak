# 安装与分发

## 普通安装目标

Codex Speak 的安装要做到四件事：

- 安装 Rust CLI 到 `~/.codex/codex-speak/bin`。
- 安装 Tauri 控制面板到 `~/.codex/codex-speak/apps`，安装 macOS 原生 Pet helper 到 `~/.codex/codex-speak/bin`。
- 配置 Codex notify Hook，让 Codex 回复结束后自动触发朗读。
- 安装 Codex Skill 和 Codex Plugin，让 Codex 可以生成儿童友好的朗读导览。
- 安装或修复本地 TTS runtime 和默认中文模型。

## macOS

### release 包安装

下载并解压发布包后进入目录：

```bash
tar -xzf codex-speak-macos.tar.gz
cd codex-speak-macos
./installers/install-macos.sh
```

release 包内已经包含：

```text
bin/codex-speak
bin/codex-speak-pet-macos
apps/Codex Speak.app
assets/pet/
installers/
docs/
```

所以普通用户不需要安装 Rust、Cargo、Node.js、npm 或 Swift 编译器。首次安装会下载本地 TTS runtime 和默认中文模型；如果机器上已经有模型，可以使用：

```bash
./installers/install-macos.sh --skip-tts-download
```

### 源码安装

开发机或源码 checkout 中运行：

```bash
./installers/install-macos.sh
```

源码安装会在本机用 Cargo、swiftc 和 npm 构建 CLI、Pet helper 和控制面板。如果机器上已经有模型，只更新 CLI、Hook、Skill 和 Plugin：

```bash
./installers/install-macos.sh --skip-tts-download
```

如果只想安装核心链路，暂时跳过控制面板 App 和桌面 Pet：

```bash
./installers/install-macos.sh --skip-control-app
```

控制面板安装位置：

```text
~/.codex/codex-speak/apps/Codex Speak.app
```

macOS 原生 Pet helper 安装位置：

```text
~/.codex/codex-speak/bin/codex-speak-pet-macos
```

Pet 透明动画素材安装位置：

```text
~/.codex/codex-speak/assets/pet/
  codex-agent.mov
  codex-agent-hit.png
  codex-agent-preview.png
  ASSET-NOTICE.txt
```

`codex-agent.mov` 是 1080x1920、约 10 秒的 HEVC-with-alpha 透明动画，macOS helper 会用原生 `AVPlayerLayer` 显示它；`codex-agent-hit.png` 是点击命中的兜底 alpha mask；`codex-agent-preview.png` 是透明静帧预览，方便快速确认角色形象。

打开控制面板：

```bash
~/.codex/codex-speak/bin/codex-speak app open
```

卸载：

```bash
./installers/uninstall-macos.sh
```

## Windows

### release 包安装

下载并解压发布包后进入目录：

```powershell
Expand-Archive .\codex-speak-windows.zip
cd .\codex-speak-windows\codex-speak-windows
powershell -ExecutionPolicy Bypass -File .\installers\install-windows.ps1
```

release 包内已经包含：

```text
bin\codex-speak.exe
apps\codex-speak-control.exe
installers\
docs\
```

所以普通用户不需要安装 Rust、Cargo、Node.js 或 npm。首次安装会下载本地 TTS runtime 和默认中文模型；如果机器上已经有模型，可以使用：

```powershell
powershell -ExecutionPolicy Bypass -File .\installers\install-windows.ps1 -SkipTtsDownload
```

### 源码安装

在源码 checkout 的 PowerShell 中运行：

```powershell
.\installers\install-windows.ps1
```

源码安装会在本机用 Cargo 和 npm 构建 CLI 和控制面板。如果机器上已经有模型，只更新本地程序和 Codex 集成：

```powershell
.\installers\install-windows.ps1 -SkipTtsDownload
```

如果只想安装核心链路，暂时跳过控制面板 App：

```powershell
.\installers\install-windows.ps1 -SkipControlApp
```

控制面板安装位置：

```text
%USERPROFILE%\.codex\codex-speak\apps\codex-speak-control.exe
```

打开控制面板：

```powershell
%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe app open
```

卸载：

```powershell
.\installers\uninstall-windows.ps1
```

## Plugin 安装位置

安装器会把插件复制到个人 Codex Plugin marketplace：

```text
~/.agents/plugins/plugins/codex-speak
~/.agents/plugins/marketplace.json
```

marketplace 条目使用本地路径：

```text
./plugins/codex-speak
```

这样 Codex App 可以发现插件，插件里的 MCP 脚本会调用同一份已安装的 Rust CLI。

## 自检

安装完成后运行：

```bash
~/.codex/codex-speak/bin/codex-speak doctor
```

机器可读自检结果：

```bash
~/.codex/codex-speak/bin/codex-speak doctor --json
```

期望看到这些检查通过：

- CLI 已安装。
- 控制面板 App 已安装，或显示为 WARN。
- macOS 原生 Pet helper 已安装；Windows 当前会跳过这项原生 helper 检查。
- Hook 已配置。
- Plugin 已安装。
- Plugin marketplace 已配置。
- Sherpa-ONNX 和默认中文模型可用。
- 本机播放器可用。Windows 会检查 `powershell.exe`，因为系统语音和 wav 播放都走 PowerShell/.NET 播放链路。

查看所有可选本地朗读引擎和模型状态：

```bash
~/.codex/codex-speak/bin/codex-speak models list
```

模型和 TTS runtime 下载会先写入临时文件，再在校验通过后替换目标文件。默认安装涉及的 macOS/Windows Sherpa runtime、MeloTTS、Kokoro、ZipVoice、ZipVoice vocoder 和 Piper 中文轻量模型都已经固定 sha256。

## 发布包

CI 配置：

```text
.github/workflows/ci.yml
```

发布构建配置：

```text
.github/workflows/release.yml
```

Release workflow 会构建：

- `codex-speak-macos.tar.gz`：包含 macOS CLI、Tauri `.app`、原生 Pet helper、透明 Pet 素材、安装/卸载脚本和文档。
- `codex-speak-windows.zip`：包含 Windows CLI、Tauri 控制面板 exe、安装/卸载脚本和文档。
- 对应的 `.sha256` 校验文件。

打 tag 时，workflow 会把这些文件发布到 GitHub Release。普通用户下载后可以先校验：

`v*-rc*` 标签会生成 draft prerelease，用来验证发布包；稳定版 `v*` 标签会生成正式 release。

macOS:

```bash
shasum -a 256 -c codex-speak-macos.tar.gz.sha256
```

Windows:

```powershell
Get-FileHash .\codex-speak-windows.zip -Algorithm SHA256
Get-Content .\codex-speak-windows.zip.sha256
```

CI 会做两层检查：

- 压缩前运行 `scripts/check-release-package.mjs` 检查 release 包目录，确保安装脚本、二进制、控制面板、文档和 macOS Pet 素材都在正确位置。
- 压缩后解包并执行 release 包里的安装脚本，使用跳过模型下载的模式做一次安装烟测；烟测会确认安装后的 CLI、控制面板、macOS Pet helper 和素材落位，并运行 `codex-speak models list` 检查 CLI 能正常启动。

发布前可以运行 readiness 检查：

```bash
node scripts/check-release-readiness.mjs
```

完整人工验收清单见：

```text
docs/release-qa.md
```

签名和公证是可选通道，配置 GitHub Secrets 后会自动启用。细节见：

```text
docs/signing.md
```

正式对外分发前还需要确认：

- macOS：签名和公证；DMG 可以在签名稳定后再打开。
- Windows：代码签名，降低 SmartScreen 提示。
