# OAuth authentication

An HTTPS ingress can use `auth: "oauth"`. Local Connector includes an authorization server; no external identity provider or account database is required. Secure Tunnel and static Bearer authentication remain separate options.

## Connect

1. Select OAuth when creating or editing an HTTPS connection, then start it.
2. Add the MCP URL to an OAuth-capable client. Clients supporting dynamic registration discover the registration endpoint automatically.
3. Keep the authorization browser page open. In Local Connector, open that connection’s details, inspect the pending card’s client name and callback domain, then click **Allow connection**. No code entry is required; the card’s Deny button rejects the request.
4. The browser returns to the client. Connection details show authorized clients and let the owner revoke each grant.

Only approve requests initiated by you. Client names are self-reported. Local consent is protected by the existing authenticated management channel; the public server has no approval endpoint. Grants authorize the ingress's allowed tools and shared tasks, not a separate user workspace. Changing the ingress identity, tool policy or public URL revokes grants but preserves registered clients, so they can authorize again with their existing client credentials. A stable HTTPS hostname avoids having to update client connection URLs.

## Protocol and routing

The authorization server implements authorization code with mandatory S256 PKCE, exact registered redirect URI matching, audience binding to the ingress MCP URL (`resource` is required on authorization and token requests), the `mcp` scope, and refresh-token rotation. Access tokens expire after one hour; grants remain valid until manually revoked or invalidated by security or connection changes. Used refresh tokens trigger grant revocation on replay. Repeated authorization URLs with the same client, redirect URI, state, PKCE challenge and resource reuse the same pending request. Independent authorization transactions remain separate. Authorization requests expire after five minutes and approved codes after one minute. Access tokens, refresh tokens and client secrets are persisted only as hashes in the ingress's private state directory; pending approvals and codes are memory-only.

Public routes on the same HTTPS origin:

- `/mcp`: protected MCP endpoint.
- `/.well-known/oauth-protected-resource/mcp` and `/.well-known/oauth-protected-resource`: resource metadata.
- `/.well-known/oauth-authorization-server`: authorization server metadata.
- `/oauth/register`: dynamic client registration (RFC 7591).
- `/oauth/authorize` and `/oauth/resume`: browser authorization and return to the client.
- `/oauth/token`: code exchange and refresh.
- `/oauth/revoke`: token revocation.

A custom reverse proxy or path-restricted named tunnel must forward all these routes to the same ingress listener. Managed whole-host tunnels already forward the routes. There is no hosted login account: the local device owner is the resource owner. OAuth browser endpoints never expose the local management API.

Clients can use `none`, `client_secret_basic` or `client_secret_post` token endpoint authentication. HTTPS callbacks and HTTP IP-loopback callbacks are accepted; custom URI schemes and wildcard callbacks are not. Client ID Metadata Documents, external authorization servers, per-user task isolation and per-tool OAuth scopes are not implemented. Register clients with DCR or local preregistration instead. A client's OAuth support does not establish end-to-end compatibility with every provider.

The implementation bounds stored clients, pending requests and grants. Dynamic registration is limited to 20 requests per hour per running ingress and 128 stored clients. Unused registrations expire after 30 days; existing grants keep their client metadata. Authorization and token responses are not cached. The authorization browser page disallows framing and sends no referrer.

## Local preregistration and management

Use the installed executable's CLI. Sensitive input goes through stdin, and the registration response contains the client secret when applicable; keep that response out of shared logs.

- `cli ingress oauth list <id>` lists pending requests and grants.
- `cli ingress oauth register <id> --stdin` accepts JSON containing `client_name`, `redirect_uris` and `token_endpoint_auth_method`. For public clients, specify `"none"`; otherwise the default is `"client_secret_basic"`.
- `cli ingress oauth revoke <id> --stdin` accepts `{"id":"grant-id"}`.

Authorization decisions are made in the local UI. Neither registration nor knowing a client ID grants access without owner approval.
