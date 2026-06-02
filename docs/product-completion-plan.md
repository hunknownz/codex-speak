# Codex Speak 产品完成计划

## 一句话目标

Codex Speak 是一个本地、免费、中文优先的 Codex 朗读助手：让孩子和初学者听懂 Codex 做了什么、结果是什么、下一步怎么推动。

## 完整产品定义

第一版完整产品不追求语音效果最终最优，先追求普通用户能安装、能看见、能控制、能稳定朗读。

必须具备：

- Codex 结束回复后自动朗读。
- Codex 可以通过 Skill/MCP 生成适合朗读的儿童友好导览。
- Hook 可以优先消费 side-channel，兜底清洗普通回复。
- 本地 TTS 至少有一个默认中文方案可用，并有系统语音兜底。
- Tauri App 可以切换儿童模式、语速、音色档位、TTS 引擎，能试听和停止。
- 桌面 Pet 可以浮在桌面上，展示待命、待朗读、朗读中、完成、错误状态。
- macOS 和 Windows 都有清楚的安装入口。
- 有自检能力，让用户知道 Hook、配置、模型、CLI 哪个环节有问题。

## 阶段计划

### P0：核心朗读链路

状态：已基本完成，继续补测试和安装边界。

包括：

- Rust CLI。
- 配置文件。
- Hook wrapper。
- Skill。
- MCP side-channel。
- HTML 协议 fallback。
- 文本清洗。
- 本地 TTS Provider 切换。

补齐项：

- 更完整的 Windows 安装逻辑。
- 更清楚的错误提示。
- 更多自动化测试覆盖 side-channel、配置和状态输出。

### P1：桌面 Pet 和控制台

状态：第一版已落地，本机已验证透明显示、Tauri App 构建和 macOS release 包安装烟测。

目标：

- Tauri App 默认打开控制台，并拉起 macOS 原生透明 Pet helper。
- Pet 通过 `~/.codex/codex-speak/state/pet-state.json` 获取状态。
- TTS、MCP、停止命令都会更新同一份 Pet 状态。
- Pet 可以拖动，朗读中点击可停止，双击打开控制台。

第一版 Pet 风格：

- 技术实现按 lil-agents 的路线：AppKit 透明浮窗 + `AVPlayerLayer` 播放 1080x1920 HEVC-with-alpha `.mov` 动画素材 + display-link 驱动移动。
- 轮廓清楚，颜色明亮，素材自带 alpha，不再用 WebView、HTML、CSS、SVG 或 canvas 绘制角色。
- 动画轻，不影响工作。
- 默认素材已经改成项目自有的 2D walking companion，由 `scripts/build-pet-assets-from-spritesheet.swift` 从 `codex-agent-source-spritesheet.png` 生成；正式品牌角色继续替换 sprite sheet 并重新生成 `~/.codex/codex-speak/assets/pet/codex-agent.mov`、`codex-agent-hit.png` 和 `codex-agent-preview.png` 即可，只要保持透明视频、alpha mask 和预览图约定。

### P2：安装和分发

状态：第一版发布链路已跑通。macOS 本地 release 包已经通过布局检查和跳过模型下载的安装烟测；GitHub Actions 已在 macOS 和 Windows 上跑过 release package smoke；`v0.1.0-rc.1` draft prerelease 的 release workflow 已成功生成并上传发布 artifacts。

目标：

- macOS 安装脚本负责 CLI、Hook、Skill、Plugin、Tauri App。
- Windows 安装脚本负责 CLI、Hook、Skill、Plugin、Tauri App。
- 模型安装支持跳过、修复、单独安装。
- GitHub Actions 构建 macOS 和 Windows 包。
- 发布物包含校验说明和最小系统要求。

本轮已推进：

- Windows 安装脚本不再只是占位，会构建 Rust CLI 并调用安装命令。
- Rust 安装器会安装个人 Codex Plugin marketplace 条目。
- 重复安装会保留已有用户配置，不会重置语速、Provider 或儿童模式。
- CI 已覆盖 macOS 和 Windows 的 Rust 核心、Tauri 后端和前端构建。
- Release workflow 会在 tag 或手动触发时构建 CLI、macOS `.app` 和 Windows 桌面包。macOS DMG 等签名/公证稳定后再打开。
- 安装脚本会把 Tauri 控制面板安装到 `~/.codex/codex-speak/apps`，没有 npm 时可以显式跳过。
- 下载资产完整性校验已补齐到 macOS/Windows Sherpa runtime、MeloTTS、Kokoro、ZipVoice、ZipVoice vocoder 和 Piper 中文轻量模型。
- 本机已打出 `codex-speak-macos.tar.gz`，通过 `scripts/check-release-package.mjs`，并从解包后的 release 目录完成一次安装烟测。
- Release 打包和安装烟测已经抽成脚本，CI 会在 macOS 和 Windows 上执行同一套 package smoke 路径，避免 release workflow 与普通 CI 逻辑分叉。
- Windows release 包只需要控制面板 `.exe`，CI 中的 Windows Tauri build 使用 `--no-bundle` 跳过额外安装器打包；真正的用户安装入口由 `install-windows.ps1` 负责。
- `v0.1.0-rc.1` 已触发 release workflow，macOS/Windows build job 和 GitHub Release 发布 job 均通过。该 RC release 是 draft prerelease，用于验证 artifact 上传链路，不作为正式公开版本。
- 发布 readiness 检查已补为脚本 `scripts/check-release-readiness.mjs`，人工验收清单已沉淀到 `docs/release-qa.md`。
- 发布 readiness 已支持 `--require-manual-qa`，可以把 macOS/Windows 真机 QA 输出目录作为正式发布门禁，并校验回传 manifest 的 git commit 是否匹配当前 HEAD。
- `doctor --json` 已补齐机器可读自检结果，控制面板健康状态也会检查播放器可用性，方便 Windows 真机和外部用户反馈问题。
- `doctor --json` 和控制面板健康状态已进一步覆盖 Plugin manifest、Plugin Skill、MCP 配置和当前平台 MCP 脚本，避免外部机器上出现“插件看起来安装了，但 side-channel/MCP 实际不可用”的隐性问题。
- `doctor` 和 `status` 已加入集成文件一致性检查：Codex Speak Skill、Hook wrapper、Plugin manifest、Plugin Skill、MCP 配置和当前平台 MCP 脚本必须与当前 CLI 内置版本一致，旧文件会提示重新运行 `codex-speak install` 刷新。
- `doctor` 的文本和 JSON 输出会给失败/警告项附带可执行修复提示，方便外部用户把自检结果发回来后快速定位安装、模型、插件、Hook 或播放器问题。
- `support-bundle` 命令已补齐，会把 doctor/status/models、环境信息和最近日志写入本地目录，并默认脱敏 home 路径和最近朗读文本；macOS/Windows release smoke 已覆盖该命令。
- `check-manual-qa-report.mjs` 已加入支持包隐私门禁，会验证默认支持包声明已脱敏、未开启 `includePrivate`，并扫描支持包中是否残留本机 home 路径或未脱敏的最近朗读文本。
- 安装器完成控制面板和桌面组件复制后会打印自检、打开控制面板、生成支持包和补装默认中文模型的下一步命令，降低外部用户安装后的迷路成本。
- CLI 已支持 `--version`，`doctor --json` 和 `status` 会输出版本、系统和 CPU 架构信息，方便远程判断用户反馈对应哪个构建和平台。
- `verify-install` 已补齐为安装后验收命令；release smoke 会用 `--allow-missing-models` 验证跳过模型下载时核心安装链路仍然通过。
- `verify-codex` 已补齐为 Codex 集成预检命令；release smoke 和手工 QA 收集脚本会验证 MCP side-channel 写入、Hook 风格消费、普通回复兜底清洗和常见英文技术缩写归一化链路。
- `verify-controls` 已补齐为控制项验收命令；release smoke 和手工 QA 收集脚本会验证自动朗读、儿童模式、语速、最大朗读字数、声音档位和 TTS 引擎能写入、重新读取，并恢复原配置。
- 英文朗读已补齐第一层兜底：常见技术缩写会在进入 TTS 前变成中文可懂词，系统语音兜底会按中英文分段选择系统声音，减少英文单词逐字母读的问题。
- 本地发音词典已补齐：用户和 Codex MCP 都可以把项目名、英文工具名或缩写写入 `pronunciation.toml`，朗读前优先使用这些规则，`verify-codex` 会验证发音词典写入、应用、列出和删除链路。
- 控制面板已接入本地发音词典：普通用户不用命令行也可以添加、预览、删除发音规则，状态区会显示词典数量或解析问题。
- 外部 QA 脚本和交付说明已把控制面板发音词典列为人工验收项，测试者需要确认能新增、预览并删除一条发音规则。
- 手工 QA 和外部 QA 交付说明已加入混合中英文验收：自动检查提取文本是否把 `OpenRouter`、`OAuth`、`M C P`、`J.S.O.N`、`CLI` 等归一化为中文说法，交互式测试还会让测试者实际听一次混合中英文样例；release readiness 会静态检查 macOS/Windows QA 脚本没有退回旧样例。
- macOS release 包已包含 `scripts/manual-qa-macos.sh` 真机 QA 收集脚本；CI 会用非交互模式验证它可运行，人工验收时它会生成 `qa-report.json` 和支持包。
- Windows release 包已包含 `scripts/manual-qa-windows.ps1` 真机 QA 收集脚本；CI 会用非交互模式验证它可运行，人工验收时它会生成 `qa-report.json` 和支持包。
- `check-manual-qa-report.mjs` 已补齐为 QA 报告校验器；release smoke 会校验非交互报告，外部真机回传后可用它判断 release 包 manifest、自检、支持包和人工确认项是否通过。
- `prepare-qa-handoff.mjs` 已补齐为外部 QA 交付说明生成器，会把 release 包 sha256、安装命令、验收命令和 QA 回传要求写入 `dist/qa-handoff`，降低外部测试漏步骤的概率。
- main CI 的 release package smoke 会上传 macOS/Windows 临时 QA artifacts，方便不打 tag 也能从最新成功 CI 下载 Windows 包给外部测试者；正式公开分发仍走 release workflow。
- Release 包已包含 `release-manifest.json`，记录版本、git commit、平台、生成时间、关键文件 sha256 和推荐验证命令，方便外部反馈时确认构建来源。
- CLI 已支持 `verify-package --package-dir .`，release 包解压后不依赖 Node.js 也能校验 manifest 和关键文件 sha256；release smoke 会先跑 CLI 版校验，再跑 Node 脚本版交叉校验。
- macOS/Windows release smoke 已补齐包级 `.sha256` 校验，会在解包安装前先验证下载包哈希，避免烟测绕过损坏或错配的压缩包。
- Release 包已包含 `check-release-manifest.mjs`，有 Node.js 时可作为 manifest 校验的脚本版交叉验证。
- 安装器会把 release 包里的 `release-manifest.json` 保留到安装目录，`support-bundle` 会随支持包回传它，避免用户安装后丢失构建来源信息。
- Release workflow 已增加稳定版签名门禁：`v*-rc*` 仍可无签名验证发布链路，正式 `v*` 且非 `-rc` 标签缺少签名或公证 secrets 时会直接失败。

剩余外部验证：

- 在外部 macOS/Windows 真机上确认控制面板实际启动、Pet/播放器播放/停止链路，以及非 CI 环境下的首次模型下载体验。
- 配置真实签名 secrets 后，验证 macOS codesign/notarization 和 Windows signtool。

### P3：语音效果优化计划

状态：等完整产品稳定后开始。

优化方向：

- 建立同一句话的多 Provider 试听面板。
- 固定一组儿童友好测试语料。
- 评估清晰度、自然度、延迟、中文数字/英文混读、低配机器表现。
- 对 MeloTTS、Kokoro、Piper、ZipVoice 分别建立推荐参数。
- 最终给出默认音色、低配音色、自然音色、实验音色四种方案。

## 当前优先级

当前优先完成 P2：安装分发的跨平台外部验证。
P2 通过后，再正式开始 P3：语音播报效果优化。
