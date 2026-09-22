# 接入 ChatGPT：OpenAI Tunnel 与 HTTPS MCP

多个入口可以同时运行，共享同一个 Core。本文中的设置页操作针对默认入口；新增入口及独立认证请用 `cli ingress`。Cloudflare Fixed 的 `httpsPort` 默认 8787，多个 Fixed 入口必须选择不同端口并同步服务端路由。

[English](../tunnel.md) | **简体中文**

> 本文对应英文版本；若有差异，以[英文版本](../tunnel.md)为准。

返回[项目首页](../../README.zh-CN.md)。推荐先复制首页消息，让本机 Codex 按[配置与排障指南](codex-setup.md)完成接入，默认使用官方 Tunnel。以下为手动操作说明。普通使用者在原生应用的设置页完成本机配置和连接启停，缺失的连接组件由应用自动准备，无须手工创建 Tunnel profile。

## HTTPS MCP

在「设置 → 连接」选择 HTTPS MCP，再选择接入方式。统一使用无需认证，仍需当前 ChatGPT 账号提供自定义 MCP 入口。本项目不提供托管中继。

| 接入方式 | 必填内容 | 应用处理 | 使用边界 |
| --- | --- | --- | --- |
| Cloudflare · 临时域名 | 无 | 下载 cloudflared、开启 Quick Tunnel、获取公网地址 | 免账号和域名；临时地址可能随重连变化，适合试用，不保证可用性 |
| Cloudflare · 固定域名 | Tunnel Token、固定域名 | 下载 cloudflared、运行正式 Tunnel，固定监听 `127.0.0.1:8787` | 用户的 Cloudflare 账号需要已接入的域名，并配置公开路由 |
| ngrok | 账号的 Authtoken | 下载 ngrok、启动隧道、获取账号分配的公网地址 | 需要 ngrok 账号，受该账号的流量、请求和并发限制 |
| Pinggy | 临时模式无需凭据；固定模式需 Token 和已绑定域名 | 下载 Pinggy CLI，启动和停止本入口隧道 | 固定域名必须事先绑定到 Token |
| LocalXpose | Access Token 和已预留域名 | 下载 loclx，启动隧道并获取 HTTPS 地址 | 仅固定域名；域名必须事先准备好 |
| 自定义域名 | 公网 MCP URL | 启动本机 MCP 监听 | 用户准备域名、有效 TLS 证书和反向代理 |

### Cloudflare 临时域名 / ngrok

1. 选择 Cloudflare，或选择 ngrok 并通过「获取令牌」取得自己的 Authtoken。
2. 点击「保存并重连」。应用自动准备官方组件，使用独立的随机 loopback 端口和临时配置运行隧道，不改动已有 cloudflared/ngrok 配置。
3. 从首页进入「接入引导」，点击「打开 Plugins」并复制接入地址，在 ChatGPT 添加自定义 MCP。认证选择 No authentication。也可提供已登录网页，让 Codex 读取 `cli onboarding` 并代填。
4. 在「发送验证消息」步骤复制并发送消息；CLC 收到匹配验证码后自动确认。地址变化后，在 ChatGPT 使用新地址重新创建连接并移除旧连接；旧地址对应的验证记录不会用于新地址。

Cloudflare 临时域名使用 [Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/)，仅用于试用，不提供独立 SSE 流支持；当前原生 MCP 使用 JSON POST，不依赖 SSE。cloudflared 需要连通 Cloudflare 的出站 7844 端口，不能假设普通 HTTP 代理能代理其数据通道。长期使用固定地址可选择 Cloudflare 固定域名、ngrok 或自定义域名。ngrok 的账号、域名和额度由用户在 [ngrok](https://ngrok.com/download) 管理，应用不会创建付费资源。

连接组件从官方 HTTPS 地址下载并缓存在本机。Cloudflare 下载包校验官方 SHA256；ngrok 从官方下载站获取，缓存文件使用本地 SHA256 检测损坏。Authtoken 只用于 ngrok 客户端，不传给 ChatGPT，也不写入命令参数或常规状态响应。关闭连接、启动失败或退出应用会停止隧道和 MCP 监听，并清理临时运行配置。

ngrok 提供「临时域名」和「固定域名」两种模式，均只需 Authtoken。临时域名模式自动获取 ngrok 分配的地址，重启后地址可能变化，需同步更新 MCP 客户端。固定域名模式由用户填写已在 ngrok 账号中配置好的域名（也可填写不含路径的 HTTPS 地址），应用使用该域名启动隧道并提供 `/mcp` 地址；不会查询账号域名、申请域名或创建付费资源。固定域名不可用时直接报错，不回退到临时域名。

### Pinggy / LocalXpose

两者复用现有的保存、启动、停止和重新获取流程。官方客户端仅在首次使用时下载，缓存在应用包外，不要求安装 Node.js。Pinggy 发布文件校验官方 SHA256；LocalXpose 使用官方 HTTPS 下载和本地缓存哈希校验。

Pinggy 临时模式无需 Token；固定模式需要已绑定持久域名的 Token。LocalXpose 需要 Access Token 和已有的预留域名或自定义域名。临时模式会把首次 MCP 请求从 `/mcp` 重定向到其他路径，因此暂不开放。应用不购买套餐、不预留域名，固定域名不匹配时直接失败，不选择其他地址。

服务商凭据与入口的 MCP 认证独立。Pinggy 通过私有配置文件接收 Token，daemon 状态归 CLC 管理；LocalXpose 通过子进程环境变量接收 Token。两者都不在命令参数或状态响应中暴露凭据。停止 Pinggy 入口只停止该入口的隧道，不停止用户的 Pinggy daemon 或其他隧道。
