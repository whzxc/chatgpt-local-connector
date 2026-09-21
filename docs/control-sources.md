# Control source support

Eight curated presets plus Custom. Other MCP clients use Custom. Presets describe client capabilities, not completed client verification or separate protocols/services. Multiple entries of the same type are allowed; ingress.id is the identity.

| Control source | Remote MCP | Recommended transport | Recommended auth | CLC availability | Caveat / official docs |
| --- | --- | --- | --- | --- | --- |
| ChatGPT | Supported | openai-tunnel | openai | supported | Prefer Secure Tunnel. HTTPS supports no-auth; CLC does not implement OAuth. Tunnel access and ChatGPT developer-mode permissions are separate. [Official 1](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels) / [Official 2](https://developers.openai.com/api/docs/guides/developer-mode) |
| Claude | Supported | https | oauth | conditional | Organization static-header beta can use Authorization: Bearer <token>. CLC has no OAuth; personal OAuth setup is unavailable. No-auth is an explicit opt-in. [Official 1](https://claude.com/docs/connectors/building) / [Official 2](https://claude.com/docs/connectors/building/authentication) |
| Microsoft Copilot | Supported | https | bearer | conditional | Copilot Studio: API key → Header → Authorization, credential value Bearer <token>; confirm actual header forwarding. OAuth and other key headers are not implemented. Teams / Microsoft 365 Copilot are publishing channels. [Official 1](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-add-existing-server-to-agent) / [Official 2](https://learn.microsoft.com/en-us/microsoft-copilot-studio/mcp-create-new-server) / [Official 3](https://learn.microsoft.com/en-us/microsoft-copilot-studio/publication-fundamentals-publish-channels) |
| Notion | Supported | https | bearer | supported | Custom Agents support header authentication. Use Authorization: Bearer <token>; workspace permission and plan availability apply. [Official 1](https://www.notion.com/help/mcp-connections-for-custom-agents) |
| Slack | Supported | https | oauth | auth-limited | Slackbot supports no-auth, signed Slack identity and OAuth. CLC only intersects at no-auth. Bearer is not compatible; configure with CLI/Codex and explicitly choose no-auth only for intentionally public tools. [Official 1](https://docs.slack.dev/ai/slackbot-mcp-client/) |
| Cursor | Supported | https | bearer | supported | Configure a remote MCP URL and Authorization: Bearer <token>. Use a client version that forwards configured headers; verify the actual request. [Official 1](https://cursor.com/docs/context/mcp) / [Official 2](https://prod.cursor.com/help/customization/mcp) |
| GitHub Copilot | Supported | https | bearer | supported | Available in supported VS Code, JetBrains and CLI surfaces; use each host’s HTTP MCP configuration and Authorization: Bearer <token>. Host capabilities and organization policy differ. [Official 1](https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp-in-your-ide/extend-copilot-chat-with-mcp) / [Official 2](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-mcp-servers) |
| Raycast | Supported | https | bearer | supported | AI Chat, Quick AI and AI Commands use HTTP MCP. Set Authorization: Bearer <token> in HTTP Headers; Raycast AI plan availability applies. [Official 1](https://manual.raycast.com/ai/model-context-protocol) |

HTTPS uses Streamable HTTP; CLC does not implement legacy standalone SSE transport. Microsoft Copilot means the Copilot Studio integration, publishable to Teams / Microsoft 365 Copilot; Teams is not a separate preset. GitHub Copilot is distinct, with VS Code/JetBrains/CLI as hosts.

## Authentication intersection

Notion, Cursor, GitHub Copilot and Raycast can reuse generic HTTPS with `Authorization: Bearer <token>`. Claude organization-admin static headers beta can use the same path; personal OAuth remains unsupported. For Copilot Studio, configure API-key Header named Authorization with the complete value `Bearer <token>`. This is a compatibility inference from the documented Header mechanism, requiring actual tenant forwarding verification, not a claim of client acceptance.

Slackbot documents none, Slack identity, DCR and manual OAuth. Its custom headers concern identity lookup, not MCP requests. Only none intersects with CLC today. Do not present bearer as supported by Slackbot or silently downgrade to no-auth; the UI requires an explicit choice. Sensitive long-lived usage still needs generic OAuth or a trusted authentication gateway. ChatGPT HTTPS OAuth/mixed auth is also unimplemented; prefer Secure Tunnel.

CLC accepts only `openai` / `none` / `bearer`. Arbitrary static key headers, query tokens, OAuth/DCR and Slack signature verification are not implemented. Existing Authorization headers cover multiple clients, so no new static-header abstraction is needed. No-auth exposes allowed tools to network-reachable callers; select it only for intentionally public tools. Auth and toolPolicy are per ingress; tasks remain shared.

## CLI / Codex

Run `cli ingress presets` offline for IDs, recommendations, client auth capabilities and limitations. Help includes presets; ingress list and onboarding attach each entry’s preset metadata. `supportedAuth` describes the client/CLC intersection; `recommendedAuth` may be unimplemented OAuth, while `httpsAuth` is a safe UI initial value, not connection proof. Custom has no brand allowlist.

Natural-language requests map to:

- “Add Cursor” → cursor + https + bearer.
- “Add another work Notion with bearer” → new notion + https + bearer entry with a custom name; preserve existing entries.
- “Configure Raycast HTTPS” → raycast + https + bearer.
- “What is missing for Copilot Studio?” → microsoft-copilot; explain conditional API-key Header compatibility and the missing OAuth server when OAuth is required.

Omitted names use the first available Notion, Notion 2, Notion 3, etc. Explicit names are preserved. Saved/ready does not mean connected: follow [Codex setup](codex-setup.md), verify connector_verify from the actual client, then perform harmless task acceptance when required.
