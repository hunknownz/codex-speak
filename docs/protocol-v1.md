# Codex Speak Protocol v1

## 一句话

Codex Speak Protocol 是一套给 Codex 朗读助手使用的“可传递、可解析、可朗读”的结构化导览协议，让 Codex 在完成回答时顺手生成一段适合小朋友和初学者听的说明。

## 为什么需要协议

Codex 的正常回答是给眼睛看的，里面经常有代码、命令、长路径、日志、表格和测试输出。直接朗读会出现三个问题：

- 听不懂：代码和命令被逐字读出来，对小朋友没有帮助。
- 太长：完整回答会占用很久，孩子听到后面容易忘记前面。
- 不稳定：只靠 Markdown 标题或自然语言段落，Hook 很难稳定找到应该朗读哪一段。

所以协议把一件事拆成两层：

- Codex 负责理解完整上下文，写出适合听的导览。
- Rust CLI 只负责按协议提取文本，再交给本地 TTS 朗读。

这样可以避免再接一个外部大模型，也避免把“理解内容”的工作塞进 Hook 或 TTS。

## 设计来源

v1 采用 HTML 微格式风格，而不是自造一套纯文本标记。参考的软件设计实践包括：

- HTML `data-*`：适合把应用私有数据挂在普通 HTML 元素上，机器容易解析。参考 [MDN data attributes](https://developer.mozilla.org/en-US/docs/Web/HTML/How_to/Use_data_attributes)。
- Microformats：用普通 HTML 加约定类名表达结构化语义，人能看，机器也能读。参考 [MDN Microformats](https://developer.mozilla.org/en-US/docs/Web/HTML/Guides/Microformats)。
- Schema.org Microdata：用 HTML 属性表达可抽取的结构化内容，说明“可见内容 + 机器语义”是成熟路线。参考 [Schema.org item](https://schema.org/item)。
- SSML：语音合成领域已有专门的朗读标记语言，但它更适合 TTS 内部，不适合直接放在 Chat Session 里。参考 [W3C SSML](https://www.w3.org/TR/speech-synthesis/)。

结论：主路径使用 MCP side-channel；正常情况下不需要在 Chat Session 中注入自定义 HTML/XML 协议；Chat Session 中的 HTML 微格式只作为 fallback；TTS 内部未来可以转换为 SSML。

## 主路径：MCP Side-Channel

当 Codex Speak Plugin 可用时，Codex 应优先调用 MCP 工具：

```text
codex_speak_prepare
```

输入结构：

```json
{
  "version": 1,
  "audience": "beginner",
  "style": "clear-bright",
  "lang": "zh-CN",
  "items": [
    {"role": "did", "text": "我刚才帮你修改了朗读规则。"},
    {"role": "code-summary", "text": "代码部分的作用是：让自动触发器优先读取本地朗读稿。"},
    {"role": "visual-summary", "text": "我还可以画一张小图，帮你看懂消息是怎么变成声音的。"},
    {"role": "result", "text": "我运行了测试，结果通过了。"},
    {"role": "next", "text": "下一步可以继续做控制面板。"}
  ]
}
```

工具写入：

```text
~/.codex/codex-speak/spool/latest.json
```

Hook 在 Codex 回复结束后触发 Rust CLI。Rust CLI 优先读取这份文件；读取成功后把它移动为：

```text
~/.codex/codex-speak/spool/last-consumed.json
```

这样可以保证 side-channel 是一次性消费的，避免下一轮回复误读上一轮朗读稿。

对于长任务中的阶段性提示，可以使用另一个 MCP 工具：

```text
codex_speak_speak_text
```

输入示例：

```json
{
  "text": "我正在运行测试，等一下会告诉你结果。",
  "background": true
}
```

`background: true` 表示由本地进程在后台生成并播放这句短提示，MCP 工具快速返回，Codex 可以继续执行任务。它适合少量进度节点，不适合逐句朗读所有思考过程。最终结果仍然用 `codex_speak_prepare` 写入完整导览，再由 Hook 在回复结束后朗读。

完整朗读时机策略见 [朗读时机策略](speech-timing.md)。Protocol v1 不要求、也不推荐读取聊天流式 token 后逐字朗读；它把“过程提示”和“最终导览”分成两个明确通道，避免孩子听到未清洗的代码、日志、路径或半成品判断。

## 历史兼容：HTML 微格式

HTML 微格式不是新架构的默认输出方式。它主要用于三种情况：

- 已经存在的旧 Chat Session。
- MCP side-channel 暂时不可用时的开发排障样例。
- 老版本 Hook/CLI 的兼容解析。

正常使用时，Skill 不应该为了朗读主动把这段 HTML 注入 Chat Session。

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你改了朗读助手的规则。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：让程序先找到适合朗读的导览，而不是直接朗读整段技术回答。</p>
  <p class="codex-speak-result" data-role="result">我运行了测试，结果通过了。</p>
  <p class="codex-speak-next" data-role="next">接下来，你可以让我继续把这个协议接进插件通道。</p>
</aside>
```

## 历史 Fallback 可折叠显示

如果某条历史消息或排障样例已经需要把协议块放进 Chat Session，可以把协议块外面包一层原生 HTML `details`：

```html
<details class="codex-speak-fold">
  <summary>朗读导览</summary>
  <aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
    <p class="codex-speak-did" data-role="did">我刚才帮你整理了协议。</p>
    <p class="codex-speak-result" data-role="result">新的规则已经可以被朗读程序识别。</p>
    <p class="codex-speak-next" data-role="next">下一步可以继续做插件的 side-channel。</p>
  </aside>
</details>
```

这是渐进增强，不是强依赖：

- 如果 Codex 渲染器支持 `details`，用户会看到一个可展开的“朗读导览”。
- 如果 Codex 渲染器不支持，Rust CLI 仍然能从里面找到 `aside[data-codex-speak="guide"]`。
- 新架构下如果想让朗读内容不显示在 Chat 中，应使用 MCP side-channel；HTML 微格式只是历史兼容和排障兜底。

## 结构解释

### aside

`aside` 表示这是一段和主回答相关、但可以独立阅读和朗读的补充导览。在新架构中，它不是常规聊天内容，而是历史兼容或排障协议。

必填属性：

- `class="codex-speak-guide"`：给人、CSS 和未来 Plugin 使用的稳定类名。
- `data-codex-speak="guide"`：给 Rust CLI 和 Hook 使用的机器入口。
- `data-version="1"`：协议版本。

推荐属性：

- `data-audience="beginner"`：说明面向小朋友或初学者。
- `data-style="clear-bright"`：说明朗读风格是清楚、明亮、像姐姐讲解。
- `lang="zh-CN"`：说明主语言是中文，后续可用于语言检测和 TTS 选择。

为什么同时要 `class` 和 `data-*`：

- `class` 适合展示、插件美化、用户理解。
- `data-*` 适合机器解析，不和样式系统绑定。
- 两者一起用，既像正常 HTML，又有稳定协议语义。

### p[data-role]

每个 `p` 是一个可朗读单元。`data-role` 表示这句话在导览中的作用。

| role | 含义 | 使用建议 |
| --- | --- | --- |
| `did` | Codex 刚才做了什么 | 必须 |
| `why` | 为什么这么做 | 可选 |
| `code-summary` | 代码大概解决什么问题 | 有代码时推荐 |
| `command-summary` | 命令、安装、测试步骤做了什么 | 有命令时推荐 |
| `visual-summary` | 可视化辅助展示了什么 | 有 Mermaid、HTML 或图片辅助时推荐 |
| `result` | 结果、验证、测试是否通过 | 推荐 |
| `next` | 小朋友下一步可以怎么继续 | 必须 |
| `warning` | 限制、风险、没完成的地方 | 需要时使用 |

类名建议与 role 对应：

```html
<p class="codex-speak-did" data-role="did">我刚才帮你检查了朗读方案。</p>
```

Rust CLI 以 `data-role` 为准，Plugin 可以用 `class` 做展示。

## 写作规则

协议块不是摘要器输出的“短摘要”，而是 Codex 理解任务后的“行动导览”。它应该让孩子听完知道现在发生了什么，并能继续推动 Codex。

应该写：

```html
<p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：让程序能找到朗读导览，并跳过代码块和长路径。</p>
```

不要写：

```html
<p data-role="code-summary">src/extract.rs clean_for_speech Regex data-role...</p>
```

规则：

- 用中文优先，英文技术词只在必要时保留。
- 不逐字朗读代码、命令、长路径、日志、diff、URL、哈希。
- 代码要讲“解决什么问题”，命令要讲“做了什么动作”。
- 结果要说清楚是否验证通过。
- 下一步要能变成用户可以继续说的一句话。
- 语气不要幼稚化，使用清楚、温暖、像姐姐解释的口吻。

## 长度建议

- 简单聊天：可以不输出协议块。
- 普通实现、调研、调试：3 到 5 个 `p`。
- 复杂任务：最多 7 个 `p`。
- 总长度建议 120 到 500 个中文字符。

## 解析规则

Rust CLI 的提取顺序：

1. 新鲜的 MCP side-channel `spool/latest.json`
2. `aside[data-codex-speak="guide"]`
3. 旧版 Markdown `朗读导览`
4. 旧版 `<!-- codex-speak -->` 调试块
5. 从最终回答清洗并压缩出来的短导览

在 HTML 协议块内：

- 只读取允许的 `data-role`。
- 去掉内部标签，只保留文本。
- 解码基本 HTML 实体。
- 跳过过长、像日志、像路径的行。
- 按 `p` 的出现顺序拼接朗读。

## 与 Plugin 的关系

没有 Plugin/MCP 时：

```text
Skill 输出自然回答 -> Hook 触发 -> Rust CLI 生成短导览兜底 -> 本地 TTS 朗读
```

Plugin 加入后主要做三件事：

- 通过 MCP side-channel 传递朗读导览。
- 提供状态、停止、试听、儿童模式、语速和声音切换等控制工具。
- 在开发和排障场景校验历史 fallback 协议是否合格。

第一版 side-channel 文件：

```json
{
  "version": 1,
  "audience": "beginner",
  "style": "clear-bright",
  "lang": "zh-CN",
  "source": "codex-speak-plugin",
  "items": [
    {"role": "did", "text": "我刚才帮你修改了插件。"},
    {"role": "result", "text": "我运行了测试，结果通过了。"},
    {"role": "next", "text": "下一步可以继续做设置界面。"}
  ]
}
```

Rust CLI 只读取最近几分钟内写入的 `latest.json`，并在成功读取后消费掉它，避免很久以前或上一轮的朗读稿误触发。

Plugin 不负责理解内容。理解发生在 Codex 生成朗读导览并调用 side-channel 工具的那一刻。

当前 Plugin 也不承诺直接隐藏或改写已经渲染的 Chat Session 消息；如果未来 Codex 提供消息渲染扩展点，可以把历史 fallback 协议显示成排障卡片，但正常主路径仍然使用 side-channel。

## 与 SSML 的关系

v1 不直接要求 Codex 输出 SSML。原因是 SSML 适合语音引擎，不适合普通用户在 Chat Session 中阅读。

未来可以在 Rust CLI 内部把协议转换成 SSML：

```xml
<speak>
  <p>我刚才帮你改了朗读助手的规则。</p>
  <break time="300ms"/>
  <p>代码部分的作用是：让程序先找到适合朗读的导览。</p>
</speak>
```

这个转换应该发生在本地 TTS 前，不直接显示给用户。

## 历史 HTML 示例

下面示例用于说明历史 HTML fallback 的结构。新回答的主路径应把同样的 role/text items 传给 `codex_speak_prepare`，不要默认把 HTML 放进 Chat Session。

### 代码修改场景

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你修改了朗读助手的提取规则。</p>
  <p class="codex-speak-code-summary" data-role="code-summary">代码部分的作用是：优先找到专门给朗读用的导览，并跳过代码块和长路径。</p>
  <p class="codex-speak-result" data-role="result">我运行了测试，新的规则已经通过。</p>
  <p class="codex-speak-next" data-role="next">接下来，你可以让我把这个协议接进插件，让设置和状态更容易看见。</p>
</aside>
```

### 安装部署场景

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你安装了本地朗读程序和中文语音模型。</p>
  <p class="codex-speak-command-summary" data-role="command-summary">安装步骤主要做了三件事：复制程序、下载声音模型、把 Codex 的回复结束提醒接到朗读程序上。</p>
  <p class="codex-speak-result" data-role="result">我检查了安装状态，所有项目都显示正常。</p>
  <p class="codex-speak-next" data-role="next">现在你可以继续和 Codex 说话，回答结束后就会自动朗读导览。</p>
</aside>
```

### 调研分析场景

```html
<aside class="codex-speak-guide" data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright" lang="zh-CN">
  <p class="codex-speak-did" data-role="did">我刚才帮你比较了几种本地朗读方案。</p>
  <p class="codex-speak-why" data-role="why">重点看的是中文效果、Windows 支持、机器要求和安装复杂度。</p>
  <p class="codex-speak-result" data-role="result">结论是第一版先用 Sherpa-ONNX 加 MeloTTS，后面再加更多声音。</p>
  <p class="codex-speak-next" data-role="next">下一步可以继续把安装器做稳，让别人也能一键安装。</p>
</aside>
```

## 不推荐格式

不要用 HTML 注释作为主协议：

```html
<!-- codex-speak
这里是一大段朗读稿。
-->
```

原因：

- 用户看不到朗读来源，不自然。
- 复制和调试时容易出现隐藏内容。
- Chat Session 仍然会保存隐藏文本。
- Agent 场景中隐藏指令更容易带来提示注入风险。

不要只用 Markdown 标题作为主协议：

```markdown
**朗读导览**

我刚才做完了。
```

原因：

- 标题容易被不同写法打破。
- 缺少 role，Plugin 不知道哪句是结果、哪句是下一步。
- 难以演进到卡片、校验、SSML 转换和 side-channel。

## 兼容策略

v1 必须保持向后兼容：

- 新回答优先调用 MCP side-channel，不主动输出 HTML 微格式协议。
- Rust CLI 继续支持历史 HTML 微格式协议。
- Rust CLI 继续支持旧版 Markdown `朗读导览`。
- Rust CLI 继续支持旧版 HTML 注释调试块，但优先级低于可见导览。
- 如果没有协议，仍然使用短导览兜底，保证不会完全失声，也不会整段朗读最终回复。

## 演进方向

- v1.1：增加 `data-priority` 或 `data-speak="false"`，允许细粒度控制是否朗读。
- v1.2：Plugin 校验 side-channel payload，并在排障场景展示历史协议卡片。
- v2：继续强化 MCP side-channel，HTML 协议只保留为历史兼容。
- TTS 层：把 role 转成停顿、语速和强调，生成内部 SSML。
