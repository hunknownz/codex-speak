# Codex App Server Realtime 接入探针记录

日期：2026-06-05

## 背景

这次实验是为了回答一个很具体的问题：

> Codex app-server 暴露的 `thread/realtime/*` 能力，是否已经可以被 Codex Speak 直接接入，用来替代或覆盖我们当前的本地朗读链路？

实验要求：

- 使用新的 `codex exec` chat/session 测试。
- 不改动当前 Codex Speak 主代码。
- 只在 `_research/` 下放隔离探针和原始产物。

## 本机环境

- Codex CLI：`codex-cli 0.128.0`
- App server：`codex app-server --listen stdio://`
- 实验目录：`_research/app-server-realtime-probe/`
- 探针脚本：`_research/app-server-realtime-probe/probe-app-server-realtime.mjs`
- 子会话输出：`_research/app-server-realtime-probe/codex-exec-last-message-danger.txt`

## Schema 发现

通过：

```bash
codex app-server generate-json-schema --experimental --out _research/app-server-realtime-probe/schema
codex app-server generate-ts --experimental --out _research/app-server-realtime-probe/ts-types
```

本机 app-server schema 已经包含实验性的 realtime 方法：

- `thread/realtime/start`
- `thread/realtime/appendAudio`
- `thread/realtime/appendText`
- `thread/realtime/stop`
- `thread/realtime/listVoices`

也包含对应通知：

- `thread/realtime/started`
- `thread/realtime/itemAdded`
- `thread/realtime/transcript/delta`
- `thread/realtime/transcript/done`
- `thread/realtime/outputAudio/delta`
- `thread/realtime/sdp`
- `thread/realtime/error`
- `thread/realtime/closed`

`ThreadRealtimeStartParams` 支持：

- `outputModality`: `text` 或 `audio`
- `transport`: `websocket` 或 `webrtc`
- WebRTC 模式需要客户端 SDP offer。

这说明“接口形状”已经在本机版本里出现了，不是纯文档概念。

## codex exec 新会话测试

按要求开了新的 `codex exec` session：

- `codex exec` thread id：`019e94ce-93fe-7e81-8c0c-f0e62be79f52`
- 执行命令：`node _research/app-server-realtime-probe/probe-app-server-realtime.mjs`

第一次用 `--sandbox read-only` 测试时，子 session 无法让 app-server 写入 Codex 会话/状态数据库，`thread/start` 没走到可用结果。

随后用无沙箱的独立 `codex exec` session 重试，但仍要求不修改主代码。该次测试成功跑完整个探针。

## 实测结果

### 1. app-server initialize：成功

返回：

- `userAgent`: `Codex Desktop/0.128.0 ...`
- `codexHome`: `/Users/sherry/.codex`
- `platformOs`: `macos`

### 2. thread/start：成功

探针内部新建了 ephemeral thread：

- thread id：`019e94ce-cfb3-7100-829b-7b7740190fd4`
- model：`gpt-5.5`
- model provider：`openai`
- sandbox：`readOnly`
- network：`false`

这说明 app-server JSON-RPC 基础链路是通的，新建 Codex thread 也可用。

### 3. thread/realtime/listVoices：成功

返回 voices：

- v1：`juniper`, `maple`, `spruce`, `ember`, `vale`, `breeze`, `arbor`, `sol`, `cove`
- v2：`alloy`, `ash`, `ballad`, `coral`, `echo`, `sage`, `shimmer`, `verse`, `marin`, `cedar`
- default v1：`cove`
- default v2：`marin`

这说明 app-server 后端已经能暴露 Realtime voice registry。

### 4. thread/realtime/start：失败

请求参数：

```json
{
  "threadId": "019e94ce-cfb3-7100-829b-7b7740190fd4",
  "outputModality": "text",
  "transport": { "type": "websocket" },
  "prompt": "You are a minimal realtime probe. Reply very briefly. Do not edit files or run tools.",
  "voice": null,
  "sessionId": null
}
```

返回错误：

```json
{
  "code": -32600,
  "message": "thread 019e94ce-cfb3-7100-829b-7b7740190fd4 does not support realtime conversation"
}
```

因此没有进入：

- `thread/realtime/started`
- `thread/realtime/appendText`
- transcript delta
- output audio delta

## 当前判断

Codex app-server Realtime 在本机 `0.128.0` 中属于“接口已出现，但普通 app-server thread 尚不能直接开启 realtime conversation”的状态。

更具体地说：

- 可以接入 JSON-RPC app-server。
- 可以新建 app-server thread。
- 可以查询 realtime voices。
- 不能对当前创建方式得到的普通 thread 调用 `thread/realtime/start`。

这意味着它现在还不能直接替代 Codex Speak 的本地朗读路线。

## 对 Codex Speak 的影响

短期不建议把 Codex Speak 主链路改成依赖 `thread/realtime/start`。

原因：

- `thread/realtime/*` 仍是 experimental。
- 普通 thread 返回“不支持 realtime conversation”。
- 目前没有在公开 schema 里找到 `thread/start` 时开启 realtime-capable thread 的稳定参数。
- Codex Speak 现在的价值仍应放在 agent event/state summarization、低打扰播报、本地离线 TTS 和通知层，而不是赌 Codex Realtime 已经可用。

但这条线值得继续跟踪：

- 如果未来 app-server thread 支持 realtime conversation，Codex Speak 可以把它作为“可选输入/输出通道”。
- Realtime voice registry 可用于参考 OpenAI 官方 voices 命名和默认选择。
- WebRTC/SDP 路线可以作为后续“远程手机控制/实时语音对话”的技术储备。

## 下一步建议

1. 继续保留 `_research/app-server-realtime-probe/`，作为跟踪 Codex app-server realtime 能力的最小复现实验。
2. 后续 Codex CLI/Desktop 升级后，优先重跑该探针，看错误是否从“不支持 realtime conversation”变成可启动。
3. 如果 OpenAI 后续文档或 changelog 明确发布 Codex thread realtime 接入方式，再做正式 POC。
4. Codex Speak 主产品路线暂时继续走本地 speech/notification layer，不把 app-server realtime 设为硬依赖。
