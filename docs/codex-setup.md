# Configure Local Connector with Codex

**English** | [简体中文](zh-CN/codex-setup.md)

Use the CLI on the target computer. Start by reading `<app executable> cli help`, `cli status` and `cli ingress list`. Help, guide and ingress presets work offline; other commands require the matching running app. Do not overwrite another ingress or install over a running version without authorization. OpenAI Secure Tunnel remains the default ChatGPT onboarding path.

On macOS the usual executable is `/Applications/Local Connector.app/Contents/MacOS/local-connector-desktop`; on Windows use the installed `local-connector-desktop.exe`. Do not require Node/npm on users' machines. Every CLI response is JSON: `{schemaVersion:1,ok:true,result:...}` or `{schemaVersion:1,ok:false,error:{code,message}}`, with exit status 0 or 1. Read operation results and per-ingress state, not just the exit code of a diagnostic command.

## Commands

| Command after `cli` | Purpose |
| --- | --- |
| `ingress presets` | Offline curated source registry, recommended transport/auth, limitations and official docs |
| `ingress list` | Redacted configuration, runtime state, verification and preset metadata for every ingress |
| `ingress add --stdin` | Create an entry from JSON; omitted id is generated |
| `ingress update <id> --stdin` | Partial update; config fields merge; stop the target first |
| `ingress remove <id>` | Stop and remove one entry, preserving tasks and other entries |
| `ingress start <id>` / `ingress stop <id>` | Start/stop only that entry |
| `ingress start-all` / `ingress stop-all` | Operate on all entries; start skips disabled entries; results report each failure |
| `ingress token rotate <id>` | While stopped, generate a bearer secret and return it once in stdout; resets target verification |
| `ingress doctor <id>` | Configuration, runtime evidence and client setup instructions |
| `ingress verify <id> --fresh` | Generate a fresh challenge for only this entry |
| `ingress verify <id>` | Read the existing challenge without resetting it |
| `status` / `doctor` / `onboarding` | Global summary and per-ingress JSON; readiness is not inbound or execution proof |
| `network --stdin` | Global proxyMode/proxyUrl; existing ingress processes need reconnecting |
| `configure --stdin`, `connect`, `disconnect` | Primary ingress onboarding shortcuts; do not operate on all entries |
| `verify`, `verify --fresh` | Read all challenges, or explicitly reset all challenges |

Add requires controlSource, transport, auth and config. Name defaults to the preset display name (or unknown source label), with an available numeric suffix, enabled to true, toolPolicy to all. `id` is immutable and contains ASCII letters, digits, hyphen or underscore. ControlSource is a descriptive client label, not an authenticated user identity. HTTPS requires auth none or bearer; OpenAI Tunnel requires auth openai. Bearer requires a unique 32+ printable ASCII secret supplied through stdin. `toolPolicy` is `"all"` or `{"allowlist":["connector_verify","agents","agent_create","agent_request","agent_read","agent_send","agent_wait","control_output"]}`. Both tool discovery and invocation enforce it. Include connector_verify to use challenge verification. This policy does not isolate tasks or filter native methods within a tool.

Configuration keys and limits are discoverable in help. No secret is accepted as a command argument. Pipe JSON from an authorized secure local source, or redirect a protected file. Do not put secrets in shell literals, chat, screenshots or logs. For rotation, arrange a private destination first (for example `umask 077` and stdout redirection to a local file); do not capture stdout into a conversation. Deliver the secret to the supported client credential field using authorized local interaction. Never substitute the desktop management token or provider credential for the ingress bearer token.

## Configuration examples

The following are configuration shapes, not real credentials. Replace secret fields through secure stdin input. Each represents a separate entry sharing the same Core and task IDs.

ChatGPT uses the official identity system:

```json
{"id":"chatgpt","name":"ChatGPT","controlSource":"chatgpt","transport":"openai-tunnel","auth":"openai","config":{"tunnelId":"tunnel_example","apiKey":"FROM_SECURE_LOCAL_SOURCE"}}
```

Obtain the Tunnel identity and runtime key through the official Platform flow and associate it with the intended workspace. Use the client's supported Tunnel connection UI. Personal sign-in, permissions, CAPTCHA and identity authorization remain user actions; do not bypass them.

Notion-labelled MCP access via an ngrok-assigned endpoint:

```json
{"id":"notion","controlSource":"notion","transport":"https","auth":"bearer","bearerToken":"FROM_SECURE_LOCAL_SOURCE_32_PLUS_CHARS","config":{"httpsProvider":"ngrok","ngrokAuthtoken":"FROM_SECURE_LOCAL_SOURCE"}}
```

Cursor MCP access via Cloudflare Fixed:

```json
{"id":"cursor","controlSource":"cursor","transport":"https","auth":"bearer","bearerToken":"FROM_ANOTHER_SECURE_LOCAL_SOURCE_32_PLUS_CHARS","config":{"httpsProvider":"cloudflare","cloudflareMode":"named","httpsUrl":"https://connector.example.com/mcp","httpsPort":8788,"cloudflareToken":"FROM_SECURE_LOCAL_SOURCE"}}
```

Set that Cloudflare tunnel's service route to `http://127.0.0.1:8788`. Each Fixed ingress needs its own route/port and Tunnel identity; do not reuse an identity to load-balance incompatible authentication contexts. Cloudflare stores remote-managed routing on its server; CLC does not change it with a Tunnel token. See [official routing setup](https://developers.cloudflare.com/tunnel/get-started/).

For Quick Tunnel use cloudflareMode quick, without a fixed URL or provider token. Use it for temporary trials. Prefer fixed addresses plus authentication for long-lived clients supporting static bearer. For Custom Domain use httpsProvider custom, httpsUrl, httpsHost and httpsPort, and configure the TLS reverse proxy separately. The listener is HTTP behind TLS termination; never expose the desktop management listener.

See the [control source matrix](control-sources.md) before choosing auth. Slackbot does not accept this static Bearer setup; only no-auth intersects with CLC today. Claude organization request-header beta and Copilot Studio API-key Header are conditional paths, not OAuth support. Never silently downgrade authentication. `ingress presets` exposes these boundaries offline; do not mistake a local probe for official client acceptance.

## Complete an authorized setup

For “add a Notion ngrok + bearer ingress”: inspect ingress list; preserve existing entries; prepare a protected bearer secret and the user's ngrok credential; add through stdin; start that id; poll its doctor until ready; read its public URL; securely configure the supported MCP client. Do not silently select a temporary URL if long-term use was requested. Report provider account/endpoint limits explicitly if startup fails.

Run `ingress verify <id> --fresh`. Call connector_verify with that code from the actual control source and read back that entry's challengeVerifiedAt, ingressId and controlSource. A successful local probe proves local transport only. Ordinary tools update historical verifiedAt but cannot satisfy a fresh challenge. Verification on another ingress must not count. Resume interrupted setup using the existing code rather than generating another one.

For full execution acceptance, create a harmless task through that source, using a unique UUID requestId and a prompt that uses no tools and only replies CLC_ONBOARDING_OK. Read the persistent receipt and taskId/threadId, then agent_wait/codex_wait and the terminal output. Unknown results require readback with the original requestId, never a fresh write. To check shared task namespace, read/send/wait the same task through another permitted ingress. Stopping an ingress must not stop Pi/ACP or Desktop tasks.

In a supplied authenticated browser, use visible supported controls to configure the connection and refresh tools; prefer the in-app Browser unless a browser was specified. Do not extract cookies or use private platform APIs. If client sign-in or authorization is required, complete independent local work and identify the precise remaining client-side action.

## Boundaries

No-auth allows any reachable caller to invoke allowed tools. Bearer authenticates possession, not a person. Namespace is globally shared; no task/user RBAC exists. OAuth, DCR, Slack Identity and service-specific APIs are outside this implementation. Local listener request processing permits 300-second waits; external proxies and provider plans may impose shorter limits, which require their own verification/configuration. Shorter waits can be repeated with the same task ID; timeout never stops a task.

An ingress failure is local to that entry. Read its error and redacted logs before changing configuration. App shutdown closes ingress resources, then AgentHost and Control; ordinary disconnect does not. The home page shows a running count and one line per entry. Credentials, policies and provider details primarily belong in the CLI.

## Monitoring tasks

After create/send, obtain the original taskId/threadId and turnId from the receipt. Codex should call agent_wait/codex_wait in 20–30-second slices (default timeoutMs=30000): on timeout, keep the same IDs and pass the previous snapshotHash as expectedHash to the next wait, until completed/failed/cancelled or interaction-required. Do not recreate a task or resend its prompt after timeout. Timeout or cancelling a wait only ends that wait; unconfirmed is not task failure. expectedHash controls changed comparison, never suppressing terminal or interaction results.

This is bounded event-driven long polling: each slice uses event wakeups and fresh owner-state checks, not repeated read/sleep polling. ChatGPT, Notion, Slack and other MCP clients may impose different outer tool-call timeouts. Use the shared 30-second default; ordinary Chat and general MCP clients should not block for minutes by default. Explicit timeoutMs up to 300000 remains available when the upstream client supports it; the OpenAI Tunnel stdio adapter retains a 330-second forwarding budget and HTTPS MCP does not impose a shorter execution deadline. CLC cannot extend external client/proxy timeouts.
