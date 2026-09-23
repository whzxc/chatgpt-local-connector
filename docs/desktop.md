# Desktop lifecycle

The production app is one Tauri/Rust process owning a single connection core. There is no separate Node service, copied runtime directory, or app/backend version pairing.

- **Connect:** Desktop mode ensures Desktop is available; background mode initializes Connector's app-server without opening Desktop. Official mode starts Tunnel Client, which launches the same executable's `stdio` adapter as needed. HTTPS mode starts a separate MCP listener and the configured Cloudflare/ngrok client. Tunnel management and auxiliary RPC use Desktop's bundled native Codex binary directly, without npm launcher scripts.
- **Disconnect:** stops only the selected ingress process group and MCP forwarding; Connector app-server and Agent processes continue until core shutdown. Unconfirmed requests retain receipts and must not be automatically retried as unexecuted.
- **Close window:** hides it while keeping the connection running. Reopen through the macOS menu bar/Dock or Windows tray.
- **Quit:** stops Connector-owned connection processes, then exits.
- **Start at sign-in:** the OS starts the native app and connects it.
- **Update:** downloads and verifies the signature, then disconnects, installs, and restarts. Codex Desktop task ownership does not change.

The home view places control sources on the left, the shared Connector in the center, and installed, supported Agents on the right. Each line shows its own status; an installed Agent can remain on standby. The top Connect/Disconnect button starts enabled ingresses or stops running ingresses. Click a built-in platform icon to open its website, or its line status icon to edit the connection, or use the plus button to add ChatGPT, Notion, Slack, or a custom MCP client. Each source has its own connection dialog with save, connect, disconnect, and remove actions. Applicable authentication and listener fields are displayed directly. Platform and display name are separate: the chosen platform stays fixed, and every source has an editable name; only ChatGPT offers OpenAI Tunnel. The editor reads back that ingress’s API key as a password, while other blank secret fields preserve saved credentials. General Settings contains application preferences; Agents management and its separate settings panel open from the home graph.

The tray uses a transparent brand logo and shows connection state, tasks and pending approval counts, Records, and Settings. Connection, startup, and approval switches share main-window state and APIs. Cloud requests can still approve or bypass optional confirmation according to user intent. Tray labels follow the UI locale on the next status refresh.

Automatically open Codex tasks is enabled by default. Creation persists a temporary empty seed before opening the task page for Desktop execution; sending, continuation, and interruption use the Desktop owner's IPC. When disabled, new tasks are created and run directly in Connector's app-server, without Desktop IPC or guaranteed Desktop continuation/interruption. Existing tasks retain their owner. Read-only background task queries do not resume execution. After Connector restarts, explicitly sending new input resumes the session; unconfirmed requests are never replayed automatically.

Connection is not gated by Desktop version numbers. It still validates the IPC protocol, socket/named-pipe user, task owner, and response source. Incompatibility returns the actual error without bypassing checks or taking ownership. External task management supports macOS and Windows. On Windows, Connector detects the current user's Microsoft Store Codex Desktop and verifies the named-pipe server process belongs to that installation. To avoid WindowsApps execution restrictions, Desktop's bundled CLI and helpers are copied by content hash into Connector's private data directory. Tasks open through the OS-registered `codex://` links.

The local management HTTP server binds only to a random `127.0.0.1` port and validates Host, Origin, and a random credential. Port and credential are stored in the private state directory for stdio/development preview. Frontend assets are embedded in Tauri; no Node web server runs.

Development builds use a separate identifier and icon and forward reads/writes to the running built app. If it is unavailable, they report an unavailable backend instead of starting an independent business connection. See [development](development.md).

First use requires Codex Desktop installation/sign-in, official Tunnel or HTTPS configuration, and adding/enabling the connection in ChatGPT. The app prepares the selected connection components. Existing Codex sign-in can be reused but does not replace provider credentials or web connection setup. See the [README](../README.md#let-codex-set-it-up-recommended) and [connection guide](tunnel.md).

Selected subscriptions are monitored by the shared core independently of
connections. macOS can show an independent non-activating edge rail after the
feature is enabled. Closing the main window keeps monitoring and the rail alive;
hiding the rail leaves monitoring active; disabling subscription monitoring stops quota refreshes. Both preserve the usual
Dock/menu-bar entry points. See [Subscription usage](subscriptions.md).
