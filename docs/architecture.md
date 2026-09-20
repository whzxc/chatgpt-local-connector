# 架构说明

采用 Vue + Tauri + Rust，单一原生核心管理连接状态、MCP 请求、幂等回执和用户日志。前端使用系统 WebView，发行包不携带 Node/npm/node_modules。

连接方式分为官方 Tunnel 与自备 HTTPS MCP。官方 Tunnel Client 独立下载管理；HTTPS 模式由 Rust 提供独立的 Streamable HTTP 监听，仅暴露无需认证的 MCP。应用管理 Cloudflare Quick Tunnel、固定域名 Tunnel 或 ngrok 客户端，也支持用户自备反向代理；TLS 与公网转发由所选服务商或代理负责。两种入口复用相同工具分发、审批和回执，不能同时开启。默认由 Codex Desktop 拥有任务执行权，Connector 通过 Desktop IPC 管理任务。关闭「自动打开 Codex 任务」后，新任务由 Connector 持有的 app-server 后台执行，不唤起 Desktop。执行归属按任务持久化，切换开关不会迁移已有任务；查询、续接和中断沿用任务原有路径。

30 个工具定义保存在 Rust 编译内嵌的 `native/src/catalog.json` 中；参数由 JSON Schema 校验，写请求有持久化回执。大量输出可分页，事件持久化与界面日志分开，常规界面不展示协议握手噪声。

Desktop 接入不按应用版本号设白名单，保留实际协议和任务 owner 校验；跨版本兼容性以实际调用为准。Desktop 外部任务管理支持 macOS 和 Windows。通信与生命周期详见 [桌面说明](desktop.md)，接口见 [MCP 工具](tools.md)。

任务审批是可由本机或云端决定的可选确认流程。任务请求内容、审批决定和提交状态共用持久化回执；任务页按 threadId 汇总请求，使用原生状态快照查询运行状态，不解析会话 JSONL。审批设置独立保存，可在连接期间切换，不改变 Codex 的执行权限。

`native/src/waiter.rs` 提供通用只读等待引擎。Codex 适配器固定连接到任务所属的 App Server 或 Desktop owner；通知只负责唤醒，状态判定依赖重新读取原生快照，每两秒兜底核实。事件缓冲截断、通知丢失或合并不会影响最终判定。读操作与等待都不持有全局锁，取消只销毁等待 future，不中断轮次。Connector 关闭以 watch 通道通知所有等待者。

当前没有统一 AgentHost 或 `agent_*` 工具。后续 AgentHost 只需实现 WaitSource 的原生状态读取与连接存活检查，映射规范化 thread/turn、pending 数据，并提供事件 Notify 与关闭 watch；工具目录注册 `agent_wait` 后即可复用等待引擎。不同 agent 的通知不能直接作为完成依据。
