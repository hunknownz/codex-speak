# Release QA

Codex Speak 的发布目标不是“能在开发机跑”，而是别人下载 release 包后，能在 macOS 或 Windows 上完成安装、打开控制面板、朗读、停止和自检。

## 一句话发布门槛

每个正式版本必须通过自动化 release readiness 检查、release package smoke、至少一次 macOS 人工验收，以及至少一次 Windows 真机人工验收。

## 自动化检查

日常检查当前 `main`：

```bash
node scripts/check-release-readiness.mjs
```

验证 RC tag：

```bash
node scripts/check-release-readiness.mjs --tag v0.1.0-rc.1
```

正式发布前要求签名环境：

```bash
node scripts/check-release-readiness.mjs --tag v0.1.0 --require-signing-env
```

离线或本地开发时只检查文件布局和本地状态：

```bash
node scripts/check-release-readiness.mjs --offline --allow-dirty
```

脚本会检查：

- 当前分支是 `main`。
- 工作区干净，除非显式传入 `--allow-dirty`。
- 本地 `HEAD` 已经推到 `origin/main`。
- CI 对当前 `HEAD` 已经成功。
- 指定 tag 已经推到远端，且 release workflow 成功。
- release 脚本、安装脚本、文档、Pet 素材和图标都存在。
- 需要正式签名发布时，签名环境变量已经配置。

## macOS 人工验收

使用 release 包验收：

```bash
tar -xzf codex-speak-macos.tar.gz
cd codex-speak-macos
./installers/install-macos.sh --skip-tts-download
```

检查项：

- `~/.codex/codex-speak/bin/codex-speak doctor` 输出核心检查通过。
- `~/.codex/codex-speak/bin/codex-speak doctor --json` 能输出可解析 JSON；如果安装时用了 `--skip-tts-download`，允许模型相关检查失败，但 CLI、Hook、Plugin manifest、Plugin Skill、MCP 配置、当前平台 MCP 脚本、控制面板、Pet helper 和播放器检查必须通过。
- `~/.codex/codex-speak/bin/codex-speak app open` 能打开控制面板。
- 控制面板能显示当前 provider、儿童模式、语速、模型状态。
- 点击试听后能听到系统兜底或已安装模型的声音。
- 点击停止后朗读会停止。
- 桌面 Pet 显示为透明原生窗口，没有白色或麦色背景块。
- Pet 朗读中点击能停止朗读，双击能打开控制面板。
- `models list` 能正常输出。

完整模型下载验收至少覆盖一次：

```bash
~/.codex/codex-speak/bin/codex-speak models install --provider sherpa_melo
~/.codex/codex-speak/bin/codex-speak speak --text "你好，这是 Codex Speak 的中文朗读测试。"
~/.codex/codex-speak/bin/codex-speak doctor --json
```

模型安装完成后，`doctor --json` 的 `ok` 应为 `true`。

## Windows 真机验收

在 Windows 真机或干净虚拟机上下载 `codex-speak-windows.zip`，解压后执行：

```powershell
.\installers\install-windows.ps1 -SkipTtsDownload
```

检查项：

- `%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe doctor` 能运行。
- `%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe doctor --json` 能输出可解析 JSON；如果安装时用了 `-SkipTtsDownload`，允许模型相关检查失败，但 CLI、Hook、Plugin manifest、Plugin Skill、MCP 配置、当前平台 MCP 脚本、控制面板和播放器检查必须通过。
- `codex-speak.exe app open` 能打开 Tauri 控制面板。
- 控制面板能切换儿童模式、语速、声音档位和 provider。
- `codex-speak.exe speak --text "你好，这是 Windows 朗读测试。"` 能播放。
- `codex-speak.exe stop` 能停止正在播放的声音。
- PowerShell 播放链路不会留下持续运行的子进程。
- `models list` 能正常输出。
- 首次模型下载能显示进度和校验错误，不会静默失败。

完整模型下载验收至少覆盖一次：

```powershell
codex-speak.exe models install --provider sherpa_melo
codex-speak.exe speak --text "你好，这是中文本地模型朗读测试。"
codex-speak.exe doctor --json
```

模型安装完成后，`doctor --json` 的 `ok` 应为 `true`。

## Codex 集成验收

安装完成后，在新的 Codex session 中检查：

- Codex Speak Skill 可用。
- MCP side-channel 可写入朗读导览。
- Hook 能优先消费 side-channel。
- 没有 side-channel 时，Hook 会清洗普通回复后朗读。
- 长代码、命令、路径不会被逐字朗读，而会变成适合听的摘要。

## 签名验收

正式公开版本需要配置 GitHub Secrets，详见 `docs/signing.md`。

macOS：

- release workflow 导入 Developer ID 证书。
- `.app`、CLI 和 Pet helper codesign 成功。
- 公证成功或明确记录当前未启用公证的原因。
- 新机器下载后能打开控制面板。

Windows：

- `signtool.exe sign` 成功。
- `signtool verify /pa` 成功。
- 下载后 SmartScreen 提示可接受，或在 release notes 中明确说明。

## 当前已知限制

- release 包仍是 `.tar.gz` 和 `.zip`，不是 DMG、PKG、MSI 或 MSIX。
- macOS Pet helper 仅支持 macOS；Windows 第一版只提供控制面板和朗读状态，不提供原生桌面 Pet。
- Windows 音频链路依赖 PowerShell/.NET 播放能力，真机差异需要持续验收。
- 没有真实签名 secrets 时，CI 会跳过签名步骤。
