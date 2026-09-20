# 接入 ChatGPT：官方 Tunnel 与自备 HTTPS MCP

返回[项目首页](../README.md)。普通使用者在原生应用的设置页完成本机配置和连接启停，缺失的连接组件由应用自动准备，无须手工创建 Tunnel profile。

## 自备 HTTPS MCP

在设置页选择「自备 HTTPS MCP」。填写公网 HTTPS URL（路径固定为 `/mcp`）、本机监听 IP 和端口，默认无需认证，也可选择访问密钥。无需官方 Tunnel ID 或 Tunnel Client；仍需当前 ChatGPT 账号提供自定义 MCP 和 API key 认证入口。

1. 同机代理使用默认 `127.0.0.1:8787`；代理在另一台设备时，填写 Connector 的局域网 IP，并允许代理访问该端口。
2. 自行配置公网域名、有效 TLS 证书和反向代理，将公网 `/mcp` 转发至 `http://监听IP:端口/mcp`。保留 `Authorization` 请求头，`Host` 使用公网域名或实际监听 IP:端口，支持 JSON POST 与长请求，代理超时建议至少 180 秒。
3. 保存并开启连接，在 ChatGPT 添加公网 MCP URL，与应用保持一致：访问密钥模式选择 API key，以 `Authorization: Bearer <访问密钥>` 发送凭据；无需认证模式选择 No authentication。无需认证时任何能访问该地址的客户端都能调用工具，可由自备代理限制访问。此模式不提供 OAuth。
4. 从接入引导复制验证消息并发起工具调用，确认公网链路。本机监听 ready 只代表端口已开启。

原生 HTTP 入口仅提供 `/mcp`，与桌面管理 API、内部随机凭据隔离。使用无会话 Streamable HTTP，POST 返回 JSON，通知返回 202；不提供独立 SSE GET 流。HTTPS 由用户的代理终止，项目不提供托管中继、证书或公网入口。

关闭连接后才能更改连接方式、监听参数、认证方式或访问密钥。密钥留空保存保留原值；生成新密钥后必须保存，再更新 ChatGPT。改变接入身份或地址会清除此前的连接验证记录。关闭连接或退出应用停止监听。

## 官方 Tunnel：需要你提供的信息

按 [Secure MCP Tunnel 官方指南](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)取得自己的 Tunnel ID、runtime API Key，并确认目标 ChatGPT workspace 的关联及访问权限。管理页面负责保存本机配置和运行官方 Tunnel Client；不提供 Tunnel 服务端，也不代办身份申请。

API Key 默认以敏感信息形式展示，设置页可通过显示密钥按钮按需回读；留空保存会保留已有密钥。不要将密钥写进命令参数、源码、截图或聊天。原生应用通过本机 IPC 管理连接；HTTP 适配使用独立的本机随机凭据，Runtime API Key 只用于 Tunnel 连接。

## 首次接入步骤

已安装并登录 Codex Desktop 的用户，仍需完成以下两项外部配置；它们不会由 Codex 登录自动创建。

1. **取得 Tunnel 凭据与权限。** 在 [Platform Tunnel 设置](https://platform.openai.com/settings/organization/tunnels)创建通道，或向管理员取得 Tunnel ID 和 runtime API Key。创建或编辑需 Tunnels Read + Manage；运行客户端及在 ChatGPT 选用通道需 Read + Use。通道必须关联目标 ChatGPT 工作区，只有 Platform 组织关联不足以让它出现在该工作区。
2. **在 ChatGPT 添加连接。** 保持本机连接开启，在 ChatGPT「设置 → 安全与登录」开启开发者模式，再进入 Plugins → ＋，填写名称和描述，选择 Tunnel 并选取通道或填入 ID，创建连接并确认工具列表。开发者模式是独立的账号/工作区权限，没有入口时联系工作区管理员。已有连接无须重复添加。

随后新建对话，从工具菜单选用 Local Connector。可复制应用提供的 `connector_verify` 验证消息，或完成一次普通只读工具调用。点击「已添加」、本机状态查询及 Tunnel ready 都不会代替真实远程调用证据。验证记录只证明曾连通；确认任务执行可用还需读取真实任务终态。

应用负责安装官方 Tunnel Client、保存凭据和启停连接，不代办身份申请或网页端授权。用户无须配置公网域名或入站端口；本机需要能下载官方客户端并通过出站 HTTPS 访问 OpenAI。权限与操作依据 [官方 Tunnel 指南](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)和[插件接入指南](https://developers.openai.com/plugins/deploy/connect-chatgpt)。

## 连接与设备

```text
ChatGPT → 官方 Tunnel → 本机 Tunnel Client → 原生 stdio 适配 → Rust 核心 → Codex Desktop owner
                                      ↑
                         原生应用负责本机配置与启停
```

每台机器使用自己的目录、Codex 登录态和原生项目列表。一个实例服务一台机器；不提供设备选择或路由。不要让多台机器同时用同一 Tunnel 身份运行后端；切换设备时停止旧设备，再启动目标设备，并通过项目查询确认请求来源。

官方模式使用 Tunnel ID；HTTPS 模式使用公网 MCP URL。两者都不能使用内部管理端口或开发预览地址。

## 页面中的状态

首页显示 ChatGPT — Connector — Codex 链路状态，异常以 banner 呈现；记录页显示连接变更和请求结果。ChatGPT 验证状态代表已收到过真实入站请求，不意味着当前传输永远可用。

连接启动后，先让 ChatGPT 查询当前能力和本机项目，确认来源；任务是否完成需要继续读取真实轮次结果。Tunnel 的本机健康检查只证明本机服务已启动，不能证明出站 HTTPS、云端轮询或 ChatGPT 工具发现成功。工具元数据变化后在 ChatGPT 刷新连接。

## 代理与网络

在「设置 → 网络 → 代理」选择：

- **跟随系统**（默认）：读取 macOS 或 Windows 当前用户的手动 HTTP/HTTPS 代理；没有代理时直连。
- **不使用代理**：始终直连，不受启动应用时继承的代理环境变量影响。
- **自定义**：填写 HTTP/HTTPS 代理 URL，例如 `http://127.0.0.1:7890`，点击保存。不支持在 URL 中嵌入账号密码。

代理设置作用于 Tunnel 安装下载、初始化与运行，以及应用更新检查和下载，不更改系统或 Codex Desktop 的代理。修改设置不会中断正在运行的连接；重新连接后 Tunnel 使用新配置，新的下载请求直接使用新配置。

跟随系统暂不执行 PAC/WPAD，不导入系统例外列表，也不支持仅 SOCKS 的代理；检测到不支持的配置时提示改用自定义 HTTP/HTTPS 代理或直连。系统读取有超时限制，失败时不会悄悄切换直连。设置优先于 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` 及其小写变量。已有 `NO_PROXY` 和 `no_proxy` 规则合并保留并补入 loopback；本机健康检查、stdio 转发和桌面后台通信始终直连。日志中的代理来源提示不包含地址或认证信息。

HTTPS 接入的公网反向代理及本机 MCP 入站监听不受这个出站代理设置影响。

浏览器能打开 ChatGPT 不代表 Tunnel 已能访问 OpenAI。若创建插件或刷新工具列表失败，检查连接日志中是否有 DNS、连接超时、TLS 或代理错误，再从 ChatGPT 发起一次 `connector_verify` 验证。仅显示本机连接已启动或存在历史验证记录，都不能代替这次远程调用。

## 停止与排障

点击「关闭连接」会停止 Tunnel、stdio 适配及 Connector 的辅助进程。Desktop 自己执行的任务继续运行，未确认的外部请求保留回执。需要撤回远程接入时，在 ChatGPT 中移除对应连接。

无法发现工具时，先检查应用中的 Tunnel 状态和记录；本机程序未找到时安装程序或指定其完整路径。使用前应在 Codex Desktop 中完成登录；连接异常时检查 Desktop 是否可用。无法调用某个原生方法时查询 `codex_schema`，核对当前二进制是否提供该方法。

写调用超时后使用原 `requestId` 回读；超时不等于取消，不换 ID 自动重试写入。连接日志会脱敏，原始任务输出不保证脱敏；分享前自行检查。
