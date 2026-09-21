# Connect ChatGPT: OpenAI Tunnel and HTTPS MCP

**English** | [简体中文](zh-CN/tunnel.md)

Return to the [project home](../README.md). For assisted setup, send the home page's message to local Codex and follow the [setup and troubleshooting guide](codex-setup.md), which defaults to the official Tunnel. The instructions below cover manual setup. Configure and control connections in the native app's Settings. Missing connection components are prepared automatically; you do not need to create a Tunnel profile manually.

## HTTPS MCP

In Settings → Connection, select HTTPS MCP and choose a provider. All options use No authentication and require your ChatGPT account to offer custom MCP connections. This project does not provide a hosted relay.

| Provider | Required information | Managed by the app | Boundaries |
| --- | --- | --- | --- |
| Cloudflare · Quick trial | None | Downloads cloudflared, starts a Quick Tunnel, obtains a public URL | No account or domain; the URL may change on reconnect. For trials, without availability guarantees |
| Cloudflare · Fixed domain | Tunnel Token, public MCP URL | Downloads cloudflared, runs a named Tunnel, listens on `127.0.0.1:8787` | Your Cloudflare account must have the domain and a configured public route |
| ngrok | Account Authtoken | Downloads ngrok, starts a tunnel, obtains the account's public URL | Requires an ngrok account; its traffic, request, and concurrency limits apply |
| Custom domain | Public MCP URL | Starts the local MCP listener | You provide a domain, valid TLS certificate, and reverse proxy |

### Cloudflare quick trial / ngrok

1. Select Cloudflare, or choose ngrok and use Get token to obtain your Authtoken.
2. Select Save and reconnect. The app prepares official components and runs the tunnel with a separate random loopback port and temporary configuration. Existing cloudflared/ngrok configuration is unchanged.
3. Open the setup guide from the home screen, select Open Plugins, and copy the connection URL to a custom MCP connection in ChatGPT. Choose No authentication. Alternatively, provide a signed-in browser session for Codex to fill in using `cli onboarding`.
4. Copy and send the message under Send verification message. CLC confirms receipt of the matching code. If the URL changes, recreate the ChatGPT connection with the new URL and remove the old one; verification for the old URL does not carry over.

Cloudflare quick trial uses [Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/). It is intended for trials and does not support a separate SSE stream; CLC's native MCP uses JSON POST and does not rely on SSE. cloudflared needs outbound access to Cloudflare on port 7844; an ordinary HTTP proxy should not be assumed to carry its data traffic. For a lasting address, use Cloudflare fixed domain, ngrok, or a custom domain. Manage ngrok accounts, domains, and quotas through [ngrok](https://ngrok.com/download); the app does not create paid resources.

Components are downloaded from official HTTPS sources and cached locally. Cloudflare downloads are checked against official SHA256 values. ngrok downloads come from the official download site; a local SHA256 detects cached-file corruption. The Authtoken is used only by the ngrok client, never sent to ChatGPT or placed in command arguments or ordinary status responses. Disconnecting, startup failure, or quitting stops the tunnel and MCP listener and removes temporary runtime configuration.

### Cloudflare fixed domain

1. Create a Tunnel under Networking → Tunnels in the [Cloudflare dashboard](https://dash.cloudflare.com/). Copy only the Token from the install command, not the entire command. No separate client or system service installation is needed.
2. Choose Cloudflare → Fixed domain in the app. Enter the Tunnel Token and public MCP URL, such as `https://connector.example.com/mcp`, then select Save and reconnect.
3. Under the Tunnel's Routes, add a Published application for your domain. Set the Service URL to the app's `http://127.0.0.1:8787`. The path can be empty; the request's `/mcp` path is preserved. The domain must belong to that Cloudflare account.
4. Add the public MCP URL in ChatGPT with No authentication, then make a verification call.

Named Tunnels use a fixed local address. If port 8787 is occupied, the app reports an error instead of silently changing ports. The Tunnel Token is passed to cloudflared in a private temporary file and omitted from normal status and logs. Saving an empty Token preserves the existing one; changing the Token or URL requires verification again. The app manages only the local client, not Cloudflare domains, routes, or account resources. Tunnel readiness does not prove the public route is configured or ChatGPT can connect. See the [Cloudflare named Tunnel guide](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel/).

### Custom domain

Enter a public `https://your-domain/mcp` URL. The default listener is `127.0.0.1:8787`. A reverse proxy on the same computer can use the displayed Proxy target. For a proxy on another device, set this computer's LAN IP and port under Advanced settings and allow that proxy to access the port.

Forward public `/mcp` requests to the proxy target. Use the public domain or actual listening IP:port as `Host`. Support JSON POST and long requests; allow at least 300 seconds plus transport overhead if you use the full five-minute task wait. After saving and connecting from the home screen, add the MCP URL in ChatGPT with No authentication.

All HTTPS options above are unauthenticated: any client that can access the public URL can call tools. A self-managed reverse proxy can restrict access. The native endpoint currently provides neither access keys nor OAuth authentication.

The native HTTP endpoint exposes only `/mcp`, separately from the desktop management API and its random internal credential. It uses stateless Streamable HTTP: POST returns JSON, notifications return 202, and no separate SSE GET stream is available. TLS terminates at the provider or your reverse proxy.

Configuration can only change while disconnected. The Settings save action disconnects, saves, and reconnects. Leaving a provider Token blank preserves it. Tunnel readiness describes local processes and provider connectivity; a successful ChatGPT tool call confirms the public path.

## Official Tunnel: information you provide

Follow the [official Secure MCP Tunnel guide](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) to obtain your Tunnel ID and runtime API Key and confirm association and permissions for the target ChatGPT workspace. The app stores local settings and runs the official Tunnel Client; it does not provide a Tunnel server or apply for an identity on your behalf.

The API Key is masked by default and can be read back on demand using Show key. Saving an empty key preserves the existing one. Do not place keys in command arguments, source code, screenshots, or chat. The native app manages connections over local IPC; the HTTP adapter uses a separate random local credential. The runtime API Key is used only for Tunnel connectivity.

## First connection

Installing and signing in to Codex Desktop does not create the following external configuration:

1. **Obtain Tunnel credentials and permissions.** Create a tunnel in [Platform Tunnel settings](https://platform.openai.com/settings/organization/tunnels), or obtain the Tunnel ID and runtime API Key from an administrator. Creation/editing needs Tunnels Read + Manage; running the client and selecting the tunnel in ChatGPT needs Read + Use. Associate the tunnel with the target ChatGPT workspace; Platform organization association alone is insufficient.
2. **Add the connection in ChatGPT.** Keep the local connection running. Enable Developer Mode in ChatGPT Settings → Security & sign-in, then go to Plugins → Add (or ＋) → Create MCP App. Enter a name and description, select Tunnel and the tunnel or its ID, create the connection, and check the tool list. Developer Mode is an independent account/workspace permission; contact your workspace administrator if it is missing. Reuse an existing matching connection.

Start a new conversation and select Local Connector from the tool menu. Copy the `connector_verify` message from Send verification message and send it in ChatGPT. Ordinary read-only tool calls create inbound history but do not complete code verification. Clicking an “added” button, reading local status, or reaching Tunnel ready is not a substitute for an actual remote call. Verification proves past connectivity; task execution also needs a real final task result.

The app installs the official Tunnel Client, stores credentials, and controls the connection. It does not apply for an identity or authorize the web account. No public domain or inbound port is needed for official Tunnel mode; the computer must download the official client and reach OpenAI over outbound HTTPS. See the [Tunnel guide](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) and [ChatGPT connection guide](https://developers.openai.com/plugins/deploy/connect-chatgpt).

## Connections and devices

```text
ChatGPT → Official Tunnel → Local Tunnel Client → Native stdio adapter → Rust core → Codex Desktop owner
                                      ↑
                       Native app manages local settings and lifecycle
```

Each computer uses its own directories, Codex sign-in, and native project list. An instance serves one computer, without device selection or routing. For multiple devices, use separate official Tunnels or HTTPS URLs, create distinctly named ChatGPT connections, and select the intended one in the conversation. Do not run backends on multiple computers with the same Tunnel identity simultaneously. Stop the old device before starting the new one and query projects to confirm the source.

Official mode uses the Tunnel ID; HTTPS mode uses the public MCP URL. Neither uses the internal management port or development preview URL.

## Minimize manual steps

Send the home page prompt to local Codex and provide a signed-in ChatGPT browser session. Codex reads connection information from the CLI and operates visible controls to configure, verify, and run a harmless task. User intervention is limited to sign-in, personal authorization, security challenges, missing permissions, or steps that cannot be reliably automated. See [automation boundaries](codex-setup.md#automation-boundaries).

Manual flow: configure and connect locally → follow Enable Developer Mode, Create MCP App, and Send verification message on the setup page. Copy the suggested name and description, select the current Tunnel (or copy Server URL for HTTPS), and choose No Authentication. A matching code automatically updates verification status. Verifying again generates a new code.

No public one-click installation or prefill interface is available to this app. CLC cannot read website installation state, and some accounts do not show the create button in Plugins. Opening a page does not install a connection; historical inbound traffic does not prove current transport availability; code verification does not mean a Codex task completed.

## Status in the app

The home page shows ChatGPT — Connector — Codex, with issues in banners. Records shows connection changes and request results. ChatGPT verification means a real inbound request was received previously, not that transport will remain available indefinitely.

After connecting, ask ChatGPT to query capabilities and local projects to confirm the source. Read actual turn results to confirm completion. Local Tunnel health checks do not prove outbound HTTPS, cloud polling, or ChatGPT tool discovery. Refresh the ChatGPT connection when tool metadata changes.

## Proxy and network

Settings → Network → Proxy offers:

- **System proxy** (default): reads the current user's manual HTTP/HTTPS proxy on macOS or Windows; connects directly if none exists.
- **No proxy**: always connects directly, ignoring inherited proxy environment variables.
- **Custom**: enter an HTTP/HTTPS proxy URL such as `http://127.0.0.1:7890` and save. Embedded usernames and passwords are unsupported.

This setting covers Tunnel downloads, initialization, and runtime, plus app update checks and downloads. It does not change system or Codex Desktop proxy settings. Saving does not interrupt a running connection; reconnect for Tunnel to use the new setting. New downloads use it immediately.

System mode does not execute PAC/WPAD, import system exception lists, or support SOCKS-only proxies. Unsupported configurations prompt you to choose a custom HTTP/HTTPS proxy or direct access. System reads have a timeout and failures do not silently fall back to direct access. App settings override `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and their lowercase equivalents. Existing `NO_PROXY` and `no_proxy` rules are merged and extended with loopback. Local health checks, stdio forwarding, and desktop backend communication always connect directly. Proxy-source log messages omit addresses and authentication details.

Public HTTPS reverse proxies and local inbound MCP listeners are unaffected by this outbound proxy setting.

A browser reaching ChatGPT does not prove Tunnel can reach OpenAI. If creating the connection or refreshing tools fails, inspect connection logs for DNS, timeout, TLS, or proxy errors, then call `connector_verify` from ChatGPT. Local readiness or historical verification is not a substitute for this remote call.

## Stop and troubleshoot

Disconnect stops Tunnel, the stdio adapter, and Connector helper processes. Desktop-owned tasks keep running; unconfirmed external requests retain their receipts. Remove the connection in ChatGPT when revoking remote access.

If tools cannot be discovered, check Tunnel status and Records. If a local binary is missing, install it or specify its full path. Sign in to Codex Desktop before use and check that Desktop is available when connections fail. For an unavailable native method, query `codex_schema` and check whether the current binary provides it.

After a write timeout, read back using the original `requestId`. A timeout is not cancellation; do not automatically retry a write with a new ID. Connection logs are redacted; raw task output is not guaranteed to be. Review it before sharing.
