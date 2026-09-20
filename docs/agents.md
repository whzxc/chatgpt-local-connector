# 本地 Agents

CLC 支持 Codex Native、Pi RPC 和 OpenCode ACP。设置页的 Agents 区域自动发现可执行文件、版本和连接方式，不保存或要求重复填写 provider/token。Codex 保持默认；调用通用工具时显式选择 `agent`。

| Agent | 连接方式 | 配置与执行归属 | 能力边界 |
| --- | --- | --- | --- |
| Codex | App Server + Desktop IPC | 继承 Codex；按现有设置由 Desktop 或 Connector 执行 | 所有 `codex_*` 原生入口保持可用；通用入口只映射公共任务能力 |
| Pi | 官方 `pi --mode rpc` JSONL | 继承 Pi 配置、provider、model、扩展；CLC 管理子进程 | 使用 `agent_settled` 确认最终完成，需要提供该事件的 Pi 版本（本版适配 0.85.1）；交互通过 extension UI，非统一工具权限沙箱 |
| OpenCode | 官方 `opencode acp`，ACP v1 JSON-RPC stdio | 继承 OpenCode 配置和登录；CLC 管理子进程 | 支持文本 prompt、流式更新、取消与权限响应；恢复要求 `loadSession` 能力 |

`installed` 表示找到 CLI 且版本命令可运行，`ready` 表示存在已初始化的活动 CLC 会话，`unavailable` 表示缺少 CLI 或版本探测失败。这些状态不保证 provider 已登录或额度可用。模型服务是否可用需通过实际 prompt 判断。Codex 的原生连接状态继续显示在原有区域。

CLC 自身不依赖 Node 运行时；Pi 等外部 CLI 仍需其自身的运行环境。自动发现搜索 PATH 和常见用户 CLI 目录。Windows npm 的 `.cmd` 启动器使用受限参数引用；包含 shell 特殊字符的路径或 manifest 参数会被拒绝，改用原生可执行入口或普通路径。

## 通用工具

| 工具 | 用途 |
| --- | --- |
| `agents` | 发现安装、版本、协议与默认 Agent |
| `agent_capabilities` | 公共能力；提供 `taskId` 读取该进程实际协商结果 |
| `agent_tasks` | 列出任务；Pi/ACP 限于 CLC 创建的任务，使用 offset/limit 分页；Codex 使用原生任务列表和 cursor 分页 |
| `agent_create` | `agent`、绝对 `cwd`、UUID `requestId`，可选文本 `prompt`；Codex 创建并执行，需要 prompt |
| `agent_read` | `agent`、`taskId`，读取状态、最近输出和协议元数据 |
| `agent_send` | `agent`、`taskId`、`prompt`、新 `requestId`；必要时恢复已确认停止的会话 |
| `agent_interrupt` | 请求取消；Codex 还需要原生 `turnId` |
| `agent_events` | 按 after/limit 读取原生流式事件；backendSession 用于识别进程重启 |
| `agent_pending` | 等待中的 ACP 权限请求、Pi 扩展交互或 Codex 原生回调 |
| `agent_respond` | 用 `interactionId` 响应；ACP 提供 `optionId` 或 `cancelled`；Pi 提供 `value`、`confirmed` 或 `cancelled`；Codex 使用原生 result/error、backendSession |
| `agent_request` | 回读幂等回执；可 approve/bypass/reject 待确认操作 |

先创建会话，再回读写入回执与任务：

```json
{"agent":"opencode","cwd":"/absolute/project","requestId":"新生成的 UUID","prompt":"不要使用工具或修改文件，只回复 OK"}
```

写操作先保存回执再提交。每个新操作生成新 UUID，同一操作重试必须保持 requestId 和全部参数不变。`agent_request.state=completed` 表示该写操作已结束，不等于任务成功：Pi 的 prompt 回执在接受输入后返回，而 ACP prompt 的响应在本轮结束后返回。两者均需 `agent_read` 确认最终状态与输出。启用任务审批时，create/send/interrupt 先进入 `awaiting-approval`，可通过 `agent_request` 决定；`approval=approved/bypass` 沿用现有确认语义。Pi/ACP 的确认目前通过 MCP 操作，现有桌面任务页仍展示 Codex 任务。

`taskId` 是 CLC UUID（Codex 直接使用 threadId），`agent` 区分驱动，`runtimeSource` 保留执行来源，`sessionId` 是上游会话 ID。Pi/ACP 的 `turnId` 是 CLC 单次 prompt 标识，不伪装成上游轮次 ID。协议原始数据位于 metadata、lastMessage 和事件中。Codex 结果保留 native 字段与原始状态；Codex events/pending 沿用原生全局事件流与收件箱，未按 taskId 过滤。

Pi/ACP 状态包含 starting、idle、running、waiting-permission、cancelling、completed、cancelled、failed、unknown。权限响应不是自动批准：必须选择上游给出的选项。Pi 扩展的 confirm/select/input/editor 被转发，但不提供 ACP 式统一工具权限控制；工具与文件权限仍由 Pi、扩展和运行环境决定。CLC 不宣称额外的文件系统沙箱。

超时、进程异常退出或写入结果不明会留下 `unconfirmed` 回执或 `unknown` 任务，不自动重放。存在未确认操作时不能通过新 prompt 自动恢复。停止后的已完成会话可由 agent_send 恢复：Pi 使用原 session 文件，ACP 使用协商后的 session/load。未发送 prompt 的 Pi 空会话可能尚未持久化，不能保证跨进程恢复。

事件有有界内存窗口，完整 JSONL 存在 CLC 私有 outputs 目录，可通过 `control_output` 读取。ACP 最近文本输出超过 256 KiB 时仅保留末尾并标记 outputTruncated；完整输出在事件归档中。进程停止后的任务快照仍可读，实时 events/pending 需要存活进程。关闭连接或退出 CLC 会关闭其拥有的 Pi/ACP 子进程；Desktop 拥有的 Codex 任务继续沿用原生命周期。

## ACP 能力协商与扩展

CLC 支持 ACP v1 的 initialize、session/new、session/load、session/prompt、session/update、session/cancel、session/request_permission。初始化时不声明客户端文件系统、terminal 或交互式登录能力；Agent 可使用自己的本地工具。未声明的客户端方法返回 method-not-found。provider 登录使用 Agent 自己的 CLI。图片、模型切换、会话 fork/list/import 及 ACP v2 尚未通过通用工具暴露，即使上游公布相应能力。

在 CLC 私有状态目录的 `agents/manifests.json` 中可增加 ACP 定义，重启应用后读取：

```json
[
  {"id":"custom-agent","command":"/absolute/path/to/agent","args":["acp"]}
]
```

manifest 仅支持 ID、command、args；ID 不可覆盖 codex/pi/opencode。CLC 不通过 MCP 接收可执行命令或 manifest 修改。该配置是本地受信任代码启动入口，不是远程命令执行工具。provider 配置继续由 Agent 自己管理。

新增符合 ACP v1 的 Agent 只需要 manifest，无需修改 Host、MCP 工具或设置页。未来的 Claude Code/Grok 若提供该入口可采用此方式；若仅提供其他官方稳定接口，需要新增 AgentDriver 协议实现及注册项，复用现有任务、幂等、事件、UI 与 MCP 边界。本版不包含这两个 Agent 的驱动，也不假设它们已有 ACP 支持。

协议参考：[ACP v1](https://agentclientprotocol.com/protocol/v1/initialization)、[OpenCode ACP](https://opencode.ai/docs/acp/)、[Pi 官方 RPC](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md)。
