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

正式发布前同时要求签名环境和两台真机 QA 回传：

```bash
node scripts/check-release-readiness.mjs \
  --tag v0.1.0 \
  --require-signing-env \
  --require-manual-qa \
  --macos-qa-dir /path/to/codex-speak-macos-qa-... \
  --windows-qa-dir C:\path\to\codex-speak-windows-qa-...
```

离线或本地开发时只检查文件布局和本地状态：

```bash
node scripts/check-release-readiness.mjs --offline --allow-dirty
```

`--offline` 是不访问 GitHub 的本地预检：它会跳过 `origin/main` 和 GitHub Actions 状态检查，适合在网络受限或外部 QA 机器上先验证文件、脚本、签名门禁和 QA 报告结构。正式发布前仍应运行不带 `--offline` 的检查，确认本地 `HEAD` 已推到 `origin/main` 且 CI/Release 状态正常。

不带 `--offline` 的检查会访问 GitHub Actions API；如果遇到匿名 API 限流，先设置 `GITHUB_TOKEN` 再运行。

发给外部测试者前，可以生成 QA 交付说明：

```bash
node scripts/prepare-qa-handoff.mjs --allow-missing
```

这个脚本会检查已构建包的 `release-manifest.json` 是否匹配当前 `HEAD`、是否来自干净的 tracked source，确认 release 包 `.sha256` 文件没有过期，计算 release 包 sha256，并生成 `dist/qa-handoff/README.md` 与 `qa-handoff.json`。测试者照 README 执行后，把整个 `codex-speak-macos-qa-*` 或 `codex-speak-windows-qa-*` 输出目录发回即可。

传入 `--allow-missing` 时，脚本可以只生成当前已有平台的部分 handoff，例如只有 macOS 包时先发给 macOS 测试者；生成的 README 和 JSON 会标明缺失的平台。正式发布 readiness 仍然需要 macOS 和 Windows 两边的 QA 回传。

如果本机缺 Windows 包，可以把最新成功的 main CI artifacts 写入 handoff：

```bash
node scripts/prepare-qa-handoff.mjs --allow-missing --include-ci-artifacts
```

它会查询当前 `HEAD` 对应的成功 CI run，把 `codex-speak-macos-ci` 和 `codex-speak-windows-ci` 的名称、大小、run 链接和下载 API 地址写入 `qa-handoff.json` 与 README。CI artifact 来自同一套 release package smoke，适合发给 Windows 测试者做外部 QA；正式公开发布仍应使用 tag 触发的 GitHub Release 产物。

如果 GitHub API 匿名额度用完，或 artifact 需要登录权限，先设置 `GITHUB_TOKEN` 再运行这个命令。

脚本会检查：

- 当前分支是 `main`。
- 工作区干净，除非显式传入 `--allow-dirty`。
- 本地 `HEAD` 已经推到 `origin/main`。
- CI 对当前 `HEAD` 已经成功。
- 指定 tag 已经推到远端，且 release workflow 成功。
- release 脚本、安装脚本、文档、Pet 素材和图标都存在。
- main CI 的 release package smoke 会上传 macOS/Windows QA artifacts，便于从最新成功 CI 取得 Windows 外测包；`check-release-readiness.mjs` 会在线确认这两个 CI artifact 存在、未过期且大小大于零。
- release workflow 对稳定版 tag 有签名门禁，避免正式版本在缺少 secrets 时继续发布。
- 如果本地 `dist` 里已经有 release 包或 QA handoff，它们的 manifest 必须匹配当前 `HEAD`，压缩包 `.sha256` 也必须匹配实际文件，避免把旧包误发给外部测试者。
- 如果本地 QA handoff 只包含单个平台，readiness 会给 warning，提醒它只是部分外部验收材料。
- 需要正式签名发布时，签名环境变量已经配置。
- 传入 `--require-manual-qa` 时，macOS 和 Windows 真机 QA 报告必须都存在、通过 `check-manual-qa-report.mjs`，且回传支持包里的 `release-manifest.json` 必须匹配当前 git commit。
- release 包内包含 `release-manifest.json`，可以追踪版本、git commit、平台和关键文件 sha256。
- release 包内包含 `scripts/check-release-manifest.mjs`，解压后可以校验 manifest 和关键文件 sha256。
- `check-manual-qa-report.mjs` 会校验支持包默认脱敏：`support-bundle-metadata.json` 必须声明 `redacted=true`、`includePrivate=false`，支持包里不能残留本机 home 路径，`logs/last-spoken.txt` 如果存在也必须是脱敏提示。

## macOS 人工验收

使用 release 包验收：

```bash
tar -xzf codex-speak-macos.tar.gz
cd codex-speak-macos
./installers/install-macos.sh --skip-tts-download
```

推荐同时运行随包提供的 QA 收集脚本：

```bash
./scripts/manual-qa-macos.sh --allow-missing-models
```

它会自动运行 release 包 manifest 校验、自检、状态、模型列表、Codex 集成预检、控制项验收、混合中英文术语归一化检查和支持包收集，并在控制面板、控制项调节、发音词典新增/预览/删除、桌面 Pet、试听、混合中英文试听、停止这几项上让测试者输入 `y`、`n` 或 `s`。最终会输出 `qa-report.json` 和 `support-bundle` 目录，方便把 macOS 真机结果发回排障。

收到 QA 输出目录后，可以在源码仓库或 release 包里校验：

```bash
node scripts/check-manual-qa-report.mjs /path/to/codex-speak-macos-qa-...
```

检查项：

- 包内 `release-manifest.json` 的 `platform` 应为 `macos`，`version`、`git.commit` 和关键文件 `sha256` 应存在。
- `./bin/codex-speak verify-package --package-dir .` 应通过，用来证明 manifest 中记录的关键文件没有缺失或被替换。
- 有 Node.js 时，`node scripts/check-release-manifest.mjs .` 也应通过，作为脚本版交叉校验。
- 安装后 `~/.codex/codex-speak/release-manifest.json` 应存在，并且 `support-bundle` 应把它复制出来。
- `~/.codex/codex-speak/bin/codex-speak doctor` 输出核心检查通过。
- `~/.codex/codex-speak/bin/codex-speak doctor --json` 能输出可解析 JSON，并包含版本、系统和 CPU 架构信息；如果安装时用了 `--skip-tts-download`，允许模型相关检查失败，但 CLI、Hook、Hook wrapper、Codex Speak Skill、Plugin manifest、Plugin Skill、MCP 配置、当前平台 MCP 脚本、控制面板、Pet helper 和播放器检查必须通过；这些集成文件不只要存在，还要与当前 CLI 内置版本一致；失败或警告项应带有可执行的 `hint`。
- `~/.codex/codex-speak/bin/codex-speak verify-install --allow-missing-models` 应通过，用来证明跳过模型下载时核心安装链路仍然可交付。
- `~/.codex/codex-speak/bin/codex-speak verify-codex` 应通过，用来证明 MCP side-channel、Hook 风格消费、普通回复兜底清洗、常见英文技术缩写归一化和本地发音词典链路可用。
- `~/.codex/codex-speak/bin/codex-speak verify-controls` 应通过，用来证明自动朗读、儿童模式、语速、最大朗读字数、声音档位和 TTS 引擎能临时切换、重新读取，并恢复原配置。
- `~/.codex/codex-speak/bin/codex-speak extract --text "我运行 hello world，并检查 README.md、codex_speak_prepare、--provider sherpa_melo、OpenRouter、OAuth、M C P、J.S.O.N、CLI、API、CPU、XYZ、build failed because timeout 和 ProjectAlpha42。"` 应把常见技术英文、文件名、命令参数、代码标识符、缩写和混在中文里的未知英文转换成中文可懂说法，例如“你好世界示例”“说明文件”“准备朗读导览的插件工具”“命令参数”“开放路由平台”“授权登录协议”“插件通道”“数据格式”“命令行工具”“处理器”“英文缩写”“英文短语”“英文编号”，不能原样留下 `README.md`、`codex_speak_prepare`、`--provider`、`OpenRouter`、`OAuth`、`M C P`、`J.S.O.N`、`build failed`、`ProjectAlpha42` 让中文语音逐字母读。
- `~/.codex/codex-speak/bin/codex-speak app open` 能打开控制面板。
- 控制面板能显示当前 provider、儿童模式、语速、模型状态，并能添加、预览、删除一条发音词典规则。
- 点击试听后能听到系统兜底或已安装模型的声音。
- 混合中英文试听时，技术英文不应一个字母一个字母地读。
- 点击停止后朗读会停止。
- 桌面 Pet 显示为透明原生窗口，没有白色或麦色背景块。
- Pet 朗读中点击能停止朗读，双击能打开控制面板。
- `models list` 能正常输出。
- `support-bundle` 能生成包含 `doctor.json`、`status.json`、`models.json`、`pronunciation-dictionary.json` 和 `support-bundle-metadata.json` 的本地排障目录；默认支持包必须脱敏本机 home 路径、最近朗读文本，并且只回传发音词典数量而不是词典明文。

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

推荐同时运行随包提供的 QA 收集脚本：

```powershell
.\scripts\manual-qa-windows.ps1 -AllowMissingModels
```

它会自动运行 release 包 manifest 校验、自检、状态、模型列表、Codex 集成预检、控制项验收、混合中英文术语归一化检查和支持包收集，并在控制面板、控制项调节、试听、混合中英文试听、停止这几项上让测试者输入 `y`、`n` 或 `s`。最终会输出 `qa-report.json` 和 `support-bundle` 目录，方便把 Windows 真机结果发回排障。

收到 QA 输出目录后，可以在源码仓库或 release 包里校验：

```powershell
node .\scripts\check-manual-qa-report.mjs C:\path\to\codex-speak-windows-qa-...
```

检查项：

- 包内 `release-manifest.json` 的 `platform` 应为 `windows`，`version`、`git.commit` 和关键文件 `sha256` 应存在。
- `.\bin\codex-speak.exe verify-package --package-dir .` 应通过，用来证明 manifest 中记录的关键文件没有缺失或被替换。
- 有 Node.js 时，`node .\scripts\check-release-manifest.mjs .` 也应通过，作为脚本版交叉校验。
- 安装后 `%USERPROFILE%\.codex\codex-speak\release-manifest.json` 应存在，并且 `support-bundle` 应把它复制出来。
- `%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe doctor` 能运行。
- `%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe doctor --json` 能输出可解析 JSON，并包含版本、系统和 CPU 架构信息；如果安装时用了 `-SkipTtsDownload`，允许模型相关检查失败，但 CLI、Hook、Hook wrapper、Codex Speak Skill、Plugin manifest、Plugin Skill、MCP 配置、当前平台 MCP 脚本、控制面板和播放器检查必须通过；这些集成文件不只要存在，还要与当前 CLI 内置版本一致；失败或警告项应带有可执行的 `hint`。
- `%USERPROFILE%\.codex\codex-speak\bin\codex-speak.exe verify-install --allow-missing-models` 应通过，用来证明跳过模型下载时核心安装链路仍然可交付。
- `codex-speak.exe verify-codex` 应通过，用来证明 MCP side-channel、Hook 风格消费、普通回复兜底清洗、常见英文技术缩写归一化和本地发音词典链路可用。
- `codex-speak.exe verify-controls` 应通过，用来证明自动朗读、儿童模式、语速、最大朗读字数、声音档位和 TTS 引擎能临时切换、重新读取，并恢复原配置。
- `codex-speak.exe extract --text "我运行 hello world，并检查 README.md、codex_speak_prepare、--provider sherpa_melo、OpenRouter、OAuth、M C P、J.S.O.N、CLI、API、CPU、XYZ、build failed because timeout 和 ProjectAlpha42。"` 应把常见技术英文、文件名、命令参数、代码标识符、缩写和混在中文里的未知英文转换成中文可懂说法，例如“你好世界示例”“说明文件”“准备朗读导览的插件工具”“命令参数”“开放路由平台”“授权登录协议”“插件通道”“数据格式”“命令行工具”“处理器”“英文缩写”“英文短语”“英文编号”，不能原样留下 `README.md`、`codex_speak_prepare`、`--provider`、`OpenRouter`、`OAuth`、`M C P`、`J.S.O.N`、`build failed`、`ProjectAlpha42` 让中文语音逐字母读。
- `codex-speak.exe app open` 能打开 Tauri 控制面板。
- 控制面板能切换儿童模式、语速、声音档位和 provider，并能添加、预览、删除一条发音词典规则。
- `codex-speak.exe speak --text "你好，这是 Windows 朗读测试。"` 能播放。
- 混合中英文试听时，技术英文不应一个字母一个字母地读。
- `codex-speak.exe stop` 能停止正在播放的声音。
- PowerShell 播放链路不会留下持续运行的子进程。
- `models list` 能正常输出。
- `support-bundle` 能生成包含 `doctor.json`、`status.json`、`models.json`、`pronunciation-dictionary.json` 和 `support-bundle-metadata.json` 的本地排障目录；默认支持包必须脱敏本机 home 路径、最近朗读文本，并且只回传发音词典数量而不是词典明文。
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

自动化预检先运行：

```bash
~/.codex/codex-speak/bin/codex-speak verify-codex
```

它不能替代真实 Codex session 的肉眼确认，但会先证明 MCP 工具写入、Hook 风格消费、兜底清洗和英文技术词归一化的本地链路没有断。

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
- 没有真实签名 secrets 时，日常 CI、手动 release workflow 和 `v*-rc*` 测试标签会跳过签名；稳定版 `v*` 且不包含 `-rc` 时 release workflow 会失败，不会继续发布未签名包。
