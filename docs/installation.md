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
scripts/manual-qa-macos.sh
scripts/check-manual-qa-report.mjs
scripts/check-release-manifest.mjs
release-manifest.json
docs/
```

所以普通用户不需要安装 Rust、Cargo、Node.js、npm 或 Swift 编译器。首次安装会下载本地 TTS runtime 和默认中文模型；如果机器上已经有模型，可以使用：

```bash
./installers/install-macos.sh --skip-tts-download
```

安装完成后，安装器会在控制面板和桌面组件复制完成后打印下一步命令：运行自检、打开控制面板、生成支持包；如果跳过了模型下载，还会提示之后如何安装默认中文模型。

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
  codex-agent-source-spritesheet.png
  codex-agent-hit.png
  codex-agent-preview.png
  ASSET-NOTICE.txt
```

`codex-agent.mov` 是 1080x1920、约 10 秒的 HEVC-with-alpha 透明动画，macOS helper 会用原生 `AVPlayerLayer` 显示它；`codex-agent-source-spritesheet.png` 是 6 帧二维行走素材源；`codex-agent-hit.png` 是点击命中的兜底 alpha mask；`codex-agent-preview.png` 是透明静帧预览，方便快速确认角色形象。

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
scripts\manual-qa-windows.ps1
scripts\check-manual-qa-report.mjs
scripts\check-release-manifest.mjs
release-manifest.json
docs\
```

所以普通用户不需要安装 Rust、Cargo、Node.js 或 npm。首次安装会下载本地 TTS runtime 和默认中文模型；如果机器上已经有模型，可以使用：

```powershell
powershell -ExecutionPolicy Bypass -File .\installers\install-windows.ps1 -SkipTtsDownload
```

安装完成后，安装器会在控制面板复制完成后打印下一步命令：运行自检、打开控制面板、生成支持包；如果跳过了模型下载，还会提示之后如何安装默认中文模型。

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

当某项检查失败或警告时，`doctor` 会在文本输出和 JSON 输出里给出 `hint`，告诉用户下一步应该重新安装、安装模型、刷新插件，还是检查系统播放器。它不只检查文件是否存在，也会检查 Skill、Hook wrapper、Plugin manifest、Plugin Skill、MCP 配置和当前平台 MCP 脚本是否与当前 CLI 内置版本一致；如果用户更新了 CLI 但插件还是旧文件，会提示重新运行 `codex-speak install` 刷新。

`doctor --json` 和 `status` 都会包含版本、系统和 CPU 架构信息，方便远程排障时确认用户正在运行哪个构建。

刚安装完、想确认这台机器是否已经具备可交付的基础链路，可以运行：

```bash
~/.codex/codex-speak/bin/codex-speak verify-install
```

如果安装时跳过了默认模型下载，可以允许模型项暂时缺失，但仍然检查 CLI、Hook、Plugin、控制面板和播放器：

```bash
~/.codex/codex-speak/bin/codex-speak verify-install --allow-missing-models
```

还可以验证 Codex 集成主路径：

```bash
~/.codex/codex-speak/bin/codex-speak verify-codex
```

这个命令不会播放声音，会模拟 MCP 写入儿童友好导览、Hook 优先消费 side-channel，并检查普通回复兜底清洗时不会逐字朗读代码、命令和长路径。

还可以验证控制项能安全切换并恢复：

```bash
~/.codex/codex-speak/bin/codex-speak verify-controls
```

这个命令会临时切换自动朗读、儿童模式、语速、最大朗读字数、声音档位和 TTS 引擎，确认配置能保存、重新读取，并且 `status` 会反映这些变化；结束时会恢复原配置。

期望看到这些检查通过：

- CLI 已安装。
- 控制面板 App 已安装，或显示为 WARN。
- macOS 原生 Pet helper 已安装；Windows 当前会跳过这项原生 helper 检查。
- Hook 已配置，并且 Hook wrapper 与当前 CLI 一致。
- Codex Speak Skill、Plugin manifest、Plugin Skill、MCP 配置和当前平台 MCP 脚本都已安装且与当前 CLI 一致。
- Plugin marketplace 已配置。
- Sherpa-ONNX 和默认中文模型可用。
- 本机播放器可用。Windows 会检查 `powershell.exe`，因为系统语音和 wav 播放都走 PowerShell/.NET 播放链路。

## 本地发音词典

如果某个英文项目名、缩写或工具名读起来别扭，可以给本机加一条发音规则：

```bash
~/.codex/codex-speak/bin/codex-speak pronunciation set --term OpenRouter --spoken "Open Router 平台"
~/.codex/codex-speak/bin/codex-speak pronunciation preview --text "我配置了 OpenRouter。"
```

规则写入 `~/.codex/codex-speak/pronunciation.toml`。`doctor` 会检查这个文件是否能解析，`status` 会显示词条数量，支持包默认只回传词条数量；只有显式加 `--include-private` 才会包含词典内容。

查看所有可选本地朗读引擎和模型状态：

```bash
~/.codex/codex-speak/bin/codex-speak models list
```

如果需要把安装问题发给别人帮忙排查，可以生成一个本地支持包：

```bash
~/.codex/codex-speak/bin/codex-speak support-bundle
```

它会输出一个目录路径，里面包含 `doctor.json`、`status.json`、`models.json`、`pronunciation-dictionary.json`、环境信息和最近日志。默认支持包会脱敏本机 home 路径，并把最近朗读文本替换成提示文字，适合直接发给维护者排查安装问题。外部 QA 回传时，`check-manual-qa-report.mjs` 会自动验证默认支持包确实是脱敏的。

如果确实需要完整本机路径和最近朗读日志做深度排障，再显式运行：

```bash
~/.codex/codex-speak/bin/codex-speak support-bundle --include-private
```

使用 `--include-private` 前请先确认可以分享这些内容，因为它可能包含本机路径和最近朗读记录。

如果用户是从 release 包安装的，支持包还会包含安装时保留下来的 `release-manifest.json`，用于确认版本、git commit、平台和关键文件 sha256。

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
- 包内 `release-manifest.json`：记录版本、git commit、平台、生成时间、关键二进制 sha256 和推荐验证命令。
- 包内 `scripts/check-release-manifest.mjs`：解压后可校验 manifest 和关键文件 sha256。
- 对应的 `.sha256` 校验文件。

打 tag 时，workflow 会把这些文件发布到 GitHub Release。普通用户下载后可以先校验：

`v*-rc*` 标签会生成 draft prerelease，用来验证发布包；稳定版 `v*` 标签会生成正式 release。

macOS:

```bash
shasum -a 256 -c codex-speak-macos.tar.gz.sha256
```

Windows:

```powershell
$Expected = ((Get-Content .\codex-speak-windows.zip.sha256 -Raw) -split '\s+')[0].ToLowerInvariant()
$Actual = (Get-FileHash .\codex-speak-windows.zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($Actual -ne $Expected) { throw "sha256 mismatch: expected $Expected but got $Actual" }
```

解压后，可以用包里的 CLI 校验 release manifest 中记录的关键文件：

macOS:

```bash
./bin/codex-speak verify-package --package-dir .
```

Windows:

```powershell
.\bin\codex-speak.exe verify-package --package-dir .
```

如果机器上有 Node.js，也可以用随包脚本做同样的校验：

```bash
node scripts/check-release-manifest.mjs .
```

CI 会做三层检查：

- 压缩前运行 `scripts/check-release-package.mjs` 检查 release 包目录，确保 release manifest、manifest 校验脚本、安装脚本、二进制、控制面板、文档和 macOS Pet 素材都在正确位置。
- 压缩后解包并执行 release 包里的安装脚本，使用跳过模型下载的模式做一次安装烟测；烟测会确认安装后的 CLI、控制面板、macOS Pet helper 和素材落位，并运行 `codex-speak models list` 检查 CLI 能正常启动。
- 烟测还会先运行包内 `codex-speak verify-package` 和 `check-release-manifest.mjs`，再运行 `codex-speak doctor --json`、`codex-speak verify-codex`、`codex-speak verify-controls` 和 `codex-speak support-bundle`，验证机器可读自检结果能解析、Codex 集成主路径可用、常见英文技术缩写会先转成中文说法、控制项能写入并恢复、支持包能生成、安装后的 release manifest 能回传，并且核心安装项已经 OK；因为烟测跳过模型下载，模型相关检查允许失败。
- macOS/Windows 烟测还会用非交互模式运行 release 包里的手工 QA 收集脚本，并用 `check-manual-qa-report.mjs` 校验生成的 `qa-report.json`，确保真机 QA 收集和回传校验脚本本身没有随包损坏。

发布前可以运行 readiness 检查：

```bash
node scripts/check-release-readiness.mjs
```

如果本地 `dist` 里已经有 release 包或 QA handoff，readiness 会同时确认它们来自当前 `HEAD`，并验证压缩包 `.sha256` 没有过期；如果刚改过代码或脚本，需要先重新打包再交给外部测试者。

给外部测试者发包前，可以生成一份 QA 交付说明：

```bash
node scripts/prepare-qa-handoff.mjs --allow-missing
```

它会读取 `dist` 里已经构建好的 release 包和 manifest，输出 `dist/qa-handoff/README.md` 与 `qa-handoff.json`，列出包哈希、安装命令、验收命令、测试者需要回传的 QA 目录，以及维护者收到回传后要跑的校验命令。

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
