# ChatGPT Desktop 独立插件

[English](../plugin.md) | **简体中文**

## 选择使用方式

| 你的需求 | 安装或配置 | 所需条件 |
| --- | --- | --- |
| 在本机 ChatGPT Desktop 中使用项目、Agent 工具和用量页面 | 本文的独立插件 | 支持本地插件的宿主；执行任务时另需对应 Agent |
| 菜单栏、额度浮窗、连接和 Agent 设置 | [Connector 桌面应用](installation.md) | 桌面安装包 |
| 从 ChatGPT 网页、手机或其他设备访问这台电脑 | [远程接入](tunnel.md) | 持续运行的本机后台、已认证的 Tunnel/HTTPS 入口以及控制端连接 |

独立插件自带原生后台，无需安装 Connector 桌面应用、Node、npm 或 Rust，也不需要 Tunnel 或 Tunnel API Key。外部 Agent 及其账号不包含在插件中。用量页面读取 Codex 本地日志；支持其他 Agent 的任务工具，不代表其 token 用量也已纳入统计。

## 安装

1. 在 [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases) 选择包含对应平台插件 ZIP 和 `SHA256SUMS.txt` 的版本，以该发布页实际附件为准。没有插件 ZIP 的版本不能按此方式安装；可选择提供插件的版本，或按[开发指南](../plugin-development.md)自行构建。桌面 DMG/EXE 不能作为插件包使用。

   | 平台 | 插件文件 |
   | --- | --- |
   | Apple Silicon macOS | `CLC.Plugin_<版本>_darwin-aarch64.zip` |
   | Windows x64 | `CLC.Plugin_<版本>_windows-x86_64.zip` |

2. 核对 ZIP 的 SHA-256 与 `SHA256SUMS.txt` 对应行。macOS 使用 `shasum -a 256 <ZIP文件>`；PowerShell 使用 `Get-FileHash <ZIP文件> -Algorithm SHA256`。解压到长期保留的目录，保留隐藏文件和可执行权限。根目录中应有 `.agents/plugins/marketplace.json` 和 `plugins/clc`；登记这个根目录，不是 ZIP 文件或 `plugins/clc` 子目录。
3. 宿主提供本地 marketplace 安装入口时，可从该入口登记并安装。支持插件命令的宿主 CLI 也可执行：

   ```sh
   codex plugin --help
   codex plugin marketplace add "<解压根目录>"
   codex plugin add clc@local-connector
   codex plugin list --marketplace local-connector --json
   ```

   替换路径占位符。Windows 中 `codex` 在 PATH 时，PowerShell 同样可执行这些命令。确认列表返回 `installed: true`、`enabled: true`。若已安装 CLC 开发版，在宿主中停用开发版，避免重复入口。
4. 重载宿主插件；没有重载入口时，等当前工作可安全中断后重启 ChatGPT，再打开下述页面。安装成功、后台可启动与页面实际可用需要分别确认。

如果提示未知命令 `plugin`，检查同一宿主自带的 CLI 或使用支持本地插件的宿主版本；全局 CLI 可能比应用旧。ChatGPT 位于 macOS 标准安装位置时，可这样调用它自带的 CLI：

```sh
"/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex" plugin --help
```

将上述安装命令中的 `codex` 替换为这个可执行文件即可。应用名称、安装位置和菜单可能随宿主变化；Windows 应从实际安装目录定位宿主自带 CLI，不应猜测 Store 包路径。工作区策略可能禁用本地插件，CLI 无法绕过该策略。

若系统拦截下载的可执行文件，先核对来源与校验值，再按系统提供的单文件授权或管理员策略处理。插件 ZIP 不是经过 macOS 公证的应用或 Authenticode 签名的 Windows 安装器，不需要关闭系统整体保护。

## 第一次使用

- 从宿主探索或侧栏打开 **Connector 总览**。首次打开会索引本机日志，随后自动刷新；已有可读日志时无需为了显示数据新建任务。
- 在本地任务的 **新建标签页 → 更多工具 → 任务用量** 打开任务页，也可点击总览中的任务。无法自动关联时，使用标题旁的 **选择任务**。部分云端任务无法加载本地面板；从总览选择的仍是本机任务，不是云端任务的用量。
- 本机尚无历史时，在已登录的 Codex 中完成一轮本地任务，再返回刷新。插件不会自行发起任务。有历史但列表为空时，清除搜索和筛选；搜索范围最多为已加载的 500 个任务。
- 需要执行任务时，先安装并登录对应 Agent。可让 CLC 列出本机项目、可用 Agent 和已有任务，再明确指定创建或继续哪个任务。用量面板打开成功不代表已验证 Agent 执行。
- 页头 **使用帮助** 提供“开始使用／功能边界／排查与恢复”；模块旁的信息符号解释指标，数据来源面板展示覆盖问题。这些说明随插件内嵌，无需访问文档网站。

账号额度与本机日志统计独立。额度缺失时，检查对应原生客户端的登录；若曾关闭订阅来源，可在 Connector 桌面应用的用量设置中启用。没有额度不影响查看本机日志，也不代表剩余额度为 100%。

## 更新或回退

桌面应用的自动更新不会重装 ChatGPT 插件。确认新版可用前保留旧解压包；使用完整插件产物，不手工修改宿主缓存。

1. 等正在由 Connector 执行的任务结束，关闭旧插件客户端／面板，以及共享同一数据目录的 Connector 桌面应用。共用后台要求原生构建一致，仅版本号相同仍可能不兼容。不要强停无关任务，也不要删除数据来绕过冲突。
2. 将目标版本解压到新的长期目录。更换来源目录时，执行：

   ```sh
   codex plugin marketplace remove local-connector
   codex plugin marketplace add "<新版解压根目录>"
   codex plugin add clc@local-connector
   codex plugin list --marketplace local-connector --json
   ```

   此处移除的是 marketplace 登记，不是项目或原始历史。若沿用同一路径，在旧客户端退出后替换完整包，再执行 `plugin add`。只替换解压文件不会更新已安装缓存。
3. 重载或重启宿主，重新打开两个页面，确认数据和手动刷新正常。回退可对保留的旧包重复相同步骤；共用数据的桌面应用也需匹配。不能保证旧版能读取新版持久化格式，降级前保留状态备份并核对目标版本支持范围；不向正在使用的状态目录覆盖恢复文件。

## 常见问题

| 现象 | 下一步 |
| --- | --- |
| 发布页没有插件 ZIP | 此版本不提供该安装方式；不要登记源码模板或 DMG |
| CLI 不认识 `plugin` | 检查宿主自带 CLI，并确认宿主支持本地插件 |
| marketplace 已登记但来源不同 | 按上文移除登记 → 新增来源 → 安装 |
| 找不到面板入口或工具重复 | 确认已启用，停用另一套 CLC，重载宿主 |
| 更新后仍显示旧界面 | 重新安装到宿主缓存，再重载并重新打开面板 |
| 首次打开正在索引 | 等待本机索引完成；隐藏面板会暂停界面刷新 |
| 无记录或未关联任务 | 检查本机 Codex 历史，清除筛选或明确选择任务 |
| 连接超时 | 先重试，初始化失败也会重新连接；仍失败则检查启用状态并重载 |
| 版本／构建不匹配 | 等旧任务可结束后关闭旧入口，使用匹配的插件和桌面产物 |
| 无额度 | 检查原生登录和订阅来源设置；未知不等于全部剩余 |

求助时提供系统与架构、宿主版本、插件版本、页面名称和实际错误。取得数据后，可在数据来源面板查看后台版本、读取时间与覆盖问题。安装排障只需相关错误片段，不需要密钥或私人任务内容。

## 功能与数据边界

- 每个状态目录只有一个 Core，插件与桌面应用共用任务、回执和统计。桌面应用负责原生窗口、菜单栏与浮窗。`CLC_STATE_DIR` 可隔离另一实例；不同原生构建不能同时占用同一个目录。
- 后台每五秒检查完整日志行，可见面板每十秒刷新，隐藏时暂停界面刷新。后台索引期间保留已显示的数据。刷新只读，不调用模型。
- 只读取当前原生 Codex home 下可读的 `sessions`、`archived_sessions` JSONL；不覆盖所有云端和其他设备的活动，也不能从日志推断账号归属。账号额度独立读取，不能分摊为单个任务的实际费用。
- 任务关联要求宿主元数据一致且存在对应原生日志。缺少、冲突或无法识别的标识必须手动选择，不用最新任务替代，也不把面板或匿名会话 ID 当任务 ID。
- 新版 response 按线程与响应身份去重；旧累计日志仅在边界可确认时取差分。缓存属于输入，推理属于输出；压缩引用不重复计量，父子和分叉任务分别展示。缺失字段为未知，已确认小计可能不完整。
- 总览按所选时间区间的事件统计，日分组采用 UTC；任务页显示该任务生命周期。整轮耗时包含工具与等待，不是纯生成速度。实际执行模型、精确费用和 credits 可能未知；大 token 数本身不能证明浪费。
- 统计检查点包含计数、标识和事件元数据，不包含提示词、工具参数或输出。日志截断、替换和时间戳变化会触发重新检查；大小、修改时间及首尾锚点均不变的原地改写可能无法检测。已删除历史无法还原为完整数据。
- 用量页面只读；插件还提供本地项目、文件和 Agent 控制工具，按用户明确请求使用。外部 Agent 沿用自身账号与授权，远程入口单独认证。
- 关闭一页不一定停止后台；最后一个本地插件或桌面连接退出后，Core 才结束，并停止 Connector 所有的执行。外部 Codex Desktop 所有的任务仍由它负责。插件安装本身不保证宿主退出后继续执行；普通聊天不提供定时唤醒或主动推送。

## 卸载

在宿主中禁用或移除 Local Connector，关闭相关页面。没有其他插件使用 `local-connector` marketplace 时，移除其登记，再删除解压包。已有项目、外部 Agent 和原始 Codex 历史独立保留。

Connector 状态默认保留在 macOS 的 `~/.local/state/chatgpt-local-connector` 或 Windows 的 `%LOCALAPPDATA%/chatgpt-local-connector`；也可能通过 `CLC_STATE_DIR` 指定其他位置。仍有插件或桌面应用使用时不要删除共享目录；卸载不要求清空配置、回执或索引。

源码开发、自动重建与打包见[插件开发指南](../plugin-development.md)。
