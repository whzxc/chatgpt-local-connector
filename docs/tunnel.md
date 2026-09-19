# 通过 Secure MCP Tunnel 接入

返回[项目首页](../README.md)。普通使用者在原生应用完成本机配置、客户端安装和连接启停，无须手工创建 Tunnel profile。

## 需要你提供的信息

按 [Secure MCP Tunnel 官方指南](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)取得自己的 Tunnel ID、runtime API Key，并确认目标 ChatGPT workspace 的关联及访问权限。管理页面负责保存本机配置和运行官方 Tunnel Client；不提供 Tunnel 服务端，也不代办身份申请。

API Key 默认以敏感信息形式展示，设置页可通过眼睛按钮按需回读；留空保存会保留已有密钥。不要将密钥写进命令参数、源码、截图或聊天。原生应用通过本机 IPC 管理连接；HTTP 适配使用独立的本机随机凭据，Runtime API Key 只用于 Tunnel 连接。

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

ChatGPT 使用 Tunnel ID 接入，不要将本机 HTTP 适配端口或开发预览地址填作 MCP 服务端点。

## 页面中的状态

首页显示 ChatGPT — Connector — Codex 链路状态，异常以 banner 呈现；记录页显示连接变更和请求结果。ChatGPT 验证状态代表已收到过真实入站请求，不意味着当前传输永远可用。

连接成功后，先让 ChatGPT 查询当前能力和本机项目，确认来源；任务是否完成需要继续读取真实轮次结果。Tunnel 健康只证明传输就绪，不等同于任务执行成功。工具元数据变化后在 ChatGPT 刷新连接。

## 停止与排障

点击「关闭连接」会停止 Tunnel、stdio 适配及 Connector 的辅助进程。Desktop 自己执行的任务继续运行，未确认的外部请求保留回执。需要撤回远程接入时，在 ChatGPT 中移除对应连接。

无法发现工具时，先检查应用中的 Tunnel 状态和记录；本机程序未找到时安装程序或指定其完整路径。Codex 未登录时点击账号卡片中的「登录 Codex」，在打开的 Desktop 完成登录后返回检查状态。无法调用某个原生方法时查询 `codex_schema`，核对当前二进制是否提供该方法。

写调用超时后使用原 `requestId` 回读；超时不等于取消，不换 ID 自动重试写入。连接日志会脱敏，原始任务输出不保证脱敏；分享前自行检查。
