import { useEffect, useRef, useState } from "react";
import { Button, Notice } from "../components/ui";
import { t } from "../i18n";
import { pluginUpdate, type PluginUpdate } from "./bridge";

export function PluginUpdates() {
  const [state, setState] = useState<PluginUpdate>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const running = useRef(false);
  async function run(action: "check" | "update", manual = true) {
    if (running.current) return;
    running.current = true;
    setBusy(true);
    if (manual) setError("");
    try {
      const result = await pluginUpdate(action, manual, state?.version);
      setState(result);
      setError("");
    } catch (error) {
      if (manual) setError(String(error));
    } finally {
      running.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    void run("check", false);
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") void run("check", false);
    }, 3600000);
    return () => clearInterval(timer);
  }, []);
  return <div className="insight-plugin-update">
    <div className="insight-header-actions">
      <span>{state?.reloadRequired ? t("pluginUpdateInstalled", { version: state.installedVersion || state.version })
        : state?.available ? t("newVersionValue", { version: state.version })
        : state?.installedVersion ? t("pluginUpdateCurrent", { version: state.installedVersion }) : t("pluginUpdates")}</span>
      <Button variant="ghost" busy={busy} disabled={busy} onClick={() => void run("check")}>{t("checkForUpdates")}</Button>
      {state?.available && state.installable && state.enabled !== false && !state.reloadRequired &&
        <Button busy={busy} disabled={busy} onClick={() => void run("update")}>{t("pluginUpdateInstall")}</Button>}
    </div>
    {state?.available && state.installable && !state.reloadRequired && <Notice>{t("pluginUpdateReloadNotice")}</Notice>}
    {state?.available && !state.installable && !state.reloadRequired && <Notice>{t("pluginUpdateManual")}</Notice>}
    {error && <Notice>{error}</Notice>}
  </div>;
}
