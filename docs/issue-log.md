# Codex Speak 问题记录

更新时间：2026-06-05

这份记录用于保留当前项目排障中遇到的问题、已经解决的问题、仍需产品化跟进的风险，以及对应验证方式。后续每次遇到新问题，都追加到这里，不只靠聊天上下文记忆。

## 当前结论

真实 Codex 新会话已经可以直接看到并调用全局 `codex_speak` MCP 工具。

已验证两条路径：

- `codex exec` 全新会话直接调用 `codex_speak_status` 和 `codex_speak_prepare`，最终返回 `direct_mcp_available=true`。
- Codex Desktop `create_thread` 新线程直接调用 `codex_speak_status` 和 `codex_speak_prepare`，最终返回 `direct_mcp_available=true`。

旧会话不会热加载新 MCP 工具。修复后必须新开 Codex thread 才能看到 `codex_speak_*`。

## 已解决

### 1. Codex 配置无法加载

现象：

- `codex mcp list` 失败。
- 报错为 `unknown variant priority, expected fast or flex in service_tier`。
- Codex Settings 里能看到 MCP server，但 CLI/agent 侧无法稳定加载配置。

原因：

- `~/.codex/config.toml` 中 `service_tier = "priority"` 不被当前 Codex 配置解析器接受。

处理：

- 改为 `service_tier = "fast"`。

验证：

- `codex mcp list` 可以正常输出 MCP server 列表。

### 2. 新 session 没有直接暴露 `codex_speak_prepare`

现象：

- 新开的验证 session 明确回答没有 `codex_speak_prepare` / `codex_speak_status`。
- 只看到 `functions`、`mcp__node_repl`、`codex_app`、`tool_search` 等工具。

原因：

- 起初配置解析失败，Codex 无法可靠加载全局 MCP。
- 后续还叠加了 MCP server framing 不兼容，导致真实 Codex 客户端初始化时得不到响应。

处理：

- 修复配置解析。
- 修复 MCP stdio framing，详见“真实 Codex MCP 握手卡住”。

验证：

- `codex exec` 全新会话中出现 `mcp_tool_call`：`server=codex_speak`，`tool=codex_speak_status`。
- Desktop 新线程中出现 `mcpToolCall`：`codex_speak_status` 和 `codex_speak_prepare` 均 completed。

### 3. 重复 MCP 注册

现象：

- `codex mcp list` 同时显示：
  - `codex-speak`
  - `codex_speak`
- 两个 server 都指向同一个 `codex-speak mcp`。

原因：

- 插件 `.mcp.json` 贡献了 `codex-speak`。
- 全局 `~/.codex/config.toml` 又配置了 `codex_speak`。

处理：

- 保留全局 `codex_speak` 作为跨新 thread 的稳定主路径。
- 在 `~/.codex/config.toml` 中禁用插件内 MCP：

```toml
[plugins."codex-speak@personal".mcp_servers."codex-speak"]
enabled = false
```

验证：

- `codex mcp list` 中 `codex-speak` 为 disabled。
- `codex_speak` 为 enabled，并列出全部 `codex_speak_*` tools。

### 4. `required = true` 导致新线程硬失败

现象：

- Desktop 创建新 thread 超时。
- 日志中出现：`required MCP servers failed to initialize: codex_speak: timed out handshaking with MCP server after 120s`。

原因：

- 全局 MCP 被设置为 `required = true`。
- 当 MCP 握手失败时，Codex session 初始化直接失败，不再降级启动。

处理：

- 删除 `required = true`。
- 让 MCP 不可用时由 Hook 播放缺失导览提示，而不是让新 session 直接失败。

验证：

- Desktop `create_thread` 可成功创建新线程。
- 修复 framing 后，新线程还能继续完成 MCP tool call。

### 5. 真实 Codex MCP 握手卡住

现象：

- 手动 MCP 自测可以 `initialize` / `tools/list`。
- 真实 `codex exec` 和 Desktop 新线程却卡在 `turn.started`，长时间没有 final answer。
- `codex-speak mcp` 子进程已启动，但 Codex session 无法继续。

根因：

- Codex 真实 MCP 客户端发送的是一行 JSON-RPC，以换行结尾：

```json
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}
```

- 旧的 `codex-speak mcp` 只支持 `Content-Length` framing。
- 自测脚本也只测了 `Content-Length`，所以漏掉了真实 Codex 客户端使用的 line framing。

处理：

- 在 `src/mcp.rs` 中让 `read_message` 自动识别：
  - `Content-Length: ...` header framing。
  - 单行 JSON-RPC line framing。
- `write_message` 按请求使用的 framing 回写响应。
- `scripts/check-mcp-stdio.mjs` 改为同时测试 `header` 和 `line` 两种 framing。

验证：

- `node scripts/check-mcp-stdio.mjs target/release/codex-speak` 通过，`header` 和 `line` 都返回 15 个工具。
- `node scripts/check-mcp-stdio.mjs ~/.codex/codex-speak/bin/codex-speak` 通过。
- `codex exec` 新会话可以正常完成 `hello`。
- `codex exec` 新会话可以直接调用 `codex_speak_status` 和 `codex_speak_prepare`。
- Desktop 新线程可以直接调用 `codex_speak_status` 和 `codex_speak_prepare`。

### 6. 已安装 binary 不是最新修复版

现象：

- 项目源码修复后，Codex 仍然通过 `~/.codex/codex-speak/bin/codex-speak` 启动旧 MCP server。

原因：

- Codex 全局 MCP 配置指向安装目录 binary，不指向工作区 `target/release/codex-speak`。

处理：

- `cargo build --release` 后，把新 binary 复制到 `~/.codex/codex-speak/bin/codex-speak`。
- 杀掉旧的 `codex-speak mcp` 子进程，让新会话启动新 binary。

验证：

- 已安装 binary 的 `check-mcp-stdio` 通过 `header` 和 `line` 两种 framing。
- 真实新 Codex session 验证通过。

### 7. 本地自检覆盖不足

现象：

- `verify-codex` 和旧的 `check-mcp-stdio.mjs` 都能通过，但真实 Codex session 仍卡住。

原因：

- 自检只证明本地 side-channel 和 `Content-Length` MCP 能跑。
- 没有模拟 Codex 真实客户端的 line JSON-RPC framing。

处理：

- `check-mcp-stdio.mjs` 同时覆盖 `header` 和 `line`。

验证：

- release binary 和 installed binary 都通过双 framing 自测。

### 8. 中间过程消息触发缺失导览提示

现象：

- 已经调用过 `codex_speak_prepare`，上一条导览也被移动到 `last-consumed.json`。
- 之后又听到：“这次回答完成了，但当前会话没有送来朗读导览。”
- `last-spoken.txt` 被更新成缺失提示，而 `spool/latest.json` 不存在。

原因：

- Codex Desktop 的 `commentary` 中间消息也会写入 session，并触发 `turn-ended` notify。
- Hook 脚本每次 `turn-ended` 都执行 `codex-speak speak`。
- 过程消息通常还没调用 `codex_speak_prepare`，旧逻辑在没有 side-channel 时直接播放 `brief_notice`。
- 第二次复现时，导览其实已经写入；但旧修复先读取 side-channel、再判断 phase，导致 `commentary` 触发的 Hook 提前消费了 `latest.json`。真正的 `final_answer` Hook 再运行时已经没有导览，于是播放缺失提示。

处理：

- `src/session.rs` 新增最近 assistant 活动判断。
- 当最近 assistant 消息是 `commentary`、`analysis` 等非 `final_answer` 阶段，并且没有 side-channel 时，Hook 静默退出。
- 调整读取顺序：Hook 在消费 side-channel 前，先判断最近 assistant 活动是不是非最终阶段；非最终阶段即使存在 `latest.json` 也不消费。
- `src/main.rs` 在提取文本为空时直接返回，不进入 TTS。

验证：

- `cargo test session -- --nocapture` 通过，新增用例覆盖 `commentary` 和 `final_answer` 阶段判断。
- `cargo check` 通过。
- `cargo build --release` 通过。
- 已把 release binary 覆盖到 `/Users/sherry/.codex/codex-speak/bin/codex-speak`。
- 在当前最近消息为 `commentary` 且没有 `latest.json` 的场景下，运行安装路径 `codex-speak speak --no-play` 后，`last-spoken.txt` 时间未变化。
- 在当前最近消息为 `commentary` 且存在 `latest.json` 的场景下，运行安装路径 `codex-speak speak --no-play` 后，`latest.json` 仍保留，`last-spoken.txt` 时间未变化。

### 9. 其他 chat session 没有导览时播放缺失提示

现象：

- 在另一个 chat session 中出现：“这次回答完成了，但当前会话没有送来朗读导览。”
- 该 session 有正常 `final_answer`，但没有 `codex_speak_prepare` 调用。
- 当前全局 Hook 对所有 Codex session 生效，不只对 Codex Speak 排障 session 生效。

原因：

- 配置里 `missing_guide_policy = "brief_notice"`。
- 其他 session 不一定触发 `codex-speak` skill，也不一定会调用 MCP 准备导览。
- 全局 Hook 在最终回答后没有收到 side-channel，于是按 `brief_notice` 播放缺失提示。

处理：

- 本机当前配置改为 `missing_guide_policy = "silent"`。
- 默认配置也改为 `silent`，避免新安装后在无导览 session 中主动播缺失提示。
- `brief_notice` 和 `diagnostic_notice` 保留为排障模式，需要检查 MCP/Hook 链路时再打开。

验证：

- `cargo test session -- --nocapture` 通过。
- `cargo check` 通过。
- `cargo build --release` 通过。
- 已把 release binary 覆盖到 `/Users/sherry/.codex/codex-speak/bin/codex-speak`。
- 在没有 `latest.json` 的场景下，运行安装路径 `codex-speak speak --no-play` 后，`last-spoken.txt` 时间未变化。

### 10. 新 chat session 不会自动准备朗读导览

现象：

- 新建的另一个 chat session 有正常 `final_answer`，但没有 `codex_speak_prepare` 调用。
- session 元信息里没有 Codex Speak 的全局行为指令。
- 插件已安装、MCP 可用，但模型没有被要求“每个最终回答都准备导览”。

原因：

- Codex plugin 的 `defaultPrompt` 只是启动建议，不是全局系统行为。
- Skill 只有在被触发时才稳定生效，不能保证所有普通新 chat 都自动调用 `codex_speak_prepare`。
- `~/.codex/AGENTS.md` 原本为空，因此没有把 Codex Speak 的默认产品规则注入新 session。

处理：

- 写入 `~/.codex/AGENTS.md`，要求当 `codex_speak_prepare` 可用且朗读开启时，每个最终回答前都准备简短中文导览。
- 明确要求不要等用户提到 `@codex-speak`。

验证：

- `codex debug prompt-input` 已能看到新写入的 Codex Speak 全局规则。
- `codex exec` 全新 session 在用户只要求“请只回答：ping”时，自动调用了 `codex_speak_status` 和 `codex_speak_prepare`。
- Codex Desktop 新 thread 在用户只要求“请只回答：desktop-ping”时，自动调用了 `codex_speak_status` 和 `codex_speak_prepare`，最终回答为 `desktop-ping`。
- Desktop 新 thread 的 side-channel 已被 Hook 消费到 `last-consumed.json`。

## 已缓解但需要继续产品化

### 1. 旧 session 不会热加载 MCP tools

现象：

- 修复后，当前已经打开的旧 session 仍然可能没有 `codex_speak_prepare`。

原因：

- Codex 工具列表在 session 启动时注入，旧 session 不会自动热加载新增 MCP。

当前处理：

- 明确要求修复后新开 Codex thread 验证。

待产品化：

- 控制面板或 `doctor` 输出应明确区分：
  - 已安装。
  - MCP server 可启动。
  - 当前 Codex session 已加载 tools。

### 2. `doctor` 还不能完全证明真实 Codex session 可调用 MCP

现象：

- `doctor --json` 和 `verify-codex --json` 可以通过，但此前真实 Codex session 仍失败。

原因：

- `doctor` 偏安装和本地链路检查。
- `verify-codex` 偏 side-channel 和 Hook 链路检查。
- 真实 Codex session 的工具注入需要额外验证。

当前处理：

- 已把 `check-mcp-stdio.mjs` 扩展为双 framing。

待产品化：

- 增加一个明确的“真实 Codex MCP framing 兼容”检查项。
- 在支持包里收集 `codex mcp list/get codex_speak` 结果和双 framing 自测结果。

### 3. Skill 不是强制执行机制

现象：

- 即使 MCP 可用，Codex 也需要被 prompt/skill 引导才会在最终回复前调用 `codex_speak_prepare`。

当前处理：

- Skill 中明确要求 MCP side-channel 优先。
- Hook 在没有 side-channel 时播放缺失提示，不猜普通 final answer。

待产品化：

- 继续收紧全局或项目级指令，让“如果 `codex_speak_prepare` 可用，最终回答前调用它”成为默认行为。
- 控制面板可显示最近一次朗读来源：MCP side-channel、缺失提示、手动试听。

### 4. 插件内 MCP 与全局 MCP 的关系需要安装器固化

现象：

- 当前手动配置为：插件内 `codex-speak` disabled，全局 `codex_speak` enabled。

原因：

- 全局 MCP 更适合跨新 thread 自动暴露工具。
- 插件内 MCP 仍有调试和兼容价值，但重复启用会混淆验证。

待产品化：

- 安装器应默认写入或维护这个状态。
- 文档应明确主路径是全局 `codex_speak`。

### 5. 全局 Codex Speak 行为规则需要安装器固化

现象：

- 当前已手动写入 `~/.codex/AGENTS.md`，让新 chat session 自动准备朗读导览。
- 但这还不是安装器保证的状态。

待产品化：

- `codex-speak install` 应写入或合并一段稳定的 Codex Speak 全局规则。
- `doctor` / `status` 应检查 `~/.codex/AGENTS.md` 是否包含这段规则。
- `uninstall` 应只移除 Codex Speak 自己写入的规则块，不破坏用户已有 AGENTS 内容。

## 仍需关注

### 1. Codex plugin manifest 警告

现象：

- `codex exec` 输出过警告：`ignoring interface.defaultPrompt[0]: prompt must be at most 128 characters`。
- 还看到若干 `interface.icon_small/icon_large: icon path must not contain '..'`。

影响：

- 当前不影响 `codex_speak` MCP 调用。
- 但会污染验证输出，也可能影响插件展示。

后续：

- 检查对应插件 manifest，把过长 default prompt 和不合法 icon 路径修掉。

### 2. Windows 真机未重新验证

现状：

- macOS 上真实 Codex session 和本地 TTS 链路已验证。
- Windows 仍需真实机器验证安装、MCP 启动、PowerShell 播放、模型路径和权限。

后续：

- 按 `docs/release-qa.md` 跑 Windows 手工 QA。

### 3. Release 包需要重新打包

现状：

- 源码和本机已安装 binary 已修复。
- 已有 release 包如果包含旧 MCP framing 实现，需要重新打包。

后续：

- 重新生成 macOS/Windows release 包。
- 跑 release readiness。

### 4. 覆盖安装 binary 时出现旧测试进程卡在 `U` 状态

现象：

- 直接覆盖安装目录 binary 后，早先手动启动的两个测试进程仍显示为 `U` / `UE` 状态：
  - `codex-speak speak --help`
  - `codex-speak status`
- 普通 `kill` 和 `kill -9` 没有立即清掉这两个旧进程。

当前影响：

- 新启动的安装路径 `codex-speak status` 已能正常返回。
- 本次缺失导览提示修复已经用新进程验证通过。
- 旧进程仍需观察，必要时重启系统清理。

后续：

- 安装器更新 binary 时应使用临时文件加原子替换，并尽量先停止旧 MCP/Hook 进程。
- 支持包可以增加“残留 codex-speak 进程状态”检查。

### 5. 长 Codex thread 触发上下文窗口耗尽

现象：

- 某个 Desktop chat session 报错：`Codex ran out of room in the model's context window. Start a new thread or clear earlier history before retrying.`
- 对应源线程为 `019e93a6-9301-7860-a74d-e64389c6fda3`。

证据：

- session 文件：`/Users/sherry/.codex/sessions/2026/06/05/rollout-2026-06-05T01-20-27-019e93a6-9301-7860-a74d-e64389c6fda3.jsonl`。
- 2026-06-05 06:49:52 上海时间，最后一次成功 token 记录为 `last_token_usage.input_tokens = 225606`，`model_context_window = 258400`。
- 2026-06-05 06:50:41 上海时间，下一条 token 记录显示 `total_tokens = 258400`，且 `last_agent_message = null`，说明请求已打满上下文窗口，未产生最终回答。
- 该线程此前多次执行宽泛日志搜索，例如搜索整个 `~/.codex`、`~/Library/Logs` 和 `~/.codex/sessions`，原始输出巨大；虽然工具层做了截断，截断后的输出仍反复进入会话历史。

判断：

- 这不是 `codex_speak_prepare` 本身导致；全局 `~/.codex/AGENTS.md` 只有数百字，Codex Speak 导览 side-channel 也很短。
- 主要原因是长线程累计历史叠加多次大工具输出，后续又触发 OpenAI/Codex 文档 skill 和 CLI help 输出，剩余上下文不足。

规避：

- 当 token 已接近 200k 或线程已多次排障时，优先新开 thread 或 fork 后继续。
- 搜索日志时不要直接 `rg ~/.codex` 或 `rg ~/.codex/sessions`；先用 `find`/`ls` 定位最近文件和文件大小，再对单个目标文件用 `rg`、`tail`、`sed -n`。
- 对高噪声命令设置小 `max_output_tokens`，并优先加 `head`、`tail`、`--glob` 排除历史 session、`_research`、`node_modules`、vendor 目录。
- 不要把完整 skill 或长文档粘进用户消息；需要时只传路径，让 agent 按 progressive disclosure 读取必要片段。

## 当前验证命令

本地构建和单元检查：

```bash
cargo check
cargo build --release
cargo test mcp -- --nocapture
cargo test session -- --nocapture
```

MCP stdio 双 framing：

```bash
node scripts/check-mcp-stdio.mjs target/release/codex-speak
node scripts/check-mcp-stdio.mjs /Users/sherry/.codex/codex-speak/bin/codex-speak
```

Codex MCP 配置：

```bash
codex mcp list
codex mcp get codex_speak
```

Codex Speak 本地链路：

```bash
/Users/sherry/.codex/codex-speak/bin/codex-speak doctor --json
/Users/sherry/.codex/codex-speak/bin/codex-speak verify-codex --json
```

真实 Codex session 验证：

```bash
codex exec --json -C "/Users/sherry/Documents/啦啦啦 2/codex-speak" -m gpt-5.5 --dangerously-bypass-approvals-and-sandbox "只回答：hello"
```

再开一个新 Desktop thread，要求只使用直接暴露的 `codex_speak_*` 工具，验证 `codex_speak_status` 和 `codex_speak_prepare` 都出现为 completed tool call。
