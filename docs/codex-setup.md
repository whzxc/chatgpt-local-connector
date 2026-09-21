# Set up and troubleshoot Local Connector with Codex

**English** | [简体中文](zh-CN/codex-setup.md)

This is the operating guide for Codex on the target computer. Default to OpenAI Secure MCP Tunnel. A full installation and configuration request includes a real ChatGPT tool call and a harmless Codex task verification. Do not send the user back to the entire guide to do the work themselves; ask for intervention only when sign-in, permissions, credentials, or a necessary choice requires it.

## Entry points and installation

Use local Codex with command execution on the target computer. A cloud task cannot directly configure the user's computer. Inspect existing installation and runtime state first; do not reinstall unnecessarily or overwrite working configuration. Desktop task integration supports Apple Silicon macOS and Windows x64; Windows requires Microsoft Store Codex Desktop.

If missing, download the matching installer and `SHA256SUMS.txt` from [GitHub Releases](https://github.com/whzxc/chatgpt-local-connector/releases/latest) and compare hashes for the same version. On macOS, install into Applications or use the installation guide's Homebrew Cask; on Windows, run the x64 EXE or MSI. Node, npm, Rust, and cloning the source are unnecessary. Follow the installation guide for OS security prompts without disabling global protections. Do not keep running the app from its DMG.

Open Local Connector on macOS. The default installation provides these CLI entry points:

```sh
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli help
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli guide
"/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop" cli doctor
```

Use the actual installation location. `CFBundleExecutable` in `Contents/Info.plist` identifies the executable. On Windows, use `local-connector-desktop.exe cli help` in the installation directory and redirect its JSON output when needed.

`cli guide` embeds the matching guide for the current binary and takes precedence over the website. If an older release has no CLI, do not repeatedly try unknown arguments. Check release support and upgrade if appropriate. Website documentation does not mean every command is already in the latest published installer.

## CLI conventions

Run `<app executable> cli <command>`; no separate CLI installation is needed. Each command outputs one JSON object; `--json` can be specified explicitly:

- Success: `{"schemaVersion":1,"ok":true,"result":...}`, exit code 0.
- Failure: `{"schemaVersion":1,"ok":false,"error":{"code":"...","message":"..."}}`, exit code 1.
- A successful `doctor` invocation does not prove configuration is complete. Read `result.checks`, `next`, and `stage`.
- `APP_UNAVAILABLE`: open the app, then retry the read-only command. `APP_VERSION_MISMATCH`: check the mismatch between the running app and CLI capabilities.
- `help` and `guide` do not need a backend. Other commands call the desktop app's existing local service and work while the tunnel is disconnected. Do not edit internal state files or expose internal management credentials.

| Command | Purpose |
| --- | --- |
| `status` | Redacted configuration, connection and Desktop IPC state, historical verification |
| `doctor` | Checks configuration, sign-in, transport, and historical inbound access for the current Tunnel / HTTPS mode and execution owner; returns the next step |
| `onboarding` | Current step, web entry points, suggested name/description, connection type and URL/ID, verification and harmless-task messages; no keys or automatic website installation |
| `configure --stdin` | Reads JSON from stdin, accepting only tunnelId and apiKey, and configures official Tunnel mode; preserve HTTPS settings through desktop Settings instead of overwriting them |
| `network --stdin` | Reads proxyMode and proxyUrl from stdin, using the app's proxy settings |
| `connect` / `disconnect` | Starts/stops through the existing backend; connect automatically prepares Tunnel Client |
| `logs` | Connection logs redacted by the app |
| `verify --fresh` | Generates a new code, clears verification for the current configuration, and starts a new verification round |
| `verify` | Current code, ChatGPT verification message, verifiedAt, and challengeVerifiedAt; does not make a remote call |

## Workflow

1. Run `doctor` and `status` to determine existing configuration, running state, and connection mode. Preserve a configured HTTPS connection instead of requesting a switch just because official Tunnel is the default. Clearly report unsupported platforms for full task integration.
2. If Codex is missing or signed out, open the official installation or sign-in page and continue after the user signs in. Preserve task ownership, approval, and sign-in startup preferences by default.
3. Only if official Tunnel is in use and details are missing, open [Platform Tunnel settings](https://platform.openai.com/settings/organization/tunnels). Prefer authorized secure local sources and read an existing Tunnel ID yourself. If a runtime API Key is missing, ask the user to enter it directly in the app. Verify workspace association and permissions; Codex sign-in does not replace these. Explain missing account/administrator requirements instead of retrying repeatedly.
4. For official Tunnel, ask the user to enter and save credentials in Settings → Connection → OpenAI Tunnel only when no secure local source exists. Continue when doctor reports configuration. Never request keys in chat or expose them in arguments, logs, or screenshots. With an authorized secure local credential source, pass JSON through stdin without embedding literal secrets in shell commands. Omitted configure fields and an empty apiKey preserve existing values; a changed Tunnel ID needs a matching key. Configuration changes while connected are rejected: establish that a change is needed, then disconnect.
5. Run connect, then poll doctor/status for readiness. A successful connect response only confirms the start request. Read logs for download/network failures and act on the observed error. Change only Connector's proxy: network accepts system, direct, or custom; custom requires an HTTP/HTTPS proxyUrl. Existing Tunnel connections need reconnecting to apply changes. Do not change the system proxy.
6. Read onboarding (status/verify for older releases) for the suggested name, description, connection type, and value. If the user supplies a signed-in browser environment, actually attempt browser / GUI / Computer Use interaction. Prefer the specified signed-in tab, then the in-app Browser. Use visible text and accessible controls; do not assume selectors, coordinates, or settings deep links. State when browser control is unavailable.
7. Check ChatGPT Settings → Security & sign-in → Developer Mode, then [Plugins](https://chatgpt.com/plugins). The create entry may be Add → Create MCP App or ＋. Match existing connections by the current URL/ID, not just name, and reuse them; create only if missing. For Tunnel, select the tunnel or enter value; for HTTPS, enter value with No authentication, never a management port or Tunnel API Key. Confirm discovery of connector_verify and other tools; refresh an existing connection from its details when needed. Rematch temporary URLs after changes and follow the active tool's confirmation requirements before deleting old connections. Pause only for personal sign-in, identity/permission authorization, security codes, missing account/workspace access, or unreliable controls. Identify the page and minimum action. Follow browser safety confirmation rules without bypassing them.
8. Start a new verification round with verify --fresh and retain the code. To resume an interrupted round, use verify without regenerating it. Create a Chat conversation in ChatGPT (switch from Work if necessary), select Local Connector, and send the returned prompt. Try in chat from an existing plugin's details can also select it. Poll verify, confirm the code is unchanged and challengeVerifiedAt appears, and confirm the corresponding ChatGPT tool call succeeded. After a bounded wait (for example two minutes), inspect logs/doctor. Ordinary successful calls update verifiedAt only. If fields are missing, check the running version; older releases require the matching code/received tool result plus current inbound records. Never call locally to impersonate ChatGPT inbound access. Inbound access alone does not authenticate the caller's identity.
9. For a full setup request, continue through the same ChatGPT connection with onboarding.executionPrompt, or create a task that calls no tools, reads or modifies no files, and only replies CLC_ONBOARDING_OK. This is a verification task, not a change to a real project. Follow the current tool schema, use a unique UUID requestId, and read the persistent receipt, native threadId/turnId, final state, and output. Handle approvals according to user authorization and tool rules. Read back unknown states with the original requestId; do not replay with a new ID. If this step is incomplete, report that task execution is not yet verified. Accepted/running does not mean complete.

## Automation boundaries

The desktop guide presents Enable Developer Mode → Create MCP App → Send verification message on one page. Name, description, HTTPS URL, and verification message can each be copied. Select the current Tunnel directly and choose No Authentication. Inbound results update automatically; verification can be repeated. CLI onboarding provides structured values, so users do not need to copy local URLs, IDs, or verification messages for Codex. The app itself neither reads browser sign-in state nor controls ChatGPT's website.

The official public flow still requires enabling Developer Mode, creating connections, refreshing tools, and selecting connections through ChatGPT UI. Public documentation links to Plugins but provides no creation/installation API, prefill protocol, or Developer Mode toggle deep link for CLC. Responses API MCP calls do not install plugins into a ChatGPT account. Do not use private interfaces, extract cookies/tokens, or run fixed DOM automation scripts.

A signed-in page with available, authorized browser tools lets Codex navigate and fill visible controls. This is not a stable fully automatic platform API and does not guarantee zero user interaction for every account. Entry names vary; local software cannot bypass missing workspace permissions, Tunnel association, personal authorization, or security challenges. Hand back only the currently blocked step when controls are unreliable, then continue. This remains a local open-source tool without cloud accounts, hosting, public relays, or a Public Plugin.

## Troubleshooting and recovery

Start with doctor/status even in a new Codex conversation. Configuration and verification records belong to the app, not prior chat memory. Resume an existing verification code with verify; use --fresh only for a new round.

Distinguish evidence for: app not running, missing details, expired sign-in, unavailable Desktop IPC, component download failure, Tunnel network errors, insufficient workspace access, ChatGPT not selecting the connection, and unknown submission results. Do not treat every error as a reason to reinstall or reset configuration. Apply authorized, evidence-supported fixes automatically; hand invalid credentials, administrator access, and account choices to the user. Read back state and verify again afterward.

Report local connection, ChatGPT inbound access, and task execution verification separately, including any outstanding action. Do not expose keys, complete private logs, or real task content.

References: [installation](installation.md), [Tunnel setup](tunnel.md), [official Tunnel documentation](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels), [ChatGPT connection documentation](https://developers.openai.com/plugins/deploy/connect-chatgpt).
