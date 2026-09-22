# Connect ChatGPT: OpenAI Tunnel and HTTPS MCP

Multiple ingresses run concurrently against one Core. Settings operate on the primary ingress; use `cli ingress` to add independently authenticated entries. Cloudflare Fixed defaults to httpsPort 8787; assign distinct ports and match the remote service routes for multiple Fixed entries.

**English** | [简体中文](zh-CN/tunnel.md)

Return to the [project home](../README.md). For assisted setup, send the home page's message to local Codex and follow the [setup and troubleshooting guide](codex-setup.md), which defaults to the official Tunnel. The instructions below cover manual setup. Configure and control connections in the native app's Settings. Missing connection components are prepared automatically; you do not need to create a Tunnel profile manually.

## HTTPS MCP

In Settings → Connection, select HTTPS MCP and choose a provider. All options use No authentication and require your ChatGPT account to offer custom MCP connections. This project does not provide a hosted relay.

| Provider | Required information | Managed by the app | Boundaries |
| --- | --- | --- | --- |
| Cloudflare · Quick trial | None | Downloads cloudflared, starts a Quick Tunnel, obtains a public URL | No account or domain; the URL may change on reconnect. For trials, without availability guarantees |
| Cloudflare · Fixed domain | Tunnel Token | Downloads cloudflared, runs a named Tunnel, discovers matching public domains | Your Cloudflare account must have the domain and a configured public route |
| ngrok | Account Authtoken | Downloads ngrok, starts a tunnel, obtains the account's public URL | Requires an ngrok account; its traffic, request, and concurrency limits apply |
| Custom domain | Public MCP URL | Starts the local MCP listener | You provide a domain, valid TLS certificate, and reverse proxy |

### Cloudflare quick trial / ngrok

1. Select Cloudflare, or choose ngrok and use Get token to obtain your Authtoken.
2. Select Save and reconnect. The app prepares official components and runs the tunnel with a separate random loopback port and temporary configuration. Existing cloudflared/ngrok configuration is unchanged.
3. Open the setup guide from the home screen, select Open Plugins, and copy the connection URL to a custom MCP connection in ChatGPT. Choose No authentication. Alternatively, provide a signed-in browser session for Codex to fill in using `cli onboarding`.
4. Copy and send the message under Send verification message. CLC confirms receipt of the matching code. If the URL changes, recreate the ChatGPT connection with the new URL and remove the old one; verification for the old URL does not carry over.

Cloudflare quick trial uses [Quick Tunnels](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/). It is intended for trials and does not support a separate SSE stream; CLC's native MCP uses JSON POST and does not rely on SSE. cloudflared needs outbound access to Cloudflare on port 7844; an ordinary HTTP proxy should not be assumed to carry its data traffic. For a lasting address, use Cloudflare fixed domain, ngrok, or a custom domain. Manage ngrok accounts, domains, and quotas through [ngrok](https://ngrok.com/download); the app does not create paid resources.

Components are downloaded from official HTTPS sources and cached locally. Cloudflare downloads are checked against official SHA256 values. ngrok downloads come from the official download site; a local SHA256 detects cached-file corruption. The Authtoken is used only by the ngrok client, never sent to ChatGPT or placed in command arguments or ordinary status responses. Disconnecting, startup failure, or quitting stops the tunnel and MCP listener and removes temporary runtime configuration.

ngrok offers Temporary domain and Fixed domain modes; both use only an Authtoken. Temporary mode automatically obtains an ngrok-assigned address, which may change after restart and require updating the MCP client. In Fixed domain mode, enter a domain already configured in your ngrok account (or an HTTPS origin without a path). The app starts the tunnel using that domain and provides the `/mcp` URL. It does not query account domains, provision domains, or create paid resources. If the fixed domain is unavailable, startup fails without falling back to a temporary address.
