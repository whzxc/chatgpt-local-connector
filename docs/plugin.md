# ChatGPT Desktop local plugin

Local Connector's plugin ships its own Rust executable. ChatGPT Desktop starts it directly; installing or opening Connector Desktop is optional. Project tools and Agent operations use the same Core, receipts, task ownership and statistics as Connector Desktop. External Agents still require their own installation and native login.

- **Connector 总览** opens device-local event-time usage, task rankings and model dimensions, with independently scoped account quota readings when configured.
- **任务用量** opens from a local task's New tab → More tools menu. Missing or conflicting thread metadata requires explicit local task selection. Some cloud-task hosts fail before loading a local MCP App; select a local task from the overview in that case. Those statistics do not describe the cloud task.
- The workflow skill uses the plugin's local project and Agent tools. Remote MCP App connections remain separately authenticated ingresses for Web, mobile and other devices.

## Install and update

Download the matching `CLC.Plugin_<version>_darwin-aarch64.zip` or `CLC.Plugin_<version>_windows-x86_64.zip` from the project's GitHub release. Check it against `SHA256SUMS.txt`, then extract it into a permanent directory. Keep hidden files. The archive contains `.agents/plugins/marketplace.json`, `plugins/clc`, the native executable, embedded panel resources, the workflow skill, icon and LICENSE. No Node runtime is required.

Register that extracted marketplace root using the host's local marketplace flow, then install **Local Connector**. A compatible host CLI also supports:

```sh
codex plugin marketplace add <extracted-marketplace-root>
codex plugin add clc@local-connector
```

Use the CLI shipped with the same host when an older global CLI lacks plugin commands. Host menus and plugin availability depend on its version and workspace policy. Installation may copy the source into a host cache; editing an exported source does not modify an already running MCP process.

For an update, close this plugin's panels and MCP processes and any Connector Desktop using the same state directory. Extract the new archive, update the marketplace source and install it again, then reload the host plugin and reopen panels. If the host offers no plugin reload, restart it only after its work can safely be interrupted. Reinstallation updates the cache; panel reopening refreshes the versioned resource. Desktop and plugin must use the same native build when sharing data. A mismatch is reported before starting another Core; an old owner is never silently replaced.

A built standalone executable can also export a plugin folder:

```sh
local-connector export <new-plugin-directory>
```

The export copies the executable into `bin`, writes a relative `.mcp.json` command, and can replace a recognized CLC export. It rejects unrelated directories. Exporting does not register a marketplace or install the plugin. The source repository's template does not contain a platform executable and cannot be used as a complete installed plugin.

## Development on macOS or Windows

Install Node 24.12+, npm and stable Rust, then run:

```sh
npm ci
npm run plugin:dev
```

This builds the native entrypoint, starts the local Core and Vite at `http://127.0.0.1:5187`, exports a repeatable `clc-dev` marketplace, and installs it with the host CLI. Keep this command running while developing, then open the plugin in ChatGPT Desktop. React/CSS changes use Vite HMR, including its WebSocket; host CSP must allow the explicit loopback HTTP/WS origin. Development HTML uses absolute script addresses without a `<base>` element. Production resources have no external domains. Rust, manifest and skill changes require restarting this command, reinstalling/reloading the host plugin and reopening its panels. They are not in-process hot replacement.

Development uses an isolated persistent data directory ending in `chatgpt-local-connector-dev` and a generated marketplace under the Codex home. `CLC_STATE_DIR` and `CLC_PLUGIN_DEV_DIR` override those locations. The development manifest uses an absolute build executable and the Vite origin outside the repository. Close an existing Vite on port 5187 before starting. Stopping the command closes its lease; an active host plugin can still retain its own Core lease. Reopening a development panel while Vite is unavailable loads the embedded UI with a status message. An already cached development page may require reopening; script loading failures show an actionable message. Development and embedded resources use different URIs.

```sh
npm run plugin:build                 # native release ZIP and checksum
npm run plugin:build -- --debug      # debug artifact, embedded UI
npm run plugin:check -- <plugin-directory>
```

Build output is under `dist/plugin-package`. Packaging checks the version, executable, MCP config, tools, embedded UI, standalone Core and last-client shutdown, then extracts and checks the ZIP again. These checks do not establish acceptance of ChatGPT's real panel sandbox, CSP, HMR or Windows host behavior; exercise those in the target host separately.

## Runtime and ownership

One Core holds a state directory under an OS file lock. Each local plugin process or Connector Desktop holds a renewable lease. Multiple chats and panels share that Core and its collector, AgentHost, receipts and task namespace. Connector Desktop connects to it and retains ownership of native windows, tray and floating usage panel. Only one Desktop may use a state directory at a time.

Closing one panel or chat does not stop a Core still used by another entrypoint. A host reload reconnects with a new lease. After the last lease closes, Core shuts down; a crashed entrypoint's lease expires after ten seconds. A short startup grace permits initial connection. No permanent system service is installed. Connector-owned execution stops with Core; tasks owned by the external Codex Desktop retain that owner. Keeping the plugin installed does not promise background execution after its host exits. Use Connector Desktop for a persistent tray entrypoint, or a separately configured remote ingress for Web/mobile access.

The default state directory is shared with Connector Desktop. `CLC_STATE_DIR` can isolate development or another instance. The Core is authenticated over loopback, and clients rediscover its port and token. Development and release builds with different native identities must use separate directories or close all old entrypoints before switching. Desktop updates refuse to replace a Core used by other local clients.

The collector polls complete local log lines every five seconds. A visible panel refreshes every ten seconds without overlapping automatic requests; a hidden panel stops polling. Initial indexing returns a collecting state while other tools remain available. Closing Core cancels collection. Detailed usage travels in tool-result `_meta`, outside model-visible text. Host colors/fonts and Connector's shared controls are reused.

## Scope and interpretation

Only readable local `sessions` and `archived_sessions` JSONL files under the native Codex home are indexed. This does not cover all cloud or other-device activity and does not establish account attribution. Account quota is a separate cached native reading, refreshed by the main app's existing subscription settings. Unknown/stale quota is not 100% remaining.

Current-thread binding requires consistent `threadId` / `thread_id` request metadata and an exact native log identity. Missing, conflicting, helper-thread or unavailable identities show unknown and offer explicit selection. An anonymous session or widget ID is not a task ID. Selection is local to each panel; there is no latest-task fallback.

Modern `token_usage_record` rows use thread plus response identity. Thread/turn cumulative values, legacy `token_count` and compaction references are not added to the response family. Legacy-only logs use cumulative differences with explicit baseline/coverage limitations. A counter regression freezes that ambiguous legacy domain; it does not restart counting a replayed suffix. Conflicts remain unreliable across archived copies. A confirmed subtotal can be shown separately when the full total is unknown. Child and fork tasks are shown separately; unverified inherited boundaries are not aggregated as an inclusive parent total. Interrupted, empty and missing-log tasks can have unknown usage.

Cached input is part of input; reasoning output is part of output. Total is input plus output. Missing fields remain unknown. The overview uses event timestamps inside the selected rolling interval; a task's lifetime is separately labeled. Whole-turn average output includes tools and waiting. TTFT/duration are shown only when recorded; resolved model, pure generation speed, task credits and exact tool charges can remain unknown. Tool return size and structured status are observations, not causal billing or proof of waste. Completion is not business acceptance.

Malformed/oversized lines, legacy boundaries and conflicting records are shown as coverage issues. The collector detects append, truncated/replaced files and same-size changes with a changed modification timestamp; head/tail anchors guard checkpoint recovery. Arbitrary in-place rewrites preserving size, timestamp and boundary anchors cannot be detected. Removed source history cannot be reconstructed as complete history. Statistics checkpoints contain counters, IDs and event metadata, never prompts, tool arguments or outputs.

## Uninstall

Remove Local Connector from the host's installed plugins and close its panels. Remove the marketplace entry only if no other plugin uses it. Its extracted source directory can then be deleted. Connector Desktop, external Agent installations, original logs and task records remain separate. Remove generated checkpoints under the state's `usage/files` directory only when every local entrypoint is closed; the next start rebuilds them from readable logs.
