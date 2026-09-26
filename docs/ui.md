# UI components

The renderer uses React in the existing Tauri system WebView. Rust owns configuration, connections, tasks, authentication, native windows, and updates. Vite builds the main window, tray panel, and usage rail. All three entry points share tokens, preferences, quota presentation, and interaction components.

## Information and navigation

Home is the product-owned connection topology: source and Agent brand nodes, curved status wires, a draggable mascot, and the tactile power switch. Node names and versions appear on demand. The mascot retains state expressions, six click effects, resisted dragging, and damped recoil; its displacement pulls the central ring and attached wires. The power knob springs into place and emits stars on a successful off-to-on transition. The default Agent limit is four; management exposes all discovered Agents.

The lower-left navigation capsule expands into a workspace sheet for Tasks, Records, and Settings. The sheet animates from the capsule, supports a wider layout, and returns focus when closed. Task documents use a masonry grid with prompt previews; selecting a document expands it into an elastic detail panel. Records retain filtering and search. Long content scrolls inside sheets and panels.

Configuration, connection details, Agent management, quota details, and updates use ElasticPanel through the shared Dialog export. Panels morph from their triggers, can be dragged by their headers, animate content height changes, and return toward their trigger on dismissal. Nested panels recess the previous panel. Clicking another exposed topology or navigation anchor transfers between panels. Short explanations use tooltips; quota details use the product's shared spring-following second bubble.

These layouts, animations, and trigger rules belong to the product. Component-library changes preserve them; Radix supplies interaction primitives, not replacement product styling.

## Tokens, typography, and icons

`ui/tokens.css` owns semantic canvas, surface, text, border, accent, success, warning, danger, focus, shadow, spacing, radius, and sizing tokens. `ui/style.css` styles standard components; `ui/product.css` owns the topology, mascot, navigation, task documents, and elastic panels. `ui/subscriptions/bubble.css` owns quota bubbles. Do not create page-specific versions of standard controls or patch library internals.

`ui/theme.ts` defaults to system appearance and observes browser and native changes. Explicit Light and Dark choices override it. The first-paint script applies the stored/system choice before rendering. The canvas is opaque white in light mode and black in dark mode; surfaces and borders establish hierarchy. Theme-color swatches update accents independently of semantic warnings and brand artwork.

Typography uses the operating system stack: system UI, Apple system, Segoe UI, PingFang SC, and Microsoft YaHei fallbacks. No proprietary font is bundled. Check actual rendered fonts in the target WebView rather than assuming the first declared family was used. Normal text is 13–14px; metadata uses 12px. Long reading content uses 14px with a relaxed line height. Standard controls are 36px high; compact icon controls are 32px. Spacing follows 4, 8, 12, 16, 24, and 32px steps. Control, row, and panel radii are centralized.

Functional icons use the shared Icon wrapper with Lucide at 16px or 20px and a 1.75px stroke. Icon-only buttons need accessible names; decorative SVGs are hidden from assistive technology. Brand marks retain their original assets and colors.

## Shared controls and floating content

`ui/components/ui/index.tsx` provides Button, IconButton, Input, Field, SingleChoice, Switch, Checkbox, Dialog, Tooltip, Popover, Menu, Progress, Notice, Status, and CopyField. Group and Row only arrange content. Simple controls use semantic HTML; Radix supplies selection, focus, keyboard navigation, dismissal, portal positioning, and modal behavior.

- Every single-choice field uses SingleChoice. At most five options render as capsule radios when they fit; larger sets or insufficient width use Radix Select. Theme colors use circular radio swatches. Preserve arrow-key navigation, labels, and disabled states.
- Forms have labels above inputs and one column by default. Field errors connect through `aria-describedby` and `aria-invalid`; invalid submission focuses the first invalid input. Errors do not close forms. Compact controls do not bypass validation.
- Dialogs have an accessible title, scrollable body, and optional header and footer. Radix traps focus and keeps covered content inert. Escape closes only the topmost dialog; focus returns to its trigger. Busy operations can block dismissal.
- Tooltips appear on hover and focus. Detail popovers also open by keyboard or click, remain interactive, and stay inside viewport bounds. One shared quota detail bubble moves and resizes with a spring when the selected entry changes. It opens after 120 ms on first hover and retains a 180 ms leave corridor. History details trigger and anchor on the right-hand values in home, tray, and rail. Quota estimates open from the pace text, rather than the progress bar or whole reading. Nested quota bubbles report padded native hit bounds throughout motion.
- Loading, empty, error, disabled, stale, and success states reflect native responses. Cached readings are marked stale after failure. Do not fabricate missing values or silently replace errors with success.

## Connections and Agents

The add-source picker uses the bundled source registry and allows repeated sources with unique default names. Selecting a source opens its form. Applicable provider, domain, credential, authentication, and tool-policy fields are visible without collapsed validation areas. Field help links sit beside labels. Configured domains omit protocol, port, and path; the service builds the MCP URL. The form saves configuration before connection details start the ingress and display its current result. Failure leaves the saved source editable and offers retry.

Opening a source shows details first; Edit opens configuration. Blank stored-secret inputs preserve credentials. Secrets are read from the authenticated ingress-specific endpoint and displayed as masked, copyable fields; they never enter global status or logs. URL refresh and token regeneration retain native lifecycle behavior. Secure Tunnel shows Tunnel ID and Runtime API Key. Fixed domains and reverse-proxy targets remain readable while stopped.

Tool policy stays compact: preset, count, and View all. A nested dialog lists tools or stages custom checkbox selection; required dependencies stay selected. OAuth details list pending clients and grants with explicit Allow, Deny, and Revoke. Waiting and expired requests follow service state. Client names are self-reported; authorization retains existing connection and task boundaries.

Agent management shows availability, installation/version details, enable switches, and official links. Enabled Agents support drag ordering and arrow-key ordering on focused cards. Discovery refreshes silently unless manually requested. Display settings control rail visibility, size, spacing, home Agent count, quota display, reset time format, refresh interval, and history periods.

## Quota, tray, and native surfaces

QuotaBubble and QuotaRing share presentation across home details, tray, and rail. Summaries show allowance and reset time; model estimates, reset-credit details, and history appear on demand. Unknown amounts remain unknown. Existing pace/cycle calculations and source limitations remain in [Subscription usage](subscriptions.md).

The React rail retains the native layout, pointer, docking, and geometry contract. The browser preview uses the same renderer. Native window placement and OS context menus remain owned by Rust. Nested popovers report bounds for native hit testing. Main-window scrolling is independent of transparent tray/rail constraints.

Update checks, downloads, installation, and release notes retain the native updater. Downloading does not stop connections; installation restores previously running connections. Windows caption controls and macOS integration remain native.

## Responsive behavior and acceptance

Narrow layouts reduce topology geometry while retaining the graph, stack settings controls, wrap actions, and constrain dialogs to the viewport. Dense quota content scrolls inside its available space. Long labels and content wrap without hiding main-page overflow. Reduced-motion preferences disable nonessential animation while retaining state changes.

Validate real loading, empty, connected, unavailable, stale, and error states; keyboard traversal, nested Escape, focus restoration, form errors, theme changes, long localized text, and narrow layouts. Verify native WebView, tray, and rail behavior separately from previews. Use existing checks and real interaction acceptance without adding test cases.
