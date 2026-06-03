# 朗读风格指南

## 目标

Codex Speak 的朗读内容不是把聊天回复念一遍，而是把 Codex 刚完成的工作变成可听、可行动的导览。

核心原则：

- 儿童模式：把对话对象当成孩子，直接对他说话。
- 成人模式：把对话对象当成想节省时间的成年人，给高信噪比摘要。
- 两种模式都不能朗读代码、长路径、命令、日志和协议细节。

## 参考原则

这些规则来自通用 plain language 和儿童听读实践：

- 写给听的人，而不是写给自己；先说重点；一个段落只讲一个想法；多用短句和日常词。
- 对口头说明，复杂信息要拆成短句，优先讲最重要的两三个概念。
- 使用主动语态和对话式表达，让听的人知道谁做了什么、下一步可以做什么。
- 儿童听读要注意语气变化和自然停顿，但产品文本本身不能靠夸张语气弥补含混表达。

参考来源：

- National Archives, Top 10 Principles for Plain Language: https://www.archives.gov/open/plain-writing/10-principles.html
- VA.gov Design System, Plain language: https://design.va.gov/content-style-guide/plain-language/
- American Academy of Pediatrics, Communication Strategies: Plain Language: https://www.aap.org/en/patient-care/healthy-active-living-for-families/communicating-with-families/plain-language/
- West Virginia University Extension, Tips for Read-Aloud: https://extension.wvu.edu/youth-family/youth-education/literacy/tips-for-read-aloud

## 儿童模式

儿童模式不是描述“我要生成儿童能听懂的内容”。儿童模式是直接把孩子当成正在听的人。

推荐结构：

```text
我刚刚做了什么。
这有什么用。
有没有成功。
你下一步可以怎么做。
```

好例子：

```text
我刚刚把小伙伴开关修好了。
现在你可以在控制面板里打开或关掉它。
我也跑了检查，结果通过了。
下一步，你可以点一下试听，看看最近朗读会不会变。
```

坏例子：

```text
要生成小孩能听懂的内容，比如我改了什么、为什么、测试有没有通过。
儿童模式下应该用短句解释架构。
代码助手负责理解，本地程序负责播放，控制面板负责配置。
```

规则：

- 用“我”和“你”，不要用“用户”“儿童”“小孩”来指代听的人。
- 大多数句子尽量短，一句话只讲一个动作或结果。
- 抽象词要换成孩子能想象的动作。
- 可以温和鼓励，但不要夸张，不要 baby talk。
- 保留真实结果，不要为了温柔而掩盖失败。

## 成人模式

成人模式也需要朗读导览，但目的不同：节省时间，帮助快速理解状态和下一步。

推荐结构：

```text
这次变更/结论是什么。
为什么重要。
验证结果是什么。
还剩什么风险或下一步决策。
```

好例子：

```text
我把最近朗读状态改成自动刷新，并增加了更新时间。
这样控制面板不会再停留在旧内容。
测试、前端构建和 Tauri 检查都通过了。
下一步可以继续优化儿童模式的表达质量。
```

规则：

- 可以使用必要技术词，但不要读 raw code、长路径、日志和命令。
- 不使用儿童口吻。
- 不绕弯，保留测试结果、风险和下一步。
- 默认 120 到 320 个中文字符，复杂任务最多 500 个。

## 风格样例库和 RAG

第一阶段不做在线 RAG。原因是朗读风格需要稳定、可控、离线可安装，不能每次依赖外部资料。

推荐路线：

1. 先把本文件作为本地风格样例库，Skill 直接引用这些规则。
2. 后续增加 `speech-style-examples.jsonl`，保存儿童模式和成人模式的好/坏样例。
3. 如果样例变多，再做本地检索：按任务类型取 2 到 3 个最相近样例，拼进 Skill 或 MCP 准备上下文。

本地 RAG 只用于风格参考，不用于替代 Codex 对当前任务的理解。当前任务的理解仍然必须由 Codex 在调用 `codex_speak_prepare` 前完成。
