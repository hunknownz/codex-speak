# Codex Speak Protocol v1

## 目标

Codex Speak Protocol 是一套专门给 Codex 朗读助手使用的结构化协议。它的目标不是把 Codex 的完整回答读出来，而是让 Codex 在最终回答中自然生成一段适合朗读的行动导览，帮助小朋友或初学者听懂：

- Codex 刚才做了什么。
- 结果是什么意思。
- 代码、命令或配置解决了什么问题。
- 下一步可以怎么继续推动 Codex。

## 核心原则

- 协议内容可以显示在 Chat Session 中，但必须像正常回答的一部分。
- 不使用 HTML 注释承载主朗读内容。
- 不逐字朗读代码、命令、长路径、日志、diff 或测试输出。
- Codex 负责理解和生成导览；Rust CLI 只负责解析和朗读。
- 协议必须稳定可解析，不能只依赖 Markdown 标题。

## 推荐格式

主格式使用语义化 HTML：

```html
<aside data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright">
  <p data-role="did">我刚才帮你改了朗读助手的规则。</p>
  <p data-role="code-summary">代码部分的作用是：让程序先找到适合朗读的导览，而不是直接朗读整段技术回答。</p>
  <p data-role="result">我运行了测试，结果通过了。</p>
  <p data-role="next">接下来，你可以让我继续把这个协议接进插件通道。</p>
</aside>
```

## 元素定义

### aside

```html
<aside data-codex-speak="guide" data-version="1">
</aside>
```

`aside` 表示这是一段和主回答相关、但可独立阅读和朗读的补充内容。

必填属性：

- `data-codex-speak="guide"`：标记这是 Codex Speak 的朗读导览。
- `data-version="1"`：协议版本。

推荐属性：

- `data-audience="beginner"`：面向初学者或小朋友。
- `data-style="clear-bright"`：清楚、明亮的讲解风格。
- `data-lang="zh-CN"`：中文优先。

### p[data-role]

每个 `p` 是一个可朗读单元。`data-role` 说明这句话的作用。

支持的 role：

| role | 含义 | 是否推荐 |
| --- | --- | --- |
| `did` | Codex 刚才做了什么 | 必须 |
| `why` | 为什么这么做 | 可选 |
| `code-summary` | 代码大概在解决什么问题 | 有代码时推荐 |
| `command-summary` | 命令或安装步骤做了什么 | 有命令时推荐 |
| `result` | 结果、验证、测试是否通过 | 推荐 |
| `next` | 小朋友下一步可以怎么继续 | 必须 |
| `warning` | 限制、风险、没完成的地方 | 需要时使用 |

## 写作规则

每段应该是自然中文短句：

```html
<p data-role="did">我刚才帮你检查了朗读方案。</p>
```

不要写成机器标签解释：

```html
<p data-role="did">did: check protocol</p>
```

不要逐字读代码：

```html
<p data-role="code-summary">代码部分的作用是：让程序能找到朗读导览，并跳过代码块。</p>
```

不要逐字读命令：

```html
<p data-role="command-summary">我运行了测试命令，确认新的解析规则可以正常工作。</p>
```

不要逐字读长路径：

```html
<p data-role="result">我已经把规则同步到本机的 Codex Speak 安装里。</p>
```

## 长度建议

- 简单问答：可以不输出协议块。
- 普通实现/分析：3 到 5 个 `p`。
- 复杂任务：最多 7 个 `p`。
- 总长度建议 120 到 500 个中文字符。

## 解析优先级

Rust CLI 应按以下顺序提取朗读内容：

1. `aside[data-codex-speak="guide"]`
2. 旧版 Markdown `朗读导览`
3. 旧版 `<!-- codex-speak -->` 调试块
4. 清洗后的最终回答

## TTS 输出

第一版直接提取纯文本朗读：

```text
did。code-summary。result。next。
```

未来可以转换为 SSML：

```xml
<speak>
  <p>我刚才帮你改了朗读助手的规则。</p>
  <break time="300ms"/>
  <p>代码部分的作用是：让程序先找到适合朗读的导览。</p>
</speak>
```

SSML 只用于 TTS 内部，不直接显示在 Chat Session 中。

## 示例

### 代码修改场景

```html
<aside data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright">
  <p data-role="did">我刚才帮你修改了朗读助手的提取规则。</p>
  <p data-role="code-summary">代码部分的作用是：优先找到专门给朗读用的导览，并跳过代码块和长路径。</p>
  <p data-role="result">我运行了测试，新的规则已经通过。</p>
  <p data-role="next">接下来，你可以让我把这个协议接进插件，让设置和状态更容易看见。</p>
</aside>
```

### 安装部署场景

```html
<aside data-codex-speak="guide" data-version="1" data-audience="beginner" data-style="clear-bright">
  <p data-role="did">我刚才帮你安装了本地朗读程序和中文语音模型。</p>
  <p data-role="command-summary">安装步骤主要做了三件事：复制程序、下载声音模型、把 Codex 的回复结束提醒接到朗读程序上。</p>
  <p data-role="result">我检查了安装状态，所有项目都显示正常。</p>
  <p data-role="next">现在你可以继续和 Codex 说话，回答结束后就会自动朗读导览。</p>
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

- 不自然。
- 不利于用户理解朗读来源。
- 容易污染源文本和调试记录。
- 隐藏内容在 Agent 场景中存在提示注入风险。

