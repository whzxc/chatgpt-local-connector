# 桌面生命周期

正式应用是一个 Tauri/Rust 进程，内部持有唯一连接核心。不存在单独的 Node Service、运行时复制目录或 App 与后台版本匹配问题。

- **开启连接**：确保 Desktop 可用，初始化并启动官方 Tunnel Client；它按需启动同一程序的 `stdio` 适配进程。Tunnel 的管理桥接与 Connector 辅助 RPC 均直接使用 Desktop 自带的原生 Codex 二进制，不经过 npm 启动脚本。
- **关闭连接**：关闭 Tunnel 进程组、MCP 转发和 Connector 的辅助 RPC；未确认的请求保留回执，不能当作未执行而自动重试。
- **关闭窗口**：隐藏到菜单栏，连接保持运行。
- **退出应用**：停止 Connector 所属连接进程，然后退出。
- **登录时启动**：系统启动原生应用并开启连接。
- **应用更新**：先下载并验签，再关闭连接、安装和重启；Codex Desktop 的任务执行权不随之转移。

菜单栏使用透明品牌 Logo，提供连接状态、任务及待审批数量、记录和设置入口；连接、登录启动与任务审批开关使用与主窗口相同的状态和接口。审批模式仍可由云端按用户意图批准或绕过。

任务创建通过临时空种子持久化后交给 Desktop；发送、续接和中断使用 Desktop owner 的 IPC。辅助 app-server 仅用于项目、历史和其他工具能力，不能接管 Desktop 任务。不读取或限制 Desktop 应用版本号；连接仍校验 IPC 协议、socket 所属用户、任务 owner 和响应来源。协议或操作不兼容时返回实际错误，不绕过校验或接管执行。当前外部任务管理仅支持 macOS。

本机 HTTP 仅绑定 `127.0.0.1` 随机端口，验证 Host、Origin 和随机凭据。端口与凭据写入私有状态目录，供 stdio/开发预览使用。前端静态资源直接随 Tauri 内嵌，不运行 Node Web 服务器。

开发版使用独立标识和图标，只读正式原生状态；正式应用未运行时使用只读原生预览，不启动业务连接。详见[开发说明](development.md)。

首次使用需要安装并登录 Codex Desktop、在应用内安装 Tunnel Client、填写 Tunnel 凭据，然后在 ChatGPT 添加并启用连接。已有 Codex 登录可以复用，但不能替代 Tunnel 凭据或网页端插件接入。完整步骤见 [README](../README.md#使用) 和 [Tunnel 接入](tunnel.md)。
