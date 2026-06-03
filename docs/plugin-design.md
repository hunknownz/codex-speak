# Codex Speak Plugin 设计

## 插件定位

Codex Speak Plugin 不替代 Hook，也不替代 Rust CLI。它负责把 Codex 的理解结果交给本地朗读系统：

- 通过 Skill 要求 Codex 生成儿童友好的朗读导览。
- 通过 MCP `codex_speak_prepare` 写入 side-channel。
- 通过 MCP `codex_speak_speak_text` 的 `background: true` 做少量任务中途进度朗读。
- 通过 MCP 工具展示状态、停止、试听、改总朗读开关、最终导览开关、过程提示开关、儿童模式、语速、声音档位和 TTS Provider。
- 让朗读内容不必以自定义协议的形式出现在最终回答里。

需要明确的是：当前 Codex Plugin 规范没有提供稳定的“改写或隐藏 Chat Session 中某条消息渲染结果”的能力。所以第一版 Plugin 不承诺强行隐藏协议块，而是让正常路径不再把协议块写进 Chat Session：

- 主路径：通过 `codex_speak_prepare` 写入本地 side-channel，让朗读内容不必完整显示在最终回答里；任务中途用 `codex_speak_speak_text background=true` 播放简短进度。
- 兼容路径：Rust CLI 继续能解析历史 HTML/Markdown fallback；MCP 不可用时，Hook 清洗普通最终回答作为最后兜底，Skill 不再默认输出新的 HTML 协议块。

核心分工：

```text
Skill：让 Codex 生成符合协议的朗读导览
Plugin/MCP：把导览写入 side-channel，提供状态/控制工具，必要时触发后台进度朗读
Hook：回复结束后触发朗读，并优先消费 side-channel
Rust CLI：读取 side-channel、解析历史兜底协议、调用本地 TTS、播放声音
```

## 当前插件功能：Side-Channel 主路径

Plugin 已经进入主链路：Codex 优先调用 MCP 写入 side-channel，Hook 在回复结束后消费它。

### 1. Side-Channel 写入

工具：

```text
codex_speak_prepare
```

输入：

```json
{
  "version": 1,
  "audience": "beginner",
  "style": "clear-bright",
  "lang": "zh-CN",
  "items": [
    {"role": "did", "text": "我刚才帮你修改了朗读规则。"},
    {"role": "visual-summary", "text": "我还可以画一张小图，帮你看懂这条流程。"},
    {"role": "result", "text": "我运行了测试，结果通过了。"},
    {"role": "next", "text": "接下来可以继续做插件控制面板。"}
  ]
}
```

Plugin/MCP 写入：

```text
~/.codex/codex-speak/spool/latest.json
```

Hook 触发后，Rust CLI 会：

```text
1. 读取新鲜的 latest.json
2. 提取 items 中允许的 role
3. 成功后把 latest.json 移到 last-consumed.json
4. 朗读纯文本
```

### 2. 后台进度朗读

工具：

```text
codex_speak_speak_text
```

输入：

```json
{
  "text": "我正在运行测试，等一下会告诉你结果。",
  "background": true
}
```

`background: true` 会启动本地后台进程播放这句短提示，MCP 调用会快速返回。它用于长任务中的自然节点，例如开始测试、构建通过、正在打包；不用于逐字朗读代码、命令、日志或整段回答。

### 3. 状态面板

展示：

- 朗读是否开启。
- 当前 TTS Provider。
- 当前声音模型。
- 最近一次朗读内容。
- 最近一次 TTS 生成耗时。
- `doctor` 自检结果。

### 4. 快捷操作

提供按钮：

- 开启朗读。
- 关闭朗读。
- 停止当前朗读。
- 重读上一条。
- 试听声音。
- 运行自检。
- 安装当前或指定 Provider 的模型。
- 打开或关闭儿童模式。
- 调慢或调快朗读速度。
- 切换声音档位。
- 切换 TTS Provider。

这些按钮本质上调用 Rust CLI：

```bash
codex-speak stop
codex-speak speak --text "这是一段试听文本"
codex-speak doctor
codex-speak models install --provider sherpa_kokoro
codex-speak config set --child-mode true --speed 0.82
codex-speak config set --provider sherpa_kokoro
```

### 5. 配置管理

可调整：

- `enabled`
- `child_mode`
- `provider`
- `speed`
- `voice_profile`
- `num_threads`
- `vits_noise_scale`
- `vits_noise_scale_w`
- `tts_silence_scale`
- `max_read_chars`

配置仍写入：

```text
~/.codex/codex-speak/config.toml
```

Provider 当前支持：

| Provider | 用途 | 说明 |
| --- | --- | --- |
| `sherpa_melo` | 默认中文朗读 | 当前安装器默认准备 |
| `sherpa_kokoro` | 更自然的备选声音 | 可通过模型安装工具下载 |
| `sherpa_zipvoice` | 参考音频实验方案 | 可通过模型安装工具下载模型、vocoder 和参考音频 |
| `piper` | 低配兜底 | 可通过模型安装工具下载中文轻量模型 |
| `system` | 系统语音 | 不需要模型，适合快速验证 |

## 第二阶段插件功能：兼容显示和校验

第二阶段开始承担更强的产品能力，但仍不依赖在 Chat Session 中注入协议块。

### 1. Legacy Protocol Preview

当历史消息或排障样例中存在：

```html
<aside class="codex-speak-guide" data-codex-speak="guide">
```

Plugin 可以识别并显示为“朗读导览卡片”：

- 做了什么
- 代码在解决什么问题
- 结果
- 下一步

如果 Codex UI 支持渲染扩展，Plugin 可以把原始 HTML 协议美化成卡片。  
如果不支持，仍保留原始 HTML 的可见文本，不影响 Rust CLI 对历史内容的提取。

### 2. 协议校验

Plugin 可以检查当前回答是否符合协议：

- 是否有 `aside[data-codex-speak="guide"]`
- 是否有 `class="codex-speak-guide"`
- 是否有 `data-version`
- 是否有 `lang`
- 是否包含 `did`
- 是否包含 `next`
- 是否出现代码块、命令、长路径或日志

如果不符合，Plugin 可以提示：

```text
这次回答没有合格的朗读导览，将使用清洗兜底。
```

### 3. Skill 安装与更新

Plugin 可以提供：

- 安装 Codex Speak Skill。
- 更新 Skill。
- 查看当前 Skill 版本。
- 恢复默认 Skill。

读取顺序：

```text
1. side-channel latest.json
2. 历史 HTML microformat protocol aside
3. 历史 Markdown 朗读导览
4. 历史 HTML comment 调试块
5. 清洗最终回答
```

这样可以做到：

- 正常 Chat 中不再出现自定义协议块。
- Rust CLI 始终有稳定结构化输入。
- Hook 不需要理解内容。

## 第三阶段插件功能：控制面板协作

第三阶段不再把 side-channel 当作未来功能，而是继续围绕它做产品体验：

- 与 Tauri App 共享同一份配置。
- 提供更细的配置工具，例如最终导览、过程提示、儿童模式、语速、声音档位。
- 当 Codex Plugin 未来支持消息渲染扩展时，只把历史 fallback 或排障样例渲染成卡片；正常朗读仍走 side-channel。

## 插件不负责什么

Plugin 不负责：

- 直接执行 TTS 推理。
- 下载大型 TTS 模型。
- 替代 Rust CLI 的跨平台核心逻辑。
- 替代 Hook 的回复结束自动触发。
- 替代 Codex 的内容理解。
- 强行改写、隐藏或折叠 Codex Chat Session 中已经渲染出来的消息。

## 推荐插件目录

后续创建插件时建议：

```text
plugins/codex-speak/
  .codex-plugin/plugin.json
  skills/
    codex-speak/SKILL.md
  scripts/
    codex-speak-mcp
  assets/
  .mcp.json
```

如果只做控制面板，可以先不启用 MCP。  
如果要做 `codex_speak_prepare` side-channel，则需要 MCP server 或等价工具入口。

## 与安装器的关系

安装器负责：

- 安装 Rust CLI。
- 安装 Hook。
- 下载 TTS 模型。
- 安装或更新 Skill。
- 可选安装 Plugin。

Plugin 负责：

- 让用户看见和控制这些能力。
- 提供可视化状态。
- 提供 side-channel 和后台进度朗读工具。

## 实施路线

### P1：协议兼容落地

- Rust CLI 支持解析 HTML Protocol v1。
- Skill 输出自然回答，并把 HTML Protocol v1 仅作为历史兼容格式记录在文档中。
- 保留 Markdown 和 HTML 注释兼容。

### P2：Plugin MCP Side-Channel

- 增加 `codex_speak_prepare` 工具。
- Rust CLI 支持读取并消费 spool。
- Skill 改为优先调用工具，不能调用时保持自然回答，由 Hook 清洗普通回复兜底。
- 提供 MCP 工具：`codex_speak_status`、`codex_speak_extract`、`codex_speak_speak_text`、`codex_speak_stop`、`codex_speak_set_enabled`、`codex_speak_update_config`、`codex_speak_set_child_mode`、`codex_speak_set_speed`、`codex_speak_set_voice_profile`、`codex_speak_list_pronunciation`、`codex_speak_set_pronunciation`、`codex_speak_remove_pronunciation`。
- `codex_speak_speak_text` 支持 `background: true`，用于长任务中的非阻塞进度提示。

### P3：Tauri 控制面板

- 用按钮控制总朗读、最终导览、过程提示、儿童模式、语速、试听和停止。
- 与 Plugin MCP 共享 Rust CLI 和配置文件。
