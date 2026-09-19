# 发布维护

源码与下载仓库：`whzxc/chatgpt-local-connector`。版本以 `package.json` 为准。使用 GitHub Releases 托管安装包、更新包和静态 `latest.json`，不需要自建分发服务。公共 npm 包不在发布范围内。

## 发布配置

1. 发布目标为上述 GitHub 公开仓库。本机状态、密钥和真实任务内容不得进入源码或发行包。
2. 更新公钥位于 `desktop/tauri.conf.json`，发布必须使用其对应私钥。私钥保存在仓库外并离线备份，切勿提交；更换公钥会影响已安装客户端的更新验签。
3. 在仓库 Actions Secrets 中设置 `TAURI_SIGNING_PRIVATE_KEY`（私钥文件内容）。私钥未加密时无需设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；如果自行使用加密私钥，则设置对应密码。
4. 允许 Release 工作流写入 Contents，以创建 Release 并同步 Cask。若默认分支受保护，需允许专用发布身份更新 `Casks/local-connector.rb`，或人工提交工作流生成的该文件；不要为此取消所有分支保护。

配置 Actions Secret：

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo whzxc/chatgpt-local-connector < "$HOME/.config/local-connector/release/updater.key"
```

不需要 Apple Developer ID、公证或 Authenticode 证书；构建脚本清除继承的 Apple/CSC 认证变量，macOS 使用本地 ad-hoc 签名。Tauri 更新验签不可关闭，私钥丢失将导致已安装客户端无法验证新的更新。

## 本地检查与构建

```sh
npm ci
npm run release:sync
npm run release:preflight
```

`release:sync` 以 `package.json` 为版本来源，同步 Tauri、两个 Cargo package 和锁文件；`CHANGELOG.md` 必须有该版本条目。它不提交、不打标签、不上传。

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

日常 push 和 PR 更新不自动运行 Actions 检查。准备发布时，先将版本、文档和代码提交到 `main`，手动运行一次双端检查：

```sh
gh workflow run check.yml --ref main
```

检查成功后，为该次检查对应的提交推送 `v<版本>` 标签；若期间又提交了改动，需要对新提交重新手动检查。标签版本须与 `package.json` 及 `CHANGELOG.md` 一致。

```sh
git tag v0.3.0
git push origin v0.3.0
```

推送前确认目标 remote 为公开源码仓库；仅推送选定分支和版本标签，不使用 `--mirror` 或 `--all`。

`Check` 仅手动触发，执行版本校验、类型检查、契约测试、前端构建、原生编译检查和格式检查。可以按需在开发分支运行；`Release` 只接受同一提交在 `main` 上成功完成的最新一次手动 `Check`，不重复测试、类型检查、格式检查或 `cargo check`。

`Release` 由版本标签触发实际发布。手动运行默认是演练（`publish=false`），执行构建、验签与清单生成，仅上传 Actions artifacts；不创建 Release、不修改公开下载和 Homebrew。手动发布必须选择版本标签并设置 `publish=true`。

工作流步骤：

1. 核对同一提交的 `Check` 结果。macOS Apple Silicon 与 Windows x64 并行恢复缓存、构建前端与原生安装包，并验证更新签名。
2. 两端全部成功后汇总产物，生成 `latest.json`、`SHA256SUMS.txt`、更新说明和真实 DMG 哈希的 Homebrew Cask。
3. 上传全部文件到 Draft Release，再一次性公开为 latest；不会提前把不完整更新推给用户。
4. 从公开地址下载清单及所有产物，逐一比对本地已验签的字节。
5. 验证通过后同步默认分支的 Cask。Cask 的版本和 SHA256 必须对应实际公开 DMG。

公开后的版本不可覆盖重建。构建失败可以重跑；如果已经公开但发布后验证或 Cask 同步失败，先检查已发布内容，使用 `node tooling/release.mjs verify-published <产物目录>` 回读，并人工同步 Cask，不重复覆盖该 Release。新修复使用新的版本号。

## 缓存与发布演练

npm 下载由 `setup-node` 缓存。Rust 使用 `rust-cache` 缓存依赖和编译产物，检查、安装包构建与清单验签使用不同缓存，按工具链、平台与依赖区分；安装包每次重新生成和验签。

只有 `main` 写入 Rust 缓存。发布标签可以读取默认分支缓存，不依赖上一个版本标签的缓存。首次使用、工具链变化或依赖大幅更新后，在已通过 `Check` 的 `main` 上运行演练以预热：

```sh
gh workflow run release.yml --ref main -f publish=false
```

演练替代独立的桌面打包工作流，使用与正式发布相同的构建配置和签名流程。已公开版本不可用演练产物覆盖。

## 阶段耗时

`Check` 与 `Release` 在结束时运行 `Timing summary`。Actions 运行摘要中提供每个作业、每个阶段与各步骤的实际耗时、结果和 Rust 精确缓存命中状态；失败步骤同样计入。`workflow-timing` artifact 提供 `timings.md` 和 `timings.json`，保留 30 天。

总耗时按墙钟时间计算，两端并行构建不能相加；统计覆盖业务作业的调度等待、缓存恢复及保存、依赖安装、检查或打包、验签、上传、发布和 Homebrew 同步。统计作业本身及其上传开销不计入该值，GitHub 页面总时长包含这部分开销。
