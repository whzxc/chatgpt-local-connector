# 发布维护

源码与下载仓库：`whzxc/chatgpt-local-connector`。版本以 `package.json` 为准。使用 GitHub Releases 托管安装包、更新包和静态 `latest.json`，不需要自建分发服务。公共 npm 包不在发布范围内。

## 发布配置

1. 发布目标为上述 GitHub 公开仓库。本机状态、密钥和真实任务内容不得进入源码或发行包。
2. 更新公钥位于 `desktop/tauri.conf.json`，发布必须使用其对应私钥。私钥保存在仓库外并离线备份，切勿提交；更换公钥会影响已安装客户端的更新验签。
3. 在仓库 Actions Secrets 中设置 `TAURI_SIGNING_PRIVATE_KEY`（私钥文件内容）。私钥未加密时无需设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；如果自行使用加密私钥，则设置对应密码。
4. 允许 Release 工作流写入 Contents 以创建 Release，并读取 Actions artifacts 以复用构建产物。工作流不提交代码、不推送分支；安装包校验值、更新清单和 Cask 仅作为发布产物保存。

配置 Actions Secret：

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo whzxc/chatgpt-local-connector < "$HOME/.config/local-connector/release/updater.key"
```

不需要 Apple Developer ID、公证或 Authenticode 证书；构建脚本清除继承的 Apple/CSC 认证变量，macOS 使用本地 ad-hoc 签名。Tauri 更新验签不可关闭，私钥丢失将导致已安装客户端无法验证新的更新。

## 本地检查与构建

```sh
npm ci
npm run release:check
npm run release:preflight
```

`release:sync` 以 `package.json` 为版本来源，同步 Tauri、两个 Cargo package 和锁文件；可在 `CHANGELOG.md` 维护该版本条目。它不提交、不打标签、不上传。

`release:preflight` 首先检查 Core、Desktop 和验签工具的 Rust 格式，再检查版本、类型、契约、前端构建与原生编译。单独检查格式可运行 `npm run check:format`。

macOS Apple Silicon 发布构建：

```sh
rustup target add aarch64-apple-darwin
# 将 TAURI_SIGNING_PRIVATE_KEY 设置为仓库外私钥文件的路径，勿把内容写入命令历史。
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.config/local-connector/release/updater.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=''
npm run desktop:build -- --release --target aarch64-apple-darwin
npm run release:stage -- darwin-aarch64 desktop/target/aarch64-apple-darwin/release/bundle release-artifacts/macos
```

Windows 在原生 Windows 环境使用 `--target x86_64-pc-windows-msvc`，stage 平台参数为 `windows-x86_64`。

macOS DMG 使用 `desktop/assets/dmg-background.png` 提供无文案的拖拽安装引导，窗口尺寸与图标位置在 `desktop/tauri.conf.json` 的 `bundle.macOS.dmg` 中配置。背景中的两个落点对应实际应用与 Applications 文件夹，箭头指向右侧。macOS 构建需要 `uv`；`desktop:build` 在 Tauri 生成应用后，通过 `tooling/build-dmg.py` 调用固定版本的 dmgbuild 直接写入布局，无需 Finder 会话。Python 与 dmgbuild 仅用于构建，不进入安装包。

Stage 目录必须为空，每次使用新目录。脚本只挑选指定平台的发行文件，统一文件名，并使用与 Tauri 相同的 Minisign 验签库验证实际更新包与内置公钥匹配。

## 发布流程

日常 push 和 PR 更新不自动运行 Actions 检查。发布前确保工作区干净，并已取得提交、推送和发布授权。先检查版本准备结果：

```sh
npm run release:prepare -- --dry-run
npm run release:prepare
npm run release:preflight
```

`release:prepare` 查询 GitHub 已公开的正式 Release，以其中最高稳定版本为基准（忽略草稿和预发布，查询失败会停止）：

- 当前版本高于已发布版本：保留现有版本和提交，只检查各版本文件一致性，不写文件、不新增提交。
- 当前版本等于或低于已发布版本：使用已发布版本的下一个 patch 版本，同步版本文件和锁文件，并将上次发布以来的提交说明写入 `CHANGELOG.md`；这些变更合并为一次 `chore(release)` 提交。
- 尚无正式 Release：直接使用当前合法版本，不新增提交。

准备命令不推送、不打标签、不运行远程构建。需要提升 minor 或 major 时，在普通开发提交中提前更新版本并运行 `release:sync`。已提前更新版本但没有对应更新日志时，发布说明直接从 Git 历史生成到产物目录，不要求补交日志。生成日志要求本地具备上次发布的版本标签且该标签为当前提交的祖先；缺失时先获取标签。

将准备好的提交推送到 `main` 后，在该提交上触发一次演练：

```sh
gh workflow run release.yml --ref main -f publish=false
gh run watch <演练运行ID> --exit-status --compact --interval 3
```

演练在同一提交上并行运行双端 `Check` 和双端安装包构建；检查和构建全部通过才生成可供发布的演练元数据。检查失败时构建可能已经消耗计算资源，但产物不会进入发布。无需提前单独运行一次 `Check`。

等待命令应持续跟踪同一次运行；执行环境返回后台会话时，继续读取该会话直至退出，不另起固定休眠轮询。

等待整个演练成功后，直接为演练对应的提交打版本标签。无需下载元数据回写源码，也无需新增提交。标签版本须与 `package.json` 一致，且高于已公开版本。

正式发布只接受与演练完全相同的提交 SHA；任何新增提交都需要重新演练。标签应指向已验收的演练提交，不依赖本地 HEAD 是否仍在该位置：

```sh
git tag v<版本> <演练提交SHA>
git push origin v<版本>
```

推送前确认目标 remote 为公开源码仓库；仅推送选定分支和版本标签，不使用 `--mirror` 或 `--all`。

`Check` 既可通过 `gh workflow run check.yml --ref <分支>` 独立执行，也可由演练复用；在恢复 Rust 缓存和安装 npm 依赖前执行格式与版本校验，再执行类型检查、契约测试、前端构建和原生编译检查。独立检查不触发构建或发布。正式发布复用成功演练中的检查证据与安装包，不重新构建。

`Release` 由版本标签触发实际发布。手动运行默认是演练（`publish=false`），执行构建、验签与清单生成，仅上传 Actions artifacts；不创建 Release、不修改公开下载和 Homebrew。手动发布必须选择版本标签并设置 `publish=true`。

工作流步骤：

1. 演练并行检查同一提交并构建 macOS Apple Silicon 与 Windows x64 安装包，验证更新签名，生成清单、更新说明与含真实 DMG 哈希的 Cask。
2. 正式发布从最近 100 次成功的 `main` 演练中，选择提交 SHA 与版本标签所指提交完全相同的一次。找不到匹配构建或产物已过期时停止发布，需在该提交重新演练。
3. 下载该次演练的两端安装包，重新验证签名并生成清单和含真实 DMG 哈希的 Cask；所有文件仅写入产物目录。
4. 上传全部文件到 Draft Release，再一次性公开为 latest；不会提前把不完整更新推给用户。
5. 从公开地址下载清单及所有产物，逐一比对本地已验签的字节。发布流程不向默认分支写入提交。

公开后的版本不可覆盖重建。构建失败可以重跑；如果已经公开但发布后验证失败，先检查已发布内容，使用 `node tooling/release.mjs verify-published <产物目录>` 回读，不重复覆盖该 Release。新修复使用新的版本号。Cask 随安装包在 Release 中公开，不提前更新源码仓库中的安装配方。

## 缓存与发布演练

桌面 release profile 使用 Thin LTO、4 个 codegen units 和符号剥离，在保留跨 crate 优化的同时允许并行生成机器码。改变这些参数会使部分编译缓存失效；比较构建耗时时应区分依赖预热与项目自身重编译，并同时检查产物体积。

npm 下载由 `setup-node` 缓存。Rust 使用 `rust-cache` 缓存依赖和编译产物，检查、安装包构建与清单验签使用不同缓存，按工具链、平台与依赖区分；演练由 `desktop:build` 统一准备前端并生成安装包，每个平台只构建一次前端；正式发布复用同一批字节并重新验签。

只有 `main` 写入 Rust 缓存。发布标签可以读取默认分支缓存，不依赖上一个版本标签的缓存。首次使用、工具链变化或依赖大幅更新后，在 `main` 上运行包含检查的演练以预热：

```sh
gh workflow run release.yml --ref main -f publish=false
```

演练是正式发布的构建阶段，产物保留 14 天；需在过期前完成发布。已公开版本不可用演练产物覆盖。

## 阶段耗时

独立 `Check` 与 `Release` 在结束时运行 `Timing summary`；演练中复用的检查统一计入 Release 汇总，不额外启动统计作业。Actions 运行摘要中提供每个作业、每个阶段与各步骤的实际耗时、结果和 Rust 精确缓存命中状态；失败步骤同样计入。`workflow-timing` artifact 提供 `timings.md` 和 `timings.json`，保留 30 天。

总耗时按墙钟时间计算，两端并行构建不能相加；统计覆盖业务作业的调度等待、缓存恢复及保存、依赖安装、检查或打包、验签、上传、发布和公开产物校验。统计作业本身及其上传开销不计入该值，GitHub 页面总时长包含这部分开销。
