import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { t } from "../i18n";
import { displayMessage } from "../messages";
import { connector, refreshConnector } from "../state/connector";
import type { Ingress } from "../state/types";
import {
  Button,
  CopyField,
  Dialog,
  DialogClose,
  Field,
  Notice,
  Status,
} from "./ui";
import OAuthGrants from "./OAuthGrants";
export default function IngressDetailsDialog({
  ingress,
  autoConnect = false,
  onClose,
  onEdit,
}: {
  ingress: Ingress;
  autoConnect?: boolean;
  onClose: () => void;
  onEdit: (ingress: Ingress) => void;
}) {
  const { status } = connector.use();
  const entry = status?.ingresses.find((i) => i.id === ingress.id) || ingress;
  const [working, setWorking] = useState(false),
    [oauthBusy, setOauthBusy] = useState(false),
    [reading, setReading] = useState(false),
    [error, setError] = useState(""),
    [credentialError, setCredentialError] = useState(""),
    [credentials, setCredentials] = useState<{
      bearerToken?: string;
      apiKey?: string;
    }>({}),
    [refreshingUrl, setRefreshingUrl] = useState(false);
  const alive = useRef(true);
  const https = entry.transport === "https",
    connecting = working || oauthBusy || entry.state === "starting";
  let endpoint = https ? entry.url : entry.config.tunnelId;
  const c = entry.config;
  if (https && !endpoint) {
    if (
      c.httpsProvider === "custom" ||
      (c.httpsProvider === "cloudflare" && c.cloudflareMode === "named") ||
      (c.httpsProvider === "pinggy" && c.pinggyMode === "named") ||
      c.httpsProvider === "localxpose"
    )
      endpoint = c.httpsUrl;
    else if (
      c.httpsProvider === "ngrok" &&
      c.ngrokMode === "named" &&
      c.ngrokEndpoint
    ) {
      try {
        endpoint =
          new URL(
            c.ngrokEndpoint.includes("://")
              ? c.ngrokEndpoint
              : `https://${c.ngrokEndpoint}`,
          ).origin + "/mcp";
      } catch {
        endpoint = "";
      }
    }
  }
  async function readCredentials() {
    if (!["bearer", "openai"].includes(entry.auth)) return;
    setReading(true);
    setCredentialError("");
    try {
      const value = await api<typeof credentials>(
        `ingress/${entry.id}/credentials`,
        "POST",
      );
      if (alive.current) setCredentials(value);
    } catch {
      if (alive.current) setCredentialError(t("credentialReadFailed"));
    } finally {
      if (alive.current) setReading(false);
    }
  }
  async function connect() {
    if (connecting) return;
    setWorking(true);
    setError("");
    try {
      await api(`ingress/${entry.id}/start`, "POST");
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally {
      await refreshConnector().catch(() => {});
      if (alive.current) setWorking(false);
    }
  }
  async function updateConnection(kind: "url" | "token") {
    if (connecting || reading) return;
    const restart =
      kind === "url" ? c.httpsProvider !== "custom" : entry.running;
    setWorking(true);
    setRefreshingUrl(restart);
    setError("");
    let stopped = false;
    try {
      if (restart) {
        await api(`ingress/${entry.id}/stop`, "POST");
        stopped = true;
      }
      if (kind === "token") {
        const result = await api<{ token: string }>(
          `ingress/${entry.id}/token`,
          "POST",
        );
        setCredentials((old) => ({ ...old, bearerToken: result.token }));
        setCredentialError("");
      }
    } catch (e) {
      setError(String(e));
    } finally {
      if (stopped)
        try {
          await api(`ingress/${entry.id}/start`, "POST");
        } catch (e) {
          setError((old) => [old, String(e)].filter(Boolean).join("\n"));
        }
      await refreshConnector().catch((e) => setError(String(e)));
      setWorking(false);
      setRefreshingUrl(false);
    }
  }
  useEffect(() => {
    alive.current = true;
    void readCredentials();
    if (autoConnect) void connect();
    return () => {
      alive.current = false;
    };
  }, [ingress, autoConnect]);
  return (
    <Dialog
      title={
        <span className="ingress-dialog-title">
          {entry.name}
          <Status
            tone={entry.error ? "danger" : entry.running ? "success" : "neutral"}
          >
            {t(
              entry.error
                ? "connectionError"
                : entry.running
                  ? "connected"
                  : entry.state === "starting"
                    ? "connectingLabel"
                    : "disconnected",
            )}
          </Status>
        </span>
      }
      onClose={onClose}
      busy={working || oauthBusy}
      footer={
        <>
          <Button disabled={connecting} onClick={() => onEdit(entry)}>
            {t("edit")}
          </Button>
          <span className="spacer" />
          {!entry.running && !connecting && (
            <Button disabled={!entry.enabled} onClick={() => void connect()}>
              {t("connect")}
            </Button>
          )}
          <DialogClose asChild>
            <Button variant="primary" disabled={working || oauthBusy}>
              {t("done")}
            </Button>
          </DialogClose>
        </>
      }
    >
      <div className="form">

        <Field
          label={https ? "MCP URL" : "Tunnel ID"}
          help={
            https && (
              <Button
                variant="ghost"
                disabled={connecting || reading || !entry.enabled}
                onClick={() => void updateConnection("url")}
              >
                {t("reobtainMcpUrl")}
              </Button>
            )
          }
        >
          <CopyField
            value={refreshingUrl ? "" : endpoint || ""}
            label={https ? "MCP URL" : "Tunnel ID"}
            loading={https && (refreshingUrl || (!endpoint && connecting))}
            placeholder={t(
              connecting ? "fetchingMcpUrl" : "generatedWhenConnected",
            )}
          />
        </Field>
        {entry.auth === "bearer" && (
          <Field
            label="Bearer token"
            help={
              <Button
                variant="ghost"
                disabled={connecting || reading}
                onClick={() => void updateConnection("token")}
              >
                {t("generateNewToken")}
              </Button>
            }
          >
            <CopyField
              value={credentials.bearerToken || ""}
              label="Bearer token"
              password
              loading={reading}
            />
          </Field>
        )}
        {entry.auth === "openai" && (
          <Field label="Runtime API Key">
            <CopyField
              value={credentials.apiKey || ""}
              label="Runtime API Key"
              password
              loading={reading}
            />
          </Field>
        )}
        {entry.auth === "oauth" && (
          <OAuthGrants
            ingressId={entry.id}
            running={entry.running}
            disabled={working}
            onBusy={setOauthBusy}
          />
        )}
        {credentialError && (
          <Notice
            action={
              <Button disabled={reading} onClick={() => void readCredentials()}>
                {t("retry")}
              </Button>
            }
          >
            {credentialError}
          </Notice>
        )}
        {https &&
          (c.httpsProvider === "custom" ||
            (c.httpsProvider === "cloudflare" &&
              c.cloudflareMode === "named")) && (
            <Field label={t("httpReverseProxyTarget")}>
              <CopyField
                value={`http://${c.httpsHost.includes(":") ? `[${c.httpsHost}]` : c.httpsHost}:${c.httpsPort}/mcp`}
                label={t("httpReverseProxyTarget")}
              />
            </Field>
          )}
        {(error || entry.error) && (
          <Notice>{displayMessage(error || entry.error)}</Notice>
        )}
      </div>
    </Dialog>
  );
}
