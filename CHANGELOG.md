# 版本说明

## 0.6.1

扩展内置 ACP Agent 支持并完善跨 Agent 等待体验。

- 新增 Claude Code、Gemini CLI、GitHub Copilot CLI、Cursor、Cline、Kiro、Junie、Qwen Code、Kimi CLI、Grok CLI、Hermes Agent 与 Devin CLI 等内置 ACP Agent 定义，自动识别已安装命令并展示品牌图标。
- 新增统一 `agent_wait` 工具，可等待 Codex、Pi、OpenCode 与 ACP Agent 的任务终态、权限请求和用户输入，支持同时等待多个任务且不影响后台执行。
- 完善 Agent 进程生命周期与任务恢复，复用持久化会话并在连接关闭时终止受管子进程。
- 优化设置页 Agent 管理、ChatGPT 接入引导与复制交互，明确连接验证、工具刷新和实际任务验收步骤。
- 调整发布流程，正式发布复用已通过验签的双平台演练产物，并校验源码、Cask 与发布标签一致。

支持 Apple Silicon Mac 与 Windows x64。新增 ACP Agent 仍依赖对应 CLI 已安装且提供 ACP 模式；具体启动参数与环境变量可在本机 manifest 中覆盖。

## 0.6.0

多 Agent 支持与任务等待能力。

- 新增 Pi 官方 RPC 与 OpenCode ACP 接入，自动发现本机安装并继承已有 provider、模型和登录配置。
- 新增通用 Agent 工具，支持创建、续接、读取、取消任务，流式事件、权限交互与持久化幂等回执；Codex Native 及全部原生工具保持可用。
- 设置页新增 Agents 区域，展示安装状态、版本与协议；支持通过本机 manifest 添加 ACP Agent。
- 新增 `codex_wait`，等待任务终态或交互请求，减少主动轮询；等待取消不影响任务执行。
- 优化 ChatGPT 接入与验收引导，区分连接就绪、实际入站调用与任务完成。

支持 Apple Silicon Mac 与 Windows x64。Pi/ACP 子进程由 Connector 管理；关闭连接会停止其进程。通用 Agent 工具目前支持文本输入，Pi 的权限交互依赖自身扩展，Codex Desktop 任务继续沿用原执行归属。

## 0.5.0

Windows Codex Desktop 任务接入。

- Windows x64 支持 Microsoft Store 版 Codex Desktop，可创建、读取、续接和中断任务，并在 Desktop 中打开。
- 新增 Windows 命名管道通信及服务端用户、进程归属校验，自动准备 Desktop 捆绑的 CLI 与辅助程序。
- 修复 Desktop 任务的默认模型继承与协作模式设置，避免创建或续接时模型为空，以及显式模型被旧设置覆盖。
- 修复应用安装更新后的重启阻塞，恢复更新前的连接状态。
- 同步安装、配置、HTTPS MCP、桌面生命周期和开发文档，补充多设备独立连接与临时地址变化后的操作说明。

支持 Apple Silicon Mac 与 Windows x64。两端均可使用官方 OpenAI Tunnel 或 HTTPS MCP；HTTPS MCP 支持 Cloudflare 临时体验、固定域名、ngrok 和自定义反向代理。

## 0.4.3

全局更新提醒与本机配置体验优化。

- 检测到新版本时弹出全局更新提示，集中展示版本说明、下载进度和安装操作，支持稍后处理与跳过此版本。
- 标题栏新增更新图标，可随时重新打开更新弹窗；移除占用内容区域的更新横幅。
- 新增应用内置 CLI，支持 Codex 查询状态、诊断配置、管理连接和读取入站验证信息，并提供匹配版本的配置指南。
- 新增 macOS 应用显示位置设置，可调整 Dock 与菜单栏显示方式。

支持范围保持不变：Apple Silicon Mac 支持 Desktop 任务接入；Windows 安装包仍为预览产物。

## 0.4.2

HTTPS MCP 接入与网络连接体验优化。

- 新增 HTTPS MCP 接入，支持 Cloudflare 临时隧道、Cloudflare 固定域名、ngrok 和自定义域名反向代理。
- 自动下载和管理 Cloudflare、ngrok 隧道组件，展示 ChatGPT 接入地址，并在关闭连接或退出应用时清理辅助进程。
- 新增系统代理、直连和自定义代理设置，应用于连接组件下载、Tunnel 连接及应用更新；本机通信保持直连。
- 优化连接配置与接入引导，统一紧凑表单控件、设置布局和更新按钮体验。

支持范围保持不变：Apple Silicon Mac 支持 Desktop 任务接入；Windows 安装包仍为预览产物。HTTPS MCP 入口当前无需认证，适用于用户自行控制访问范围的接入环境。

## 0.4.1

可选后台执行与任务打开体验优化。

- 新增默认开启的“自动打开 Codex 任务”设置；关闭后，新任务由 Connector 后台执行，不自动跳转。
- 按任务创建时的执行方式继续、读取和中断任务，切换设置不影响已有任务的执行归属。后台任务不保证可在 Desktop 中操作。
- 优化 Desktop 任务创建时的 Prompt 准备与通信流程，减少重复连接和查询。
- 修复任务详情中“在 Codex 中打开”的跳转，并在弹窗内显示操作错误。
- 精简设置说明，统一偏好标题字号与更新按钮尺寸。

支持范围保持不变：Apple Silicon Mac 支持 Desktop 任务接入；Windows 安装包仍为预览产物。

## 0.4.0

任务审批与持久化任务管理。

- 新增可选任务审批模式，可在设置和菜单栏切换；ChatGPT 可按用户意图批准或绕过审批。
- 持久保存 Connector 管理的任务请求，汇总创建、续接和中断记录，重启后仍可查看。
- 新增任务瀑布流卡片，展示标题、模型、推理等级、项目及创建时间；平滑展开查看 Prompt 和请求记录。
- 同步 Codex 运行与归档状态，已归档任务隐藏不可用的打开入口。
- 更新菜单栏状态和任务入口，显示待审批数量，使用透明背景品牌 Logo。

支持范围保持不变：Apple Silicon Mac 支持 Desktop 任务接入；Windows 安装包仍为预览产物。

## 0.3.0

原生桌面连接器。

- 原生桌面连接器，通过官方 Secure MCP Tunnel 将 ChatGPT 连接到本机 Codex Desktop。
- 引导安装 Tunnel Client、复用 Codex 登录，配置通道并验证 ChatGPT 实际入站调用。
- 查询本机项目与文件、创建和继续 Desktop 任务、读取结果及定向中断；任务执行由 Codex Desktop 独立管理。
- 支持连接日志、菜单栏、外观偏好和登录系统后自动连接。
- 提供 GitHub Releases 下载、Homebrew 安装及验签的应用内更新。

当前支持范围：Apple Silicon（arm64）Mac 的 Desktop 任务接入。Windows 安装包为预览产物，尚未实现 Desktop 任务接入，不能完成同等的首次设置流程。

安装包未做 Apple Developer ID 签名、公证或 Windows Authenticode 签名。首次安装说明见仓库的 `docs/installation.md`。更新包使用独立的项目密钥验签，与系统代码签名无关。
