# 安装与更新

## 下载

从 [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases/latest) 获取安装包和 `SHA256SUMS.txt`。可下载版本与文件以发布页为准。

| 平台 | 文件 | 支持范围 |
| --- | --- | --- |
| macOS | `Local.Connector_<版本>_universal.dmg` | 包含 Apple Silicon 与 Intel |
| Windows x64 | `…-setup.exe` / `….msi` | 预览包；Desktop 任务接入尚未实现，不能完成同等的首次引导 |

应用不需要用户安装 Node、npm、Rust 或 Cargo。使用前需安装并登录 Codex Desktop；Tunnel Client 可以在应用引导中安装。取得 Tunnel ID/runtime API Key，并在 ChatGPT 添加插件后才可远程使用，详见 [首次使用](../README.md#使用)。

## macOS 安装

打开 DMG，将 Local Connector 拖入 Applications，弹出镜像后从应用程序目录启动。不要长期从 DMG 中运行。覆盖安装前从菜单栏退出 Local Connector，拖入新版替换；配置和 Codex 历史不在 App 内，不会因替换 App 而清空。

本项目暂不使用 Apple Developer ID 签名和公证。首次打开可能被系统阻止。确认文件来自上述发布页并核对 SHA256 后，可在系统设置的「隐私与安全性」中选择「仍要打开」。如果系统报告应用损坏，可仅对已确认来源的此 App 移除隔离标记：

```sh
xattr -rd com.apple.quarantine "/Applications/Local Connector.app"
```

若提示没有权限，可使用管理员账户处理该 App 的权限；不需要关闭系统整体的 Gatekeeper。若文件哈希不一致，应重新下载，而不是移除隔离标记。

核对下载文件：

```sh
shasum -a 256 Local.Connector_0.3.0_universal.dmg
```

将结果与同版本 `SHA256SUMS.txt` 的对应行比较。

## Homebrew

使用仓库提供的 Cask 安装：

```sh
brew tap whzxc/chatgpt-local-connector https://github.com/whzxc/chatgpt-local-connector
brew install --cask local-connector
```

若遇到隔离提示，可在确认来源后使用 `brew install --cask --no-quarantine local-connector`。已手动安装同名 App 时，退出 App 后使用 `brew install --cask --force local-connector` 覆盖。Cask 不自动执行 sudo 或清除整个 App 的扩展属性。

应用内更新可直接使用；通过 Homebrew 更新则执行：

```sh
brew update
brew upgrade --cask --greedy local-connector
```

## Windows 预览安装

选择 x64 的 NSIS `.exe` 或 MSI，运行安装器。Windows 包尚未完成 Desktop 外部任务管理，请勿把安装成功视为业务可用。未使用 Authenticode 签名，系统可能显示未知发布者；企业策略禁止运行时需要管理员处理，应用不能绕过策略。

PowerShell 校验示例：

```powershell
Get-FileHash .\Local.Connector_0.3.0_x64-setup.exe -Algorithm SHA256
```

升级时退出 App，使用同一种安装器覆盖安装。配置默认保留，不要求用户先卸载。

## 应用内更新

默认在应用可见时每六小时检查一次，有新版本时显示入口；可在设置中关闭自动检查、手动检查、查看更新说明或跳过某个版本。手动检查会重新显示已跳过的版本。

点击下载并安装后可查看进度，下载阶段可以取消。安装阶段不可取消。应用先下载并验签，再关闭 Connector 连接、替换并重启，恢复更新前的连接状态；不会重启 Codex Desktop 或接管其任务。

更新失败会显示错误，可重试或打开手动下载页。安装失败会尝试恢复原连接；若恢复也失败，显示实际错误。没有可用发行版、离线或无法访问 GitHub 时可能无法检查更新，连接功能仍可使用。

更新包使用本项目独立密钥签名，客户端内置公钥验签；它不替代 Apple/Windows 系统代码签名。

## 卸载

先在设置关闭登录系统启动，再从菜单栏退出应用，然后删除 App 或使用系统卸载器。Homebrew 用户可执行 `brew uninstall --cask local-connector`。

Connector 配置默认保留在 macOS 的 `~/.local/state/chatgpt-local-connector` 或 Windows 的 `%LOCALAPPDATA%/chatgpt-local-connector`。只有不再需要 Tunnel 配置、回执与日志时才自行删除该目录；Codex 项目和历史由 Codex 管理。
