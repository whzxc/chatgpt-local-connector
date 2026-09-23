# Development and builds

## Everyday development

Use Node 24.12+, npm, stable Rust, and the macOS/Windows platform SDK. Node runs development tools only; it is not a product runtime dependency.

```sh
npm ci
npm run dev:ui
```

Vite listens on `http://127.0.0.1:5187`. Vue/CSS edits hot-reload without packaging, installing, or restarting the built app. The authenticated local proxy shares the built app's connections, configuration, and tasks: saves, connection changes, and approvals affect its real backend immediately. Open the built app first. An unavailable backend produces an error rather than starting a fallback service. The proxy retains local Host, Origin, and write-request checks and does not automatically retry writes.

For native window development, use `npm run desktop:dev`; Tauri incrementally compiles Rust changes. The development window forwards reads and writes to the built app and owns no separate connection. Rebuild and run the built app to exercise backend changes. Updates are still handled by the built app.

## Code layout

- `ui/`: Vue pages, state types, and interactions.
- `native/`: Rust core for Desktop IPC, auxiliary Codex RPC, AgentHost and Pi/ACP processes, 42 MCP tools, receipts, events, configuration, and Tunnel lifecycle.
- `desktop/`: Tauri entry point, tray, windows, OS integration, and updates; calls the in-process Rust core.
- `tooling/`: development/build scripts, excluded from runtime resources.
- `tests/`: contract tests against the Rust core with an isolated simulated upstream; the test-only feature is excluded from release builds.

The native executable's `stdio` subcommand forwards MCP requests to the connection-owning main process. It receives a local port and random credential through its parent environment and exposes no public interface. Connection configuration and credentials are not stored in browser storage.

Its `cli` subcommand provides configuration and diagnostics through the existing authenticated local transport without starting a second service. `cli guide` embeds the English `docs/codex-setup.md` and ships with the app version. See that guide for command conventions.

## UI languages

`ui/i18n.ts` configures Vue I18n in Composition API mode. Vue I18n handles reactive translation, interpolation, and fallback; VueUse handles browser language detection and persistent preferences. English is the default and fallback. `ui/locales/en.json` defines typed keys; `zh-CN.json` supplies Simplified Chinese. Language uses the same localStorage preference mechanism as theme and notifications. Missing or invalid preferences select Auto; unsupported system/browser languages resolve to English. Manual choices take priority. WebView and browser previews have separate storage origins.

Use `t(key, params)` for display text, with whole messages and named placeholders. Put derived label dictionaries in computed values so switching language updates them. Dates use the resolved locale; filters and protocol state retain stable identifiers. `ui/messages.ts` translates recognized CLC messages at the rendering boundary, including retained feedback and service errors. Unknown third-party diagnostics remain verbatim. Do not apply it to user task content, identifiers, or entire API responses.

The WebView sends its resolved locale to `desktop/src/i18n.rs`; the tray uses the same JSON resources and refreshes through its existing polling loop (normally within two seconds). It defaults to English until the WebView reports its preference. This state is presentation-only: the service, MCP schemas/descriptions, error codes, and Agent/Codex contracts never read it. Text copied into ChatGPT as a model instruction or connection description stays English regardless of UI language.

To add a language, add its JSON resource with matching keys/placeholders, register the locale, option and language matching in `ui/i18n.ts`, and extend the tray resource selection in `desktop/src/i18n.rs`. Missing translated keys fall back to English. Check both languages' rendered pages, long labels, Auto/manual switching, persisted and invalid preferences, dates, retained errors, and tray labels. README and the three core user guides have English canonical versions and corresponding `zh-CN` translations; link each translation to its source. No translation pipeline is required.

## Extending ACP Agents

Add descriptions to `native/src/agents/builtins.json` after verifying the official entry point, version requirements, authentication ownership, and limits. Built-ins and Custom manifests share `manifest.rs`. Static descriptions are not negotiated capabilities; do not duplicate Driver/wait logic by brand. Discovery rules can specify aliases with identical arguments, installation locations, version probes, and CLI help conditions. Different argument entry points need separate Custom manifests. Keep built-in IDs unique and update the Agents support matrix and existing tool/list expectations; do not add test files or cases.

For real calls, use an isolated `CLC_STATE_DIR` and temporary working directory with the currently compiled AgentHost/transport. An installed old backend does not validate current source. Run harmless tasks only with existing authentication; do not automatically sign in or change providers. Retain receipts and use agent_wait to verify output, continuation/resumption, cancellation, and safely triggered permission interactions. Report missing installation or authentication as verification limits. Keep temporary evidence outside the repository; documentation describes current support and scope.

## UI controls

`ui/tokens.css` defines desktop control sizing. Buttons, single-line inputs, and selects in `ui/style.css` use a 28px height, 12px font, 18px line height, 6px radius, and 10px horizontal padding. Textareas share typography and radius with content-appropriate heights; switches have their own shape.

Settings uses `SettingsGroup` and `SettingsRow`. Add layout and necessary widths rather than locally overriding control height, typography, radius, or vertical padding. Change shared tokens to adjust density, then inspect Settings and Tasks rendering.

## Checks

```sh
npm run check
npm test
npm run check:native
npm run build
cargo build --manifest-path desktop/Cargo.toml
cargo fmt --manifest-path native/Cargo.toml -- --check
cargo fmt --manifest-path desktop/Cargo.toml -- --check
```

Rust outputs are generated under the ignored `desktop/target`, `native/target`, and
`tooling/verifier/target` directories. The npm test, Desktop check, dev, and
build commands prune the oldest inactive output groups when the local total
exceeds 10 GiB; the current command's output and packaged app/installers under
`bundle` are retained. CI skips this local
cleanup. The contract fixture uses a stable Cargo project path, and dev builds
use limited debug information without incremental compilation to avoid a new
cache for every test run or source change. Direct `cargo` commands bypass the
automatic check; run `npm run cache:prune` after them when needed. To reclaim
all Rust outputs, run `cargo clean --manifest-path desktop/Cargo.toml` and
`cargo clean --manifest-path native/Cargo.toml`; the next build recompiles them.

Desktop IPC task management supports macOS Unix sockets and Windows named pipes. Windows uses the installed Microsoft Store Codex Desktop.

## Release builds

```sh
npm run desktop:build
npm run check:package
```

`desktop:build` rebuilds the frontend before native packaging; a prior `dist/ui` directory is not reused. macOS installers target Apple Silicon (arm64) and are written to `desktop/target/aarch64-apple-darwin/release/bundle/`. macOS builds need `uv` to run a pinned dmgbuild version for the drag-to-install layout without text; build dependencies are excluded from the app. `check:package` checks for Node, npm, node_modules, and old runtime directories and reports size; it accepts another artifact directory. Windows builds use NSIS/MSI.

Packages contain the native executable, frontend static resources, icons, and applicable third-party license notices. Build scripts remap local user/repository paths in Rust source to generic build paths, avoiding private paths in binaries. Official Tunnel Client is downloaded and verified separately on first use. Codex uses the binary bundled in the user's Desktop installation rather than packaging another copy. Building does not overwrite the installed app.

Keep versions synchronized across `package.json`, `native/Cargo.toml`, `desktop/Cargo.toml`, and Tauri configuration. Update URLs and the project public key are fixed in `desktop/tauri.conf.json`; release builds need `TAURI_SIGNING_PRIVATE_KEY` outside the repository. See [release maintenance](release.md).

## Subscription development

`npm run dev:ui` renders the same usage rail component inside the browser
viewport when subscription monitoring has selected providers. Its position is
saved in browser local storage. Drag the capsule or a ring to move it; hover or
focus a ring to inspect quota details, and click it to open Agent quota details.
The browser adapter supplies viewport layout and pointer state; the desktop
and browser share the rail renderer, shapes, animations, and quota components.
Drag into a viewport edge zone to preview left/right/top/bottom docking; drag
back into the interior to restore the floating capsule. Docked rails collapse to a sliver after the pointer leaves and expand when the
pointer returns to the edge wake zone. Floating rails and active drags stay expanded. This browser-only interaction does not move or configure
the native panel. Native screen management and notch behavior require the desktop window.

The usage rail is built from `ui/usage-rail.html`. Desktop views receive
`subscriptions:changed` events; browser previews receive snapshots through the authenticated
`/api/subscriptions/events` SSE route, proxied by Vite. Each completed provider refresh
publishes independently. Browser reconnection follows the native backend owner; polling
is a fallback when the stream is unavailable. Local token scans run with quota refreshes,
not a continuous log watcher.

 Subscription snapshots expose
`rawUsage`, the original successful quota API response for each provider. Authentication
files, request headers, and tokens are not part of this payload. The core retains
normalized quota windows for scheduling, expiry, and native alerts; display-only
fields such as plan names and available resets are extracted by the shared frontend
adapter in `ui/subscriptions/response.ts`. Changing that extraction does not require
a new native build when the existing endpoint already supplies the data. Account
changes clear both the normalized reading and the raw response. New native routes need
a matching build owner. For an isolated desktop check, use the existing Tauri
build `--config` override with a distinct application identifier/product name,
launch that build with `--state-dir` pointing to a separate private directory,
and leave ingress configuration disabled. A development window or Vite preview
may forward to that isolated owner through `CLC_STATE_DIR`. Do not point an old
production owner at new routes and silently start a second development core.


### Subscription sources and usage estimates

Cursor reads the desktop login from its local state database (or macOS Keychain), then
queries DashboardService GetPlanInfo and GetCurrentPeriodUsage. REST usage-summary is
a quota fallback. Free is a reported plan, not an inference from missing usage. Expired
Cursor credentials require refreshing the login in Cursor.

Antigravity discovers the running language server or agy CLI and reads quota summaries
over loopback. On macOS it can also use the existing gemini/antigravity Keychain OAuth
access token with Google Cloud Code. Token renewal is handled by Antigravity; CLC does
not bundle OAuth client credentials. If the token expires, open Antigravity and refresh
its login. Only loopback language-server TLS permits a self-signed
certificate; remote services retain certificate validation. Windows supports running
language-server discovery; closed-app Keychain fallback is macOS-only. Quota buckets
remain separate for Gemini and third-party models, with 5h and weekly windows. Legacy
model responses supply only 5h windows; missing fractions never become a full allowance.

Quota-less successful responses retain their raw data so that plan names remain available.
Agent management groups Cursor subscription information with Cursor Agent, and Antigravity
subscription information with Gemini CLI. Execution keeps the original adapter identifiers.
Usage settings select which history periods appear in both the rail and detail panels.

Codex local rollout accounting, Antigravity conversation SQLite accounting, and Cursor's
token CSV export supply Today, Yesterday, Last 7 Days, and Last 30 Days (including today). Local sources describe this device's logs, not a complete current-account invoice.
Only token accounting is retained; prompts and responses are not exposed. Changed local
files are rescanned, bounded records are used, and skipped records mark estimates incomplete.
Codex child replay and duplicated rollout events are excluded.

Dollar values are API-equivalent USD estimates, not subscription charges. Input, cache
reads, cache writes, and output are priced separately; Codex priority and long-context
rules apply to individual requests. Unknown models retain token counts and mark the
estimate incomplete rather than receiving an invented price. Cursor CSV rows are
aggregated and therefore use base rates, without inferring per-request context tiers.
The bundled pricing snapshot and aliases derive from OpenUsage's MIT-licensed catalogs
(LiteLLM, models.dev, and its supplement); attribution is in shared/pricing/LICENSE.OpenUsage.
Prices load from bundled snapshots and the local cache, with background revalidation once
per hour. Failed sources retry after 30 minutes without replacing last-good prices. The
OpenUsage supplement takes precedence over LiteLLM, which takes precedence over models.dev.
Conditional requests use ETags; bounded downloads follow the application network proxy.
The next usage scan reprices cached token events using a consistent price snapshot; the
price data timestamp is returned alongside history. No credentials or usage records are
sent to pricing feeds.

Quota bubbles project consumption from the elapsed reset window and used quota.
Projection starts after at least 60 seconds or 1% of the window; near-empty meters
(under 5% used) suppress unstable over-limit warnings. An even-pace tick follows
the Used/Remaining display mode, and the warning respects countdown/absolute time.
Expired or stale readings do not produce pace forecasts. Cursor uses reported billing
cycle dates when available; monthly windows otherwise use the same 30-day convention
as OpenUsage.
