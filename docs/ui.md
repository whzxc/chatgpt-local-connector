# UI components

The Vue desktop frontend uses Naive UI for standard controls. Import components by name rather than registering the entire library. `UiProvider` owns shared typography, control sizes, corner radii, brand colors, dark mode, and the English/Chinese component locale. `theme.ts` is the single reactive appearance preference.

## Forms and dialogs

- Use `NForm` and `NFormItem`, with labels above inputs and one column by default. Listener addresses and ports are managed by the application; show a copyable reverse-proxy target when needed, not editable fields.
- Use `NInput`, `NInputNumber`, `SingleChoice`, `NSwitch`, and `NButton` for standard controls. Supply accessible input names.
- Put credential and provider help links on the right of the field label, on the same line. Show the selected platform in the dialog title; the editable display name remains a form field. Platform selection uses the bundled brand icons, with a dashed plus for custom sources.
- All single-choice fields use `SingleChoice`: up to four options render as capsule tabs; larger sets render as a dropdown. Radio colors and capsule radii belong to `UiProvider`. Preserve accessible labels, keyboard navigation, disabled states, and form validation.
- Keep validation next to its field. Connection dialogs render every applicable field directly, without collapsed sections. Errors do not submit or close the form.
- Use `FormDialog` for editor and update dialogs. It provides a shared width, scrollable body, focus handling, dismiss behavior, and footer alignment. Actions that must finish before dismissal set `busy`.
- New HTTPS connections first acquire a managed MCP URL or accept a valid custom HTTPS /mcp URL. Authentication fields appear only after that step, and Save stays disabled until required values are valid. Managed URL discovery uses a private, leased draft runtime, excluded from saved ingress lists and denied public tool access. Cancel or a two-minute lost heartbeat stops the draft; saving adopts its existing tunnel and URL.
- For new Bearer connections, the local CLC UI generates a cryptographically random token. Saving persists it through the ingress secret store and presents a masked, copyable handoff before dismissal. Existing tokens stay unchanged unless explicitly regenerated and saved.
- Blank stored-secret fields preserve credentials. The ChatGPT editor loads its saved API key through the authenticated, ingress-specific local credentials endpoint and renders it as a password. Readback is never part of global status or logs. Other blank secret fields preserve stored values.
- `CopyField` combines the library input and button with clipboard handling. `NTooltip` owns hover/focus overlay positioning.

The overview shows ingress transport connectivity independently of inbound verification. Agent links show availability when installed and enabled, and connected status for live sessions. Both sides render logos only, with names and status available on hover or focus. Saving display metadata preserves verification; changing transport identity or authentication invalidates it.

## Styling boundaries

Customize library components through `UiProvider` theme overrides and documented props. Do not patch internal selectors from page styles. `SettingsGroup` and `SettingsRow` arrange content without defining their controls. The overview graph, mascot, and main connection button retain custom product visuals.

`native-controls.css` styles existing custom shell controls and excludes library component subtrees. New standard forms do not use these native styles. `tokens.css` applies to custom shell controls; library control tokens live in `UiProvider`.

Settings and editor/update dialogs load as separate frontend chunks. Keep appearance, localization, focus, keyboard navigation, validation, and save/readback checks part of UI acceptance; use the existing project checks without adding test cases.

Agent icons gently enlarge on hover. Clicking Codex opens its desktop application; CLI icons open their interactive executable in a user-owned terminal, without ACP/RPC arguments. On macOS, the system handler for executable `.command` files selects the terminal; on Windows, a new PowerShell console uses the configured console host. Interactive launches start in the home directory and are independent of ingress/task lifecycle.

The default window is an edge-to-edge connection canvas with a lower-left navigation capsule. Tasks, records, and settings expand from that capsule into an inset, independently scrollable sheet. Its lower-left close button (or Escape) returns to the still-mounted home canvas; keyboard focus returns to the selected navigation button. Native window dragging and Windows window controls remain available. Reduced-motion preferences disable the expansion animation. Collected platform logos and colored Agent SVGs are preferred; brands without a colored asset retain their original monochrome mark.

Curated control sources are ChatGPT, Claude, Microsoft Copilot, Notion, Slack, Cursor, GitHub Copilot and Raycast, plus Custom. Repeated sources are allowed and default names receive an available numeric suffix. See the [support matrix](control-sources.md) for authentication limits.

The compact add-source grid shows logo and name, with a short caveat for conditional/auth-limited presets. IngressDialog and SourceIcon consume the shared preset registry; overview tooltips show instance names. Advanced provider/auth configuration remains available through CLI/Codex. Control-source assets have their own semantic map and may reuse Agent brand artwork.
