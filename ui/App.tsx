import {
  lazy,
  Suspense,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  LayoutDashboard,
  Logs,
  Settings,
  X,
  Download,
  Maximize2,
  Minimize2,
} from "lucide-react";
import { animate } from "motion";
import { surfaceMotion, reducedMotion } from "./motion/surface";
import { t, locale } from "./i18n";
import { isDesktop } from "./platform";
import { connector, notify, startConnector } from "./state/connector";
import { startAgents } from "./state/agents";
import { startAgentActivity, startSubscriptions } from "./state/subscriptions";
import { startTasks, tasks } from "./state/tasks";
import {
  startUpdateChecks,
  updates,
  checkUpdate,
  showUpdate,
  updateVisible,
  installDirect,
} from "./state/updates";
import { navigation, type Page } from "./state/navigation";
import { panelPreferences } from "./usage-rail/preferences";
import type { PanelPreferences } from "./usage-rail/layout";
import { panelLayers, panelHandoff, panelAnchorAt } from "./state/panels";
import {
  capturePanelOrigin, takePanelOrigin, animateOriginContent, surfaceOf,
  type PanelOrigin,
} from "./components/panelMorph";
import { displayMessage } from "./messages";
import ConnectionOverview from "./components/ConnectionOverview";
import TasksPage from "./components/TasksPage";
import RecordsPage from "./components/RecordsPage";
import { AppUpdateDialogs } from "./components/AppUpdate";
import AgentDisplaySettings from "./components/AgentDisplaySettings";
import TrayPanel from "./tray-panel/TrayPanel";
import { Dialog, Icon, IconButton, Loading, Notice, HoverScope } from "./components/ui";
const SettingsPage = lazy(() => import("./components/SettingsPage"));
const BrowserRailPreview = import.meta.env.DEV
  ? lazy(() => import("./usage-rail/BrowserRailPreview"))
  : null;
export default function App() {
  locale.use();
  const { feedback } = connector.use(),
    taskState = tasks.use(),
    nav = navigation.use();
  updates.use();
  const [page, setPage] = useState<Page>(() => {
    const saved = localStorage.getItem("clc-page");
    return ["overview", "settings", "logs", "tasks"].includes(saved ?? "") ? saved as Page : "overview";
  }),
    [usage, setUsage] = useState(false);
  const [sheetWide, setSheetWide] = useState(false);
  const sheet = useRef<HTMLElement>(null),
    lastPage = useRef<Page>("tasks"),
    sheetOrigin = useRef<PanelOrigin | undefined>(undefined),
    sheetVisualCleanup = useRef<(() => void) | undefined>(undefined),
    sheetMotion = useRef<ReturnType<typeof surfaceMotion> | null>(null),
    sheetClosing = useRef(false),
    backdropDown = useRef(false);
  const layers = panelLayers.use();
  useEffect(() => { localStorage.setItem("clc-page", page); }, [page]);
  useEffect(() => {
    if (!isDesktop) return;
    // Keep drafts and active dialogs alive; only retire an idle hidden surface.
    const timer = setInterval(() => {
      if (document.querySelector('[role="dialog"], form') || updates.get().phase !== "idle" || updates.get().checking || updates.get().dialogOpen || updateVisible()) return;
      void import("@tauri-apps/api/core").then(({ invoke }) => invoke("release_main_window")).catch(() => {});
    }, 30000);
    return () => clearInterval(timer);
  }, []);
  useLayoutEffect(() => {
    if (page === "overview" || !sheet.current) return;
    const node = sheet.current,
      rect = node.getBoundingClientRect(),
      source =
        ((panelHandoff.until ?? 0) > performance.now()
          ? panelHandoff.from
          : undefined) ??
        sheetOrigin.current?.rect ??
        document.querySelector(".navigation-surface")?.getBoundingClientRect();
    const handedOff = (panelHandoff.until ?? 0) > performance.now() && !!panelHandoff.from;
    panelHandoff.from = undefined;
    const surface = surfaceMotion(node);
    sheetMotion.current = surface;
    if (source && !reducedMotion()) {
      const visual = handedOff ? undefined : sheetOrigin.current;
      const animation = surface.to(
        { transform: "translate(0px,0px)", width: `${rect.width}px`, height: `${rect.height}px`, opacity: 1, ...surfaceOf(node) },
        { from: {
          transform: `translate(${source.x - rect.x}px,${source.y - rect.y}px)`,
          width: `${source.width}px`, height: `${source.height}px`, opacity: 1,
          ...(visual?.surface ?? {}),
        } },
      );
      const contentAnimations = Array.from(node.children).map((child) =>
        animate(child, { opacity: [0, 1] }, { duration: 0.15 }));
      const cleanup = sheetVisuals(false, !handedOff);
      sheetVisualCleanup.current = () => { cleanup(); contentAnimations.forEach((a) => a.stop()); };
      void animation.then(cleanup);
    }
    document.querySelector<HTMLElement>(".capsule-title")?.focus({ preventScroll: true });
    const observer = new ResizeObserver(() => {
      if (sheetClosing.current) return;
      const surface = sheetMotion.current;
      if (!surface) return;
      const layout = node.firstElementChild!.getBoundingClientRect();
      const next = { width: layout.width + 2, height: layout.height + 2 };
      if (Math.abs(next.width - rect.width) <= 1 && Math.abs(next.height - rect.height) <= 1) return;
      surface.to({ width: `${next.width}px`, height: `${next.height}px`, transform: "translate(0px,0px)" }, {
        from: surface.running ? undefined : { width: `${rect.width}px`, height: `${rect.height}px` },
      });
      rect.width = next.width;
      rect.height = next.height;
    });
    if (node.firstElementChild) observer.observe(node.firstElementChild);
    return () => {
      observer.disconnect();
      sheetMotion.current?.dispose();
      sheetMotion.current = null;
      sheetVisualCleanup.current?.();
    };
  }, [page]);

  function sheetVisuals(closing: boolean, showOrigin = true) {
    const visual = showOrigin ? sheetOrigin.current : undefined;
    const nodes = Array.from(document.querySelectorAll<HTMLElement>(".navigation-capsule, .navigation-surface"));
    const opacities = nodes.map((el) => el.style.opacity);
    const transitions = nodes.map((el) => el.style.transition);
    nodes.forEach((el) => { el.style.opacity = "0"; el.style.transition = "none"; });
    const clear = visual ? animateOriginContent(visual, 51, closing) : () => {};
    let done = false;
    return () => {
      if (done) return;
      done = true;
      clear();
      nodes.forEach((el, i) => {
        // Settle the capsule's new layout before enabling its CSS transitions.
        void el.offsetHeight;
        el.style.opacity = opacities[i];
        el.style.transition = transitions[i];
      });
    };
  }

  async function closeSheet(replacement?: HTMLElement) {
    const node = sheet.current;
    if (!node || sheetClosing.current) return;
    sheetClosing.current = true;
    if (
      node &&
      !matchMedia("(prefers-reduced-motion: reduce)").matches &&
      !replacement
    ) {
      const dock = document.querySelector(".navigation-surface")!.getBoundingClientRect();
      const visual = sheetOrigin.current;
      if (visual) {
        visual.rect = new DOMRect(dock.x, dock.bottom - visual.rect.height, visual.rect.width, visual.rect.height);
      }
      const to = visual?.rect ?? dock;
      sheetVisualCleanup.current?.();
      const surface = sheetMotion.current ?? surfaceMotion(node);
      sheetMotion.current = surface;
      const placement = getComputedStyle(node);
      const natural = { x: parseFloat(placement.left), y: parseFloat(placement.top) };
      const animation = surface.to({
        transform: `translate(${to.x - natural.x}px,${to.y - natural.y}px)`,
        width: `${to.width}px`, height: `${to.height}px`,
        ...(visual?.surface ?? {}),
      }, { retain: true });
      const cleanup = sheetVisuals(true);
      const contentAnimations = Array.from(node.children).map((child) =>
        animate(child, { opacity: 0 }, { duration: 0.18 }));
      sheetVisualCleanup.current = () => { cleanup(); contentAnimations.forEach((a) => a.stop()); };
      await animation;
    }

    if (replacement && node) {
      panelHandoff.from = node.getBoundingClientRect();
      panelHandoff.until = performance.now() + 100;
      panelHandoff.trigger = replacement;
    }
    sheetClosing.current = false;
    setPage("overview");
    requestAnimationFrame(() => {
      const target =
        replacement ??
        document.querySelector<HTMLElement>(
          `[data-page="${lastPage.current}"]`,
        );
      target?.focus({ preventScroll: true });
      if (replacement) replacement.click();
    });
  }

  function navigate(next: Page) {
    if (next !== "overview") {
      sheetOrigin.current = takePanelOrigin();
      lastPage.current = next;
      setSheetWide(false);
    }
    setPage(next);
  }
  useEffect(() => {
    const stops = [
      startConnector(),
      startSubscriptions(),
      startAgentActivity(),
      startAgents(),
      startTasks(),
      startUpdateChecks(),
    ];
    return () => stops.forEach((stop) => stop());
  }, []);
  useEffect(() => {
    if (!isDesktop) return;
    let stopped = false;
    const stops: (() => void)[] = [];
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const listeners = await Promise.all([
        listen("updates:check", () => {
          showUpdate();
          void checkUpdate();
        }),
        listen<string>("navigate", (e) => {
          if (["overview", "settings", "logs", "tasks"].includes(e.payload))
            navigate(e.payload as Page);
        }),
        listen<PanelPreferences>("usage-panel:preferences", (e) =>
          panelPreferences.set(e.payload),
        ),
        listen("agents:settings", () =>
          navigation.set((old) => ({ ...old, agentSettings: true })),
        ),
        listen<string>("subscriptions:open", (e) => {
          navigation.set((old) => ({ ...old, providerId: e.payload }));
          navigate("overview");
        }),
        listen<string>("connection-error", (e) => notify(e.payload, true)),
      ]);
      if (stopped) listeners.forEach((stop) => stop());
      else {
        stops.push(...listeners);
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("main_window_ready");
      }
    });
    return () => {
      stopped = true;
      stops.forEach((stop) => stop());
    };
  }, []);
  const pending = taskState.records.filter(
    (r) => r.state === "awaiting-approval",
  ).length;
  const pages = [
    { id: "tasks" as const, label: t("tasks"), icon: LayoutDashboard },
    { id: "logs" as const, label: t("records"), icon: Logs },
    { id: "settings" as const, label: t("settings"), icon: Settings },
  ];
  return (
    <div
      className={`app-scene ${sheetWide ? "sheet-wide" : ""} ${isDesktop ? "desktop" : ""} ${isDesktop && navigator.platform.toLowerCase().includes("mac") ? "mac" : ""}`}
      onClickCapture={(e) => capturePanelOrigin(e.target)}
      onKeyDown={(e) => {
        if (
          e.key === "Escape" &&
          !e.defaultPrevented &&
          page !== "overview" &&
          !layers.length
        ) {
          e.preventDefault();
          void closeSheet();
        }
        if (e.key === "Tab" && page !== "overview" && !layers.length) {
          const nodes = Array.from(
            document.querySelectorAll<HTMLElement>(
              '.page-sheet button:not(:disabled),.page-sheet input:not(:disabled),.page-sheet [tabindex="0"],.navigation-capsule .capsule-close',
            ),
          ).filter((n) => n.getClientRects().length && !n.closest("[inert]"));
          const index = nodes.indexOf(document.activeElement as HTMLElement);
          if (
            (e.shiftKey && index <= 0) ||
            (!e.shiftKey && index === nodes.length - 1)
          ) {
            e.preventDefault();
            (e.shiftKey ? nodes.at(-1) : nodes[0])?.focus();
          }
        }
      }}
    >
      {isDesktop && (
        <div className="window-drag-strip" data-tauri-drag-region />
      )}
      <main
        className="home-workspace"
        inert={page !== "overview" || layers.length > 0}
      >
        <ConnectionOverview onUsage={() => setUsage(true)} />
      </main>
      {page !== "overview" && (
        <div
          className="sheet-backdrop"
          onPointerDown={(e) =>
            (backdropDown.current =
              e.button === 0 && e.target === e.currentTarget)
          }
          onPointerCancel={() => (backdropDown.current = false)}
          onClick={(e) => {
            if (
              backdropDown.current &&
              e.target === e.currentTarget &&
              !layers.length
            )
              void closeSheet(panelAnchorAt(e.clientX, e.clientY));
            backdropDown.current = false;
          }}
        />
      )}
      <div
        className={`navigation-surface ${page !== "overview" ? "expanded" : ""} ${updateVisible() ? "has-update" : ""}`}
        aria-hidden="true"
      />
      {page !== "overview" && (
        <section
          ref={sheet}
          className="page-sheet"
          data-panel-depth="1"
          data-recessed={layers.length > 0 || undefined}
          inert={layers.length > 0}
          aria-label={pages.find((item) => item.id === page)?.label}
        >
          <div className="page-sheet-content">
            <main
              className={`workspace ${page === "logs" ? "records-workspace" : ""}`}
            >
              {page === "tasks" ? (
                <TasksPage />
              ) : page === "logs" ? (
                <RecordsPage />
              ) : (
                <Suspense fallback={<Loading />}>
                  <SettingsPage />
                </Suspense>
              )}
            </main>
            <footer className="sheet-footer">
              <IconButton
                className="capsule-close"
                icon={X}
                label={t("backToHome")}
                onClick={() => void closeSheet()}
              />
              <h1 className="capsule-title" tabIndex={-1}>
                {pages.find((item) => item.id === page)?.label}
              </h1>
              <IconButton
                className="panel-width-toggle"
                icon={sheetWide ? Minimize2 : Maximize2}
                label={t(sheetWide ? "restorePanelWidth" : "expandPanelWidth")}
                aria-pressed={sheetWide}
                onClick={() => setSheetWide(!sheetWide)}
              />
            </footer>
          </div>
        </section>
      )}
      <nav
        className={`navigation-capsule ${page !== "overview" ? "expanded" : ""} ${updateVisible() ? "has-update" : ""}`}
        inert={page !== "overview" || layers.length > 0}
        aria-hidden={page !== "overview"}
        aria-label={t("mainNavigation")}
      >
        <HoverScope
          activeKey={page !== "overview" || layers.length > 0 ? "" : undefined}
          className="capsule-icons"
          inert={page !== "overview"}
          aria-hidden={page !== "overview"}
        >
          {updateVisible() && (
            <IconButton
              className="update-shortcut"
              icon={Download}
              label={t("viewUpdate")}
              onClick={() => void installDirect()}
            />
          )}{" "}
          {pages.map((item) => (
            <button
              key={item.id}
              data-panel-anchor
              data-page={item.id}
              data-hover-target={item.id}
              aria-label={item.label}
              onClick={() => navigate(item.id)}
            >
              <Icon icon={item.icon} size={20} />
              {item.id === "tasks" && pending > 0 && (
                <span className="task-count">{pending}</span>
              )}
            </button>
          ))}
        </HoverScope>
      </nav>
      {feedback.text && (
        <div className="app-feedback">
          <Notice
            tone={feedback.error ? "danger" : "success"}
            action={
              <IconButton
                icon={X}
                label={t("dismissMessage")}
                onClick={() => notify("")}
              />
            }
          >
            {displayMessage(feedback.text)}
          </Notice>
        </div>
      )}
      <AppUpdateDialogs />
      {nav.agentSettings && (
        <AgentDisplaySettings
          onClose={() =>
            navigation.set((old) => ({ ...old, agentSettings: false }))
          }
        />
      )}
      {usage && (
        <Dialog
          onClose={() => setUsage(false)}
          title={t("trayUsage")}
          headerless
          width={386}
        >
          <TrayPanel
            embedded
            onNavigate={(next) => {
              setUsage(false);
              navigate(next);
            }}
          />
        </Dialog>
      )}
      {BrowserRailPreview && !isDesktop && page === "overview" && (
        <Suspense fallback={null}>
          <div inert={layers.length > 0}>
            <BrowserRailPreview />
          </div>
        </Suspense>
      )}
    </div>
  );
}
