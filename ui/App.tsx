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
import { displayMessage } from "./messages";
import ConnectionOverview from "./components/ConnectionOverview";
import TasksPage from "./components/TasksPage";
import RecordsPage from "./components/RecordsPage";
import { AppUpdateDialogs } from "./components/AppUpdate";
import AgentDisplaySettings from "./components/AgentDisplaySettings";
import TrayPanel from "./tray-panel/TrayPanel";
import { Dialog, Icon, IconButton, Loading, Notice } from "./components/ui";
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
  const [page, setPage] = useState<Page>("overview"),
    [usage, setUsage] = useState(false);
  const [sheetWide, setSheetWide] = useState(false);
  const sheet = useRef<HTMLElement>(null),
    lastPage = useRef<Page>("tasks"),
    sheetMotion = useRef<Animation | null>(null),
    backdropDown = useRef(false);
  const layers = panelLayers.use();
  useLayoutEffect(() => {
    if (page === "overview" || !sheet.current) return;
    const node = sheet.current,
      rect = node.getBoundingClientRect(),
      source =
        ((panelHandoff.until ?? 0) > performance.now()
          ? panelHandoff.from
          : undefined) ??
        document.querySelector(".navigation-surface")?.getBoundingClientRect();
    panelHandoff.from = undefined;
    if (source && !matchMedia("(prefers-reduced-motion: reduce)").matches)
      sheetMotion.current = node.animate(
        [
          {
            transform: `translate(${source.x - rect.x}px,${source.y - rect.y}px) scale(${source.width / rect.width},${source.height / rect.height})`,
            opacity: 0,
          },
          { transform: "none", opacity: 1 },
        ],
        { duration: 480, easing: "cubic-bezier(.22,1,.36,1)" },
      );
    document.querySelector<HTMLElement>(".capsule-title")?.focus();
    return () => sheetMotion.current?.cancel();
  }, [page]);
  async function closeSheet(replacement?: HTMLElement) {
    const node = sheet.current;
    if (
      node &&
      !matchMedia("(prefers-reduced-motion: reduce)").matches &&
      !replacement
    ) {
      const rect = node.getBoundingClientRect(),
        dock = document
          .querySelector(".navigation-surface")!
          .getBoundingClientRect();
      sheetMotion.current?.cancel();
      sheetMotion.current = node.animate(
        [
          { transform: "none", opacity: 1 },
          {
            transform: `translate(${dock.x - rect.x}px,${dock.y - rect.y}px) scale(${dock.width / rect.width},${dock.height / rect.height})`,
            opacity: 0,
          },
        ],
        { duration: 220, easing: "cubic-bezier(.4,0,.6,1)", fill: "forwards" },
      );
      await sheetMotion.current.finished.catch(() => {});
    }
    if (replacement && node) {
      panelHandoff.from = node.getBoundingClientRect();
      panelHandoff.until = performance.now() + 100;
      panelHandoff.trigger = replacement;
    }
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
      else stops.push(...listeners);
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
          inert={layers.length > 0}
          aria-label={pages.find((item) => item.id === page)?.label}
        >
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
          <IconButton
            className="panel-width-toggle"
            icon={sheetWide ? Minimize2 : Maximize2}
            label={t(sheetWide ? "restorePanelWidth" : "expandPanelWidth")}
            aria-pressed={sheetWide}
            onClick={() => setSheetWide(!sheetWide)}
          />
        </section>
      )}
      <nav
        className={`navigation-capsule ${page !== "overview" ? "expanded" : ""} ${updateVisible() ? "has-update" : ""}`}
        inert={layers.length > 0}
        aria-label={t("mainNavigation")}
      >
        {page !== "overview" && (
          <>
            <button
              className="capsule-close"
              aria-label={t("backToHome")}
              onClick={() => void closeSheet()}
            >
              <Icon icon={X} size={20} />
            </button>
            <h1 className="capsule-title" tabIndex={-1}>
              {pages.find((item) => item.id === page)?.label}
            </h1>
          </>
        )}
        <div
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
              aria-label={item.label}
              title={item.label}
              onClick={() => navigate(item.id)}
            >
              <Icon icon={item.icon} size={20} />
              {item.id === "tasks" && pending > 0 && (
                <span className="task-count">{pending}</span>
              )}
            </button>
          ))}
        </div>
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
