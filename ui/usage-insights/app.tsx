import { useEffect, useRef, useState } from "react";
import { AnimatePresence, MotionConfig, motion } from "motion/react";
import {
  ArrowLeft,
  RefreshCw,
} from "lucide-react";
import {
  Button,
  IconButton,
  SingleChoice,
  ErrorCallout,
  LoadingIndicator,
  Tooltip,
  TooltipProvider,
} from "../components/ui";
import { locale } from "../i18n";
import {
  initialize,
  navigate,
  onRoute,
  onResult,
  refresh,
  type Snapshot,
  type TaskSelection,
} from "./bridge";
import { Empty, UsageStats } from "./components";
import { Details, type DetailView } from "./details";
import { TaskTimeline } from "./timeline";
import { Overview } from "./overview";
import { Guide, type GuideSection } from "./guide";
import { PluginUpdates } from "./plugin-update";
import {
  text,
  date,
  short,
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
    [selected, setSelected] = useState("");
  const [taskSelection, setTaskSelection] = useState<TaskSelection>({ page: 1, search: "" });
  const [guide, setGuide] = useState<GuideSection | null>(null);
  const [view, setView] = useState<DetailView | null>(null);
  const [routeVersion, setRouteVersion] = useState(0);
  const browserRoute = useRef(false);
  const entryScope = useRef<"global" | "thread">("thread"),
    receivedInitial = useRef(false),
    initialized = useRef(false);
  const current = useRef(data);
  current.current = data;
  const alive = useRef(true),
    request = useRef(0),
    inFlight = useRef(false);
  const filter = useRef({ days, selected, taskSelection });
  filter.current = { days, selected, taskSelection };
  const accept = (next: Snapshot) => {
    if (alive.current) {
      setData(next);
      setError("");
    }
  };
  const update = async (mode: "auto" | "filter" | "manual" = "manual", refreshQuota = false) => {
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
        ...(refreshQuota ? { refreshQuota: true } : {}),
        ...(f.selected ? { threadId: f.selected } : {}),
        taskPage: f.taskSelection.page,
        taskSearch: f.taskSelection.search,
      });
      if (serial === request.current) {
        // A restarted Core may still be restoring its first snapshot.
        // Keep the last visible result until the same selection has a new snapshot.
        if (!(
          mode === "auto" &&
          next.state === "collecting" &&
          current.current?.state === "ready"
        )) {
          accept(next);
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
    const offRoute = onRoute(route => {
      browserRoute.current = true;
      entryScope.current = route.threadId ? "global" : route.scope;
      request.current++;
      inFlight.current = false;
      filter.current = { ...filter.current, selected: route.threadId };
      setSelected(route.threadId);
      setView(null);
      setData(previous => previous?.state === "ready" ? { ...previous, thread: null, binding: "unknown", scope: route.scope } : previous);
      setRouteVersion(version => version + 1);
      window.scrollTo(0, 0);
    });
    const off = onResult((next) => {
      const selection = filter.current.selected;
      if (browserRoute.current && (next.scope !== (selection ? "thread" : entryScope.current) || (next.state === "ready" && selection && next.thread && next.thread.id !== selection))) return;
      if (!receivedInitial.current) {
        if (!browserRoute.current) entryScope.current = next.scope;
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
    return () => {
      alive.current = false;
      off();
      offRoute();
    };
  }, []);
  useEffect(() => {
    const timer = setInterval(() => {
      if (document.visibilityState === "visible" && current.current)
        void updateRef.current("auto");
    }, data?.state === "collecting" || (data?.state === "ready" && (data.refreshing || data.quota?.refreshing)) ? 500 : 10000);
    return () => clearInterval(timer);
  }, [data?.state, data?.state === "ready" && data.refreshing, data?.state === "ready" && data.quota?.refreshing]);
  useEffect(() => {
    if (current.current) void updateRef.current("filter");
  }, [days, selected, taskSelection, routeVersion]);
  const choose = (id: string) => {
    navigate(id);
    if (id) window.scrollTo(0, 0);
    request.current++;
    inFlight.current = false;
    setView(null);
    setSelected(id);
    setData((previous) =>
      previous?.state === "ready"
        ? { ...previous, thread: null, binding: "unknown" }
        : previous,
    );
  };
  const global = data?.scope === "global" && !selected;
  const ready = data?.state === "ready" ? data : undefined;
  const failure = error || (data && data.state !== "ready" && data.state !== "collecting"
    ? data.message || text(data.state) : "");
  const detail = ready?.thread;
  const label =
    detail?.label ??
    ready?.tasks.find((task) => task.id === (detail?.id ?? selected))?.label ??
    (detail ? short(detail.id) : text("unbound"));
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
              <h1 title={global || !data ? undefined : label}>{global ? text("overview") : !data ? text("usage") : label}</h1>
            </div>
            <div className="insight-header-actions">
              {ready && (
                <div className="insight-title insight-refresh-status">
                  <IconButton
                    icon={RefreshCw}
                    size={16}
                    busy={busy}
                    label={text("refresh")}
                    disabled={busy}
                    onClick={() => void update()}
                  />
                  <Tooltip
                    text={
                      <>
                        {text(ready.refreshing ? "restoringHelp" : "freshHelp")}
                        <br />
                        {date(ready.observedAt)}
                      </>
                    }
                  >
                    <span tabIndex={0}>{ready.refreshing ? text("restoring") : date(ready.observedAt)}</span>
                  </Tooltip>
                  <span
                    className={`insight-status ${error || ready.refreshing ? "interrupted" : "completed"}`}
                    role="img"
                    aria-label={text(error || ready.refreshing ? "stale" : "connected")}
                  />
                </div>
              )}
              {global && (
                <div className="insight-range">
                  <SingleChoice
                    compact
                    label={text("range")}
                    value={days}
                    onChange={(value) => { setDays(value); setTaskSelection(previous => ({...previous, page: 1})); }}
                    options={["day", "week", "month"].map((key, i) => ({
                      value: ["1", "7", "30"][i]!,
                      label: text(key),
                    }))}
                  />
                </div>
              )}
              <PluginUpdates />
            </div>
          </header>
          {failure && (
            <ErrorCallout action={<Button variant="ghost" busy={busy} onClick={() => void update()}>{text("retry")}</Button>}>
              {failure}
            </ErrorCallout>
          )}
          {!data || data.state !== "ready" ? (
            !failure && <div className="insight-state">
              <LoadingIndicator label={text(data?.state === "collecting" ? "collecting" : "loading")} />
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
                    <Overview data={data} selection={taskSelection} selectTasks={setTaskSelection} choose={choose} guide={setGuide} refreshQuota={() => update("manual", true)} />
                  ) : (
                    <>
                      {detail ? (
                        <>
                          <UsageStats detail={detail} open={kind => setView({ kind })} />
                          <TaskTimeline key={detail.id} detail={detail} />

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
            </>
          )}
          {guide && <Guide section={guide} close={() => setGuide(null)} />}
          {view && ready && (
            <Details
              key={view.kind}
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
