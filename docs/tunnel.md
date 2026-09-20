# 接入 ChatGPT：OpenAI Tunnel 与 HTTPS MCP

返回[项目首页](../README.md)。推荐先复制首页消息，让本机 Codex 按[配置与排障指南](codex-setup.md)完成接入，默认使用官方 Tunnel。以下为手动操作说明。普通使用者在原生应用的设置页完成本机配置和连接启停，缺失的连接组件由应用自动准备，无须手工创建 Tunnel profile。

## HTTPS MCP

在「设置 → 连接」选择 HTTPS MCP，再选择接入方式。统一使用无需认证，仍需当前 ChatGPT 账号提供自定义 MCP 入口。本项目不提供托管中继。

| 接入方式 | 必填内容 | 应用处理 | 使用边界 |
| --- | --- | --- | --- |
| Cloudflare · 临时体验 | 无 | 下载 cloudflared、开启 Quick Tunnel、获取公网地址 | 免账号和域名；临时地址可能随重连变化，适合试用，不保证可用性 |
| Cloudflare · 固定域名 | Tunnel Token、公网 MCP URL | 下载 cloudflared、运行正式 Tunnel，固定监听 `127.0.0.1:8787` | 用户的 Cloudflare 账号需要已接入的域名，并配置公开路由 |
| ngrok | 账号的 Authtoken | 下载 ngrok、启动隧道、获取账号分配的公网地址 | 需要 ngrok 账号，受该账号的流量、请求和并发限制 |
| 自定义域名 | 公网 MCP URL | 启动本机 MCP 监听 | 用户准备域名、有效 TLS 证书和反向代理 |

### Cloudflare 临时体验 / ngrok

1. 选择 Cloudflare，或选择 ngrok 并通过「获取令牌」取得自己的 Authtoken。
2. 点击「保存并连接」。应用自动准备官方组件，使用独立的随机 loopback 端口和临时配置运行隧道，不改动已有 cloudflared/ngrok 配置。
3. 复制页面显示的「ChatGPT 接入地址」，点击「前往 ChatGPT 添加连接」，在 ChatGPT 添加自定义 MCP。认证选择 No authentication。
4. 从接入引导复制验证消息并发起工具调用，确认公网链路。地址变化后，在 ChatGPT 使用新地址重新创建连接并移除旧连接；旧地址对应的验证记录不会用于新地址。

Cloudflare 临时体验使用 [Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/)，仅用于试用，不提供独立 SSE 流支持；当前原生 MCP 使用 JSON POST，不依赖 SSE。cloudflared 需要连通 Cloudflare 的出站 7844 端口，不能假设普通 HTTP 代理能代理其数据通道。长期使用固定地址可选择 Cloudflare 固定域名、ngrok 或自定义域名。ngrok 的账号、域名和额度由用户在 [ngrok](https://ngrok.com/download) 管理，应用不会创建付费资源。

连接组件从官方 HTTPS 地址下载并缓存在本机。Cloudflare 下载包校验官方 SHA256；ngrok 从官方下载站获取，缓存文件使用本地 SHA256 检测损坏。Authtoken 只用于 ngrok 客户端，不传给 ChatGPT，也不写入命令参数或常规状态响应。关闭连接、启动失败或退出应用会停止隧道和 MCP 监听，并清理临时运行配置。

### Cloudflare 固定域名

1. 在 [Cloudflare 控制台](https://dash.cloudflare.com/)的 Networking → Tunnels 创建 Tunnel，复制安装命令中的 Token（不要复制整个命令）。无需另行安装客户端或系统服务。
2. 在应用选择 Cloudflare → 固定域名，填写 Tunnel Token 和公网 MCP URL，例如 `https://connector.example.com/mcp`，点击「保存并连接」。
3. 在该 Tunnel 的 Routes 中添加 Published application，选择对应的域名，将 Service URL 设置为应用提供的 `http://127.0.0.1:8787`，路径可留空；请求的 `/mcp` 路径会保留。域名必须已接入该 Cloudflare 账号。
4. 将公网 MCP URL 添加到 ChatGPT，认证选择 No authentication，再发起验证调用。

正式 Tunnel 使用固定的本机地址；若 8787 端口被占用，应用会报错，不会静默换端口。Tunnel Token 通过私有临时文件交给 cloudflared，普通状态和日志不会显示它；留空保存保留已有 Token，更换 Token 或地址后需重新验证。应用只负责本机客户端，不创建或修改 Cloudflare 的域名、路由及账号资源。隧道就绪不代表公开路由已配置或 ChatGPT 已连通。操作细节见 [Cloudflare 正式 Tunnel 指南](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel/)。

### 自定义域名

填写公网 `https://你的域名/mcp`，默认监听 `127.0.0.1:8787`。同机反向代理可直接复制「代理目标」；代理位于其他设备时，在「高级设置」中填写本机局域网 IP 和端口，并允许代理访问该端口。

将公网 `/mcp` 转发至代理目标。`Host` 使用公网域名或实际监听 IP:端口，支持 JSON POST 与长请求，代理超时建议至少 180 秒。保存并从首页开启连接后，在 ChatGPT 添加对应 MCP URL，认证选择 No authentication。

以上 HTTPS 接入方式均无需认证，任何能访问公网地址的客户端都能调用工具；自备反向代理可自行限制访问。当前原生入口不提供访问密钥或 OAuth 认证。

原生 HTTP 入口仅提供 `/mcp`，与桌面管理 API、内部随机凭据隔离。使用无会话 Streamable HTTP，POST 返回 JSON，通知返回 202；不提供独立 SSE GET 流。HTTPS 由所选服务商或用户的代理终止。

关闭连接后才能更改配置。服务商 Token 留空保存保留原值。隧道就绪仅表示本机进程和服务商连接状态，成功的 ChatGPT 工具调用才确认公网链路。

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

每台机器使用自己的目录、Codex 登录态和原生项目列表。一个实例服务一台机器；不提供设备选择或路由。多台设备可分别使用独立的官方 Tunnel 或 HTTPS MCP 地址，在 ChatGPT 创建名称可区分的连接，并在对话中选用对应设备。不要让多台机器同时用同一 Tunnel 身份运行后端；切换设备时停止旧设备，再启动目标设备，并通过项目查询确认请求来源。

官方模式使用 Tunnel ID；HTTPS 模式使用公网 MCP URL。两者都不能使用内部管理端口或开发预览地址。

## 页面中的状态

首页显示 ChatGPT — Connector — Codex 链路状态，异常以 banner 呈现；记录页显示连接变更和请求结果。ChatGPT 验证状态代表已收到过真实入站请求，不意味着当前传输永远可用。

连接启动后，先让 ChatGPT 查询当前能力和本机项目，确认来源；任务是否完成需要继续读取真实轮次结果。Tunnel 的本机健康检查只证明本机服务已启动，不能证明出站 HTTPS、云端轮询或 ChatGPT 工具发现成功。工具元数据变化后在 ChatGPT 刷新连接。

## 代理与网络

在「设置 → 网络 → 代理」选择：

- **系统代理**（默认）：读取 macOS 或 Windows 当前用户的手动 HTTP/HTTPS 代理；没有代理时直连。
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
