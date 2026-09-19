# 开发与构建

## 日常开发

工具链：Node 24.12+、npm、Rust stable 与 macOS/Windows 平台 SDK。Node 只运行开发工具，不是产品运行依赖。

```sh
npm ci
npm run dev:ui
```

Vite 保持在 `http://127.0.0.1:5173`。Vue/CSS 修改热更新，不打包、不安装、不重启正式应用。预览通过本机认证代理读取正在运行的原生应用；若没有原生应用，则按需编译并启动 `native` 中的只读预览程序。预览不启动 Tunnel、不执行 MCP 工具、不修改连接配置。

原生窗口开发使用 `npm run desktop:dev`，Rust 变动由 Tauri 增量编译。开发窗口为只读，正式应用的连接生命周期不受它控制。

## 代码结构

- `ui/`：Vue 页面、状态类型与交互。
- `native/`：Rust 连接核心。负责 Desktop IPC、辅助 Codex RPC、29 个 MCP 工具、回执、事件、配置和 Tunnel 生命周期。
- `desktop/`：Tauri 主程序、托盘、窗口、系统集成与更新。直接调用同进程的 Rust 核心。
- `tooling/`：开发和构建脚本；不进入应用运行资源。
- `tests/`：契约测试，以隔离的模拟上游检查 Rust 核心；测试专用 feature 不用于发行构建。

MCP stdio 由同一个原生可执行文件的 `stdio` 子命令承担，仅向持有连接的主进程转发。它使用父进程环境传递的本机端口和随机凭据，不暴露公共接口。配置与凭据不进入浏览器持久存储。

## 检查

```sh
npm run check
npm test
npm run check:native
cargo fmt --manifest-path native/Cargo.toml -- --check
cargo fmt --manifest-path desktop/Cargo.toml -- --check
```

Desktop IPC 外部任务管理仅支持 macOS。

## 发行构建

```sh
npm run desktop:prepare
npm run desktop:build
npm run check:package
```

macOS 安装包仅构建 Apple Silicon（arm64），产物在 `desktop/target/aarch64-apple-darwin/release/bundle/`。`check:package` 检查 App 中没有 Node、npm、node_modules 或旧 runtime 目录，并报告体积；可传入其他产物目录。Windows 使用 NSIS/MSI，按平台构建。

发行包只包含原生可执行文件、前端静态资源和图标。构建脚本将 Rust 源码中的本机用户目录与仓库路径映射为通用构建路径，避免在二进制中嵌入私人路径。官方 Tunnel Client 首次使用时独立下载并校验，Codex 使用用户安装的 Desktop 随附二进制，不重复打包。构建命令生成安装产物，不自动覆盖已安装应用。

版本需同步 `package.json`、`native/Cargo.toml`、`desktop/Cargo.toml` 与 Tauri 配置。更新地址和项目公钥固定在 `desktop/tauri.conf.json`；发布构建需要仓库外的 `TAURI_SIGNING_PRIVATE_KEY`。完整流程见 [发布维护](release.md)。
