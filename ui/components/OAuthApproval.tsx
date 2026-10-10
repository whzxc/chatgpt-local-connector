import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { t } from "../i18n";
import { connector } from "../state/connector";
import { useInterval } from "../state/hooks";
import { Button, Dialog, Notice } from "./ui";

type Request = { id: string; status: string; clientName: string; redirectUri: string; resource: string };
type LocalRequest = Request & { ingressId: string; connectionName: string };

export default function OAuthApproval({ requested, onClose }: {
  requested?: { id: string }; onClose: () => void;
}) {
  const { status } = connector.use();
  const [selected, setSelected] = useState<LocalRequest>();
  const [missing, setMissing] = useState(false);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const dismissed = useRef(new Set<string>());
  const desired = useRef<string | undefined>(undefined);
  const reading = useRef(false);
  const writing = useRef(false);
  const revision = useRef(0);
  const alive = useRef(true);
  useEffect(() => () => { alive.current = false; revision.current++; }, []);
  useEffect(() => {
    desired.current = requested?.id;
    if (requested) dismissed.current.delete(requested.id);
    setSelected(undefined); setMissing(false); setError(""); revision.current++;
    void refresh();
  }, [requested]);

  async function refresh() {
    if (!status || reading.current || writing.current) return;
    reading.current = true;
    const version = revision.current;
    try {
      const entries = status.ingresses.filter(i => i.auth === "oauth" && i.running);
      const results = await Promise.all(entries.map(async i => {
        const value = await api<{ pending: Request[]; grants: { id: string; clientName: string }[] }>(`ingress/${i.id}/oauth`);
        return [...value.pending, ...value.grants.map(g => ({ id: g.id, clientName: g.clientName, status: "completed", redirectUri: "", resource: i.url }))].map(p => ({ ...p, ingressId: i.id, connectionName: i.name }));
      }));
      if (!alive.current || version !== revision.current) return;
      const requests = results.flat();
      if (desired.current) {
        const match = requests.find(p => p.id === desired.current);
        setSelected(match); setMissing(!match);
      } else {
        setSelected(current => {
          if (current) return requests.find(p => p.id === current.id) || { ...current, status: "expired" };
          return requests.find(p => p.status === "pending" && !dismissed.current.has(p.id));
        });
      }
      setError("");
    } catch (e) {
      if (alive.current && version === revision.current) setError(String(e));
    } finally { reading.current = false; }
  }
  useInterval(() => void refresh(), 2000, true);
  function close() {
    if (writing.current) return;
    if (selected) dismissed.current.add(selected.id);
    desired.current = undefined; revision.current++;
    setSelected(undefined); setMissing(false); setError(""); onClose();
  }
  async function decide(allow: boolean) {
    if (!selected || selected.status !== "pending" || writing.current) return;
    writing.current = true; revision.current++; setBusy(true); setError("");
    try {
      const result = await api<Request>(`ingress/${selected.ingressId}/oauth/decision`, "POST", { id: selected.id, allow });
      if (alive.current) setSelected({ ...selected, ...result });
    } catch (e) {
      if (alive.current) setError(String(e));
    } finally { writing.current = false; setBusy(false); }
  }
  if (!selected && !missing && !requested) return null;
  const pending = selected?.status === "pending";
  return <Dialog title={t("oauthApprovalTitle")} width={480} onClose={close} busy={busy}
    footer={pending ? <><Button disabled={busy} onClick={() => void decide(false)}>{t("oauthDeny")}</Button>
      <span className="spacer" /><Button variant="primary" disabled={busy} onClick={() => void decide(true)}>{t("oauthAllow")}</Button></>
      : <Button onClick={close}>{t("done")}</Button>}>
    {selected ? <div className="form">
      <div className="oauth-grant-card"><div className="oauth-grant-identity">
        <strong>{selected.clientName}</strong><span>{selected.connectionName}</span>
        <span className="muted small">{selected.resource}</span>
        <span className="muted small">{t("oauthCallback")}: {selected.redirectUri}</span>
      </div></div>
      <p>{t(pending ? "oauthApprovalScope" : selected.status === "approved" ? "oauthApproved" : selected.status === "denied" ? "oauthDenied" : "oauthFinished")}</p>
    </div> : <p>{t(missing ? "oauthWrongDevice" : "oauthReading")}</p>}
    {error && <Notice>{error}</Notice>}
  </Dialog>;
}
