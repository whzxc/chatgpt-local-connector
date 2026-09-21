# 用 Codex 配置和排查 Local Connector

[English](../codex-setup.md) | **简体中文**

> 本文对应英文版本；若有差异，以[英文版本](../codex-setup.md)为准。

本文是本机 Codex 的操作指南。默认使用 OpenAI Secure MCP Tunnel。完整安装配置请求默认包含真实 ChatGPT 工具调用和无害 Codex 任务验收。不要把用户引回整篇文档自行操作，只在需要登录、权限、凭据或必要选择时请用户介入。

## 入口与安装

必须使用能够在目标电脑执行命令的本机 Codex。云端任务不能直接配置用户电脑。先检查现有安装和运行状态，不重复安装，不覆盖已有可用配置。Desktop 任务接入支持 Apple Silicon macOS 和 Windows x64；Windows 需要安装 Microsoft Store 版 Codex Desktop。

未安装时，从 https://github.com/whzxc/chatgpt-local-connector/releases/latest 获取对应安装包和 SHA256SUMS.txt，核对同一版本的哈希；macOS 安装到 Applications，也可使用安装指南中的 Homebrew Cask；Windows 运行 x64 EXE 或 MSI 安装器。无需 Node、npm、Rust 或克隆源码。系统拦截按安装指南处理，不关闭系统整体安全机制。不要让用户从 DMG 内长期运行应用。

在 macOS 打开 Local Connector。默认安装位置的 CLI：

```sh
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli help
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli guide
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli doctor
```

以实际安装位置为准，应用包中的 `Contents/Info.plist` 的 `CFBundleExecutable` 指明可执行文件。Windows 可用安装目录中的 `local-connector-desktop.exe cli help`，通过重定向读取 JSON 输出。

`cli guide` 是当前二进制内嵌的匹配版本指南，优先于网页。若旧版本不提供 CLI，不反复尝试未知参数；核对发行包是否已提供该能力，再升级。网页文档不代表最新发行包已经包含所有命令。

## CLI 约定

命令为 `<应用可执行文件> cli <command>`，无需安装额外 CLI。所有命令输出单个 JSON 对象，允许显式添加 `--json`：

- 成功：`{"schemaVersion":1,"ok":true,"result":...}`，退出码 0。
- 失败：`{"schemaVersion":1,"ok":false,"error":{"code":"...","message":"..."}}`，退出码 1。
- `doctor` 检查执行成功不等于配置成功：读取 `result.checks`、`next` 和 `stage`。
- `APP_UNAVAILABLE`：先打开应用，再重试只读命令。`APP_VERSION_MISMATCH`：运行中的应用与 CLI 能力不匹配，核对版本。
- `help`、`guide` 无需后台；其他命令调用桌面应用持有的本机服务，隧道断开时仍可用。不直接编辑内部状态文件，不读出内部管理凭据。

| 命令 | 用途 |
| --- | --- |
| `status` | 脱敏配置、连接状态、Desktop IPC 状态和历史验证记录 |
| `doctor` | 按当前 Tunnel / HTTPS 和任务执行方检查配置、登录、传输及历史入站；返回下一步 |
| `onboarding` | 读取当前步骤、网页入口、名称/描述、连接方式及 URL/ID、验证消息和无害任务消息；不包含密钥，不代办网页安装 |
| `configure --stdin` | 从标准输入读取 JSON，仅接收 tunnelId、apiKey，切换并配置官方 Tunnel；HTTPS 配置使用桌面设置，不调用此命令覆盖 |
| `network --stdin` | 从标准输入读取 proxyMode、proxyUrl，复用应用的代理设置 |
| `connect` / `disconnect` | 通过现有后台启停连接；connect 会自动准备 Tunnel Client |
| `logs` | 读取经过应用脱敏的连接日志 |
| `verify --fresh` | 生成新的验证 code，清除当前配置的旧验证记录，开始本轮验收 |
| `verify` | 返回本轮 code、ChatGPT 验证消息、verifiedAt 和 challengeVerifiedAt，不发送远程调用 |

## 执行流程

1. 执行 `doctor` 和 `status`，确认是否已经配置、是否在运行、是否使用官方 Tunnel。已有 HTTPS 配置直接保留并继续；不要因为默认推荐而要求用户切换。完整任务接入不支持的平台应明确报告限制。
2. 缺少 Codex 或未登录时，打开官方安装或登录界面，用户完成登录后继续。默认保留用户的任务执行归属、审批和开机启动选择。
3. 仅在当前使用官方 Tunnel 且缺少资料时，打开 https://platform.openai.com/settings/organization/tunnels 。优先复用用户授权的安全本机资料，自行读取已有 Tunnel ID；确实缺少 runtime API Key 才请用户在应用直接填写。确认通道关联目标 ChatGPT 工作区并具备权限。Codex 登录不能替代这些信息。账号或管理员操作无法完成时明确说明缺什么，不反复重试。
4. 官方 Tunnel 仅在缺少安全本机凭据来源时，让用户直接在应用「设置 → 连接 → OpenAI Tunnel」输入凭据并保存；Codex 通过 doctor 发现已配置后继续。不要求用户把密钥贴进聊天，不在参数、日志或截图中显示密钥。需要程序配置且已有安全本机凭据来源时，通过 stdin 传递 JSON，不将密钥字面量写进 shell 命令。configure 省略字段保留原值，apiKey 空字符串也保留原值；修改 Tunnel ID 时必须使用与新通道匹配的密钥。连接开启期间修改配置会被拒绝，先确认确需更改再 disconnect。
5. 执行 connect，随后读取 doctor/status 等待就绪；connect 返回成功只代表启动请求已完成。下载或网络失败时读取 logs，依据实际错误处理。仅调整 Connector 自身代理；network 的 proxyMode 支持 system、direct、custom，custom 需要 HTTP/HTTPS proxyUrl。代理变更需要重新连接才能作用于已有 Tunnel，不修改系统代理。
6. 读取 onboarding（旧版本回退 status/verify），直接取得表单名称、描述、连接类型及 value。若用户已提供登录后的网页或浏览器环境，必须实际尝试浏览器 / GUI / Computer Use；优先用户指定的已登录标签页，其次 in-app Browser。按当前可见的文字或无障碍控件操作，不假定页面选择器、坐标或设置 deep link。没有浏览器控制能力时明确说明这一限制。
7. 在 ChatGPT 检查「设置 → 安全与登录 → Developer Mode」，进入 https://chatgpt.com/plugins ，创建入口可能显示 Add → Create MCP App 或 ＋。先检查并复用与当前 URL/ID 匹配的连接，不能只按名称判断；没有才创建。Tunnel 选择通道或自行填入 value；HTTPS 填入 value 并选 No authentication，不填管理端口或 Tunnel API Key。完成发现后确认 connector_verify 等工具存在；已有连接需要更新工具时打开详情选择 Refresh。临时地址变化需重新匹配，删除旧连接前遵循当前工具的确认要求。只有本人登录、身份/权限授权、验证码、账号工作区权限不足或无法可靠操作时暂停，指出当前页面与最少动作；遵守浏览器工具的安全确认规则，不绕过限制。
8. 新一轮验收执行 verify --fresh，保存 code；中断恢复直接 verify，勿重复刷新 code。在 ChatGPT 新建 Chat 对话（若默认进入 Work，先切换 Chat）、选用 Local Connector 并实际发送返回的 prompt。已有插件详情的 Try in chat 也可用于选用连接。自动轮询 verify，核对 code 未变、challengeVerifiedAt 出现，同时确认 ChatGPT 中该次工具调用成功；有界等待（例如两分钟）后读取 logs/doctor 排障。普通工具成功只更新 verifiedAt，不能替代验证码验收。新字段缺失时核对运行版本，旧版只能结合 ChatGPT 工具返回的匹配 code/received 和本轮入站记录确认。不能从本机直接调用工具来冒充 ChatGPT 入站；入站本身不认证调用方身份。
9. 完整配置请求继续通过同一 ChatGPT 连接发送 onboarding.executionPrompt，或要求创建任务：不调用工具、不读取或修改文件，只回复 CLC_ONBOARDING_OK。这一步只创建验收任务，不在真实项目实施改动。依据当前工具 schema 使用唯一 UUID requestId，读取持久化回执、原生 threadId/turnId 和终态及输出；遇到任务审批按用户授权与工具规则处理。请求状态未知时以原 requestId 回读，不换 ID 重放。没有完成此步骤必须报告“任务执行尚未验收”，不能以 accepted/running 作为完成。

## 自动化边界

CLC 的桌面引导在一页展示「开启开发者模式 → Create MCP App → 发送验证消息」。名称、描述、HTTPS 地址和验证消息支持分别复制；Tunnel 直接选择当前通道，Authentication 选择 No Authentication。运行中自动读取入站结果，支持重新验证。CLI 提供结构化接入资料，Codex 无需让用户转抄 URL、ID 或验证消息。应用本身不读取浏览器登录态，也不控制 ChatGPT 网页。

官方公开流程仍要求在 ChatGPT UI 开启 Developer Mode、创建连接、刷新工具和在对话选用连接。官方文档提供 Plugins 页面入口，但未提供可供 CLC 使用的创建/安装 API、参数预填协议或 Developer Mode 开关 deep link；Responses API 的 MCP 调用不会给 ChatGPT 账号安装插件。不要使用私有接口、Cookie/令牌提取或固定 DOM 自动化脚本。

已登录网页加上可用且获授权的浏览器工具，可以让 Codex 按可见界面完成导航、填写和点击，但不是稳定的全自动平台接口，也不保证所有账号零操作。入口名称可能因账号而异；缺失工作区权限、Tunnel 工作区关联、本人授权和安全挑战不能由本机程序绕过。没有可靠控件时只交回当前阻塞步骤，之后自动继续，不把后续整套流程交回用户。项目仍是本地开源工具，不提供云账号、托管服务、公共 relay 或 Public Plugin。

## 故障与恢复

换一个 Codex 对话也从 doctor/status 开始。现有配置和验证记录由应用保存，不依赖上次聊天记忆。之前验收中断且已有待验证 code 时直接继续 verify；仅开始新一轮验收才使用 --fresh。

根据具体证据分辨：应用未启动、资料缺失、登录失效、Desktop IPC 不可用、组件下载失败、Tunnel 网络错误、工作区权限不足、ChatGPT 未选用连接、任务提交结果未知。不要把全部错误都处理为重装或重置配置。自动执行已授权且有证据支持的修复；凭据失效、管理员权限和账号选择交给用户。操作后重新读取相关状态并复验。

最终向用户说明本机连接、ChatGPT 入站、任务执行分别是否验证成功，以及尚缺的具体动作。不输出密钥、完整私有日志或真实任务内容。

参考：[安装指南](installation.md)、[Tunnel 接入](tunnel.md)、[官方 Tunnel 文档](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)、[ChatGPT 接入文档](https://developers.openai.com/plugins/deploy/connect-chatgpt)。
