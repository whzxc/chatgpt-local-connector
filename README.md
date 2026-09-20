# ChatGPT Local Connector

**在 ChatGPT 里聊想法，让本机的 Codex 接着干。**

Local Connector 是 ChatGPT 和 Codex Desktop 之间的小小联络员。它通过官方 Secure MCP Tunnel，把对话接到你的电脑上：查项目、读代码、看 Git 状态，再把任务交给 Codex，回来接着聊进展。

![Local Connector 主界面：ChatGPT、Connector 与 Codex 已连接](docs/images/local-connector.png)

- **少一点复制粘贴**：让 ChatGPT 直接读取本机项目、文件和 Git 状态，讨论有据可依。
- **聊到哪，做到哪**：在对话里创建、续接或中断 Codex 任务，也能查看任务进展和结果。
- **连接有人照看**：应用负责 Tunnel Client 的下载、校验、配置和启停，连接状态一眼可见。

日常使用无需安装 Node、npm、Rust 或 Cargo。当前支持 **Apple Silicon Mac** 的完整接入；Windows 提供桌面预览包，尚不支持同等的 Desktop 任务接入。

## 安装

安装包与可用版本以 [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases/latest) 为准。直接安装、Homebrew、系统拦截、更新与卸载见 [安装指南](docs/installation.md)。

## 使用

1. 安装并登录 Codex Desktop，已有登录可直接复用。首次引导中的「登录 Codex」会打开 Desktop；完成后返回检查登录状态。
2. 打开 Local Connector，按引导安装官方 Tunnel Client。应用自动下载、校验并配置，无须手工安装运行环境或编写 Tunnel profile。
3. 在 OpenAI Platform 创建通道，或向管理员取得 Tunnel ID 和 runtime API Key，并确认通道关联目标 ChatGPT 工作区及账号具备使用权限。将两项信息填入应用并保存；Codex 登录不能替代 Tunnel 凭据。
4. 点击「开启连接」，等待 Tunnel 和 Codex 就绪。
5. 在 ChatGPT 开启开发者模式，进入 Plugins → ＋，填写名称和描述，连接方式选择 Tunnel，选择通道或填写 Tunnel ID，然后创建连接。已有连接无须重复添加；本机安装不会自动完成网页端授权。
6. 新建 ChatGPT 对话并选用 Local Connector，发送应用接入引导提供的验证消息，或执行一次本机项目只读查询。收到成功的远程工具调用后，应用会记录「已验证」。若要确认任务执行能力，再发起一个无害任务并读取完成结果。

Tunnel 身份、权限及 ChatGPT 接入详见[接入指南](docs/tunnel.md)。首次配置后，日常使用保持本机联网、Desktop 可用且 Connector 连接开启即可；登录时启动为可选设置。关闭窗口保留菜单栏和连接，退出应用则关闭连接。

### 分工与边界

任务执行进程由 Codex Desktop 管理。Connector 通过 Desktop IPC 管理任务，不替换 Desktop app-server；关闭 Connector 不会主动中断 Desktop 中的任务。Desktop 不可用时，Connector 不会接管任务执行，也不会把未知状态显示为空闲。

不按 Codex Desktop 应用版本号限制连接；可用性取决于实际 IPC 握手和所需操作是否受支持。私有协议可能随 Desktop 更新变化，连接成功不代表所有操作均兼容。

## 开发

```sh
npm ci
npm run dev:ui
```

技术栈是 **Tauri + Rust + Vue**：Rust 管理连接、配置、MCP 与 Desktop 通信，Vue 运行在系统 WebView 中。安装包不携带 Node、npm、Rust 或 Cargo，Node 仅用于源码开发和前端构建。

开发需要 Node 24.12+、Rust stable 和对应平台 SDK。开发预览使用 Vite 热更新，仅读取原生应用状态；没有原生应用时启动只读 Rust 预览进程，不启动 Tunnel。

相关文档：

- [安装与更新](docs/installation.md)
- [发布维护](docs/release.md)
- [版本说明](CHANGELOG.md)
- [开发与构建](docs/development.md)
- [桌面生命周期](docs/desktop.md)
- [MCP 工具与任务边界](docs/tools.md)
- [架构说明](docs/architecture.md)

本机数据默认位于 macOS 的 `~/.local/state/chatgpt-local-connector` 或 Windows 的 `%LOCALAPPDATA%/chatgpt-local-connector`，可通过 `CLC_STATE_DIR` 指定。密钥、回执和日志只保存在本机；卸载应用不会删除 Codex 历史。

采用 [MIT 许可证](LICENSE)。源码与桌面安装包使用 GitHub 分发，不发布公共 npm 包。
