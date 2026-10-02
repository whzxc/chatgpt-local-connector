import { useEffect, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import {
  ArrowLeft,
  RefreshCw,
  Layers,
  GitBranch,
  Wrench,
  ChevronRight,
  Search,
  AlertCircle,
  Clock,
  Info,
  Link,
  ChartColumn,
  CircleHelp,
} from "lucide-react";
import {
  Button,
  Icon,
  IconButton,
  SingleChoice,
  Progress,
  Notice,
  Tooltip,
  TooltipProvider,
  HoverScope,
  Input,
  Popover,
} from "../components/ui";
import { locale } from "../i18n";
import {
  initialize,
  onResult,
  refresh,
  type ReadySnapshot,
  type Snapshot,
} from "./bridge";
import {
  Empty,
  Heading,
  Help,
  Pager,
  TokenBreakdown,
  Value,
} from "./components";
import { Details, type DetailView } from "./details";
import { Guide, type GuideSection } from "./guide";
import {
  text,
  number,
  date,
  short,
  status,
  percent,
  windowLabel,
  count,
} from "./format";
import "../style.css";
import "../product.css";
import "./style.css";

export default function App() {
  locale.use();
  const [data, setData] = useState<Snapshot>(),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const [days, setDays] = useState("7"),
    [selected, setSelected] = useState(""),
    [turnPage, setTurnPage] = useState(0);
  const [displayTurnPage, setDisplayTurnPage] = useState(0);
  const [guide, setGuide] = useState<GuideSection | null>(null);
  const [view, setView] = useState<DetailView | null>(null);
  const entryScope = useRef<"global" | "thread">("thread"),
    receivedInitial = useRef(false),
    initialized = useRef(false);
  const current = useRef(data);
  current.current = data;
  const alive = useRef(true),
    request = useRef(0),
    inFlight = useRef(false);
  const filter = useRef({ days, selected, turnPage });
  filter.current = { days, selected, turnPage };
  const accept = (next: Snapshot) => {
    if (alive.current) {
      setData(next);
      setError("");
    }
  };
  const update = async (mode: "auto" | "filter" | "manual" = "manual") => {
    if (inFlight.current && mode !== "filter") return;
    inFlight.current = true;
    const serial = ++request.current;
    setBusy(mode !== "auto");
    const f = filter.current;
    try {
      if (!initialized.current) {
        await initialize();
        initialized.current = true;
      }
      const next = await refresh({
        scope: f.selected ? "thread" : entryScope.current,
        days: Number(f.days),
        ...(f.selected ? { threadId: f.selected } : {}),
        turnOffset: f.turnPage * 4,
      });
      if (serial === request.current) {
        // Index collection can temporarily return no rows during a background refresh.
        // Keep the last visible result until the same selection has a new snapshot.
        if (!(
          mode === "auto" &&
          next.state === "collecting" &&
          current.current?.state === "ready"
        )) {
          accept(next);
          setDisplayTurnPage(f.turnPage);
        }
      }
    } catch (e) {
      if (alive.current && serial === request.current)
        setError(e instanceof Error ? e.message : text("refreshError"));
    } finally {
      if (alive.current && serial === request.current) {
        inFlight.current = false;
        setBusy(false);
      }
    }
  };
  const updateRef = useRef(update);
  updateRef.current = update;
  useEffect(() => {
    alive.current = true;
    const off = onResult((next) => {
      if (!receivedInitial.current) {
        entryScope.current = next.scope;
        receivedInitial.current = true;
        setDays(String(next.state === "ready" ? next.range.days : 7));
      }
      accept(next);
    });
    void initialize()
      .then(() => {
        initialized.current = true;
      })
      .catch((e) => setError(String(e)));
    const timer = setInterval(() => {
      if (document.visibilityState === "visible" && current.current)
        void updateRef.current("auto");
    }, 10000);
    return () => {
      alive.current = false;
      off();
      clearInterval(timer);
    };
  }, []);
  useEffect(() => {
    if (current.current) void updateRef.current("filter");
  }, [days, selected, turnPage]);
  const choose = (id: string) => {
    request.current++;
    inFlight.current = false;
    setView(null);
    setTurnPage(0);
    setSelected(id);
    setData((previous) =>
      previous?.state === "ready"
        ? { ...previous, thread: null, binding: "unknown" }
        : previous,
    );
  };
  const global = data?.scope === "global" && !selected;
  const ready = data?.state === "ready" ? data : undefined;
  const detail = ready?.thread;
  const label =
    ready?.tasks.find((task) => task.id === detail?.id)?.label ??
    (detail ? short(detail.id) : text("unbound"));
  const sourceIssues = [
    ...new Set([...(ready?.issues ?? []), ...(detail?.issues ?? [])]),
  ];
  return (
    <MotionConfig
      reducedMotion="user"
      transition={{ duration: 0.24, ease: [0.2, 0.8, 0.2, 1] }}
    >
      <TooltipProvider delayDuration={250}>
        <main
          className={`insight-app ${global ? "insight-overview" : "insight-task"}`}
        >
          <header className="insight-header">
            <div className="insight-title">
              {selected && (
                <IconButton
                  icon={ArrowLeft}
                  label={text("back")}
                  onClick={() => choose("")}
                />
              )}
              <Icon icon={Layers} size={20} />
              <strong className="insight-brand">Connector</strong>
              <span className="insight-divider" />
              <h1>{text(global ? "overview" : "task")}</h1>
            </div>
            <div className="insight-header-actions">
              {global && (
                <div className="insight-range">
                  <SingleChoice
                    label={text("range")}
                    value={days}
                    onChange={setDays}
                    options={["day", "week", "month"].map((key, i) => ({
                      value: ["1", "7", "30"][i]!,
                      label: text(key),
                    }))}
                  />
                </div>
              )}
              <IconButton
                icon={CircleHelp}
                label={text("guide")}
                onClick={() => setGuide("start")}
              />
            </div>
          </header>
          {error && (
            <Notice>
              <span role="alert">{error}</span>
              <IconButton
                icon={RefreshCw}
                label={text("retry")}
                onClick={() => void update()}
              />
            </Notice>
          )}
          {!data || data.state !== "ready" ? (
            <div className="insight-state">
              <Icon
                icon={
                  data?.state === "incompatible" ? AlertCircle : ChartColumn
                }
                size={20}
              />
              <h2>
                {text(
                  !data
                    ? "loading"
                    : data.state === "collecting"
                      ? "collecting"
                      : data.state === "incompatible"
                        ? "incompatible"
                        : "unavailable",
                )}
              </h2>
              <Help name="guide">{text(`guide.${data?.state === "collecting" ? "empty" : data?.state === "incompatible" ? "version" : "connection"}.body`)}</Help>
              <Button variant="ghost" onClick={() => setGuide(data?.state === "collecting" ? "start" : "recovery")}>{text(data?.state === "collecting" ? "guide.start" : "guide.recovery")}</Button>
              <IconButton
                icon={RefreshCw}
                label={text("retry")}
                onClick={() => void update()}
                disabled={busy}
              />
            </div>
          ) : (
            <>
              <AnimatePresence mode="wait" initial={false}>
                <motion.div
                  key={global ? "global" : (detail?.id ?? "unbound")}
                  className="insight-content"
                  initial={{ opacity: 0, y: 5 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -3 }}
                >
                  {global ? (
                    <Overview data={data} choose={choose} open={setView} guide={setGuide} />
                  ) : (
                    <>
                      <div className="insight-task-heading">
                        <div className="insight-title">
                          <h2>{label}</h2>
                          <Popover
                            label={text("binding")}
                            trigger={
                              <Button
                                variant="ghost"
                                className={detail ? "icon-button" : undefined}
                                aria-label={text("choose")}
                              >
                                <Icon icon={Link} />
                                {!detail && text("choose")}
                              </Button>
                            }
                          >
                            <div className="insight-picker">
                              <SingleChoice
                                label={text("choose")}
                                value={selected || detail?.id || ""}
                                onChange={choose}
                                options={[
                                  { value: "", label: text("choose") },
                                  ...data.tasks.map((task) => ({
                                    value: task.id,
                                    label: task.label,
                                  })),
                                ]}
                              />
                              <p>{text("bindingHelp")}</p>
                            </div>
                          </Popover>
                        </div>
                        {detail && (
                          <div className="insight-task-meta">
                            <span>{count("Turns", detail.turnCount)}</span>
                            <span>
                              {count("Responses", detail.responseCount)}
                            </span>
                            <span>{date(detail.lastEventAt)}</span>
                            <Help name="turn" />
                          </div>
                        )}
                      </div>
                      {detail ? (
                        <>
                          <section className="insight-card">
                            <TokenBreakdown value={detail.usage} />
                          </section>
                          <section className="insight-card">
                            <Heading name="turn">
                              <span className="insight-count">
                                {number(detail.turnCount)}
                              </span>
                            </Heading>
                            <motion.div
                              key={displayTurnPage}
                              initial={{ opacity: 0 }}
                              animate={{ opacity: 1 }}
                            >
                              <HoverScope className="insight-turns">
                                {detail.turns.slice(0, 4).map((turn, index) => (
                                  <Button
                                    variant="ghost"
                                    data-hover-target={turn.id}
                                    disabled={
                                      busy && turnPage !== displayTurnPage
                                    }
                                    className="insight-turn-row"
                                    key={turn.id}
                                    onClick={() =>
                                      setView({ kind: "turn", turn })
                                    }
                                  >
                                    <span className="insight-turn-index">
                                      {String(
                                        detail.turnCount -
                                          displayTurnPage * 4 -
                                          index,
                                      ).padStart(2, "0")}
                                    </span>
                                    <span className="insight-row-main">
                                      <strong>
                                        {turn.model ?? short(turn.id)}
                                      </strong>
                                      <small>
                                        {date(turn.startedAt)} ·{" "}
                                        {count("Tools", turn.toolCount)}
                                      </small>
                                    </span>
                                    <span className="insight-turn-value">
                                      {number(turn.usage.total, true)}
                                    </span>
                                    <Tooltip text={status(turn.status)}>
                                      <i
                                        role="img"
                                        aria-label={status(turn.status)}
                                        className={`insight-status ${turn.status}`}
                                      />
                                    </Tooltip>
                                    <Icon icon={ChevronRight} />
                                  </Button>
                                ))}
                              </HoverScope>
                            </motion.div>
                            {!detail.turns.length && <Empty />}
                            <Pager
                              page={displayTurnPage}
                              count={detail.turnCount}
                              size={4}
                              change={setTurnPage}
                              busy={busy}
                            />
                          </section>
                          <section className="insight-card insight-links">
                            <HoverScope>
                              {(
                                [
                                  {
                                    kind: "responses",
                                    icon: GitBranch,
                                    count: detail.responseCount,
                                  },
                                  {
                                    kind: "tools",
                                    icon: Wrench,
                                    count: detail.toolCount,
                                  },
                                  ...(detail.parentId ||
                                  detail.forkedFromId ||
                                  detail.children.length ||
                                  detail.compactions.length
                                    ? [
                                        {
                                          kind: "relations",
                                          icon: Layers,
                                          count:
                                            detail.children.length +
                                            detail.compactions.length +
                                            (detail.parentId ? 1 : 0) +
                                            (detail.forkedFromId ? 1 : 0),
                                        },
                                      ]
                                    : []),
                                ] as const
                              ).map((row) => (
                                <Button
                                  variant="ghost"
                                  data-hover-target={row.kind}
                                  className="insight-link-row"
                                  key={row.kind}
                                  onClick={() =>
                                    setView({
                                      kind: row.kind as DetailView["kind"],
                                    })
                                  }
                                >
                                  <Icon icon={row.icon} size={20} />
                                  <span>{text(row.kind)}</span>
                                  <span className="insight-link-value">
                                    {number(row.count)}
                                  </span>
                                  <Icon icon={ChevronRight} />
                                </Button>
                              ))}
                            </HoverScope>
                          </section>
                        </>
                      ) : (
                        <Empty>
                          {busy ? text("loading") : text("unbound")}
                          {!busy && <Button variant="ghost" onClick={() => setGuide("start")}>{text("guide")}</Button>}
                        </Empty>
                      )}
                    </>
                  )}
                </motion.div>
              </AnimatePresence>
              <footer className="insight-footer">
                <div className="insight-title">
                  <IconButton
                    icon={RefreshCw}
                    className={busy ? "insight-refreshing" : ""}
                    label={text("refresh")}
                    disabled={busy}
                    onClick={() => void update()}
                  />
                  <Tooltip
                    text={
                      <>
                        {text("freshHelp")}
                        <br />
                        {date(data.observedAt)}
                      </>
                    }
                  >
                    <span tabIndex={0}>{date(data.observedAt)}</span>
                  </Tooltip>
                  <span
                    className={`insight-status ${error ? "interrupted" : "completed"}`}
                    role="img"
                    aria-label={text(error ? "stale" : "connected")}
                  />
                </div>
                <Tooltip
                  text={text(sourceIssues.length ? "coverage" : "source")}
                >
                  <Button
                    variant="ghost"
                    onClick={() => setView({ kind: "source" })}
                    className="insight-source"
                    aria-label={text("source")}
                  >
                    <Icon icon={sourceIssues.length ? AlertCircle : Info} />
                    {sourceIssues.length || null}
                  </Button>
                </Tooltip>
              </footer>
            </>
          )}
          {guide && <Guide section={guide} close={() => setGuide(null)} />}
          {view && ready && (
            <Details
              key={`${view.kind}-${view.turn?.id ?? ""}`}
              view={view}
              data={ready}
              close={() => setView(null)}
              choose={choose}
            />
          )}
        </main>
      </TooltipProvider>
    </MotionConfig>
  );
}

function Overview({
  data,
  choose,
  open,
  guide,
}: {
  data: ReadySnapshot;
  choose: (id: string) => void;
  open: (view: DetailView) => void;
  guide: (section: GuideSection) => void;
}) {
  const [query, setQuery] = useState(""),
    [filter, setFilter] = useState("all"),
    [page, setPage] = useState(0),
    [searching, setSearching] = useState(false);
  const tasks = data.tasks.filter(
    (task) =>
      (filter === "all" || task.period.records > 0) &&
      `${task.label} ${task.id}`
        .toLocaleLowerCase()
        .includes(query.toLocaleLowerCase()),
  );
  const safePage = Math.min(page, Math.max(0, Math.ceil(tasks.length / 5) - 1));
  const models = [...data.models].sort(
    (a, b) => (b.usage.total ?? 0) - (a.usage.total ?? 0),
  );
  const modelTotal = models.reduce(
      (sum, model) => sum + (model.usage.total ?? 0),
      0,
    ),
    modelComplete = models.every((model) => model.usage.total != null);
  const byDay = new Map(data.daily.map((day) => [day.day, day]));
  const daily: ReadySnapshot["daily"] = [];
  const startDay = Math.floor(data.range.start / 86400000) * 86400000;
  for (
    let at = startDay;
    at <= data.range.end && daily.length < 32;
    at += 86400000
  ) {
    const day = new Date(at).toISOString().slice(0, 10);
    daily.push(
      byDay.get(day) ?? {
        day,
        usage: {
          input: null,
          cached: null,
          output: null,
          reasoning: null,
          total: null,
          records: 0,
        },
      },
    );
  }
  const max = Math.max(
    1,
    ...daily.map((day) => day.usage.total ?? day.usage.knownTotal ?? 0),
  );
  const quota = data.quota;
  const stale =
    !!quota &&
    (!quota.selected ||
      quota.state !== "ready" ||
      !quota.observedAt ||
      Date.now() - Date.parse(quota.observedAt) > 300000);
  return (
    <>
      <div className="insight-quota-strip">
        <div className="insight-title">
          <Icon icon={Clock} />
          <strong>{text("quota")}</strong>
          <Help name="quota" />
        </div>
        {quota?.windows.length ? (
          <div className="insight-quota-summary">
            {quota.windows.slice(0, 2).map((w) => (
              <Tooltip
                key={w.id}
                text={`${w.poolId} · ${w.label} · ${text("reset")} ${date(w.resetsAt)}`}
              >
                <div
                  className={`insight-quota-window ${stale ? "is-stale" : ""}`}
                  tabIndex={0}
                >
                  <span>{windowLabel(w.label)}</span>
                  <Progress
                    value={Math.max(0, 100 - w.usedPercent)}
                    label={text("remaining", {
                      n: number(Math.max(0, 100 - w.usedPercent)),
                    })}
                  />
                  <span>
                    {text("remaining", {
                      n: number(Math.max(0, 100 - w.usedPercent)),
                    })}
                  </span>
                </div>
              </Tooltip>
            ))}
          </div>
        ) : (
          <Button variant="ghost" onClick={() => guide("start")}>{text("noQuota")}<Icon icon={CircleHelp} /></Button>
        )}
        <Popover
          label={text("quota")}
          trigger={
            <Button
              variant="ghost"
              className="icon-button"
              aria-label={text("quota")}
            >
              <Icon icon={stale ? AlertCircle : ChevronRight} />
            </Button>
          }
        >
          <div className="insight-quota-details">
            <p>{text("quotaHelp")}</p>
            {quota?.windows.map((w) => (
              <div key={w.id}>
                <div className="insight-between">
                  <strong>
                    {w.poolId} · {windowLabel(w.label)}
                  </strong>
                  <span>
                    {text("remaining", {
                      n: number(Math.max(0, 100 - w.usedPercent)),
                    })}
                  </span>
                </div>
                <Progress
                  value={Math.max(0, 100 - w.usedPercent)}
                  label={w.label}
                />
                <small>
                  {text("reset")} {date(w.resetsAt)}
                </small>
              </div>
            ))}
            <small>
              {text("observed")} {date(quota?.observedAt)}{" "}
              {stale ? text("stale") : ""}
            </small>
          </div>
        </Popover>
      </div>
      <div className="insight-overview-grid">
        <section className="insight-card insight-chart-card">
          <Heading name="usage">
            <IconButton
              icon={ChevronRight}
              label={text("usage")}
              onClick={() => open({ kind: "usage" })}
            />
          </Heading>
          <div className="insight-hero">
            <Value value={data.usage.total ?? data.usage.knownTotal} />
            <small>
              {data.usage.total == null && data.usage.knownTotal != null
                ? text("known")
                : "tokens"}
            </small>
          </div>
          <div className="insight-chart">
            <div className="insight-axis">
              <span>{number(max, true)}</span>
              <span>{number(max / 2, true)}</span>
              <span>0</span>
            </div>
            <div className="insight-bars">
              {daily.map((day) => {
                const total = day.usage.total ?? day.usage.knownTotal;
                const complete =
                  day.usage.input != null && day.usage.output != null;
                return (
                  <Tooltip
                    key={day.day}
                    text={
                      <>
                        {day.day} UTC
                        <br />
                        {text("input")} {number(day.usage.input)}
                        <br />
                        {text("output")} {number(day.usage.output)}
                        <br />
                        {text("total")} {number(total)}
                        {day.usage.total == null
                          ? ` · ${text("uncertain")}`
                          : ""}
                      </>
                    }
                  >
                    <div
                      className="insight-day"
                      tabIndex={0}
                      aria-label={`${day.day} UTC, ${number(total)} tokens`}
                    >
                      <div className="insight-bar-space">
                        <motion.div
                          className={`insight-bar ${complete ? "" : "is-partial"}`}
                          initial={false}
                          animate={{
                            height:
                              total == null
                                ? "3px"
                                : `${Math.max(1, (total / max) * 100)}%`,
                          }}
                        >
                          {complete && (
                            <>
                              <div
                                className="insight-output"
                                style={{ flex: day.usage.output ?? 0 }}
                              />
                              <div
                                className="insight-input"
                                style={{ flex: day.usage.input ?? 0 }}
                              />
                            </>
                          )}
                        </motion.div>
                      </div>
                      <span>{day.day.slice(5).replace("-", "/")}</span>
                    </div>
                  </Tooltip>
                );
              })}
              {!daily.length && <Empty />}
            </div>
          </div>
          <div className="insight-chart-legend">
            <span>
              <i className="insight-swatch insight-input" />
              {text("input")}
            </span>
            <span>
              <i className="insight-swatch insight-output" />
              {text("output")}
            </span>
          </div>
        </section>
        <section className="insight-card insight-model-card">
          <Heading name="models">
            {models.length > 3 && (
              <IconButton
                icon={ChevronRight}
                label={text("more")}
                onClick={() => open({ kind: "models" })}
              />
            )}
          </Heading>
          <div className="insight-models">
            {models.slice(0, 3).map((model, index) => (
              <div className="insight-model" key={model.name}>
                <div className="insight-between">
                  <span className="insight-title">
                    <i className={`insight-swatch insight-series-${index}`} />
                    <span className="insight-model-name">{model.name}</span>
                  </span>
                  <Value value={model.usage.total} />
                </div>
                <div className="insight-model-meter">
                  <Progress
                    color={`var(--usage-${["input", "output", "other"][index]})`}
                    value={
                      modelComplete && modelTotal
                        ? ((model.usage.total ?? 0) / modelTotal) * 100
                        : 0
                    }
                    label={model.name}
                  />
                  <span>
                    {modelComplete && modelTotal
                      ? percent(((model.usage.total ?? 0) / modelTotal) * 100)
                      : "—"}
                  </span>
                </div>
              </div>
            ))}
            {!models.length && <Empty />}
          </div>
        </section>
      </div>
      <section className="insight-card insight-task-list">
        <div className="insight-section-heading">
          <div className="insight-title">
            <h2>{text("tasks")}</h2>
            <span className="insight-count">{number(tasks.length)}</span>
            <Help name="task">
              {text("taskHelp")}
              <br />
              {text("loaded", { n: data.tasks.length, total: data.taskCount })}
            </Help>
          </div>
          <div className="insight-task-actions">
            <IconButton
              icon={Search}
              label={text("search")}
              aria-expanded={searching}
              onClick={() => {
                setSearching(!searching);
                if (searching) {
                  setQuery("");
                  setPage(0);
                }
              }}
            />
            <SingleChoice
              label={text("tasks")}
              value={filter}
              onChange={(v) => {
                setFilter(v);
                setPage(0);
              }}
              options={[
                { value: "all", label: text("all") },
                { value: "recent", label: text("recent") },
              ]}
            />
          </div>
        </div>
        <AnimatePresence initial={false}>
          {searching && (
            <motion.div
              className="insight-collapse"
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
            >
              <Input
                autoFocus
                aria-label={text("search")}
                placeholder={text("search")}
                value={query}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setPage(0);
                }}
              />
            </motion.div>
          )}
        </AnimatePresence>
        <div className="insight-table-heading">
          <span>{text("task")}</span>
          <span>{text("total")}</span>
          <span>{text("updated")}</span>
          <span />
        </div>
        <motion.div
          key={`${safePage}-${filter}-${query}`}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
        >
          <HoverScope className="insight-table">
            {tasks.slice(safePage * 5, safePage * 5 + 5).map((task) => (
              <Button
                variant="ghost"
                data-hover-target={task.id}
                className="insight-task-row"
                key={task.id}
                onClick={() => choose(task.id)}
              >
                <span className="insight-task-name">
                  {task.label}
                  {task.issues.length > 0 && <Icon icon={AlertCircle} />}
                </span>
                <span>{number(task.period.total, true)}</span>
                <small>{date(task.lastEventAt)}</small>
                <Icon icon={ChevronRight} />
              </Button>
            ))}
          </HoverScope>
        </motion.div>
        {!tasks.length && (
          <Empty>
            {text(query || filter !== "all" ? "noMatch" : "empty")}
            {query || filter !== "all" ? (
              <Button variant="ghost" onClick={() => { setQuery(""); setFilter("all"); setPage(0); }}>{text("clearFilters")}</Button>
            ) : <Button variant="ghost" onClick={() => guide("start")}>{text("guide.start")}</Button>}
          </Empty>
        )}
        <Pager page={safePage} count={tasks.length} size={5} change={setPage} />
      </section>
    </>
  );
}
