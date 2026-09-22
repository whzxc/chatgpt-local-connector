# 用 Codex 配置 Local Connector

[English](../codex-setup.md) | **简体中文**

在目标电脑使用应用自带 CLI，先读 `cli help`、`cli status`、`cli ingress list`。ChatGPT 默认推荐 OpenAI Secure Tunnel；保留已有入口，新增入口不覆盖默认连接。完整配置任务包含真实控制源入站与无害任务终态验证。

macOS 通常使用 `/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop cli help`；Windows 使用安装目录中的 `local-connector-desktop.exe cli help`。无需 Node/npm。help、guide、ingress presets 离线可用，其余命令需要同版本应用正在运行。成功输出 `{schemaVersion:1,ok:true,result:...}`，失败输出 `{schemaVersion:1,ok:false,error:{code,message}}`；退出码分别为 0、1。诊断成功不等于每个入口已经 ready。

## CLI 配置与生命周期

- `ingress presets`：离线查看 8 个精选控制源、推荐 transport/auth 与支持边界。
- `ingress list`：每入口脱敏配置、URL、运行状态、日志和验证。
- `ingress add --stdin`：JSON 新建入口。
- `ingress update <id> --stdin`：局部更新，config 按字段合并；先停止目标入口。
- `ingress remove <id>`：停止并删除目标入口，保留任务和其他入口。
- `ingress start <id>` / `ingress stop <id>`：独立启停。
- `ingress start-all` / `ingress stop-all`：批量操作；start 跳过 enabled=false，每个结果独立报告。
- `ingress token rotate <id>`：停止后轮换 bearer；新密钥仅此次 stdout 返回，重置该入口验证。
- `ingress doctor <id>`：查看该入口配置、状态和接入指引。
- `ingress verify <id> --fresh`：只重置该入口验证码；不带 --fresh 回读现有验证。
- `status` / `doctor` / `onboarding`：全局摘要及多入口 JSON。
- `network --stdin`：设置全局 proxyMode/proxyUrl；现有进程需重连生效。
- `configure --stdin`、`connect`、`disconnect`：默认入口的简化操作，不影响其他入口。
- `verify` 读取全部入口；`verify --fresh` 明确重置全部入口。

新增必填 controlSource、transport、auth、config。name 默认预设名称或未知标签，重名自动追加数字，enabled 默认 true，toolPolicy 默认 all；id 可省略自动生成，不能修改。id 只允许 ASCII 字母、数字、连字符、下划线。controlSource 是标签，不是已认证用户。

transport 为 `openai-tunnel` 时 auth 为 `openai`；HTTPS 支持 `none` 或 `bearer`。bearerToken 至少 32 个可打印 ASCII 字符，每入口独立，通过 stdin 输入。toolPolicy 为 `"all"` 或 `{"allowlist":["connector_verify","agents","agent_create","agent_request","agent_read","agent_send","agent_wait","control_output"]}`；tools/list 和 tools/call 同时执行限制。允许任务工具即允许访问共享任务，不按来源隔离；也不细分单个工具中的原生方法权限。

完整字段与 provider 选项见 `cli help` 和[英文配置示例](../codex-setup.md#configuration-examples)。凭据只能来自授权的安全本机来源并经 stdin 传递，不放在命令参数、shell 字面量、聊天、日志或截图中。轮换前先准备受保护的本机输出文件，例如设置 `umask 077` 并重定向 stdout，不让模型读取密钥输出。使用客户端支持的安全凭据输入完成交付。不要拿管理 API token、OpenAI key 或 provider token 代替 ingress bearer。

## 配置示例

ChatGPT：`controlSource=chatgpt`、`transport=openai-tunnel`、`auth=openai`；config 中填写官方 tunnelId 和 apiKey。保留官方身份申请、工作区关联及客户端接入流程。

Notion：创建独立的 `transport=https`、`auth=bearer`、`httpsProvider=ngrok` 入口。提供 ngrokAuthtoken 和独立 bearerToken；启动后读取 ngrok 返回的可用 MCP URL。ngrok 自动使用独立 loopback 端口。

Cursor：创建独立的 `transport=https`、`auth=bearer`、`httpsProvider=cloudflare`、`cloudflareMode=named` 入口。配置 cloudflareToken、httpsUrl、httpsPort；例如 httpsPort=8788，对应 Cloudflare 服务端路由 `http://127.0.0.1:8788`。每个 Fixed 入口使用不同端口和 Tunnel 身份，不将不同认证上下文用同一身份做负载均衡。

Cloudflare token 模式路由保存在服务端，CLC 不用 token 改写路由。Quick 模式自动分配临时地址，仅用于试用；长期入口优先固定地址加认证。Custom Domain 使用 httpsHost/httpsPort 接收自管 TLS 反向代理请求。不要公开桌面管理端口。

先查看[控制源矩阵](control-sources.md)。Slackbot 不支持静态 Bearer，目前仅无认证与 CLC 相交；不要自动降级。Claude 组织静态请求头 Beta 与 Copilot Studio API-key Header 是有条件路径，不代表已实现 OAuth。CLI help、ingress presets（离线）、list 和 onboarding 均提供 preset metadata。不能把本机探针成功报告为官方客户端已接入。

## 完整执行流程

“给 Notion 加一个 ngrok + bearer 入口”：检查现有入口 → 准备安全本机凭据 → stdin 创建 → 启动该 id → doctor 回读 ready 和公网 URL → 在支持的客户端安全配置 URL 与 bearer → 独立验证。账号额度、固定域名权限或 provider 并发限制必须按实际错误说明，不自动覆盖其他入口或悄悄降级为临时地址。

执行 `ingress verify <id> --fresh`，从真实控制源调用 connector_verify，回读同一入口的 challengeVerifiedAt、ingressId、controlSource。普通调用只更新历史 verifiedAt；本机直接调用只能证明本机链路。恢复中断的验证用原验证码，不重置其他入口。

完整验收继续创建无害任务：不使用工具、不读写文件，只回复 CLC_ONBOARDING_OK。每次新操作用唯一 requestId，回读持久回执、taskId/threadId、turnId 和 agent_wait/codex_wait 终态输出。未知状态回读原 requestId，不重复提交。允许的另一入口能读取、续接、等待同一 taskId；停止创建入口不终止 Agent。

用户提供已登录浏览器时，实际使用可见受支持界面配置和验证；未指定浏览器优先 in-app Browser。只在登录、身份授权、验证码、权限或无法可靠操作时交还精确步骤；不提取 Cookie、不调用私有接口。

## 边界

none 允许所有可达调用方调用已授权工具。bearer 只证明持有凭据；task namespace 全局共享，没有用户/组织 RBAC。本轮不实现 OAuth/DCR、Slack Identity、Notion API 或消息编排。listener 不提前截断 300 秒等待；外部代理和服务商限制需另行核对。重复短等待可沿用 taskId，超时不停止任务。

入口失败只影响自身。先读该入口错误与脱敏日志。停止入口只释放其进程、listener 和临时资源；退出 Core 才关闭 AgentHost/Control，Desktop-owned 任务沿用 Desktop 生命周期。UI 仅展示入口数量和每入口一行状态及基本动作，主要配置通过 CLI 完成。

## 监控任务

Codex 在 create/send 后从回执取得原 taskId/threadId、turnId，默认以 30 秒（建议 20–30 秒）一个 slice 调用 agent_wait/codex_wait。收到 timeout 后保留原 ID，将上一轮 snapshotHash 作为 expectedHash 再次 wait，直到 completed/failed/cancelled 或 interaction-required。timeout/取消 wait 只结束当前等待，不终止源任务，不应重新创建任务或重发 prompt；unconfirmed 不代表失败。expectedHash 只控制 changed 比较，不屏蔽终态或交互。

这是 bounded event-driven long poll；每个 slice 内仍由事件唤醒和真实 owner 状态复核驱动，不使用 read/sleep 高频轮询。ChatGPT、Notion、Slack 及其他 MCP Client 可能具有不同的外层工具超时，统一默认 timeoutMs=30000，不引入 per-client 配置。普通 Chat / 通用 MCP Client 不推荐默认阻塞数分钟。确认上游支持时仍可显式传 timeoutMs=300000；这是上限而非推荐值。OpenAI Tunnel stdio adapter 保留 330 秒转发预算，HTTPS MCP 没有更短的等待执行限时；CLC 无法延长外部客户端或代理的超时。
