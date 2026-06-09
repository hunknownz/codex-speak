# 儿童表达语料方案

## 目标

建立一套本地、可审计、可扩展的表达语料体系，帮助 Codex Speak 在儿童模式下自然说话、轻轻教学、必要时画图。

这不是训练模型。当前阶段的“蒸馏”是：

```text
可靠资料和公开数据
  -> 来源登记
  -> 许可和质量筛选
  -> 提炼表达原则
  -> 生成原创好/坏/改写样例
  -> 写入 Skill 和本地样例库
```

## 当前结论

短期不做完整 RAG，也不直接把大数据集塞进产品。

原因：

- 目标是表达风格，不是知识问答。
- 儿童模式需要稳定、可控、可解释。
- 很多教育对话数据有非商用、ShareAlike、隐私或年龄段不匹配问题。
- 高质量儿童表达样例更适合“小而精”的本地 style pack。

当前落地路线：

```text
阶段 1：来源登记表 + 主样例库 + CLI 校验
阶段 2：本地检索 2-3 条相似风格样例
阶段 3：质量守卫自动打分
```

## 第一阶段已交付

第一阶段先把语料变成可验证的数据资产，暂不接 Qdrant。

已落地文件：

- `data/speech-style/source-candidates.jsonl`
- `data/speech-style/distilled-principles.jsonl`
- `data/speech-style/style-examples.jsonl`
- `skills/codex-speak/speech-style-examples.jsonl`

已落地命令：

```bash
codex-speak style validate
codex-speak style sources
```

`style-examples.jsonl` 是主库，至少 50 条，包含儿童表达、成人摘要和视觉支架。Skill 里只安装精选小样本，用来给 Codex 提供少量稳定参考。

## 已发现的候选来源

| 来源 | 类型 | 语言 | 许可/限制 | 用法建议 |
| --- | --- | --- | --- | --- |
| `lumees/age-specific-text-simplification` | Hugging Face 文本简化数据 | 英文 | Apache-2.0 | 可少量采样研究“年龄化简化”模式；年龄 3-5 偏小，不能照搬 |
| `Eedi/Question-Anchored-Tutoring-Dialogues-2k` | Hugging Face 真实辅导对话 | 英文 | CC-BY-NC-4.0 | 只能研究参考，不进入商业产品语料 |
| `eth-nlped/mathdial` | GitHub 数学辅导对话 | 英文 | CC-BY-SA-4.0 | 可研究支架式提问；直接入库会带 ShareAlike 风险 |
| `google-research-datasets/Education-Dialogue-Dataset` | GitHub 合成教育对话 | 英文 | 许可待核验，仓库已归档 | 只作为研究参考，暂不入库 |
| `blcuicall/mcts` | GitHub 中文文本简化数据 | 中文 | 许可需逐项核验 | 可研究中文简化操作；未确认前不入商业语料 |
| `jantrienes/text-simplification-datasets` | GitHub 数据集索引 | 多语言 | 索引本身 | 用于持续发现，不直接入库 |
| CDC / NARA / Digital.gov plain language | 写作规则 | 英文 | 政府公开资料 | 提炼清晰表达规则，不复制长文 |
| Reading Rockets / Dialogic Reading 资料 | 对话式阅读规则 | 英文/中文资料可交叉 | 多数为教育资料 | 提炼“对话、提示、扩展、联系生活”原则 |
| UNICEF / 教育部《3-6 岁儿童学习与发展指南》 | 儿童发展指导 | 中文 | 官方公开 PDF | 提炼中文儿童倾听表达原则 |
| 协康会儿童语言表达资料 | 儿童语言发展/表达 | 中文 | 机构公开资料 | 提炼日常互动、句式示范原则 |

## 来源分级

许可门禁：

- `ingest_allowed: true` 只允许宽松、商业友好许可，例如 Apache-2.0、MIT、CC-BY、CC0 或项目原创样例。
- 非商用、ShareAlike、许可未知、隐私敏感数据不进入产品语料。
- 第一批外部数据里，只有 `hf_lumees_age_specific` 进入可采样入库队列。
- Eedi、MathDial、Google Education Dialogue、MCTS 暂时只做研究参考或等待许可复核。

### A 类：可直接蒸馏成规则

标准：

- 权威机构或高质量教育资料。
- 主要提供原则，不依赖原文样例。
- 许可或公开属性适合引用和归纳。

用途：

- 写入 `docs/speech-style-guide.md`。
- 写入 Skill 的规则。
- 形成质量检查项。

### B 类：可少量采样做风格参考

标准：

- 有明确可用许可。
- 数据质量较高。
- 无明显隐私风险。
- 与儿童表达、文本简化或教学对话相关。

用途：

- 抽样人工看。
- 不直接复制长文本。
- 改写成原创样例。

### C 类：只做研究参考

标准：

- 非商用许可。
- ShareAlike 可能污染产品许可。
- 真实学生对话、隐私风险较高。
- 许可不清晰。

用途：

- 研究结构和标注方式。
- 不进入产品样例库。
- 不用于商业分发数据。

## 数据结构

### 来源登记

文件：

```text
data/speech-style/source-candidates.jsonl
```

字段：

```json
{
  "id": "hf_lumees_age_specific",
  "name": "Age-Specific Text Simplification Dataset",
  "url": "https://huggingface.co/datasets/lumees/age-specific-text-simplification",
  "platform": "huggingface",
  "language": ["en"],
  "kind": ["text_simplification", "age_adaptation"],
  "license": "apache-2.0",
  "use_level": "sample_reference",
  "risks": ["age_too_young", "generated_data"],
  "status": "candidate",
  "license_status": "approved_permissive",
  "ingest_allowed": true,
  "attribution_required": true
}
```

### 蒸馏原则

文件：

```text
data/speech-style/distilled-principles.jsonl
```

字段：

```json
{
  "id": "plain_short_active",
  "mode": "child",
  "principle": "Use short active sentences with one idea each.",
  "zh_rule": "一句话只讲一个动作或结果。",
  "source_ids": ["cdc_plain_language", "nara_plain_language"],
  "quality_check": "Sentence should be short, active, and directly useful."
}
```

### 样例库

主库文件：

```text
data/speech-style/style-examples.jsonl
```

Skill 精选样例：

```text
skills/codex-speak/speech-style-examples.jsonl
```

字段：

```json
{
  "id": "child_debug_stale_state_001",
  "mode": "child",
  "scene": "debugging",
  "label": "good",
  "language": "zh-CN",
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

固定字段：

```text
id, mode, scene, label, language, text, source, source_ids, tags, quality
```

## 收集流程

1. **发现来源**
   - 搜索 Hugging Face、GitHub、论文、官方教育资料。
   - 记录到 `source-candidates.jsonl`，不直接下载数据。

2. **许可门禁**
   - 明确 license。
   - 非商用、ShareAlike、许可未知的数据默认不入产品。
   - 真实儿童或学生数据默认只研究结构。

3. **抽样验证**
   - 每个候选来源先看 20 到 50 条。
   - 标注：语言、年龄段、是否自然、是否过度教学、是否有隐私、是否适合 Codex 任务。

4. **规则蒸馏**
   - 把可复用模式提炼成规则。
   - 不复制长文本。
   - 保留 source id 和推理说明。

5. **原创样例生成**
   - 用项目真实场景生成原创好/坏/改写样例。
   - 场景优先级：调试、测试、架构、按钮、失败、图解、下一步。

6. **质量审核**
   - 自动检查 + 人工抽检。
   - 通过后进入 Skill 样例库。

## 清洗规则

必须删除：

- 姓名、地址、学校、账号、电话、邮箱、链接中的个人信息。
- 长路径、命令、代码、日志。
- 成人化抽象术语堆叠。
- 强教学口吻。
- 元话术，例如“我要生成儿童能听懂的内容”。
- 夸张表扬、幼稚化语气。
- 无关文化背景或需要额外解释的典故。

## 自动校验

`codex-speak style validate` 会检查：

- JSONL 是否能逐行解析。
- 必填字段是否完整。
- `id` 是否重复。
- `source_ids` 是否存在。
- `ingest_allowed` 是否搭配商业友好许可。
- 儿童样例是否出现元话术、过度教学、长句。
- 朗读文本是否泄漏代码、命令、长路径、HTML 或图表源码。
- 主样例库是否至少包含 50 条样例、30 条儿童样例、10 条成人样例、10 条视觉支架样例。

必须保留或改写：

- 当前任务的结果。
- 能帮助孩子行动的下一步。
- 一个自然的小知识点。
- 能体现“观察、测试、比较、排障”的方法感。

## 验证标准

每条儿童模式样例必须通过：

- **自然度**：像是在对孩子说话，不像产品说明。
- **短句**：大多数句子不超过 18 个汉字。
- **目标相关**：贴着当前成果，不硬教。
- **行动性**：孩子知道下一步能做什么。
- **非元话术**：不出现“儿童模式”“生成给小孩”等产品逻辑。
- **非过度教学**：最多一个小知识点。
- **可听性**：不包含代码、路径、命令、图表源码。
- **诚实性**：失败或未完成要说清楚。

成人模式样例必须通过：

- 信息密度高。
- 保留验证结果和风险。
- 不用儿童口吻。
- 不读技术噪声。

## 自动质量守卫建议

第一版可以用规则检查：

```text
meta_talk_detector
over_teaching_detector
raw_code_or_path_detector
sentence_length_checker
next_action_checker
visual_source_leak_checker
```

第二版再让 Codex 自检：

```text
请检查这段儿童模式朗读稿：
1. 有没有元话术？
2. 有没有过度教学？
3. 有没有一处自然小知识点？
4. 孩子听完能不能知道下一步？
```

## RAG 方案

暂时不做在线 RAG。

后续做本地 RAG 时，统一使用 Qdrant，只检索风格样例：

```text
输入：当前任务类型 + child_mode + 是否有错误/测试/架构
检索：2 到 3 条相似好例子 + 1 条坏例子
输出：作为 Codex 生成朗读稿的风格参考
```

RAG 不负责事实知识，不替代 Codex 对当前任务的理解。

Qdrant 详细设计见 [Qdrant 风格 RAG 方案](qdrant-style-rag.md)。

## 第一批建设任务

1. 维护 `source-candidates.jsonl`。
2. 维护 `distilled-principles.jsonl`。
3. 把现有 Skill 样例迁移到规范化 `style-examples.jsonl`。
4. 增加 30 条原创中文儿童模式样例。
5. 增加 10 条成人模式摘要样例。
6. 增加 10 条视觉支架样例。
7. 写一个本地校验脚本，检查 JSONL、禁词、句长和源码泄漏。
8. 当样例超过 50 条并通过质量检查后，再接入 Qdrant 本地检索。
