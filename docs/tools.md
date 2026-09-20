# 工具与运行语义

返回[项目首页](../README.md)。

## 能力清单

当前入口包含 29 个 MCP 工具。原生方法及参数 Schema 从本机 Codex 二进制动态发现；
工具数量不保证不同 Codex 版本拥有相同的原生能力，各项功能取决于当前原生服务提供的能力。

| 工具 | 用途 |
| --- | --- |
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
领域工具顶层 timeoutMs 是 RPC 等待超时（写操作默认 24 小时、读操作默认 60 秒），与 params.timeoutMs 独立。

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
本机 outputs 目录保留完整结果，没有自动删除；需要清理时在宿主按文件日期处理。
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

关闭连接停止 Connector 的 Tunnel、转发与 App Server，包括 Connector 后台任务；界面保持可用，Desktop 自行管理其任务生命周期。
重启后旧提交中回执标记 unconfirmed，不自动重放；尚未提交的待审批请求继续保留。
事件缓冲最多保留最近 2000 条或 8 MiB，gap/reset 明确报告缺口；完整已落盘结果应从原生任务/回执读取。
Chat 页面不会因任务完成自动被唤醒，交互请求也需要调用方主动查询。


## 连接与持久输出

stdio 仅为传输代理，使用父进程传入的本机端口与随机凭据连接应用内的 Rust 核心。stdio 退出不会销毁应用内核心。工具调用产生本机活动日志，来源缺失时不推测 Chat ID。

`codex_events` 提供当前会话的 `persistedOutput.outputId`，通过 `control_output` 分页读取 JSONL；写入失败会返回持久化错误。事件文件、回执和输出保存在本机状态目录，不自动清理。

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
