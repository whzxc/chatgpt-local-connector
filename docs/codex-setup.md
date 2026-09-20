# 用 Codex 配置和排查 Local Connector

本文是本机 Codex 的操作指南。默认使用 OpenAI Secure MCP Tunnel。目标是完成真实远程调用验收；任务执行能力单独验收。不要把用户引回整篇文档自行操作，只在需要登录、权限、凭据或必要选择时请用户介入。

## 入口与安装

必须使用能够在目标电脑执行命令的本机 Codex。云端任务不能直接配置用户电脑。先检查现有安装和运行状态，不重复安装，不覆盖已有可用配置。Desktop 任务接入支持 Apple Silicon macOS 和 Windows x64；Windows 需要安装 Microsoft Store 版 Codex Desktop。

未安装时，从 https://github.com/whzxc/chatgpt-local-connector/releases/latest 获取对应安装包和 SHA256SUMS.txt，核对同一版本的哈希后安装到 Applications。也可使用安装指南中的 Homebrew Cask。无需 Node、npm、Rust 或克隆源码。系统拦截按安装指南处理，不关闭系统整体安全机制。不要让用户从 DMG 内长期运行应用。

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
| `doctor` | 配置缺项、Codex 登录、Desktop、Tunnel 和入站验证检查；返回下一步 |
| `configure --stdin` | 从标准输入读取 JSON，仅接收 tunnelId、apiKey，切换并配置官方 Tunnel |
| `network --stdin` | 从标准输入读取 proxyMode、proxyUrl，复用应用的代理设置 |
| `connect` / `disconnect` | 通过现有后台启停连接；connect 会自动准备 Tunnel Client |
| `logs` | 读取经过应用脱敏的连接日志 |
| `verify --fresh` | 生成新的验证 code，清除当前配置的旧验证记录，开始本轮验收 |
| `verify` | 返回本轮 code、ChatGPT 验证消息和 verifiedAt，不发送远程调用 |

## 执行流程

1. 执行 `doctor` 和 `status`，确认是否已经配置、是否在运行、是否使用官方 Tunnel。已有 HTTPS 配置时请用户决定是否切换；不要因为默认推荐而覆盖。完整任务接入不支持的平台应明确报告限制。
2. 缺少 Codex 或未登录时，打开官方安装或登录界面，用户完成登录后继续。默认保留用户的任务执行归属、审批和开机启动选择。
3. 缺少官方 Tunnel 资料时，打开 https://platform.openai.com/settings/organization/tunnels 。用户需取得 Tunnel ID 和 runtime API Key，确认通道关联目标 ChatGPT 工作区并具备权限。Codex 登录不能替代这些信息。账号或管理员操作无法完成时明确说明缺什么，不反复重试。
4. 优先让用户直接在应用「设置 → 连接 → OpenAI Tunnel」输入凭据并保存；Codex 通过 doctor 发现已配置后继续。不要求用户把密钥贴进聊天，不在参数、日志或截图中显示密钥。需要程序配置且已有安全本机凭据来源时，通过 stdin 传递 JSON，不将密钥字面量写进 shell 命令。configure 省略字段保留原值，apiKey 空字符串也保留原值；修改 Tunnel ID 时必须使用与新通道匹配的密钥。连接开启期间修改配置会被拒绝，先确认确需更改再 disconnect。
5. 执行 connect，随后读取 doctor/status 等待就绪；connect 返回成功只代表启动请求已完成。下载或网络失败时读取 logs，依据实际错误处理。仅调整 Connector 自身代理；network 的 proxyMode 支持 system、direct、custom，custom 需要 HTTP/HTTPS proxyUrl。代理变更需要重新连接才能作用于已有 Tunnel，不修改系统代理。
6. 读取 verify --fresh，保存返回的 code。若具备可用且已授权的浏览器控制能力，可协助用户操作 ChatGPT；否则只给出当前必需的几步。ChatGPT「设置 → 安全与登录」开启开发者模式，Plugins → ＋，填写名称和描述，选择 Tunnel 和对应 Tunnel ID。已有连接直接复用，无需重建。没有开发者模式或通道不可见时检查账号/工作区政策、关联与使用权限，不宣称本机可以绕过。
7. 在 ChatGPT 新对话中选用 Local Connector，发送 verify 返回的 prompt。随后只轮询 verify，不重复执行 --fresh，否则正在等待的验证码会失效。只有本轮 verifiedAt 出现，才确认收到真实远程工具调用；本机 health、Tunnel ready、点击“已添加”都不能替代这个证据。
8. 用户要求整套任务能力可用时，通过 ChatGPT 发起一次无文件修改、无外部副作用的任务，并读取持久化回执、原生 threadId/turnId 和完成结果。请求状态未知时以原 requestId 回读，不换 ID 重放。没有执行此步骤就明确报告“远程调用已验证，任务执行尚未验收”。

## 故障与恢复

换一个 Codex 对话也从 doctor/status 开始。现有配置和验证记录由应用保存，不依赖上次聊天记忆。之前验收中断且已有待验证 code 时直接继续 verify；仅开始新一轮验收才使用 --fresh。

根据具体证据分辨：应用未启动、资料缺失、登录失效、Desktop IPC 不可用、组件下载失败、Tunnel 网络错误、工作区权限不足、ChatGPT 未选用连接、任务提交结果未知。不要把全部错误都处理为重装或重置配置。自动执行已授权且有证据支持的修复；凭据失效、管理员权限和账号选择交给用户。操作后重新读取相关状态并复验。

最终向用户说明本机连接、ChatGPT 入站、任务执行分别是否验证成功，以及尚缺的具体动作。不输出密钥、完整私有日志或真实任务内容。

参考：[安装指南](installation.md)、[Tunnel 接入](tunnel.md)、[官方 Tunnel 文档](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels)、[ChatGPT 接入文档](https://developers.openai.com/plugins/deploy/connect-chatgpt)。
