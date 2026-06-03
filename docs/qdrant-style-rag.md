# Qdrant 风格 RAG 方案

## 结论

如果 Codex Speak 后续需要向量数据库，本项目统一使用 Qdrant。

理由：

- Qdrant 本身是 Rust 技术栈，和 Codex Speak 的 Rust CLI 更贴合。
- 可以本地运行，适合隐私友好和离线优先的产品方向。
- 适合存放小规模风格样例、视觉样例和质量检查案例。
- 避免引入多个向量库造成安装和维护分裂。

当前阶段不立刻启用 Qdrant。先把数据 schema、清洗和验证做好；当样例库规模超过手工规则能稳定覆盖的范围时，再接入本地 Qdrant 检索。

## 使用边界

Qdrant 只用于检索表达风格样例，不用于回答事实问题。

它可以做：

- 找 2 到 3 条相似的儿童模式好例子。
- 找 1 条相似坏例子，提醒 Codex 避免。
- 找视觉支架样例，比如流程图、状态图、前后对比。
- 找成人模式摘要风格样例。

它不做：

- 不替代 Codex 理解当前任务。
- 不存储用户隐私对话原文。
- 不检索外部知识答案。
- 不默认依赖在线 embedding 服务。

## 数据流

```text
style-examples.jsonl
  -> 清洗和质量检查
  -> 本地 embedding
  -> Qdrant collection
  -> 当前任务检索相似样例
  -> Codex Skill / MCP 准备朗读导览
```

## Collection 设计

建议 collection：

```text
codex_speak_style_examples
```

Payload 字段：

```json
{
  "id": "child_debug_stale_state_001",
  "mode": "child",
  "scene": "debugging",
  "label": "good",
  "language": "zh",
  "text": "刚才小伙伴没有出现，是因为旧记录挡住了它。我们清掉它，再打开一次，就能看到结果了。",
  "source": "original",
  "source_ids": ["internal_project_case"],
  "tags": ["debugging", "cause_effect", "next_action"],
  "quality": {
    "meta_talk": false,
    "over_teaching": false,
    "visual_source_leak": false,
    "has_next_action": true
  }
}
```

Vector 文本建议由这些字段拼接：

```text
mode: child
scene: debugging
tags: debugging cause_effect next_action
text: 刚才小伙伴没有出现，是因为旧记录挡住了它...
```

## 检索策略

输入：

```text
mode + scene + task summary + signals
```

signals 示例：

- `has_error`
- `has_test_result`
- `has_architecture`
- `needs_visual`
- `simple_direct_answer`
- `adult_briefing`

检索：

```text
filter mode = child/adult
filter language = zh first
prefer label = good
topK = 3
also fetch 1 similar bad example when available
```

输出给 Codex：

```text
Use these as style references only.
Do not copy them.
Keep facts from the current task.
```

## Embedding 方案

优先本地 embedding，保持免费和可离线。

候选：

- `fastembed` Rust 生态，优先考虑，和 Qdrant 组合自然。
- ONNX embedding 模型，后续可和现有本地模型安装器统一管理。

暂不默认使用云端 embedding。

## 本地运行方式

开发阶段可以用 Docker：

```bash
docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

产品化阶段再评估：

- 是否内置 Qdrant 二进制。
- 是否作为可选组件安装。
- 是否只在样例数量超过阈值时启用。

## CLI 规划

后续可增加命令：

```text
codex-speak style index
codex-speak style search --mode child --scene debugging --query "刷新后还是旧内容"
codex-speak style validate
codex-speak style sources
```

MCP 可增加：

```text
codex_speak_style_search
```

但 MCP 工具不直接暴露原始大语料，只返回少量经过验证的风格参考。

## 启用门槛

满足以下条件再接入 Qdrant：

- 规范化 `style-examples.jsonl` 超过 50 条。
- 已有本地校验脚本。
- 至少 30 条儿童模式样例、10 条成人模式样例、10 条视觉样例。
- 样例全部通过人工抽检。
- 有本地 embedding 安装方案。

在此之前，Skill 规则和小样例库足够。
