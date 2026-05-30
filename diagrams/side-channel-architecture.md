# Codex Speak Side-Channel 架构图

```mermaid
flowchart TB
    U[用户提问] --> C[Codex 理解上下文\n执行任务 / 分析结果]

    C --> A[正常最终回答\n显示在 Chat Session]
    C --> B[朗读复述稿\n由 Codex 生成]

    B --> P[Codex Speak Plugin / Tool\n接收 spoken_text]
    P --> S[本地 Sidecar 文件\n~/.codex/codex-speak/spool/latest.json]

    A --> H[Codex 回复结束\nHook 被触发]
    H --> R[Rust CLI: codex-speak speak\n读取 latest.json]

    R --> T[本地 TTS\nSherpa-ONNX + MeloTTS]
    T --> O[系统播放器\nafplay / Windows 播放器]
    O --> V[用户听到朗读]

    R -. 如果 latest.json 缺失或过期 .-> F[兜底：清洗最终回答]
    F --> T
```

## 关键分工

- **Codex 理解发生在这里**：`Codex 理解上下文 -> 朗读复述稿`
- **Plugin / Tool 不理解内容**：只接收 Codex 写好的 `spoken_text`
- **Rust CLI 不理解内容**：只读取、校验、调用 TTS
- **Hook 不理解内容**：只在回复结束后触发播放
- **TTS 不理解内容**：只把文字变成声音

## 为什么不用 HTML 注释

旧方案：

```text
Codex 最终回答里塞 HTML 注释
Hook 从最终回答里提取朗读稿
```

问题：

```text
朗读稿会进入 Chat Session 原文
长复述会污染回答
复制和调试时会看到隐藏文本
```

新方案：

```text
Codex 正常回答显示给用户
Codex 朗读稿走 side-channel 存到本地
Hook 只负责播放本地朗读稿
```

