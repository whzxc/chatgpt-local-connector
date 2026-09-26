import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ChevronRight } from "lucide-react";
import { api } from "../api";
import { t, locale } from "../i18n";
import { isDesktop } from "../platform";
import { subscriptions, readSubscriptions } from "../state/subscriptions";
import type { Page } from "../state/navigation";
import QuotaBubble from "../subscriptions/QuotaBubble";
import { brandIcon } from "../subscriptions/presentation";
import { copyShareCard } from "./shareCard";
import { Button, Empty, Icon, Loading, Menu, Notice } from "../components/ui";
export default function TrayPanel({
  embedded = false,
  onNavigate,
}: {
  embedded?: boolean;
  onNavigate?: (page: Page) => void;
}) {
  locale.use();
  const { snapshot, error: subscriptionError } = subscriptions.use();
  const providers = snapshot?.settings.enabled
    ? snapshot.providers.filter((p) => p.selected && p.eligible)
    : [];
  const [side, setSide] = useState<"left" | "right">("left"),
    [error, setError] = useState(""),
    [version, setVersion] = useState(""),
    [connection, setConnection] = useState(false),
    [busy, setBusy] = useState(false),
    [copied, setCopied] = useState(false),
    [dismissKey, setDismissKey] = useState(0);
  const content = useRef<HTMLDivElement>(null),
    footer = useRef<HTMLElement>(null),
    cards = useRef(new Map<string, HTMLElement>());
  async function action(action: string) {
    if (isDesktop && !embedded) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("tray_action", { action });
    } else if (["overview", "settings", "tasks", "logs"].includes(action)) {
      if (onNavigate) onNavigate(action as Page);
      else window.location.href = "/";
    }
  }
  async function select(key: string) {
    setError("");
    try {
      await action(key);
    } catch (e) {
      setError(String(e));
    }
  }
  async function readStatus() {
    const status = await api<{
      connection?: { running: boolean };
      version: string;
    }>("status");
    setConnection(!!status.connection?.running);
    setVersion(status.version);
  }
  async function share(id: string) {
    setError("");
    setCopied(false);
    setBusy(true);
    try {
      const card = cards.current.get(id),
        provider = providers.find((p) => p.providerId === id);
      if (!card || !provider) throw new Error(t("trayEmpty"));
      await copyShareCard(card, brandIcon(provider.agentId));
      setCopied(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 4000);
    return () => clearTimeout(timer);
  }, [copied]);
  async function openOptions(event: React.MouseEvent<HTMLButtonElement>) {
    if (!isDesktop || embedded || busy) return;
    const rect = event.currentTarget.getBoundingClientRect();
    setError("");
    setDismissKey((k) => k + 1);
    let menu: import("@tauri-apps/api/menu").Menu | undefined;
    try {
      await readStatus();
      const [{ Menu }, { LogicalPosition }] = await Promise.all([
        import("@tauri-apps/api/menu"),
        import("@tauri-apps/api/dpi"),
      ]);
      menu = await Menu.new({
        items: [
          {
            id: "status",
            text: `${t("trayConnectionStatus")}：${t(connection ? "trayConnected" : "trayDisconnected")}`,
            action: () => void select("overview"),
          },
          {
            id: "settings",
            text: t("traySettings"),
            accelerator: "CmdOrCtrl+,",
            action: () => void select("settings"),
          },
          { item: "Separator" },
          {
            id: "share",
            text: t("trayShareScreenshot"),
            enabled: providers.length > 0,
            items: providers.map((p) => ({
              id: `share:${p.providerId}`,
              text: p.name,
              action: () => void share(p.providerId),
            })),
          },
          {
            id: "updates",
            text: t("checkForUpdates") + "…",
            action: () => void select("updates"),
          },
          { item: "Separator" },
          {
            item: {
              About: {
                name: "Local Connector",
                version,
                license: "MIT",
                website: "https://github.com/whzxc/chatgpt-local-connector",
              },
            },
            text: t("trayAbout"),
          },
          {
            id: "quit",
            text: t("trayQuit"),
            accelerator: "CmdOrCtrl+Q",
            action: () => void select("quit"),
          },
        ],
      });
      await action("menu-open");
      await menu.popup(new LogicalPosition(rect.left, rect.top));
    } catch (e) {
      setError(String(e));
    } finally {
      await action("menu-close").catch((e) => setError(String(e)));
      await menu?.close();
    }
  }
  useEffect(() => {
    void readStatus().catch((e) => setError(String(e)));
    if (!isDesktop || embedded) return;
    let stopped = false,
      off: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(async ({ listen }) => {
      const dispose = await listen<{ side: "left" | "right" }>(
        "tray-panel:open",
        (e) => {
          setSide(e.payload.side);
          setDismissKey((k) => k + 1);
          void readSubscriptions();
          void readStatus().catch((e) => setError(String(e)));
        },
      );
      if (stopped) dispose();
      else off = dispose;
    });
    const key = (e: KeyboardEvent) => {
      if (
        e.key === "Escape" &&
        !document.querySelector("[data-radix-popper-content-wrapper]")
      ) {
        e.preventDefault();
        void select("hide");
      }
    };
    document.addEventListener("keydown", key);
    return () => {
      stopped = true;
      off?.();
      document.removeEventListener("keydown", key);
    };
  }, [embedded]);
  useLayoutEffect(() => {
    if (!isDesktop || embedded || !content.current || !footer.current) return;
    let frame = 0,
      lastHeight = 0;
    const resize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const height = Math.ceil(
          content.current!.getBoundingClientRect().height +
            footer.current!.getBoundingClientRect().height +
            2,
        );
        if (height === lastHeight) return;
        lastHeight = height;
        void import("@tauri-apps/api/core")
          .then(({ invoke }) => invoke("tray_panel_resize", { height }))
          .catch((e) => {
            lastHeight = 0;
            setError(String(e));
          });
      });
    };
    const observer = new ResizeObserver(resize);
    observer.observe(content.current);
    observer.observe(footer.current);
    resize();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [embedded]);
  const optionsButton = (
    <Button
      variant="ghost"
      className="tray-options"
      aria-label={t("trayOptions")}
      busy={busy}
    >
      {t(connection ? "trayConnected" : "trayDisconnected")}
      <Icon icon={ChevronRight} />
    </Button>
  );
  return (
    <div className={`tray-surface ${embedded ? "embedded" : ""}`}>
      <section className={`tray-card ${side}`} aria-label={t("trayUsage")}>
        <div
          className="tray-scroll"
          onScroll={() => setDismissKey((k) => k + 1)}
        >
          <div ref={content} className="tray-content">
            {(error || subscriptionError) && (
              <Notice>{error || subscriptionError}</Notice>
            )}
            {copied && (
              <Notice tone="success">{t("trayScreenshotCopied")}</Notice>
            )}
            {!snapshot && !subscriptionError && <Loading />}
            {providers.map((provider) => (
              <div
                key={provider.providerId}
                ref={(el) => {
                  if (el) cards.current.set(provider.providerId, el);
                  else cards.current.delete(provider.providerId);
                }}
                className="provider-card"
              >
                <QuotaBubble
                  key={`${provider.providerId}:${dismissKey}`}
                  provider={provider}
                  detailPlacement={side}
                />
              </div>
            ))}
            {snapshot && !providers.length && (
              <Empty
                action={
                  <Button onClick={() => void select("settings")}>
                    {t("traySettings")}
                  </Button>
                }
              >
                {t("trayEmpty")}
              </Empty>
            )}
          </div>
        </div>
        <footer ref={footer}>
          <span>Local Connector {version}</span>
          {isDesktop && !embedded ? (
            <Button
              variant="ghost"
              className="tray-options"
              busy={busy}
              aria-label={t("trayOptions")}
              onClick={(event) => void openOptions(event)}
            >
              {t(connection ? "trayConnected" : "trayDisconnected")}
              <Icon icon={ChevronRight} />
            </Button>
          ) : (
            <Menu
              label={t("trayOptions")}
              trigger={optionsButton}
              items={[
                {
                  label: t("traySettings"),
                  action: () => void select("settings"),
                },
                ...providers.map((p) => ({
                  label: `${t("trayShareScreenshot")} · ${p.name}`,
                  action: () => void share(p.providerId),
                  disabled: !isDesktop || busy,
                })),
              ]}
            />
          )}
        </footer>
      </section>
    </div>
  );
}
