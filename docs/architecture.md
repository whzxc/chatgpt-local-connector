# 架构说明

采用 Vue + Tauri + Rust，单一原生核心管理连接状态、MCP 请求、幂等回执和用户日志。前端使用系统 WebView，发行包不携带 Node/npm/node_modules。

官方 Tunnel Client 独立下载管理。Codex Desktop 拥有任务执行权，Connector 通过 Desktop IPC 管理任务；辅助 app-server 提供项目、历史和工具查询，不接管 Desktop 任务。

29 个工具定义保存在 Rust 编译内嵌的 `native/src/catalog.json` 中；参数由 JSON Schema 校验，写请求有持久化回执。大量输出可分页，事件持久化与界面日志分开，常规界面不展示协议握手噪声。

Desktop 接入不按应用版本号设白名单，保留实际协议和任务 owner 校验；跨版本兼容性以实际调用为准。当前 Desktop 外部任务管理仅支持 macOS。通信与生命周期详见 [桌面说明](desktop.md)，接口见 [MCP 工具](tools.md)。
