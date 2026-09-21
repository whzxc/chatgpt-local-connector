# Architecture

Vue + Tauri + Rust provides a single native core for connection state, MCP requests, idempotent receipts, and user logs. The frontend runs in the system WebView; packages include no Node/npm/node_modules.

Connections use either official Tunnel or HTTPS MCP. The official Tunnel Client is downloaded and managed separately. In HTTPS mode, Rust exposes an independent unauthenticated Streamable HTTP MCP listener. The app manages Cloudflare Quick Tunnel, named Tunnel, or ngrok clients, or accepts a user-managed reverse proxy. The provider/proxy handles TLS and public forwarding. Both entry points share dispatch, approvals, and receipts and cannot run simultaneously. Codex Desktop owns task execution by default, controlled through Desktop IPC. Disabling Automatically open Codex tasks gives new tasks to Connector's app-server without opening Desktop. Ownership is persisted per task; toggling the setting does not migrate existing tasks. Reads, continuation, and interruption retain the original path.

The 42 tool definitions in `native/src/catalog.json` are embedded in Rust builds. JSON Schema validates parameters, and writes have persistent receipts. Large outputs support pagination. Persistent events are separate from UI logs, which omit routine protocol handshake noise.

Desktop access has no app-version allowlist; actual protocol and task ownership checks determine compatibility. External task management supports macOS and Windows. See [desktop lifecycle](desktop.md) and [MCP tools](tools.md).

Task approval is an optional confirmation flow controlled locally or from the cloud. Request content, approval decisions, and submission state share persistent receipts. Tasks groups requests by threadId and reads native runtime snapshots without parsing conversation JSONL. Approval settings persist independently, can change while connected, and do not change Codex execution permissions.

`native/src/waiter.rs` provides a shared read-only waiting engine. The Codex adapter stays attached to the task's App Server or Desktop owner. Notifications wake the waiter; fresh native snapshots determine state, with a two-second fallback check. Truncated buffers, missing notifications, and coalesced events do not affect the final classification. Reads and waits do not hold a global lock. Cancelling drops the wait future without interrupting the turn. Connector shutdown notifies waiters through a watch channel.

## Agent boundaries

`Service` sends `agent_*` routes to `AgentHost`; existing `codex_*` routes still go directly to `Control`. The Host owns common task identity, persistent receipts, confirmation, and lifecycle. `AgentDriver::CodexNative` adapts shared arguments/results while reusing Control; it preserves Desktop ownership, schema, events, account, plugin, and skills semantics.

`agents/builtins.json` and user manifests share a strict Manifest type for launch arguments, discovery, version probing, and compatibility. Static descriptions do not claim negotiated capabilities. `AgentDriver::Acp` shares descriptor-driven discovery/startup and negotiates ACP v1; `AgentDriver::Pi` uses official RPC. They share bounded JSONL transport while handling responses and completion separately. Each task has its own process; raw sessions and CLC identities persist separately. Restarting a process never automatically replays unknown writes. See [Local Agents](agents.md) for capabilities and limits.

`agent_wait` and `codex_wait` share waiter timeouts, event wakeups, periodic checks, snapshot hashes, and condition evaluation. Codex delegates to its native wait entry point. Pi/ACP use a lightweight `WaitSource` over existing Host/Driver process observations without starting or resuming tasks. Pi checks `get_state`; ACP uses live process protocol state and the final prompt response. Waiting holds no global lock. MCP cancellation and disconnect release only the current wait.
