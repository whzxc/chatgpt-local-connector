# 控制源支持矩阵

固定 8 个 curated preset 加 Custom；其他 MCP 客户端使用 Custom。预设只提供客户端类型与接入建议，不代表已完成官方客户端验收，也不创建独立协议或 Service。同一类型允许任意多个入口，以 ingress.id 区分。

| 控制源 | Remote MCP | 推荐 transport | 推荐认证 | CLC 当前程度 | 主要边界 / 官方资料 |
| --- | --- | --- | --- | --- | --- |
| ChatGPT | 支持 | openai-tunnel | openai | 现有路径可用 | 优先 Secure Tunnel；HTTPS 可用无认证，CLC 尚无 OAuth。Tunnel 与 ChatGPT 开发者模式权限分别管理。 [官方 1](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) / [官方 2](https://developers.openai.com/api/docs/guides/developer-mode) |
| Claude | 支持 | https | oauth | 有条件支持 | 组织静态请求头 Beta 可用 Authorization: Bearer；CLC 尚无 OAuth，个人 OAuth 接入不可用。无认证需主动选择。 [官方 1](https://claude.com/docs/connectors/building) / [官方 2](https://claude.com/docs/connectors/building/authentication) |
| Microsoft Copilot | 支持 | https | bearer | 有条件支持 | Copilot Studio 选择 API key → Header → Authorization，凭据填 Bearer <token>，需验证实际转发。尚无 OAuth 或其他密钥头。可发布到 Teams / Microsoft 365 Copilot。 [官方 1](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent) / [官方 2](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-create-new-server) / [官方 3](https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-fundamentals-publish-channels) |
| Notion | 支持 | https | bearer | 现有路径可用 | Custom Agents 支持请求头认证；使用 Authorization: Bearer，需有工作区权限及相应套餐。 [官方 1](https://www.notion.com/help/mcp-connections-for-custom-agents) |
| Slack | 支持 | https | oauth | 认证受限 | Slackbot 支持无认证、Slack 签名身份和 OAuth；CLC 目前仅无认证有交集，Bearer 不兼容。请用 CLI/Codex 配置，仅对允许公开的工具主动选择无认证。 [官方 1](https://docs.slack.dev/ai/slackbot-mcp-client/) |
| Cursor | 支持 | https | bearer | 现有路径可用 | 配置远程 MCP URL 与 Authorization: Bearer；客户端版本须支持转发配置的请求头，并验证实际请求。 [官方 1](https://cursor.com/docs/context/mcp) / [官方 2](https://prod.cursor.com/help/customization/mcp) |
| GitHub Copilot | 支持 | https | bearer | 现有路径可用 | 可承载在支持的 VS Code、JetBrains、CLI；按宿主配置 HTTP MCP 与 Authorization: Bearer。宿主能力及组织策略不同。 [官方 1](https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp-in-your-ide/extend-copilot-chat-with-mcp) / [官方 2](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers) |
| Raycast | 支持 | https | bearer | 现有路径可用 | AI Chat、Quick AI、AI Commands 使用 HTTP MCP；HTTP Headers 填 Authorization: Bearer，需相应 Raycast AI 套餐。 [官方 1](https://manual.raycast.com/ai/model-context-protocol) |

所有 HTTPS 路径使用 Streamable HTTP；CLC 不实现旧式独立 SSE transport。Microsoft Copilot 指 Copilot Studio 接入，可发布到 Teams / Microsoft 365 Copilot；Teams 不作为独立预设。GitHub Copilot 是另一个控制源，VS Code/JetBrains/CLI 是它的宿主。

## 认证交集

Notion、Cursor、GitHub Copilot、Raycast 可通过 `Authorization: Bearer <token>` 复用 generic HTTPS。Claude 组织管理员静态请求头 Beta 也可走该路径；这不等于个人账号 OAuth 已支持。Copilot Studio 的 API-key Header 可配置为 Authorization，完整值为 `Bearer <token>`；这是基于官方 Header 机制的兼容推断，必须在实际租户验证转发，没有平台端验收不能宣称已接通。

Slackbot 官方支持 none、Slack identity、DCR 和 manual OAuth；文档的 custom headers 用于身份查询，不是 MCP 请求。当前只有 none 与 CLC 相交。不要把 bearer 当作 Slackbot 已支持，也不要自动降级为无认证。UI 要求主动选择；长期敏感工具接入仍需通用 OAuth 或可信认证网关。ChatGPT HTTPS 的 OAuth/mixed auth 同样尚未实现，默认使用 Secure Tunnel。

CLC 仅接受 `openai` / `none` / `bearer`。不实现任意静态密钥头、query token、OAuth/DCR、Slack 签名身份验证。现有 Authorization Header 已覆盖多个控制源，因此没有新增 static-header abstraction 的必要。none 会让网络可达方访问允许的工具；只对明确允许公开的工具选择它。认证与 toolPolicy 以入口隔离，任务命名空间仍共享。

## CLI / Codex

运行 `cli ingress presets`（离线）发现 ID、推荐配置、官方认证能力和当前边界。`cli help` 同样包含预设，`ingress list` 和 onboarding 带每项 preset metadata。`supportedAuth` 是客户端与 CLC 的交集；`recommendedAuth` 可为尚未实现的 OAuth，`httpsAuth` 是 UI 的安全初始值，并非已接通证明。Custom 不验证品牌白名单。

自然语言请求映射：

- “加一个 Cursor” → cursor + https + bearer。
- “再加公司 Notion，使用 bearer” → 新入口，notion + https + bearer，name 自定义为公司 Notion；不更新已有入口。
- “给 Raycast 配 HTTPS” → raycast + https + bearer。
- “Copilot Studio 缺什么？” → microsoft-copilot，先说明 API-key Header 兼容条件；需要 OAuth 时当前缺服务端授权能力。

省略 name 时自动使用 Notion、Notion 2、Notion 3 等未占用名称；自定义 name 原样保留。保存或 ready 不等于接入完成：按 [Codex 配置](codex-setup.md) 从实际客户端完成 connector_verify，再按需要做无害任务验收。

## 工具列表

连接编辑弹窗末尾提供 **工具列表：全部 / 常用 / 只读 / 自定义**。新增连接默认全部，保存为 `"all"`，随注册表包含新工具；其它模式保存为明确的 `{ "allowlist": [...] }` 快照。一级表单显示当前工具数量，点击“查看全部”打开二级弹窗。预设在其中展示已选工具；自定义支持组选择与逐项选择，“完成”将选择带回连接表单，“取消”放弃本次选择。选择操作时会保留必需的回执、状态、等待和结果读取依赖，不自动开启可选写操作。编辑器生成的受限策略始终保留 `connector_verify`：它只写连接验证记录，是只读模式的连接维护例外。

常用包含项目事实查询和完整的 Agent/Codex 创建、续接、中断、交互、进度、回执及结果链路，省略通用主机写入、命令和任意原生 RPC。只读包含项目/代码读取、固定只读 Git、Agent/Codex 能力、任务列表、状态、历史、事件、待处理交互、等待、Schema 与输出读取；排除两个 `*_request`（approve/bypass/reject 会改变任务）、create/send/interrupt/respond，以及混合入口 `fs`、`command`、`process`、`mcp`、`codex_call`、`codex_thread`。`codex_query` 透传的原生参数包括插件/应用 `forceRefetch` 和技能 `forceReload`，`codex_account` 的账户读取允许刷新凭据，均保守排除。

这是同时作用于 `tools/list` 和 `tools/call` 的顶层 MCP 工具暴露策略，不细分工具内部 action/method，不隔离任务所有权，也不代表设备的 OS 沙箱只读；任务执行仍使用其自身权限。经过本机认证的管理目录始终返回完整注册表。

既有 allowlist 仅在完整集合与预设一致时回显对应预设，否则显示自定义。打开弹窗或仅保存其它字段不会改变它；明确修改工具选择才会归一化依赖、去重并按注册表排序。新工具不自动加入已保存的 allowlist，重新选择预设可采用当前定义。保存只重连当前连接；策略修改保留验证身份，公网 URL 或认证改变仍遵循原身份重置规则。客户端可能缓存工具列表，保存后需在客户端刷新 MCP 工具或重新连接；CLC 不保证客户端自动刷新。
