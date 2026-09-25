# 工具与运行语义

返回[项目首页](../README.md)。

## 能力清单

当前入口按连接的[工具列表策略](zh-CN/control-sources.md#工具列表)暴露 MCP 工具。原生方法及参数 Schema 从本机 Codex 二进制动态发现；
工具数量不保证不同 Codex 版本拥有相同的原生能力，各项功能取决于当前原生服务提供的能力。

| 工具 | 用途 |
| --- | --- |
| agents / agent_capabilities / agent_tasks / agent_create / agent_read / agent_wait / agent_send / agent_interrupt / agent_events / agent_pending / agent_respond / agent_request | 多 Agent 公共任务与幂等入口，详见[本地 Agents](agents.md) |
| agent_context | 已有任务的只读 Review/Handoff；声明、原生证据和当前工作区变化分开返回 |
| connector_verify | 回传引导中的验证码，确认连接调用到达本机，不执行任务 |
| projects / overview / tree / search / read / git | 实时 Codex 本机项目、目录文件事实和只读 Git 查询 |
| fs | 主机绝对路径的文件读写、目录、元数据、复制、删除和监听 |
| command | Codex sandbox/permission 下的独立命令及 PTY/stdin/终止控制 |
| process | App Server 主机上的非 Codex sandbox 进程及 PTY/stdin/终止控制 |
| mcp | 已配置 downstream MCP 的目录、资源、工具调用、OAuth 和事件订阅 |
| file_search | fuzzyFileSearch 模糊文件名/路径查找 |
| codex_thread | 原生线程领域参数入口；Desktop 任务的扩展操作受接入范围限制；后台任务转发到 Connector app-server |
| codex_account | 当前账号、用量、额度和工作区消息 |
| codex_capabilities | 服务版本、安装版本、原生模型目录和连接边界 |
| codex_wait | 可续接的事件驱动长轮询，等待原生轮次终态或交互，默认 30 秒、最长 5 分钟 |
| codex_tasks / codex_read / codex_items | 所有原生可访问任务、状态、历史与完整条目分页 |
| codex_create / codex_send / codex_interrupt | 创建、续接、追加输入和中断 |
| codex_schema | 搜索当前原生方法、获取完整参数 JSON Schema；server 方向含回调响应 Schema |
| codex_query | 常用原生只读方法快捷入口，包含文件、项目、配置、模型、权限、插件等查询 |
| codex_call | 任意原生方法，完整透传参数；长操作异步回执 |
| codex_request | 回读操作状态与原始结果；批准、绕过或拒绝待审批任务，不重放已提交请求 |
| codex_pending / codex_respond | 原生审批、用户输入、动态工具等回调收件箱与响应 |
| codex_events | 带游标的原生通知，包括轮次进度、请求解决、命令输出 |
| control_output | 完整分页读取大结果 JSON |

领域工具使用一个顶层工具和浅层 action，params 保持原生字段名、嵌套结构和 null 语义，
普通调用的必要字段直接列在工具参数描述中。file_search 直接接受 query、roots、cancellationToken。
不复制 App Server 参数校验、默认值或状态机；复杂类型、低频接口与未来新增参数可查询 codex_schema。
混合读写领域的 MCP annotation 按可能写入声明，读 action 本身不要求 requestId；
command/process 的所有 action，以及其他领域的 mutation 必须带 requestId。

codex_call 提供原生 RPC 入口，任务写操作受 Desktop 所有权和支持范围限制。其他能力包括项目/分组管理、线程删除/设置/目标/队列、
实时语音、review、配置写入、模型/权限目录、技能/插件/应用管理、账号登录/退出/充值相关操作，
以及 fuzzyFileSearch 的增量 sessionStart/sessionUpdate/sessionStop。已有 codex_query 的只读分类保持可用。
这些能力是否可用由当前原生服务决定。initialize/initialized 由连接负责。

## 领域 action 与最小示例

下表 action 按原生后缀命名，仅为常用入口省去固定前缀：

| 工具 | action → 原生方法 |
| --- | --- |
| fs | readFile、writeFile、createDirectory、getMetadata、readDirectory、remove、copy、watch、unwatch → fs/同名方法 |
| command | exec → command/exec；write、terminate、resize → command/exec/同名方法 |
| process | spawn、writeStdin、kill、resizePty → process/同名方法 |
| mcp | list → mcpServerStatus/list；resource/read、tool/call、oauth/login、event/stream/start、event/stream/stop → mcpServer/同名方法 |
| codex_thread | Schema 保留原生 action 定义；Desktop 任务的扩展操作返回 `DESKTOP_METHOD_UNSUPPORTED`，后台任务转发至其 App Server |
| codex_account | read、usage/read、rateLimits/read、workspaceMessages/read → account/同名方法 |
| file_search | 无 action → fuzzyFileSearch |

以下为 MCP 工具名和 JSON 参数示例；mutation 中的 requestId 每个新操作使用新 UUID：

```text
fs {"action":"readFile","params":{"path":"/tmp/example.txt"}}
fs {"action":"writeFile","params":{"path":"/tmp/example.txt","dataBase64":"aGVsbG8K"},"requestId":"<UUID>"}
fs {"action":"watch","params":{"path":"/tmp/example.txt","watchId":"example-watch"},"requestId":"<UUID>"}
fs {"action":"unwatch","params":{"watchId":"example-watch"},"requestId":"<UUID>"}
file_search {"query":"connector","roots":["/absolute/project"]}
command {"action":"exec","params":{"command":["pwd"],"cwd":"/tmp","timeoutMs":10000},"requestId":"<UUID>"}
process {"action":"spawn","params":{"command":["cat"],"cwd":"/tmp","processHandle":"example-cat","streamStdin":true,"streamStdoutStderr":true},"requestId":"<UUID>"}
process {"action":"writeStdin","params":{"processHandle":"example-cat","deltaBase64":"aGVsbG8K","closeStdin":true},"requestId":"<UUID>"}
mcp {"action":"list","params":{}}
mcp {"action":"resource/read","params":{"server":"local","uri":"example://resource"}}
mcp {"action":"tool/call","params":{"server":"local","threadId":"<已有线程 ID>","tool":"inspect","arguments":{}},"requestId":"<UUID>"}
codex_account {"action":"read"}
codex_account {"action":"usage/read","params":{"threadId":"<线程 ID>"}}
codex_account {"action":"rateLimits/read"}
codex_account {"action":"workspaceMessages/read"}
```

fs 操作主机文件系统，不解析项目别名、不使用 tree/search 的 Git 文件列表；文件内容使用 base64。
remove 的 recursive/force、createDirectory 的 recursive 等默认值完全由原生决定。
watchId、processId、processHandle、MCP subscriptionId 属于当前 App Server 连接；重启后不重建。

command/exec 不创建 Codex thread/turn，使用 Codex sandboxPolicy 或 permissionProfile；省略时继承原生设置。
exec 的最终结果在退出后返回，超过短等待窗口的调用通过 codex_request 读取回执。
PTY 使用 tty:true 和 size:{rows,cols}；PTY、streamStdin、streamStdoutStderr 和后续控制都需要 processId。
command 的 disableTimeout 与 timeoutMs 互斥，disableOutputCap 与 outputBytesCap 互斥。

process/spawn 在 App Server 主机上运行，**不使用 Codex sandbox**，也不创建线程；它注册 processHandle 后即返回。
退出状态位于 process/exited，spawn 回执 completed 只表示启动 RPC 完成。cwd 必须为绝对路径；
timeoutMs/outputBytesCap 显式 null 禁用对应限制，省略时使用原生默认值。
两类执行的 env 都支持 null 删除继承变量，stdin 使用 deltaBase64；权限和参数冲突由上游判定。
领域工具顶层 timeoutMs 是 RPC 等待超时（写操作默认 24 小时、读操作默认 30 秒），与 params.timeoutMs 独立。

fs/changed、command/exec/outputDelta、process/outputDelta/exited、MCP 订阅通知共用 codex_events 游标缓冲，
无需另一个事件服务。流式字节按原生 base64 返回；流式输出不会自动补进原生最终结果。
此缓冲不是持久输出存储，超过 2000 条或 8 MiB，或连接重启会报告 gap/reset，无法从缓冲恢复已丢失的字节。

mcp 的 list 可携带 threadId 读取线程上下文中的目录；tool/call 与 event/stream/start 的 threadId 是原生必填，
使用已有线程，不隐式创建或续接线程。服务名、工具名、arguments、_meta 完整透传；下游不可用或不支持时返回原生错误。
oauth/login 返回原生登录结果，认证仍由原生完成；订阅启动/停止的 mutation 使用现有 requestId 回执。

当前支持的任务操作为创建、读取、续接、追加输入和中断。Desktop 任务的其他 `thread/`、`turn/` 扩展操作由连接器明确拒绝，需在 Desktop 中完成；原生 Schema 中存在某方法不代表 Connector 支持调用。账号结果直接来自 App Server。

## 参数与默认行为

`codex_create` 的 project 接受 Codex 原生项目 ID、唯一名称或本机绝对目录；多目录项目需指定具体目录，也可以省略。
`thread` 透传 ThreadStartParams，`turn` 透传 TurnStartParams；`codex_send` 的 `resume` 仅支持任务 ID，其他恢复参数会被拒绝。输入使用 `prompt` 或完整原生 `input` 数组；两者同时提供时 input 优先。
thread/turn 中的原生参数优先于便利字段；任务 ID 与 input 由顶层字段统一确定。

省略参数时使用原生宿主或已有任务设置。MCP 不强制 approvalPolicy=never、网络关闭、
限定写入根、恢复必须声明 mode，也不注入额外 developerInstructions。
model、effort、serviceTier、provider、permissions、sandboxPolicy、approvalPolicy、
collaborationMode、outputSchema、runtimeWorkspaceRoots、附件和其他原生参数由上游解释与校验。
模型目录仅用于发现，不由 MCP 额外阻止未列出的模型或档位。Desktop 新任务继承原生创建结果的模型；续接时继承 owner 快照的模型与协作模式。未显式提供 collaborationMode 时，显式 model/effort 同步到继承的协作模式设置，避免旧设置覆盖本次请求。无法确定模型时返回明确错误。

便利字段 mode 支持 read-only、workspace-write、danger-full-access。
显式使用前两种 mode 而省略 networkAccess 时，此便利预设关闭网络；完全省略 mode 时不覆盖
宿主权限。要精确控制权限，直接传 turn.sandboxPolicy 或 turn.permissions。
只有 networkAccess 而没有 mode/sandboxPolicy 不能形成完整原生权限对象，会明确返回参数错误。
活动轮次的原生 steer 只支持输入，配置新轮次可使用 turn/start。

```json
{"project":"/absolute/directory","title":"分析任务","prompt":"检查当前目录并汇报发现。","requestId":"<新 UUID>","thread":{"permissions":"read-only"}}
```

```json
{"method":"command/exec","params":{"command":["git","status","--short"],"cwd":"/absolute/directory"},"requestId":"<新 UUID>"}
```

原始 method 请求不会擅自补参数、改写方法、切换 owner 或创建替代任务。
`resolved` 标注 thread/start 响应，不能代替轮次实际配置；read 中的 effectiveConfiguration
从该任务已落盘 turn_context 读取，缺失时为 unknown。

## 文件和输出

项目列表唯一来源为当前设备 Core 的 `project/list`，完整读取分页并保留原生 ID、名称和全部 roots；不维护登记文件、别名或一分钟缓存，读取失败不回退旧数据。
仍支持直接指定本机绝对目录、非 Git 目录、隐藏文件、Git 忽略文件、任意扩展名以及符号链接和硬链接。
相对路径按 project 解析，绝对路径按宿主文件系统解析。不自动删改或脱敏返回内容。
访问范围由服务进程的系统权限决定；连接拥有者应按实际需要发起读取，不把凭据贴入对话。

tree/search 默认按 Git 工作文件列出以减少噪声，includeIgnored=true 包含忽略文件。
read 可直接读取被忽略文件；Git 内部数据或完整目录枚举可使用 fs/readDirectory。
Git revision 支持分支、标签、SHA 和表达式；diff 可省略 path 查询整个工作区。
文件事实附来源、时间与哈希，Git 状态不代表服务健康或已发布。

行读取使用 UTF-8 文本，单文件预算 16 MiB；二进制使用原生 fs/readFile 的 base64。
文本搜索按页扫描，nextOffset 非空时继续，skipped 报告无法读取的文件数量。
原生工具数据通过 codex_items 或原生 thread/items/list 读取，不固定省略工具输出。
MCP 单次请求帧预算 16 MiB；结果超过 64 KiB 自动保存并返回 outputId。
control_output 每页最多 12000 个 UTF-16 code units，拼接所有 text 后解析 JSON。
本机 outputs 目录中的大结果 JSON 快照保留 7 天，共享 1 GiB 配额；写入新快照时清理过期快照，并按最旧优先腾出空间。过期读取返回 OUTPUT_EXPIRED，已淘汰结果不可再读；单个快照超过配额或无法腾出空间返回 OUTPUT_STORAGE_LIMIT。读取采用有界内存解码，继续使用 UTF-16 偏移。
分页与帧预算用于传输和资源管理，不是业务权限。长输入也可以先 fs/writeFile 再引用原生输入路径。

## 交互、回执与生命周期

原生服务器请求通过显式回调交互处理。codex_pending 返回 id、method、params 和 backendSession；
先查看对应 server Schema，再通过 codex_respond 提交 result 或 JSON-RPC error（二选一）。
已生成的原生响应 Schema 用于校验回传结构，格式错误时保留请求等待纠正。
服务不自动批准命令、不替用户填写决定，也不伪造认证令牌或 attestation。
如果某类回调需要调用方不具备的能力，应显式返回 error；不能把“有回传入口”理解为自动完成所有回调。
响应回执只证明已发出，是否接受须跟进原生事件和轮次。重启或已解决请求不能继续作答。

写工具使用 UUID requestId，参数按递归稳定排序摘要。同一 ID 重试只回读原回执；不同参数拒绝。领域 action 与等价 codex_call 共享同一原生回执。
原生 JSON-RPC 错误的 code/message/data 保留在 error.rpcError，写入失败时位于回执 result.error.rpcError；
未生成 RPC 错误的连接/超时失败仍使用现有控制错误。null、数组等原生结果不强制转换为对象。
回执包含阶段、线程/轮次 ID 和可能包含原始输出的结果，存于权限受限的本机状态目录。
codex_call 与领域 mutation 在短等待后可返回 pending；随后 codex_request 查询 completed/rejected/unconfirmed 等状态。
写入原生 RPC 默认超时 24 小时，可通过 timeoutMs 指定；超时不等于操作取消，不自动重放。
completed 是控制操作完成，任务完成需要检查对应轮次终态。

桌面任务执行归 Desktop 所有，Connector 通过本机版本化协调 IPC 管理，不接管其 app-server。
关闭「自动打开 Codex 任务」后，新任务归 Connector 的 App Server 后台执行，读取、续接和中断按持久化的任务归属路由。后台任务不保证可在 Desktop 中操作。没有执行方实时状态时 runtimeStatus 为 unknown，不表示空闲。
桌面 Thread/Turn 查询是 owner 快照的明确投影；显示层 steeringUserMessage 等 item 可能不同于公开 App Server union。
兼容性边界和支持的任务写操作见 [桌面说明](desktop.md)，未支持的方法明确拒绝；不使用受签名保护的 app-tools 管道、不绕过账号和系统权限。
跨设备不是自动路由能力；可通过原生命令在用户已有 SSH 环境中执行明确的远端操作。

停止入口只停止该入口的 Tunnel 和转发；退出 Core 才停止 App Server 和 Connector 后台任务；界面保持可用，Desktop 自行管理其任务生命周期。
重启后旧提交中回执标记 unconfirmed，不自动重放；尚未提交的待审批请求继续保留。
事件缓冲最多保留最近 2000 条或 8 MiB，gap/reset 明确报告缺口；完整已落盘结果应从原生任务/回执读取。
Chat 页面不会因任务完成自动被唤醒，交互请求也需要调用方主动查询。


## 连接与持久输出

stdio 仅为传输代理，使用父进程传入的本机端口与随机凭据连接应用内的 Rust 核心。stdio 退出不会销毁应用内核心。工具调用产生本机活动日志，来源缺失时不推测 Chat ID。

`codex_events` 提供当前会话的 `persistedOutput.outputId`，通过 `control_output` 分页读取 JSONL；写入失败会返回持久化错误。事件 JSONL 文件与回执保存在本机状态目录，不参与大结果 JSON 快照的过期和配额清理。

原生进程、文件监听和订阅句柄属于当前 App Server 连接；断线后不自动重建。旧会话的审批响应会被拒绝，未确认的写操作不自动重放。

## 可选任务审批与记录

Connector 设置页的任务审批模式只决定新任务管理请求是否默认等待确认，不是权限门槛。用户意图优先：创建、续接、中断以及等价的 `codex_call` 任务请求可传 `approval: "approved"` 表示已确认，或 `approval: "bypass"` 声明直接执行；省略或 `default` 遵循本机设置。其他文件、命令等工具不受此任务审批模式影响，Codex 原生执行权限与回调审批保持独立。

等待时立即返回 `state: "awaiting-approval"` 和原 requestId，尚不提交任务。云端可按用户意图调用：

```text
codex_request {"requestId":"<原请求 UUID>","action":"approve"}
codex_request {"requestId":"<原请求 UUID>","action":"bypass"}
codex_request {"requestId":"<原请求 UUID>","action":"reject"}
```

本机任务页也可批准或拒绝。批准、绕过后 Connector 自动提交已保存的原请求，不需要重新创建。重复决定仅回读当前回执；已提交、已拒绝或结果未确认的请求不会重放。`codex_request` 省略 action 或传 `read` 只查询结果。初次请求的 approval 属于参数摘要；推进已等待的请求使用上述 action，不修改原参数后重发。

模式可在连接期间切换，仅影响之后的新请求，已等待的请求保留。待审批请求跨应用重启保留；提交中断且结果不明的请求标记 unconfirmed，需检查原生状态。

任务页展示此 Connector 记录的请求，按原生任务 ID 汇总创建、续接和中断，按创建时间倒序排列内容自适应高度的瀑布流文档卡片，展示标题、项目、模型、推理等级、创建时间、当前状态及 Prompt 预览；点击卡片平滑放大到窗口中央，查看完整 Prompt 和请求记录。关闭按钮、Esc 或点击背景可缩回卡片；未归档任务的详情中也可打开 Codex，已归档任务显示归档状态并隐藏打开按钮。Prompt、请求模型、推理等级和项目来自持久化请求；未提供的配置不显示，不冒充实际执行配置。原生覆盖参数与 input 优先级反映在详情中。任务列表读取全部已持久化的任务回执，不限定当前连接或轮次；已有回执也纳入列表，未保存的 Prompt、配置和未知运行状态不显示，不扫描会话 JSONL 补造内容。

运行状态通过原生任务快照刷新并保存最后一次基本信息，不解析会话 JSONL；关闭页面、应用重启或无法刷新时，仍保留任务记录和最后一次状态及时间。卡片只显示 Codex 原生运行状态，不将 Connector 回执状态映射为任务状态；原生 idle 显示“空闲”，不代表用户工作目标已完成。任务正文保存在权限受限的本机状态目录，不写入连接日志或系统通知。

## 任务等待

多 Agent 使用 `agent_wait(agent, taskId, turnId?, timeoutMs?, until?, expectedHash?)`，Codex 参数 taskId 对应 threadId，直接复用下面的原生等待链路。Pi/ACP 状态和权限差异见[本地 Agents 等待语义](agents.md#等待语义)。

创建或发送任务后，先从原请求回执取得 threadId / turnId，再优先调用 `codex_wait`。不要重复创建任务，也无需用 `codex_read` 配合 sleep 高频轮询。`codex_read`、`codex_events`、`codex_request` 的用途和参数保持不变。

```json
{"threadId":"<thread-id>","turnId":"<turn-id>","timeoutMs":30000,"until":"terminal-or-interaction"}
```

这是带 `snapshotHash` 的 bounded event-driven long poll：长任务推荐每次 20–30 秒的 wait slice，默认 30 秒。每个 slice 内仍由事件唤醒与真实 owner 状态复核驱动，不是 `read → sleep → read` 高频轮询。

```text
create / send → 取得原 taskId/threadId、turnId → wait(timeoutMs=30000)
  → completed / failed / cancelled / interaction-required：处理结果或交互
  → timeout + snapshotHash=A：沿用原 ID，wait(timeoutMs=30000, expectedHash=A)
  → timeout + snapshotHash=B：沿用原 ID，wait(timeoutMs=30000, expectedHash=B)
  → terminal / interaction
```

timeout 或取消 wait 只释放本次等待，不终止、中断或重建源任务，也不应重新 create/send。续接时保留已返回的 turnId，以固定同一轮次；`expectedHash` 仅比较快照，不要求状态变化后才返回。`unconfirmed` 不代表任务失败，应先核实原任务和回执。

ChatGPT、Notion、Slack 及其他 MCP Client 的外层 Tool Call timeout 可能不同，因此统一使用有界 slice，不提供 per-client timeout 配置。普通 Chat / 通用 MCP Client 不推荐默认阻塞数分钟。明确确认上游允许长调用时仍可显式传 `timeoutMs=300000`，这是能力上限而非推荐默认。OpenAI Tunnel stdio adapter 对两个 wait 工具的内部转发预算为 330 秒；HTTPS MCP 的请求头/请求体读取限时不限制等待执行时间。这些 CLC 预算无法延长外部客户端或代理的超时。

输入是禁止额外属性的 object：

| 字段 | 类型与含义 |
| --- | --- |
| threadId | 必填、非空 string |
| turnId | 可选、非空 string；省略时固定首个观察到的当前活跃轮次，否则最近一轮；不会跳到后续轮次 |
| timeoutMs | integer，1–300000，默认 30000；包括建立读取连接和原生读取时间 |
| until | terminal / interaction-required / terminal-or-interaction（默认）；声明期望目标 |
| expectedHash | 可选 string，上次返回的 snapshotHash，用于 changed 比较 |

终态与需要交互都是退出条件：即使 until=terminal，遇到交互也会退出，避免等待一个需要用户推进的任务；until=interaction-required 遇到终态也退出。`conditionMet` 表示返回状态是否符合 until。expectedHash 不屏蔽已经存在的终态或交互；未提供时 changed 比较本次第一次成功读取的快照。观察时间不参与哈希。

返回示例（位于 MCP structuredContent.result）：

```json
{
  "state":"completed",
  "reason":"native-turn-terminal",
  "threadId":"<thread-id>",
  "turnId":"<turn-id>",
  "recordedStatus":"completed",
  "runtimeStatus":"idle",
  "runtimeSource":"connector-app-server",
  "finalResponse":{"itemId":"<item-id>","type":"agentMessage","text":"完成","textTruncated":false,"nextOffset":null},
  "interaction":[],
  "conditionMet":true,
  "changed":true,
  "snapshotHash":"<hash>",
  "observedAt":"<last-native-observation-time>",
  "returnedAt":"<return-time>",
  "elapsedMs":1234
}
```

`finalResponse` 是选中轮次最近一条 assistant 输出，并不把 running 输出认定为最终交付；沿用条目的 textHash、UTF-16 offset/nextOffset，文本最多 6000 个 UTF-16 单位。余下内容通过 `codex_items` 读取，大结果沿用 `control_output` 分页。

- completed / failed / cancelled 来自所选原生轮次；原生 interrupted 映射为 cancelled，recordedStatus 保留原值。
- interaction-required 返回当前 Connector 原生 pending 的 id、responseId、method、params、backendSession；`codex_respond.id` 使用 responseId（保留原生数值/字符串 ID 的 JSON 编码）。不自动 approve。Desktop 通过 activeFlags 报告等待审批/输入时会立即退出，但其 IPC 不提供原生回调 ID；此时 interactionAction 指向 Desktop 处理，不伪造 ID。
- timeout / deadline 仅代表这一次等待用尽时间；保留最后观察到的任务状态，不中断、不重发。继续等可再次调用 wait。
- unconfirmed 表示本次无法确认，reason 区分 connection-closed、connector-shutdown、native-read-timeout、native-connect-timeout、runtime-owner-changed 等。运行状态 notLoaded/unknown、目标轮次不存在、状态矛盾也不会被当作成功或停止。读取本身未能在期限内确认时返回 unconfirmed，而不是用旧快照断言当前状态。observedAt 保留最后成功读取的时间，没有成功读取则为 null。

等待固定使用实际运行连接，不在断线后重新启动/恢复任务。事件驱动唤醒并合并 250ms 内的流式通知，空闲每两秒回读原生状态；Desktop 重复的相同 revision 快照不会唤醒等待者，避免多个读取者互相触发回读；不依赖有限事件缓冲的连续 cursor。每个 wait 独立等待，不持有共享锁跨越网络读操作或等待。

MCP notifications/cancelled 可取消对应 wait，返回 unconfirmed / wait-cancelled（若客户端仍接受响应），不会发送 turn/interrupt。当前无会话 MCP 入口要求并发等待使用唯一 JSON-RPC request ID，重复活跃 ID 会被拒绝。客户端关闭 HTTP/stdio 连接后释放等待；Connector shutdown 通知等待者退出。连接已经关闭时无法向原调用方交付响应，调用方应把结果视为未知并重新读取任务。客户端、Tunnel 或反向代理可能有更短的请求期限；本机 stdio 转发支持五分钟等待，但不能保证外部入口接受同样时长。

这是有界同步等待，不是持久化监控订阅、定时任务或后台推送。普通 Chat 结束当前回复后不会继续执行等待。

所有内置 ACP 与 Custom ACP 自动进入相同的 `agent_*` 和 `agent_wait` 路径，不提供品牌化工具。`agents` 返回 displayName、integration、installed、available、status、version、discoveryError 和 descriptor；静态 `agent_capabilities` 不承诺会话能力，传 taskId 才读取 initialize 及 session 协商快照。完整启动和支持边界见[内置 ACP 矩阵](agents.md#内置-acp-支持矩阵)。

## Review 事实视图与完整性

`overview/tree/search/read/git` 默认按宿主权限读取。显式传入 `view: "review"` 时，以真实工作区路径为边界，排除越界路径及 Connector 自身运行状态目录；不按文件名或内容猜测敏感性，不隐藏提交消息。正常 root 内符号链接仍可读取，Git diff 核对重命名两侧是否位于读取边界内。

`data.coverage` 说明分页、筛选和跳过原因；`read.nextLine` 非空代表尚未读完，即使 `truncated` 为 false。搜索的 `matchesComplete` 表示单文件片段完整性，超出 5 处命中或 500 字符片段时用 `read` 继续读取。tree/search 续页可携带上一页 `listingHash` 作为 `expectedHash`；read 使用 `sha256`。哈希变化时重新分页。列表哈希只验证列表，不能验证搜索期间各文件内容。

`source` 包括观察窗口、HEAD 和 Git 状态文本哈希。`statusHash` 不是全仓库内容哈希；同为 M 的文件可以改变而不改变该值。`changedDuringRead` 仅表示已观察到的 Git 状态变化，不能证明全仓库不变。一致性为 best-effort，文件读取另有正文哈希及前后元数据核对。非 Git 目录与 Git 查询失败分别处理，不在失败后扩大遍历范围。

```json
{"project":"<workspace>","path":"src/main.rs","view":"review","startLine":1,"lineCount":80}
```

Review 不改变原生 fs/command/process 权限，不是用户隔离或完整 DLP。调用方收到的内容会进入其服务；文件名筛选不能保证普通源码不含秘密。

## 任务上下文

```json
{"agent":"codex","taskId":"<existing-task-id>","view":"handoff"}
```

`agent_context` 默认 view=review；可传 `turnId` 和 `project`。省略轮次时选择最新可确认轮次；显式轮次只在有界窗口内查找，查不到明确失败，不切换轮次。任务 cwd 优先，显式项目不一致返回 `TASK_WORKSPACE_MISMATCH`。无 cwd 的显式项目关联标为 caller-unconfirmed。

结果按 runtime、workspace、evidence、claims、coverage、readNext 分组；Handoff 另给可再生 Markdown。命令退出码仅证明该命令退出结果，Agent 文本不证明测试通过，回执完成不证明任务完成。工作区变化不归因给该任务，也不能用历史命令给当前代码背书。

读取不启动或恢复任务、不执行命令、不推理、不写 checkpoint。聚合采用 12 秒截止时间、最多 20 个条目及 100 个变化路径；摘录保留原文并限制至 1 KiB，输出预算 32 KiB。原生读取本身仍受已有协议分页/快照能力限制。缺失、超时、未采集回执/交互会明确标为 partial；非 Codex 驱动只投影已有任务记录，不补建进程或猜测结构化命令结果。原始存储保持不变。

入口策略按模块生效：agent_read 控制任务事实、codex_items 控制 Codex 条目、git 控制工作区；readNext 仅推荐允许的工具。四档预设及默认全部不变，已有显式 allowlist 不自动扩容。

### 本机诊断日志

连接运行日志和管理操作日志独立保存在状态目录的 `logs/YYYY-MM-DD.jsonl`，按 UTC 日期追加，删除连接不会删除日志。已有连接目录中的 `logs.json` 在加载时迁入独立日志目录。日志页面显示最近 200 条，磁盘历史不受这个显示上限影响，目前不自动删除历史日志。

管理写操作记录操作编号、方法、路由及开始和成功/失败状态；OAuth 接入记录端点和 HTTP 结果。请求正文、认证头、令牌与密钥不进入操作日志，连接运行日志对已知凭据脱敏。开始记录写入失败时，不执行管理写操作；完成记录写入失败会输出进程错误。日志用于诊断，不是防篡改审计；进程或磁盘故障可能只留下开始记录。
