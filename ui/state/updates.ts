import { createStore, preference } from "./store";
import { t } from "../i18n";
import { isDesktop, openUrl } from "../platform";
export type Update = {
  available: boolean;
  version?: string;
  notes?: string;
  date?: string;
  restarting?: boolean;
  ready?: boolean;
};
export const autoCheck = preference(
  "update-auto-check",
  true,
  (value) => value !== "off",
  (value) => (value ? "on" : "off"),
);
export const autoDownload = preference(
  "update-auto-download",
  true,
  (value) => value !== "off",
  (value) => (value ? "on" : "off"),
);
export const skipped = preference(
  "update-skipped-version",
  "",
  (value) => value,
  (value) => value,
);
export const updates = createStore<{
  update?: Update;
  checking: boolean;
  phase: "idle" | "downloading" | "installing";
  downloaded: number;
  total?: number;
  error: string;
  message: string;
  dialogOpen: boolean;
  announcement?: Update;
  pluginMessage?: string;
  pluginError?: string;
  pluginSyncing?: boolean;
}>({
  checking: false,
  phase: "idle",
  downloaded: 0,
  error: "",
  message: "",
  dialogOpen: false,
});
const patch = (next: Partial<ReturnType<typeof updates.get>>) =>
  updates.set((old) => ({ ...old, ...next }));
export const available = () =>
  !!updates.get().update?.available &&
  updates.get().update?.version !== skipped.get();
export const updateVisible = () =>
  available() && (!autoDownload.get() || !!updates.get().update?.ready);
let dismissedVersion = "",
  directRequested = false;
export function showUpdate() {
  patch({ dialogOpen: true });
}
export function dismissUpdate() {
  if (updates.get().phase !== "idle") return;
  dismissedVersion = updates.get().update?.version || "";
  patch({ dialogOpen: false });
  directRequested = false;
}
export async function checkUpdate(manual = true) {
  if (!isDesktop || updates.get().checking || updates.get().phase !== "idle")
    return;
  patch({ checking: true });
  if (manual) {
    patch({ error: "", message: "" });
    skipped.set("");
  }
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const update = await invoke<Update>("check_update");
    patch({ update, error: "", message: "" });
    localStorage.setItem("update-last-check", String(Date.now()));
    if (available() && autoDownload.get() && !update.ready)
      await transfer(false);
    if (updateVisible() && (manual || update.version !== dismissedVersion))
      showUpdate();
    if (manual && !update.available) patch({ message: t("youAreUpToDate") });
  } catch (e) {
    if (manual)
      patch({
        error: t("updateCheckFailedValueRetryOrVisitThe", { error: String(e) }),
      });
  } finally {
    patch({ checking: false });
  }
}
async function transfer(installing: boolean, direct = false) {
  const update = updates.get().update;
  if (!update?.version || updates.get().phase !== "idle") return;
  patch({
    error: "",
    message: "",
    downloaded: 0,
    total: undefined,
    phase: installing && update.ready ? "installing" : "downloading",
  });
  let off: (() => void) | undefined,
    restarting = false;
  try {
    const [{ invoke }, { listen }] = await Promise.all([
      import("@tauri-apps/api/core"),
      import("@tauri-apps/api/event"),
    ]);
    off = await listen<{
      downloaded?: number;
      total?: number;
      phase: "downloading" | "installing";
    }>("update-progress", (e) => patch(e.payload));
    if (installing) {
      if (direct)
        localStorage.setItem("update-announcement", JSON.stringify(update));
      else localStorage.removeItem("update-announcement");
    }
    const result = await invoke<Update>(
      installing ? "install_update" : "download_update",
      { version: update.version },
    );
    if (result.restarting) {
      restarting = true;
      patch({ phase: "installing" });
      return;
    }
    patch({ update: result });
    if (installing) localStorage.removeItem("update-announcement");
    if (!result.available) patch({ message: t("youAreUpToDate") });
  } catch (e) {
    if (installing) {
      localStorage.removeItem("update-announcement");
      patch({ update: { ...update, ready: false } });
    }
    if (direct) showUpdate();
    if (String(e) === "UPDATE_CANCELLED")
      patch({ message: t("downloadCancelledTheCurrentConnectionIsUnchanged") });
    else patch({ error: t("updateFailedValue", { error: String(e) }) });
  } finally {
    off?.();
    if (!restarting) patch({ phase: "idle" });
  }
}
export const installUpdate = () => transfer(true, directRequested);
export const installDirect = () => {
  directRequested = true;
  return transfer(true, true);
};
export async function cancelUpdate() {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("cancel_update");
  } catch (e) {
    patch({ error: String(e) });
  }
}
export function skipUpdate() {
  if (updates.get().phase !== "idle") return;
  skipped.set(updates.get().update?.version || "");
  dismissUpdate();
}
export function setAutoDownload(value: boolean) {
  autoDownload.set(value);
  if (value && updates.get().phase === "idle") {
    patch({ dialogOpen: false });
    void checkUpdate(false);
  }
}
export async function openDownloads() {
  try {
    await openUrl(
      "https://github.com/whzxc/chatgpt-local-connector/releases/latest",
    );
  } catch (e) {
    patch({ error: t("unableToOpenDownloadsValue", { error: String(e) }) });
  }
}
export function startUpdateChecks() {
  if (!isDesktop) return () => {};
  void syncPlugin();
  void (async () => {
    try {
      const saved = localStorage.getItem("update-announcement");
      if (!saved) return;
      const pending = JSON.parse(saved) as Update;
      const { getVersion } = await import("@tauri-apps/api/app");
      if (pending.version === (await getVersion()))
        patch({ announcement: pending });
      localStorage.removeItem("update-announcement");
    } catch {
      localStorage.removeItem("update-announcement");
    }
  })();
  let lastAttempt = 0;
  const tick = () => {
    if (!autoCheck.get()) return;
    const previous = Math.max(
      lastAttempt,
      Number(localStorage.getItem("update-last-check") || 0),
    );
    if (Date.now() - previous < 3600000) return;
    lastAttempt = Date.now();
    void checkUpdate(false);
  };
  const first = setTimeout(tick, 5000),
    timer = setInterval(tick, 60000);
  return () => {
    clearTimeout(first);
    clearInterval(timer);
  };
}
export async function syncPlugin() {
  if (!isDesktop || updates.get().pluginSyncing) return;
  patch({ pluginSyncing: true, pluginError: "" });
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await invoke<{state: string; version?: string}>("sync_plugin");
    patch({ pluginMessage: result.state === "installed"
      ? t("pluginUpdateInstalled", { version: result.version })
      : result.state === "current" ? t("pluginUpdateCurrent", { version: result.version })
      : result.state === "host_managed" ? t("pluginUpdateHostManaged")
      : result.state === "not_installed" ? t("pluginUpdateNotInstalled") : "" });
  } catch (error) {
    patch({ pluginError: t("pluginUpdateFailed", { error: String(error) }) });
  } finally { patch({ pluginSyncing: false }); }
}
