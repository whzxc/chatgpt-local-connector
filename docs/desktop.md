# 桌面生命周期

正式应用是一个 Tauri/Rust 进程，内部持有唯一连接核心。不存在单独的 Node Service、运行时复制目录或 App 与后台版本匹配问题。

- **开启连接**：Desktop 模式确保 Desktop 可用；后台模式初始化 Connector 的 app-server，不唤起 Desktop。官方模式初始化并启动 Tunnel Client，它按需启动同一程序的 `stdio` 适配进程；HTTPS 模式启动独立的 MCP 监听，并按配置启动 Cloudflare 或 ngrok。Tunnel 的管理桥接与 Connector 辅助 RPC 均直接使用 Desktop 自带的原生 Codex 二进制，不经过 npm 启动脚本。
- **关闭连接**：关闭 Tunnel 进程组、MCP 转发和 Connector 的 app-server（包括后台执行）；未确认的请求保留回执，不能当作未执行而自动重试。
- **关闭窗口**：隐藏窗口，连接保持运行；可通过 macOS 菜单栏、Dock 或 Windows 系统托盘重新打开。
- **退出应用**：停止 Connector 所属连接进程，然后退出。
- **登录时启动**：系统启动原生应用并开启连接。
- **应用更新**：先下载并验签，再关闭连接、安装和重启；Codex Desktop 的任务执行权不随之转移。

菜单栏使用透明品牌 Logo，提供连接状态、任务及待审批数量、记录和设置入口；连接、登录启动与任务审批开关使用与主窗口相同的状态和接口。审批模式仍可由云端按用户意图批准或绕过。

「自动打开 Codex 任务」默认开启：任务创建通过临时空种子持久化后打开任务页交给 Desktop，发送、续接和中断使用 Desktop owner 的 IPC。关闭时，新任务直接在 Connector 的 app-server 创建并执行；此路径不依赖 Desktop IPC，不保证 Desktop 可继续或中断这些任务。已有任务的执行归属不随开关改变。后台任务的只读查询不触发恢复；Connector 重启后，明确发送新输入才会恢复会话，未确认请求不会自动重放。不按 Desktop 应用版本号限制连接；连接仍校验 IPC 协议、socket 或命名管道所属用户、任务 owner 和响应来源。协议或操作不兼容时返回实际错误，不绕过校验或接管执行。外部任务管理支持 macOS 和 Windows。Windows 识别当前用户安装的 Microsoft Store 版 Codex Desktop，额外校验命名管道服务端进程属于该安装；为避开 WindowsApps 的执行限制，将 Desktop 捆绑的 CLI 与辅助程序按内容散列复制到 Connector 私有数据目录，并通过系统注册的 `codex://` 链接打开任务。

本机 HTTP 仅绑定 `127.0.0.1` 随机端口，验证 Host、Origin 和随机凭据。端口与凭据写入私有状态目录，供 stdio/开发预览使用。前端静态资源直接随 Tauri 内嵌，不运行 Node Web 服务器。

开发版使用独立标识和图标，将读写操作转发给正在运行的构建版；构建版未运行时提示后台不可用，不启动独立业务连接。详见[开发说明](development.md)。

首次使用需要安装并登录 Codex Desktop，选择官方 Tunnel 或 HTTPS MCP 并配置对应信息，然后在 ChatGPT 添加并启用连接。应用自动准备所选方式需要的连接组件。已有 Codex 登录可以复用，但不能替代服务商凭据或网页端插件接入。完整步骤见 [README](../README.md#让-codex-帮你配置推荐) 和 [接入指南](tunnel.md)。
