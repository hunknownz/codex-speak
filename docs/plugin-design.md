# Codex Speak Plugin 设计

## 插件定位

Codex Speak Plugin 不替代 Hook，也不替代 Rust CLI。它负责产品体验层：

- 设置朗读开关。
- 管理朗读协议和 Skill。
- 展示朗读状态。
- 提供重读、停止、试听。
- 后续提供 side-channel，避免朗读内容必须出现在最终回答里。

需要明确的是：当前 Codex Plugin 规范没有提供稳定的“改写或隐藏 Chat Session 中某条消息渲染结果”的能力。所以第一版 Plugin 不承诺强行隐藏协议块。它采用两种现实方案：

- 可靠方案：通过 `codex_speak_prepare` 写入本地 side-channel，让朗读内容不必完整显示在最终回答里。
- 渐进增强：协议块可以外包一层 HTML `details`，如果 Codex 渲染器支持，就折叠显示；如果不支持，也不影响解析和朗读。

核心分工：

```text
Skill：让 Codex 生成符合协议的朗读导览
Plugin：提供控制面板、状态展示、工具入口
Hook：回复结束后触发朗读
Rust CLI：解析协议、调用本地 TTS、播放声音
```

## 第一阶段插件功能

第一阶段 Plugin 做轻量控制，不改变主链路。

### 1. 状态面板

展示：

- 朗读是否开启。
- 当前 TTS Provider。
- 当前声音模型。
- 最近一次朗读内容。
- 最近一次 TTS 生成耗时。
- `doctor` 自检结果。

### 2. 快捷操作

提供按钮：

- 开启朗读。
- 关闭朗读。
- 停止当前朗读。
- 重读上一条。
- 试听声音。
- 运行自检。

这些按钮本质上调用 Rust CLI：

```bash
codex-speak stop
codex-speak speak --text "这是一段试听文本"
codex-speak doctor
```

### 3. 配置管理

可调整：

- `enabled`
- `speed`
- `num_threads`
- `vits_noise_scale`
- `vits_noise_scale_w`
- `tts_silence_scale`
- `max_read_chars`

配置仍写入：

```text
~/.codex/codex-speak/config.toml
```

## 第二阶段插件功能

第二阶段开始承担更强的产品能力。

### 1. Protocol Preview

当 Codex 回复中存在：

```html
<aside class="codex-speak-guide" data-codex-speak="guide">
```

Plugin 可以识别并显示为“朗读导览卡片”：

- 做了什么
- 代码在解决什么问题
- 结果
- 下一步

如果 Codex UI 支持渲染扩展，Plugin 可以把原始 HTML 协议美化成卡片。  
如果不支持，仍保留原始 HTML 的可见文本，不影响 Rust CLI 提取。

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

## 第三阶段插件功能

第三阶段做 side-channel，但不作为第一版强依赖。

### Tool/Side-Channel

如果 Codex Plugin 支持工具调用，可以提供：

```text
codex_speak_prepare
```

输入：

```json
{
  "version": 1,
  "audience": "beginner",
  "style": "clear-bright",
  "items": [
    {"role": "did", "text": "我刚才帮你修改了朗读规则。"},
    {"role": "result", "text": "我运行了测试，结果通过了。"},
    {"role": "next", "text": "接下来可以继续做插件控制面板。"}
  ]
}
```

Plugin 写入：

```text
~/.codex/codex-speak/spool/latest.json
```

Hook 触发后，Rust CLI 优先读 `latest.json`。

读取顺序变成：

```text
1. side-channel latest.json
2. HTML microformat protocol aside
3. Markdown 朗读导览
4. HTML comment 调试块
5. 清洗最终回答
```

这样可以做到：

- Chat 中自然显示导览，或不显示导览。
- Rust CLI 始终有稳定结构化输入。
- Hook 不需要理解内容。

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
- 后续提供 side-channel。

## 实施路线

### P1：协议落地

- Rust CLI 支持解析 HTML Protocol v1。
- Skill 改为输出 `<aside class="codex-speak-guide" data-codex-speak="guide">`。
- 保留 Markdown 和 HTML 注释兼容。

### P2：Plugin 控制面板

- 展示状态。
- 开关朗读。
- 重读/停止/试听。
- 运行 doctor。
- 提供 MCP 工具：`codex_speak_status`、`codex_speak_extract`、`codex_speak_speak_text`、`codex_speak_stop`、`codex_speak_set_enabled`。

### P3：Plugin Side-Channel

- 增加 `codex_speak_prepare` 工具。
- Rust CLI 支持读取 spool。
- Skill 改为优先调用工具，不能调用时退回 HTML Protocol。
