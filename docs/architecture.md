# 架构说明

采用 Vue + Tauri + Rust，单一原生核心管理连接状态、MCP 请求、幂等回执和用户日志。前端使用系统 WebView，发行包不携带 Node/npm/node_modules。

官方 Tunnel Client 独立下载管理。默认由 Codex Desktop 拥有任务执行权，Connector 通过 Desktop IPC 管理任务。关闭「自动打开 Codex 任务」后，新任务由 Connector 持有的 app-server 后台执行，不唤起 Desktop。执行归属按任务持久化，切换开关不会迁移已有任务；查询、续接和中断沿用任务原有路径。

29 个工具定义保存在 Rust 编译内嵌的 `native/src/catalog.json` 中；参数由 JSON Schema 校验，写请求有持久化回执。大量输出可分页，事件持久化与界面日志分开，常规界面不展示协议握手噪声。

Desktop 接入不按应用版本号设白名单，保留实际协议和任务 owner 校验；跨版本兼容性以实际调用为准。当前 Desktop 外部任务管理仅支持 macOS。通信与生命周期详见 [桌面说明](desktop.md)，接口见 [MCP 工具](tools.md)。

任务审批是可由本机或云端决定的可选确认流程。任务请求内容、审批决定和提交状态共用持久化回执；任务页按 threadId 汇总请求，使用原生状态快照查询运行状态，不解析会话 JSONL。审批设置独立保存，可在连接期间切换，不改变 Codex 的执行权限。
