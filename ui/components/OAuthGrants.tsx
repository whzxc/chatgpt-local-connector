import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { t } from "../i18n";
import { useInterval } from "../state/hooks";
import { Button, Empty, Notice } from "./ui";
type Grant = { id: string; clientName: string; clientId: string };
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
      setError("");
      return;
    }
    const version = revision.current;
    try {
      const result = await api<{ grants: Grant[] }>(
        `ingress/${ingressId}/oauth`,
      );
      if (alive.current && version === revision.current) {
        setGrants(result.grants);
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
      <h3>OAuth</h3>
      <h4>{t("oauthAuthorizedClients")}</h4>
      {grants.map((grant) => (
        <div className="oauth-grant-card" key={grant.id}>
          <div className="oauth-grant-identity"><strong>{grant.clientName}</strong>
            <span className="muted small">{t("oauthClientId")}: {grant.clientId}</span>
          </div>
          <Button
            disabled={busy || disabled || !running}
            onClick={() => void action("revoke", { id: grant.id })}
          >
            {t("oauthRevoke")}
          </Button>
        </div>
      ))}
      {!grants.length && !error && (
        <Empty>{t("oauthNoGrants")}</Empty>
      )}
      {error && <Notice>{error}</Notice>}
    </section>
  );
}
