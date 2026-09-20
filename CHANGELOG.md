# 版本说明

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
