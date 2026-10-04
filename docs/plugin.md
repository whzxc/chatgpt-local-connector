# Local Connector for ChatGPT Desktop

**English** | [简体中文](zh-CN/plugin.md)

## Choose how to use CLC

| Your goal | Install/configure | Needed for this route |
| --- | --- | --- |
| Use local projects, Agent tools and usage panels inside ChatGPT Desktop | This standalone plugin | A host that supports local plugins; external Agents only when you use them |
| Use the tray, floating panels or manage connections and Agents | [Connector Desktop](installation.md) | The desktop installer |
| Reach this computer from ChatGPT Web, mobile or another device | [Remote ingress](tunnel.md) | A running local Core, an authenticated Tunnel/HTTPS ingress and client-side connection |

The local plugin includes its native backend. It does not require Connector Desktop, Node, npm, Rust, a Tunnel or a Tunnel API key. It does not include external Agents or their accounts. Local usage panels read Codex logs; support for other Agents' task tools does not mean their token usage is included.

## Install from the Git marketplace

The dedicated `whzxc/clc-plugins` repository keeps marketplace `local-connector` and plugin identity `clc@local-connector`. Its `stable` branch changes only with formal releases and includes native Apple Silicon macOS and Windows x64 executables. Git must be available to the host.

```sh
codex plugin marketplace add whzxc/clc-plugins --ref stable
codex plugin add clc@local-connector
codex plugin list --marketplace local-connector --json
```

Update with `codex plugin marketplace upgrade local-connector`, then reload the host and reopen the panels. Refresh reconciles configured plugins into the installed cache; running MCP processes keep their previous binary until reloaded. No host restart or task interruption is automated.

For an existing local ZIP installation, run `<plugin or Desktop bundled executable> cli plugin migrate-git` from a version that provides this command. It replaces the same-name source through the host, preserves enablement, retains the original package and data, and attempts to restore the local source on failure. Older versions can use the `marketplace add` command above directly; omit `plugin add` for an existing installation to preserve disabled state.

If the old `.mcp.json` has explicit `env` values, first move them to the host process environment and remove the old `env` block; supported names are listed in `env_vars`. The migration command stops without changing the installation when explicit values remain. This prevents losing an isolated data directory or proxy configuration. Personal configuration does not belong in the Git bundle or host cache. Close old clients before migration; Desktop and plugin clients sharing state must use matching builds.

Git installations are updated by the host; Desktop synchronization and CLC's download updater do not replace them. To pin or roll back, register the marketplace with `--ref v<version>` instead of `stable`; use `--ref stable` to resume tracking releases. Close shared Core clients and check persistent-format compatibility before downgrading.

## Install from a ZIP

1. From [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases), choose a release containing your platform's plugin ZIP and `SHA256SUMS.txt`. Available files on that release are authoritative. If it has no plugin ZIP, use another release that includes one or follow [source development](plugin-development.md); a desktop installer is not a plugin package.

   | Platform | Plugin file |
   | --- | --- |
   | Apple Silicon macOS | `CLC.Plugin_<version>_darwin-aarch64.zip` |
   | Windows x64 | `CLC.Plugin_<version>_windows-x86_64.zip` |

2. Compare its SHA-256 with the matching line in `SHA256SUMS.txt`. On macOS use `shasum -a 256 <zip-file>`; in PowerShell use `Get-FileHash <zip-file> -Algorithm SHA256`. Extract to a permanent folder, preserving hidden files and executable permissions. The root must contain `.agents/plugins/marketplace.json` and `plugins/clc`. Select this root, not the ZIP or its `plugins/clc` subfolder.
3. Use the host's local marketplace installation flow if available. With a compatible host CLI, run:

   ```sh
   codex plugin --help
   codex plugin marketplace add "<extracted-marketplace-root>"
   codex plugin add clc@local-connector
   codex plugin list --marketplace local-connector --json
   ```

   Replace the path placeholder. On PowerShell the same commands work when `codex` is on PATH. Confirm the list reports `installed: true` and `enabled: true`. Disable an existing CLC development variant in the host before using the installed variant.
4. Reload the host's plugins, or restart ChatGPT when active work can safely be interrupted, then open the panels below. Installation and backend startup alone do not confirm the panel loaded successfully.

If `plugin` is an unknown command, use the CLI bundled with the same host or a host version that supports local plugins. A global CLI may be older than the app. For ChatGPT installed at the standard macOS location, the bundled CLI can be called as:

```sh
"/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex" plugin --help
```

Use that executable in place of `codex` in the commands above. App names, installation paths and menu labels can differ. On Windows, use the bundled executable from the installed host's directory rather than guessing a Store package path. Workspace policy may disable local plugins; a CLI cannot override that policy.

If macOS or Windows blocks the downloaded executable, first verify its origin and hash, then follow the operating system's per-file authorization or your administrator's policy. The plugin archive is not a notarized macOS application or an Authenticode-signed Windows installer; do not disable system-wide protections.

## First use

- Open **Connector 总览** from the host's Explore/sidebar. The first opening loads the selected range; later refreshes reuse incremental checkpoints. Existing readable Codex logs are sufficient; there is no need to run a new task solely to populate them.
- Open **任务用量** through a local task's **New tab → More tools**, or select a task in the overview. If the host cannot bind a task, use **Select task** beside the title. Cloud-task hosts can reject a local panel before it loads; a task selected in the overview still represents local logs only.
- With no local history, complete a local turn in a signed-in Codex client and refresh. The panel does not create tasks for you. Clear search if it excludes your records. Search covers all indexed tasks; pages default to 10 tasks, with 5, 10, 20, 50 or 100 selectable.
- To use task controls, install/sign in to the intended Agent first. Ask CLC to list local projects, available Agents and existing tasks; explicitly specify which task to create or continue. A usage panel opening is not verification of Agent execution.
- The header's **Help** opens Get started, Scope and Troubleshooting. Module information icons explain their metrics; the data-source details show coverage issues. Help is bundled and works without a documentation website.

Account quota is separate from local log statistics. Check the relevant native client's sign-in if quota is missing. If a subscription source was disabled, enable it in Connector Desktop's usage settings. Missing quota does not block local history and does not mean a full remaining allowance.

## Update or roll back a local ZIP installation

With Connector Desktop installed, starting an updated release synchronizes an older, enabled `clc@local-connector` plugin through the host CLI. Settings → About → Plugin updates shows the result and offers a retry. It does not install an absent plugin or enable a disabled one. Close old plugin clients when their work finishes before updating a shared Core.

Plugin-only installations can check and update using `connector_plugin_check` and `connector_plugin_update`, or run `<plugin executable> cli plugin check` then `cli plugin update <version>`. These operations work without Connector Desktop or a compatible running Core. A plugin with no update support needs a one-time manual install of a release that includes it, or a Desktop synchronization.

Downloads use the same release signing key as Desktop. The verified native payload exports its bundled manifest, skills and UI; the host CLI installs the full plugin and its cache. Explicit environment values supported by the plugin, including an isolated data directory and proxy settings, are preserved. The managed update directory keeps the current and previous package, preserves the original manually extracted package, and removes the temporary download. Metadata is cached for one hour. Updating never edits the host cache directly, restarts the host or interrupts Agent tasks. After installation, reload the host and reopen the panels; an existing session still runs its old binary. If you also use Desktop, keep both on the same release/build. Network, signature or host errors remain retryable; installation failures attempt to restore the previous marketplace source and report any recovery failure.

For manual installation or rollback, preserve the previous package and use the following steps:

1. Let active Connector-owned work finish, then close old plugin clients/panels and any Connector Desktop sharing the same state. Matching version numbers are insufficient if the native builds differ. Do not force-stop unrelated tasks or delete state to bypass a mismatch.
2. Extract the desired package to a new permanent folder. If you are changing the source folder, replace the marketplace registration:

   ```sh
   codex plugin marketplace remove local-connector
   codex plugin marketplace add "<new-extracted-marketplace-root>"
   codex plugin add clc@local-connector
   codex plugin list --marketplace local-connector --json
   ```

   Removal here changes the marketplace registration, not your projects or original history. If keeping the same source path, replace its package only after old clients close and run `plugin add` again. Merely replacing extracted files does not refresh the installed cache.
3. Reload/restart the host and reopen both panels. Confirm data loads and manual refresh works. For rollback, repeat using the preserved previous package; any Desktop sharing state must match that build. Older releases are not guaranteed to read newer persistent formats, so preserve state backups and consult that release's documented support before downgrading. Never restore over a live state directory.

## Troubleshooting

| Symptom | Next action |
| --- | --- |
| No plugin ZIP on the release | That release does not provide this installation route; do not register the source template or a DMG |
| `plugin` command not found | Check the CLI bundled with the host; use a host supporting local plugins |
| Marketplace already added from another source | Use the remove → add → install sequence above |
| No panel entry or duplicate tools | Confirm the plugin is enabled, disable the other CLC variant, reload the host |
| Still seeing an old UI | Reinstall the package into the host cache, reload and reopen the panel |
| Loading a large range or task | Wait for the selected sources to finish reading; hidden panels pause UI refresh |
| No records or unbound task | Check readable local Codex history, clear filters or explicitly select a task |
| Connection timeout | Retry; it reconnects after failed initialization. Then check enablement and reload the host |
| Build/version mismatch | Close old clients when their work can stop and use matching plugin/Desktop builds |
| Missing quota | Check native sign-in and subscription-source settings; do not interpret missing data as 100% |

When asking for help, include OS/architecture, host version, plugin version, panel name and the displayed error. The source-details panel provides backend version, observation time and coverage when data is available. Share only the necessary error excerpt; credentials and private task content are not needed for installation diagnosis.

## Uninstall

Disable or remove Local Connector in the host and close its panels. Remove the `local-connector` marketplace registration if no other plugin uses it, then delete its extracted package. Existing projects, external Agent installations and original Codex history are separate.

Connector state is retained by default: `~/.local/state/chatgpt-local-connector` on macOS or `%LOCALAPPDATA%/chatgpt-local-connector` on Windows. `CLC_STATE_DIR` can select another directory. Do not remove shared state while another plugin/Desktop uses it. Removing a plugin does not require clearing configuration, receipts or log checkpoints.

## Runtime and ownership

One Core holds a state directory under an OS file lock. Each local plugin process or Connector Desktop holds a renewable lease. Multiple chats and panels share that Core and its collector, AgentHost, receipts and task namespace. Connector Desktop connects to it and retains ownership of native windows, tray and floating usage panel. Only one Desktop may use a state directory at a time.

Closing one panel or chat does not stop a Core still used by another entrypoint. A host reload reconnects with a new lease. After the last lease closes, Core shuts down; a crashed entrypoint's lease expires after ten seconds. A short startup grace permits initial connection. No permanent system service is installed. Connector-owned execution stops with Core; tasks owned by the external Codex Desktop retain that owner. Keeping the plugin installed does not promise background execution after its host exits. Use Connector Desktop for a persistent tray entrypoint, or a separately configured remote ingress for Web/mobile access.

The default state directory is shared with Connector Desktop. `CLC_STATE_DIR` can isolate development or another instance. The Core is authenticated over loopback, and clients rediscover its port and token. Development and release builds with different native identities must use separate directories or close all old entrypoints before switching. Desktop updates refuse to replace a Core used by other local clients.

Overview queries index only sources relevant to the selected Today, 7-day or 30-day range. For older unchanged logs, the task catalogue reads session identity and trailing event timestamps instead of parsing all message bodies; files with missing catalogue information are scanned normally. File modification time and trailing event time both participate in range selection, and copied files with recent events remain eligible. Statistics decode only usage, model and turn metadata. Opening a task independently indexes that task's full trajectory; it does not wait for subscription history or another overview range. Complete-line offsets and source fingerprints support incremental reads, with separate compressed checkpoints for statistics and task details sharing a 40 MiB budget. Queries reuse a recent completed view for up to four seconds; a visible panel refreshes every ten seconds without overlapping automatic requests, and a hidden panel stops polling. Large or rewritten sources may still require longer reads. Closing Core cancels collection. Detailed usage travels in tool-result `_meta`, outside model-visible text. Host colors/fonts and Connector's shared controls are reused.

## Scope and interpretation

Only readable local `sessions` and `archived_sessions` JSONL files under the native Codex home are indexed. This does not cover all cloud or other-device activity and does not establish account attribution. Account quota is a separate cached native reading, refreshed using Core's subscription settings. Unknown/stale quota is not 100% remaining.

Task labels use native task names from the local Codex state database and session index, then the native title. Tasks without name metadata display a short ID. The task list is sorted by latest local event first, with server-side pagination defaulting to 10 tasks per page (5, 10, 20, 50 or 100 selectable); changing the statistics period preserves this order. Turn counts deduplicate native turn IDs observed during the selected period, including start/completion events and responses; missing turn metadata is shown as unavailable. The task page presents a continuous trajectory below the always-visible nine-cell summary. All turn headers are visible and initially collapsed; expanding a turn loads its complete call timeline. The turn navigator scrolls horizontally when needed. Navigator hover previews turn summaries; clicking scrolls to and expands that turn. Request rows preview associated assistant output or a tool operation excerpt. Selecting a call shows context, usage and full recorded assistant output inline. Message association uses the next same-turn usage record without crossing compression boundaries; complete outbound request bodies are unavailable. Tools are grouped with a request when their emission-to-completion interval contains exactly one reliable response in the same turn without an intervening compression event. One request can contain multiple tools; ambiguous and unfinished calls remain separate. Grouping preserves request token/cost totals and tool byte totals independently. Tool Details loads original input/output on demand in a dialog; the turn header’s Prompt label opens full prompt text. Turns with compression use amber number markers. Plaintext reasoning summaries are included in the next same-turn request inspector when no compression boundary intervenes; they do not add timeline nodes. Consecutive identical summaries within a request are combined with their count, time range and recorded duration. They are read on demand for the expanded turn and deduplicated by item ID; empty summaries and encrypted payloads are omitted. Compression rows show time and recorded duration without expanding. Context uses observed request input and recorded model window capacity, never cumulative billed tokens. Tool return bytes cannot be converted directly into context occupancy.

Current-thread binding requires consistent `threadId` / `thread_id` request metadata and an exact native log identity. Missing, conflicting, helper-thread or unavailable identities show unknown and offer explicit selection. An anonymous session or widget ID is not a task ID. Selection is local to each panel; there is no latest-task fallback.

Modern `token_usage_record` rows use thread plus response identity. Thread/turn cumulative values, legacy `token_count` and compaction references are not added to the response family. Legacy-only logs use cumulative differences with explicit baseline/coverage limitations. A counter regression freezes that ambiguous legacy domain; it does not restart counting a replayed suffix. Conflicts remain unreliable across archived copies. A confirmed subtotal can be shown separately when the full total is unknown. Child and fork tasks are shown separately; unverified inherited boundaries are not aggregated as an inclusive parent total. Interrupted, empty and missing-log tasks can have unknown usage.

Cached input is part of input; reasoning output is part of output. Total is input plus output. Missing fields remain unknown. The overview uses event timestamps inside the selected interval: Today starts at midnight in the Connector device’s local time; 7 and 30 days are rolling intervals. Daily groups use the same local time. A task's lifetime is separately labeled. Whole-turn average output includes tools and waiting. TTFT/duration are shown only when recorded; resolved model, pure generation speed, task credits and exact tool charges can remain unknown. Tool return size and structured status are observations, not causal billing or proof of waste. Completion is not business acceptance.

Oversized compaction records are streamed for event metadata without loading their embedded replacement history. Other malformed/oversized lines, legacy boundaries and conflicting records are shown as coverage issues. The collector detects append, truncated/replaced files and same-size changes with a changed modification timestamp; head/tail anchors guard checkpoint recovery. Arbitrary in-place rewrites preserving size, timestamp and boundary anchors cannot be detected. Removed source history cannot be reconstructed as complete history. Statistics checkpoints contain counters, IDs and event metadata, never prompts, tool arguments or outputs.

The overview combines the shared account quota card (plan, reset time, available resets and a positive credits balance) with token, observed-request, estimated-cost and input-cache-hit summaries. A Recharts stacked bar chart switches between tokens, estimated cost and requests, split by model; Today uses hourly buckets and longer ranges use local dates. Cost uses the existing reference-price calculation and is API-equivalent, not subscription billing or credits consumption. Unpriced records are excluded and marked; legacy cumulative increments are not counted as observed requests.
