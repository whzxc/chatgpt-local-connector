# Subscription usage

Quota monitoring uses the shared native core and works independently of ingress
connections. A working CLI or a free plan does not guarantee that its provider exposes
quota readings. Missing quota data is not treated as a full allowance.

## Sources

| Brand | Data source and scope |
| --- | --- |
| Codex / ChatGPT | Existing Codex account: quota windows, reported plan including Pro 5x/20x, available reset credits and their expiry dates; local rollout token history |
| OpenCode Go | Existing OpenCode credentials and subscription usage response |
| Claude | Existing Claude login and reported subscription quota windows |
| Cursor | Existing desktop login, plan and billing-cycle quotas; account token CSV export |
| Antigravity | Running local language server, or supported macOS login fallback; model-group quotas and local conversation token history |
| Grok | Existing login and reported plan, including Free; quota only when the response provides it |
| Kimi | Existing credentials and reported membership/quota data |

Sources depend on provider availability, authentication and platform support. Agent
execution and subscription authentication retain their own identities. Display grouping
combines Cursor with Cursor Agent, Antigravity with Gemini CLI, and Grok CLI/Build under
Grok; it does not change the underlying execution adapters.

Antigravity’s macOS login fallback uses the existing access token. If it expires, open
Antigravity and refresh its login; CLC does not renew the token itself.

## Controls and display

Open Agents management from the home graph's More button, then use the settings icon
beside its title. Settings have three groups:

- **Appearance:** show the quota rail, small/standard/large size, compact/standard/roomy
  spacing, and 4/6/8/10 home Agents (default 4). Home entries must be installed and enabled.
- **Quota:** remaining or used percentages, reset countdown or date/time, and automatic
  refresh every 1/3/5/10 minutes (default 3).
- **Usage:** independently show Today, Yesterday, Last 7 Days and Last 30 Days. The latter
  two include today and use local calendar days.

Each quota bubble has a manual refresh button. Plan titles open the provider's usage page or service homepage. OpenCode currently opens
Console because its usage response does not supply a workspace identity.
Quota windows are named 5h, weekly or monthly. Remaining display uses 额度 in Chinese;
used display uses 限额. Countdown text uses units such as `4d 2h`; 5h windows also show minutes.

Hover anywhere on the reset-count row to see individual credit expiry dates. Hover
anywhere on a period row to see model-level tokens and estimated cost. The rail uses horizontal
secondary bubbles matching its colors. Missing subscription information is shown as
“未获取到订阅信息”; a reported plan can still appear without quotas.

The macOS rail supports edge docking and a floating capsule. Its context menu contains
Settings, Restore defaults and Hide. Settings opens the preferences panel, not Agent
management. Hide only hides the rail; it does not disable quota monitoring. Docked rails
collapse when idle and expand on pointer entry. The browser preview shares these visuals
but stores its position separately from the native screen placement.

## Consumption pace

For a known reset window, average consumption is used quota divided by elapsed time.
The bubble projects when the remaining quota would run out at that rate. A reference
tick marks even consumption across the cycle and flips with Used/Remaining mode.
Green means at least 10% is projected to remain at reset, amber means less headroom,
and red warns of exhaustion. Hovering anywhere in the quota block (label, bar or remaining/reset row) opens its secondary bubble.

Projection waits for at least 60 seconds or 1% of the cycle. Under 5% used, unstable
near-limit/over-limit projections are suppressed. Zero usage, stale readings and unknown
or expired windows do not generate a burn-rate forecast. Estimates are cycle averages,
not measurements of only the last few minutes; a change in usage rate changes the outcome.

## Token and dollar estimates

Codex and Antigravity history describes local device logs, not a complete account invoice.
Cursor history comes from its account export. Dollar values are API-equivalent USD
estimates, not subscription charges. Unknown prices preserve token counts but exclude
unpriced costs; refreshing prices can change historical estimates without changing tokens.

Price precedence is:

1. [OpenUsage supplement](https://robinebers.github.io/openusage/pricing_supplement.json):
   additional model rates, aliases and fast-tier multipliers.
2. [LiteLLM](https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json).
3. [models.dev](https://models.dev/api.json).

Prices load from bundled data and local caches, with hourly background revalidation and
30-minute retries after a source failure. Failed downloads preserve cached prices.
A subsequent usage refresh reprices token records. Price requests transmit no account
credentials or usage records. OpenUsage-derived data and logic retain MIT attribution in
`shared/pricing/LICENSE.OpenUsage`. Implementation details are in [Development](development.md#subscription-development).

The header does not open hover details. When present, the warning/error icon opens
a secondary bubble containing the notices. Quota blocks and history rows use secondary
bubbles; native title tooltips are not used. Quota blocks share the usage-detail
animation and hover handling. These bubbles estimate the full-cycle API-equivalent USD value by dividing
observed cost since the inferred cycle start by the used quota fraction. The
cycle start is the reported reset time minus its window duration (including
5-hour, weekly and monthly windows). Timestamped history is retained for 32 days;
calendar-day history totals are not used for this calculation. The sample ends
at the earlier quota/history observation, with a maximum five-minute timestamp
skew. Estimates require at least 1% usage, a current unsaturated quota, and priced
tokens throughout the sample. Separate model quota pools are excluded when their
usage cannot be isolated. The secondary bubble contains the pace forecast, full-cycle
estimate and observed sample only. Incomplete logs, other devices, account switches,
early resets and changes in model mix can skew the estimate.
This is an inferred API-price equivalent, not a subscription balance or a promise
of future capacity.
