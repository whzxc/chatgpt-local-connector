import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { t } from "../i18n";
import { useInterval } from "../state/hooks";
import { Button, Empty, Loading, Notice, IconButton } from "./ui";
import { CircleHelp } from "lucide-react";
type Pending = {
  id: string;
  clientName: string;
  redirectUri: string;
  status: "pending" | "waiting" | "expired";
};
type Grant = { id: string; clientName: string };
export default function OAuthGrants({
  ingressId,
  running,
  disabled,
  onBusy,
}: {
  ingressId: string;
  running: boolean;
  disabled?: boolean;
  onBusy: (value: boolean) => void;
}) {
  const [grants, setGrants] = useState<Grant[]>([]),
    [pending, setPending] = useState<Pending[]>([]),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const revision = useRef(0);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      revision.current++;
    };
  }, []);
  async function refresh(afterAction = false) {
    if (busy && !afterAction) return;
    if (!running) {
      setGrants([]);
      setPending([]);
      setError("");
      return;
    }
    const version = revision.current;
    try {
      const result = await api<{ grants: Grant[]; pending: Pending[] }>(
        `ingress/${ingressId}/oauth`,
      );
      if (alive.current && version === revision.current) {
        setGrants(result.grants);
        setPending(result.pending);
        setError("");
      }
    } catch (e) {
      if (alive.current && version === revision.current) setError(String(e));
    }
  }
  useInterval(() => void refresh(), 3000, true);
  useEffect(() => {
    void refresh();
  }, [running]);
  async function action(route: string, body: object) {
    if (busy || disabled || !running) return;
    revision.current++;
    setBusy(true);
    onBusy(true);
    setError("");
    try {
      await api(`ingress/${ingressId}/oauth/${route}`, "POST", body);
      await refresh(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      onBusy(false);
    }
  }
  return (
    <section className="oauth-grants">
      <div className="actions">
        <h3>OAuth</h3>
        <IconButton icon={CircleHelp} label={t("oauthConnectHint")} />
      </div>
      {pending.map((request) => (
        <div className="oauth-request" key={request.id}>
          <div>
            <strong>{request.clientName}</strong>
            <p className="muted small">
              {(() => {
                try {
                  return new URL(request.redirectUri).hostname;
                } catch {
                  return "";
                }
              })()}
            </p>
          </div>
          <div className="actions">
            {request.status === "waiting" ? (
              <Loading label={t("oauthWaiting")} />
            ) : request.status === "expired" ? (
              <Notice>{t("oauthWaitExpired")}</Notice>
            ) : (
              <Button
                variant="primary"
                disabled={busy || disabled || !running}
                onClick={() =>
                  void action("decision", { id: request.id, allow: true })
                }
              >
                {t("oauthAllow")}
              </Button>
            )}
            <Button
              variant="danger"
              disabled={busy || disabled || !running}
              onClick={() =>
                void action("decision", { id: request.id, allow: false })
              }
            >
              {t("oauthDeny")}
            </Button>
          </div>
        </div>
      ))}
      {grants.map((grant) => (
        <div className="settings-row" key={grant.id}>
          <span>{grant.clientName}</span>
          <Button
            disabled={busy || disabled || !running}
            onClick={() => void action("revoke", { id: grant.id })}
          >
            {t("oauthRevoke")}
          </Button>
        </div>
      ))}
      {!grants.length && !pending.length && !error && (
        <Empty>{t("oauthNoGrants")}</Empty>
      )}
      {error && <Notice>{error}</Notice>}
    </section>
  );
}
