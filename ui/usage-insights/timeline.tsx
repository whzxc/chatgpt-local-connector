import { useEffect, useMemo, useRef, useState } from "react";
import { Collapsible } from "radix-ui";
import { ChevronsDown, Diamond, Search, X } from "lucide-react";
import { Button, Icon, IconButton, Input, AnimatedSize, AnimatedCollapse, Popover, DetailDialog, ErrorCallout, LoadingIndicator, Progress, HoverPreviewGroup } from "../components/ui";
import { refresh, type Compaction, type ContextObservation, type Detail, type Response, type Reasoning, type AssistantMessage, type Tool, type ToolDetail, type Turn } from "./bridge";
import { Empty } from "./components";
import { text, number, duration, percent, promptPreview, status, date } from "./format";
import { locale } from "../i18n";
import "./timeline.css";

type Event = { kind: "response"; row: Response; tools: Tool[]; reasoning: Reasoning[]; messages: AssistantMessage[]; at: number } | { kind: "tool"; row: Tool; at: number } | { kind: "compaction"; row: Compaction; at: number };
const keyOf = (event: Event) => `${event.kind}:${event.kind === "compaction" ? event.row.responseId : event.row.id}`;
const clock = (at: number) => new Date(at).toLocaleTimeString(locale.get(), { hour: "2-digit", minute: "2-digit", second: "2-digit" });
const tokens = (n: number | null | undefined) => `${number(n, true)} Tokens`;
const contextPoint = (point: ContextObservation | null) => `${number(point?.input, true)}${point?.input != null && point.contextWindow && point.contextWindow > 0 ? ` (${percent(point.input / point.contextWindow * 100)})` : ""}`;
const total = (r: Response) => r.reliable && r.tokens.input != null && r.tokens.output != null ? r.tokens.input + r.tokens.output : null;
const cost = (n: number | null) => n == null ? "—" : `≈${new Intl.NumberFormat(locale.get(), { style: "currency", currency: "USD", maximumFractionDigits: 4 }).format(n)}`;

function TurnSummary({ turn, index, compactions }: { turn: Turn; index: number; compactions: number }) {
  return <div className="trace-turn-summary">
    <div className="insight-between"><strong>{text("turnNumber", { n: index + 1 })}</strong><span>{status(turn.status)}</span></div>
    <p>{promptPreview(turn.prompt, turn.promptImages) || text("noPrompt")}</p>
    <small>{[turn.model, turn.effort].filter(Boolean).join(" · ") || "—"}</small>
    <dl>
      <div><dt>{text("requests")} / {text("tools")}</dt><dd>{number(turn.usage.requests)} / {number(turn.toolCount)}</dd></div>
      <div><dt>Tokens</dt><dd>{number(turn.usage.total, true)}</dd></div>
      <div><dt>{text("duration")}</dt><dd>{duration(turn.durationMs)}</dd></div>
      <div><dt>{text("estimatedCost")}</dt><dd>{cost(turn.usage.estimatedUsd)}</dd></div>
      <div><dt>{text("cacheHit")}</dt><dd>{turn.usage.input && turn.usage.cached != null ? percent(turn.usage.cached / turn.usage.input * 100) : "—"}</dd></div>
      <div><dt>{text("contextCompaction")}</dt><dd>{number(compactions)}</dd></div>
    </dl>
    <div className="trace-summary-context"><span>{text("contextObserved")}</span><strong>{number(turn.contextStart?.input, true)} → {number(turn.contextEnd?.input, true)}</strong></div>
  </div>;
}

export function TaskTimeline({ detail }: { detail: Detail }) {
  const turns = useMemo(() => [...detail.turns].sort((a, b) => (a.startedAt ?? 0) - (b.startedAt ?? 0) || a.id.localeCompare(b.id)), [detail.turns]);
  const latest = turns.at(-1)?.id;
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(latest ? [latest] : []));
  const [active, setActive] = useState(latest);
  const [searching, setSearching] = useState(false);
  const [search, setSearch] = useState("");
  const anchors = useRef(new Map<string, HTMLButtonElement>());
  const previousLatest = useRef(latest);
  const pendingJump = useRef<string | null>(null);
  const jumpReady = useRef<string | null>(null);
  useEffect(() => {
    if (!previousLatest.current && latest) { setExpanded(new Set([latest])); setActive(latest); }
    previousLatest.current = latest;
  }, [latest]);
  const jump = (id: string) => {
    pendingJump.current = id;
    jumpReady.current = null;
    setSearch(""); setActive(id); setExpanded(new Set([id]));
    requestAnimationFrame(() => { anchors.current.get(id)?.scrollIntoView({ block: "start", behavior: "instant" }); anchors.current.get(id)?.focus({ preventScroll: true }); });
  };
  const settleJump = (id: string) => {
    if (pendingJump.current !== id || jumpReady.current !== id) return;
    requestAnimationFrame(() => {
      if (pendingJump.current !== id || [...anchors.current.values()].some(node => node.closest(".trace-turn")?.querySelector('[data-resizing="true"]'))) return;
      pendingJump.current = null;
      anchors.current.get(id)?.scrollIntoView({ block: "start", behavior: "instant" });
      anchors.current.get(id)?.focus({ preventScroll: true });
    });
  };
  const matches = turns.filter(turn => `${promptPreview(turn.prompt, turn.promptImages)} ${turn.model ?? ""} ${turn.effort ?? ""}`.toLowerCase().includes(search.toLowerCase()));
  return <div className="task-trace">
    <div className="trace-toolbar">
      <h2>{text("trajectory")} <small>· {number(detail.turnCount)} {text("turns")}</small></h2>
      <IconButton icon={searching ? X : Search} label={text(searching ? "closeSearch" : "searchTrace")} onClick={() => { setSearching(!searching); setSearch(""); }} />
    </div>
    {searching && <Input autoFocus aria-label={text("searchTrace")} placeholder={text("searchTrace")} value={search} onChange={e => setSearch(e.target.value)} onKeyDown={e => { if (e.key === "Escape") { setSearch(""); setSearching(false); } }} />}
    <nav aria-label={text("turnNavigation")}><HoverPreviewGroup className="trace-map" preview={id => {
      const index = turns.findIndex(turn => turn.id === id), turn = turns[index];
      return turn ? <TurnSummary turn={turn} index={index} compactions={detail.compactions.filter(row => row.turnId === id).length} /> : null;
    }}>
      {turns.map((turn, index) => {
        const observations = [turn.contextStart, turn.contextEnd];
        const compactions = detail.compactions.filter(c => c.turnId === turn.id).length;
        return <Button key={turn.id} data-hover-target={turn.id} variant="ghost" className={`trace-map-turn ${active === turn.id ? "is-active" : ""}`} aria-current={active === turn.id ? "step" : undefined} aria-label={`${text("turnNumber", { n: index + 1 })} · ${promptPreview(turn.prompt, turn.promptImages)}`} onClick={() => jump(turn.id)}>
          <span className="trace-map-bars" aria-hidden="true">{observations.map((point, i) => <i key={i} style={{ height: point?.input != null && point.contextWindow ? `${Math.max(8, Math.min(100, point.input / point.contextWindow * 100))}%` : "8%" }} />)}{compactions > 0 && <Icon icon={Diamond} />}</span>
          <span>{String(index + 1).padStart(2, "0")}</span>
        </Button>;
      })}
    </HoverPreviewGroup></nav>
    <div className="trace-turns">
      {matches.map(turn => {
        const index = turns.indexOf(turn), isOpen = expanded.has(turn.id);
        const summary = promptPreview(turn.prompt, turn.promptImages) || text("noPrompt");
        return <Collapsible.Root key={turn.id} className="trace-turn" open={isOpen} onOpenChange={value => { setActive(turn.id); setExpanded(previous => { const next = new Set(previous); value ? next.add(turn.id) : next.delete(turn.id); return next; }); }}>
          <AnimatedSize onSettled={() => { if (pendingJump.current) settleJump(pendingJump.current); }}><div className="trace-turn-header"><Collapsible.Trigger asChild><Button variant="ghost" ref={node => { if (node) anchors.current.set(turn.id, node); else anchors.current.delete(turn.id); }} className="trace-turn-heading">
            <span className={`trace-turn-number ${detail.compactions.some(row => row.turnId === turn.id) ? "has-compaction" : ""}`} title={detail.compactions.some(row => row.turnId === turn.id) ? text("contextCompaction") : undefined}>{String(index + 1).padStart(2, "0")}</span>
            <span className="trace-turn-prompt"><span className="trace-turn-title">{summary}</span><span className="trace-turn-meta"><small>{[turn.model, turn.effort].filter(Boolean).join(" · ") || "—"} · {number(turn.usage.requests)} {text("requests")} · {number(turn.toolCount)} {text("tools")} · {status(turn.status)}{turn.startedAt != null && ` · ${date(turn.startedAt)}`}</small>
              {!isOpen && <span className="trace-turn-context">{number(turn.contextStart?.input, true)} → {number(turn.contextEnd?.input, true)}</span>}
            </span></span>
          </Button></Collapsible.Trigger>
          <Popover label="Prompt" trigger={<Button variant="ghost" className="trace-prompt-label">Prompt</Button>}><div className="trace-prompt-popover"><div className="trace-prompt-text">{turn.prompt ?? summary}</div></div></Popover></div></AnimatedSize>
          <AnimatedCollapse open={isOpen} className="trace-turn-body" onSettled={() => { if (pendingJump.current) settleJump(pendingJump.current); }}>
            <TurnActivity turn={turn} threadId={detail.id} revision={detail.lastEventAt} onReady={() => { if (pendingJump.current === turn.id) { jumpReady.current = turn.id; settleJump(turn.id); } }} />
          </AnimatedCollapse>
        </Collapsible.Root>;
      })}
      {!matches.length && <Empty />}
    </div>
    {latest && <div className="trace-current"><Button onClick={() => jump(latest)}><Icon icon={ChevronsDown} />{text("backToCurrent")}</Button></div>}
  </div>;
}

function TurnActivity({ turn, threadId, revision, onReady }: { turn: Turn; threadId: string; revision: number | null; onReady: () => void }) {
  const [detail, setDetail] = useState<Detail>();
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  useEffect(() => { if (detail) onReady(); }, [detail, onReady]);
  useEffect(() => {
    let alive = true;
    void refresh({ scope: "thread", threadId, turnId: turn.id }).then(result => {
      if (!alive) return;
      if (result.state !== "ready" || !result.thread) throw new Error(result.message || text("unavailable"));
      setDetail(result.thread); setError("");
    }).catch(e => { if (alive) setError(e instanceof Error ? e.message : String(e)); });
    return () => { alive = false; };
  }, [threadId, turn.id, revision, retry]);
  const responses = useMemo(() => detail?.responses.filter(row => row.turnId === turn.id).sort((a, b) => a.at - b.at || a.id.localeCompare(b.id)) ?? [], [detail, turn.id]);
  const events = useMemo<Event[]>(() => {
    if (!detail) return [];
    const compactions = detail.compactions.filter(row => row.turnId === turn.id);
    const groups = new Map<string, Tool[]>();
    const independent: Tool[] = [];
    for (const tool of detail.tools.filter(row => row.turnId === turn.id)) {
      // Records have no request foreign key. Only group a unique observation
      // inside the original tool emission/result interval in the same turn.
      const candidates = tool.completedAt == null ? [] : responses.filter(row => row.at >= tool.at && row.at <= tool.completedAt!);
      const response = candidates.length === 1 ? candidates[0] : undefined;
      if (response?.reliable && response.family === "response" && !compactions.some(row => row.at >= tool.at && row.at <= response.at)) {
        groups.set(response.id, [...(groups.get(response.id) ?? []), tool]);
      } else independent.push(tool);
    }
    const reasoning = new Map<string, Reasoning[]>();
    for (const entry of detail.reasoning.filter(row => row.turnId === turn.id)) {
      const end = entry.at + (entry.durationMs ?? 0);
      const next = responses.find(row => row.at >= end);
      // Usage is recorded after the response. Associate summaries within that
      // response interval, never across turns or a compaction boundary.
      if (!next || responses.filter(row => row.at === next.at).length !== 1 || compactions.some(row => row.at >= entry.at && row.at < next.at)) continue;
      reasoning.set(next.id, [...(reasoning.get(next.id) ?? []), entry]);
    }
    const messages = new Map<string, AssistantMessage[]>();
    for (const message of detail.messages.filter(row => row.turnId === turn.id)) {
      const next = responses.find(row => row.at >= message.at);
      if (!next || responses.filter(row => row.at === next.at).length !== 1 || compactions.some(row => row.at >= message.at && row.at < next.at)) continue;
      messages.set(next.id, [...(messages.get(next.id) ?? []), message]);
    }
    return [
      ...responses.map(row => {
        const tools = (groups.get(row.id) ?? []).sort((a, b) => a.at - b.at || a.id.localeCompare(b.id));
        return { kind: "response" as const, row, tools, reasoning: reasoning.get(row.id) ?? [], messages: messages.get(row.id) ?? [], at: Math.min(row.at, ...tools.map(tool => tool.at)) };
      }),
      ...independent.map(row => ({ kind: "tool" as const, row, at: row.startedAt ?? row.at })),
      ...compactions.map(row => ({ kind: "compaction" as const, row, at: row.at })),
    ].sort((a, b) => a.at - b.at || keyOf(a).localeCompare(keyOf(b)));
  }, [detail, responses, turn.id]);
  const render = (event: Event) => {
    const id = keyOf(event);
    if (event.kind === "compaction") return <div className="trace-compaction" key={id}>
      <span className="trace-event-dot is-compaction" aria-hidden="true" /><span>{text("contextCompaction")}</span>
      <span>{contextPoint(event.row.before)} → {contextPoint(event.row.after)}</span>
      <span className="trace-compaction-time">{event.row.durationMs != null && <span>{duration(event.row.durationMs)}</span>}<time>{clock(event.at)}</time></span>
    </div>;
    const response = event.kind === "response" ? event.row : null;
    const tool = event.kind === "tool" ? event.row : null;
    const tools = event.kind === "response" ? event.tools : tool ? [tool] : [];
    const toolLabel = tools.length === 1 ? tools[0].name ?? text("toolCall") : `${number(tools.length)} ${text("tools")}`;
    const preview = (event.kind === "response" ? event.messages.map(message => message.text).join(" ") : "") || tools.map(item => detail?.toolPreviews[item.id]).filter(Boolean).join(" · ");
    const isOpen = selected === id;
    return <Collapsible.Root key={id} className={`trace-event ${isOpen ? "is-selected" : ""}`} open={isOpen} onOpenChange={value => setSelected(value ? id : null)}>
      <Collapsible.Trigger asChild><Button variant="ghost" className="trace-event-trigger">
        <span className={`trace-event-dot ${tool ? "is-tool" : ""}`} aria-hidden="true" />
        <span className="trace-event-name">{response ? `${text("request")} ${responses.indexOf(response) + 1}` : text("toolCall")}{tool && <strong>{tool.name ?? "—"}</strong>}{response && tools.length > 0 && <strong className="trace-linked-tools">{toolLabel}</strong>}</span>
        {preview && <span className="trace-output-preview">{preview.replace(/\*\*(.*?)\*\*/g, "$1").replace(/^#{1,6}\s+/gm, "").replace(/\s+/g, " ")}</span>}
        <span className="trace-event-summary">{response ? tokens(total(response)) : tool?.outputBytes != null ? `${number(tool.outputBytes, true)} B` : text("pending")}</span>
        <time>{clock(response?.at ?? event.at)}</time>
      </Button></Collapsible.Trigger>
      <AnimatedCollapse open={isOpen} className="trace-inspector">
        {event.kind === "response" && event.reasoning.length > 0 && <ReasoningSummary entries={event.reasoning} />}
        {response && <ContextView current={response.context} previous={response.previousContext} compactions={detail?.compactions ?? []} />}
        {response && <Consumption response={response} tool={null} responses={responses} tools={detail?.tools ?? []} />}
        {tools.map(item => <div key={item.id} className={response ? "trace-linked-tool" : undefined}>
          {response && <div className="trace-linked-tool-heading"><span>{text("toolCall")} · {item.name ?? "—"}</span><time>{clock(item.startedAt ?? item.at)}</time></div>}
          <div className="trace-tool-summary"><Consumption response={null} tool={item} responses={responses} tools={detail?.tools ?? []} />
            <div className="trace-detail-action"><DetailDialog title={item.name ?? text("toolCall")} trigger={<Button variant="ghost">{text("details")}</Button>}><ToolPayload threadId={threadId} turnId={turn.id} tool={item} /></DetailDialog></div>
          </div>
        </div>)}
        {event.kind === "response" && event.messages.length > 0 && <section className="trace-assistant-output" aria-label={text("assistantOutput")}><h3>{text("assistantOutput")}</h3>{event.messages.map(message => <div className="trace-output-message" key={message.id}><time>{clock(message.at)}</time><div>{message.text}</div></div>)}</section>}
      </AnimatedCollapse>
    </Collapsible.Root>;
  };
  return <div className="trace-activity">
    {error && <ErrorCallout action={<Button variant="ghost" onClick={() => setRetry(n => n + 1)}>{text("retry")}</Button>}>{error}</ErrorCallout>}
    {detail?.contentError && <ErrorCallout action={<Button variant="ghost" onClick={() => setRetry(n => n + 1)}>{text("retry")}</Button>}>{detail.contentError}</ErrorCallout>}
    {!detail && !error && <div className="trace-loading"><LoadingIndicator label={text("loading")} /></div>}
    {events.map(render)}
    {detail && events.length === 0 && <Empty />}
  </div>;
}

function ReasoningSummary({ entries }: { entries: Reasoning[] }) {
  const groups: { text: string; entries: Reasoning[] }[] = [];
  for (const entry of [...entries].sort((a, b) => a.at - b.at || a.id.localeCompare(b.id))) {
    const previous = groups.at(-1);
    if (previous?.text.trim() === entry.text.trim()) previous.entries.push(entry);
    else groups.push({ text: entry.text, entries: [entry] });
  }
  return <section className="trace-reasoning-summary" aria-label={text("thinking")}>
    <h3>{text("thinking")}</h3>
    {groups.map(group => {
      const first = group.entries[0], last = group.entries.at(-1)!;
      const elapsed = group.entries.every(entry => entry.durationMs != null) ? group.entries.reduce((sum, entry) => sum + entry.durationMs!, 0) : null;
      return <div className="trace-reasoning-part" key={first.id}>
        <div className="trace-reasoning-text">{group.text.replace(/\*\*(.*?)\*\*/g, "$1").replace(/^#{1,6}\s+/gm, "")}</div>
        <div className="trace-reasoning-meta">{group.entries.length > 1 && <span>×{number(group.entries.length)}</span>}{elapsed != null && <span>{duration(elapsed)}</span>}<time>{clock(first.at)}{group.entries.length > 1 && ` – ${clock(last.at + (last.durationMs ?? 0))}`}</time></div>
      </div>;
    })}
  </section>;
}

function ContextView({ current, previous, compactions }: { current: ContextObservation; previous: ContextObservation | null; compactions: Compaction[] }) {
  const input = current?.input ?? null, capacity = current?.contextWindow ?? null;
  const before = previous?.input ?? null;
  const changed = previous && (previous.model !== current?.model || previous.contextWindow !== capacity);
  const compressed = previous && compactions.some(c => c.at >= previous.at && c.at < current.at);
  const delta = input != null && before != null ? input - before : null;
  return <div className="trace-context">
    <div className="trace-context-value"><span><small>{text("contextObserved")}</small><strong>{before != null && <span>{number(before, true)} → </span>}{number(input, true)}<small> / {number(capacity, true)}</small></strong></span>
      {input != null && capacity != null && <strong className="trace-occupancy">{percent(input / capacity * 100)}</strong>}
    </div>
    {input != null && capacity != null && <Progress value={input / capacity * 100} label={text("contextObserved")} />}
    <div className="trace-context-caption"><span>{delta == null ? text("contextBaselineUnknown") : `${text("observedChange")} ${delta > 0 ? "+" : ""}${number(delta, true)}`}</span><span>{compressed ? text("afterCompaction") : changed ? text("modelChanged") : text("requestInput")}</span></div>
  </div>;
}

function ToolPayload({ threadId, turnId, tool }: { threadId: string; turnId: string; tool: Tool }) {
  const [data, setData] = useState<ToolDetail>();
  const [error, setError] = useState("");
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let alive = true;
    setError("");
    void refresh({ scope: "thread", threadId, turnId, toolId: tool.id }).then(result => {
      if (result.state !== "ready" || !result.toolDetail) throw new Error(result.message || text("unavailable"));
      if (alive) setData(result.toolDetail);
    }).catch(error => { if (alive) setError(String(error.message ?? error)); });
    return () => { alive = false; };
  }, [threadId, turnId, tool.id, retry]);
  const format = (value: unknown): string => {
    if (value == null) return "—";
    if (typeof value === "string") {
      try { return format(JSON.parse(value)); } catch { return value; }
    }
    if (Array.isArray(value) && value.every(part => typeof part?.text === "string")) return value.map(part => format(part.text)).join("\n\n");
    if (typeof value === "object" && "output" in value && typeof value.output === "string") {
      return Object.entries(value).map(([key, part]) => `${key}: ${typeof part === "string" ? part : JSON.stringify(part, null, 2)}`).join("\n");
    }
    return JSON.stringify(value, null, 2);
  };
  return <div className="trace-tool-payload">
    {error && <ErrorCallout action={<Button variant="ghost" onClick={() => setRetry(n => n + 1)}>{text("retry")}</Button>}>{error}</ErrorCallout>}
    {!data && !error && <div className="trace-tool-loading"><LoadingIndicator label={text("loading")} /></div>}
    {data && <><h3>{text("input")}</h3><pre>{format(data.input)}</pre><h3>{text("output")}</h3><pre>{format(data.output)}</pre></>}
  </div>;
}

function Consumption({ response, tool, responses, tools }: { response: Response | null; tool: Tool | null; responses: Response[]; tools: Tool[] }) {
  const precedingTools = tools.filter(row => (row.startedAt ?? row.at) <= (tool?.startedAt ?? tool?.at ?? 0));
  const knownBytes = precedingTools.filter(row => row.outputBytes != null);
  const sumBytes = knownBytes.reduce((sum, row) => sum + row.outputBytes!, 0);
  if (!response) return <dl className="trace-values trace-tool-values"><div><dt>{text("callDuration")}</dt><dd>{duration(tool?.startedAt != null && tool.completedAt != null ? Math.max(0, tool.completedAt - tool.startedAt) : null)}</dd></div><div><dt>{text("bytes")}</dt><dd>{number(tool?.outputBytes, true)} B</dd></div><div><dt>{text("status")}</dt><dd>{tool?.status ? status(tool.status) : text(tool?.outputBytes == null ? "pending" : "returned")}</dd></div><div><dt>{text("turnOutputBytes")}</dt><dd>{knownBytes.length !== precedingTools.length && text("known")} {number(knownBytes.length ? sumBytes : null, true)} B</dd></div></dl>;
  const earlier = responses.slice(0, responses.indexOf(response) + 1);
  const known = earlier.some(r => total(r) == null), unknownCost = earlier.some(r => r.estimatedUsd == null);
  const cumulative = earlier.reduce((sum, r) => sum + (total(r) ?? 0), 0);
  const cumulativeCost = earlier.reduce((sum, r) => sum + (r.estimatedUsd ?? 0), 0);
  const entire = responses.reduce((sum, r) => sum + (total(r) ?? 0), 0);
  return <div className="trace-consumption">
    <div className="insight-between"><strong>{tokens(total(response))}</strong><strong>{cost(response.estimatedUsd)}</strong></div>
    <div className="trace-cost-bar" aria-label={text("turnCumulative")}><span style={{ width: `${entire ? (cumulative - (total(response) ?? 0)) / entire * 100 : 0}%` }} /><strong style={{ width: `${entire ? (total(response) ?? 0) / entire * 100 : 0}%` }} /></div>
    <div className="insight-between"><small>{text("turnCumulative")} {known && text("known")} {tokens(known && !cumulative ? null : cumulative)}</small><small>{unknownCost && text("known")} {cost(unknownCost && !cumulativeCost ? null : cumulativeCost)}</small></div>
    <dl className="trace-values"><div><dt>{text("input")}</dt><dd>{number(response.tokens.input, true)}</dd></div><div><dt>{text("output")}</dt><dd>{number(response.tokens.output, true)}</dd></div><div><dt>{text("cacheHit")}</dt><dd>{response.tokens.input && response.tokens.cached != null ? percent(response.tokens.cached / response.tokens.input * 100) : "—"}</dd></div></dl>
  </div>;
}
