# ChatGPT Local Connector

**English** | [简体中文](README.zh-CN.md)

**Discuss an idea in ChatGPT. Let a coding agent on your computer do the work.**

Local Connector connects ChatGPT to local coding agents: Codex Native, Pi, and built-in ACP Agents including Gemini, the Claude adapter, Cursor, and Grok. Through OpenAI Secure MCP Tunnel or HTTPS MCP, ChatGPT can inspect projects, read code, check Git status, hand tasks to Codex, and follow their progress.

![Local Connector home screen showing ChatGPT, Connector, and Codex connected](docs/images/local-connector.png)

- **Less copying and pasting:** let ChatGPT read local projects, files, and Git status to ground the discussion.
- **Keep work in the conversation:** create, continue, or interrupt Codex tasks and inspect progress and results.
- **Wait for meaningful progress:** after creating or continuing a task, use `agent_wait` (`codex_wait` for native Codex) to wait for completion, failure, or required interaction, for up to five minutes per call. A timeout does not stop the task.
- **Managed connections:** the app downloads, verifies, configures, starts, and stops the Tunnel Client and displays its connection status.

CLC itself requires no Node, npm, Rust, or Cargo installation. External Agents still need their own runtimes. Desktop task integration supports **Apple Silicon Mac** and **Windows x64**.

Codex remains the default, with its native capabilities preserved. External Agents use their own configuration and sign-in. Settings → Agents lists built-in Agents, detected installation status and versions, and lets you enable installed Agents. Native/adapter distinctions and capabilities are documented in [Local Agents](docs/agents.md). Other built-in ACP Agents include Copilot, Kimi, Qwen, Kiro, Devin, Cline, Junie, Hermes, and OpenCode. Shared task operations use the `agent_*` tools.

## Installation

See [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases/latest) for available versions and installers. The [installation guide](docs/installation.md) covers direct installation, Homebrew, operating system security prompts, updates, and removal.

## Let Codex set it up (recommended)

Send the following message to Codex on the target computer. It checks existing progress and handles installation, configuration, troubleshooting, and verification. You can provide a signed-in ChatGPT browser session for it to operate. You only need to handle steps that require your identity or confirmation, or that its tools cannot reliably complete. The default connection uses the official OpenAI Secure MCP Tunnel.

```text
Install, configure, and fully verify ChatGPT Local Connector on this computer. Default to the official Secure MCP Tunnel, preserve any working Tunnel / HTTPS configuration, and do not introduce a CLC cloud service or public relay. Check the installation first and read cli help, cli guide, and cli doctor. If the CLI is unavailable, read https://github.com/whzxc/chatgpt-local-connector/blob/main/docs/codex-setup.md and follow the actual installed release.
Complete all automatable steps. If I provide a signed-in ChatGPT page or browser environment, use the available browser / GUI / Computer Use tools on the visible page: check Developer Mode, reuse or create a custom MCP connection, enter its details, refresh tools, and select it. Do not just give me instructions. Do not use private APIs, extract cookies, run fixed DOM scripts, or bypass security controls.
Read URLs, Tunnel ID, configuration, and verification messages from cli onboarding / status without asking me to copy values already on this computer. Configure existing secure local credentials through stdin. Ask me to enter missing keys directly in the app, never in chat. Pause only for required sign-in, identity confirmation, authorization, verification codes, administrator access, or a step the available tools cannot reliably perform. Explain the exact blocker and the minimum action, then continue.
Call connector_verify from ChatGPT and check the current code, tool result, and local challengeVerifiedAt. Then run a harmless Codex task over the same connection: do not call tools or read or modify files; only reply CLC_ONBOARDING_OK. Read the persistent receipt, native threadId / turnId, and completed output. Read back unknown states with the original requestId instead of submitting again. Report separate evidence for local readiness, ChatGPT inbound access, and task completion. Opening a page or reaching Tunnel ready is not proof of successful setup.
```

You can reuse this message for connection problems. Use a Codex session that can execute commands on the target computer. Your ChatGPT account/workspace permissions, Tunnel identity, and required authorizations are still necessary.

The [Codex setup and CLI guide](docs/codex-setup.md) covers configuration and diagnostics; the installed release's `cli help` / `cli guide` take precedence. For self-service setup, see the [manual connection guide](docs/tunnel.md).

## Everyday use

Choose Auto / English / 简体中文 in **Settings → General → Language**. Auto follows your system or browser language, falling back to English for unsupported languages. A manual selection takes priority, is saved, and takes effect immediately. Like the theme, this preference belongs to the current WebView/browser; it does not affect MCP tools, task content, or protocols.

After setup, keep the computer online, Desktop available, and Connector connected. Connecting at sign-in is optional. Closing the window leaves the connection running; quitting the app disconnects it. On macOS, **Settings → General → Show app in** offers All, Menu bar only, or Dock only. Changes apply immediately and are saved automatically.

`agent_wait` supports Codex, Pi, all built-in ACP Agents, and Custom ACP while the local connection is running; `codex_wait` preserves native semantics. Use the corresponding read/events tools for history and events. An ordinary Chat response does not keep waiting after it ends; these tools provide neither scheduled wakeups nor proactive push notifications. See [task waiting](docs/tools.md#任务等待).

### Responsibilities and boundaries

**Automatically open Codex tasks** is enabled by default: Desktop owns and runs new tasks. When disabled, Connector runs new tasks in the background without opening Desktop, and Desktop control of those tasks is not guaranteed. The setting affects only new tasks; existing tasks retain their execution owner. Closing Connector stops its background execution but does not actively interrupt Desktop-owned tasks. Unknown states are not shown as idle.

Connections are not restricted by Codex Desktop version numbers. Availability depends on the actual IPC handshake and support for each operation. Private protocols can change with Desktop updates; a successful connection does not guarantee compatibility with every operation.

## Development

```sh
npm ci
npm run dev:ui
```

The stack is **Tauri + Rust + Vue**. Rust manages connections, configuration, MCP, and Desktop communication; Vue runs in the system WebView. Installers contain no Node, npm, Rust, or Cargo runtime. Node is used only for source development and frontend builds.

Development requires Node 24.12+, stable Rust, and the platform SDK. The Vite development UI shares the running built app's backend and can change its configuration, connections, and tasks. Open the built app first; development does not start a separate backend.

Documentation:

- [Installation and updates](docs/installation.md)
- [Release maintenance](docs/release.md)
- [Changelog](CHANGELOG.md)
- [Development and builds](docs/development.md)
- [Desktop lifecycle](docs/desktop.md)
- [MCP tools and task boundaries](docs/tools.md)
- [Architecture](docs/architecture.md)

Local data defaults to `~/.local/state/chatgpt-local-connector` on macOS or `%LOCALAPPDATA%/chatgpt-local-connector` on Windows. Override it with `CLC_STATE_DIR`. Keys, receipts, and logs are stored locally; removing the app does not remove Codex history.

Licensed under [MIT](LICENSE). Source and desktop installers are distributed through GitHub. No public npm package is published.
