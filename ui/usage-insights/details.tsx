import { useEffect, useState } from "react";
import { motion } from "motion/react";
import {
  Dialog,
  SingleChoice,
  Notice,
  Button,
  HoverScope,
} from "../components/ui";
import { refresh, type ReadySnapshot, type Turn } from "./bridge";
import {
  Coverage,
  Empty,
  Expandable,
  Help,
  Pager,
  TokenBreakdown,
  Value,
} from "./components";
import { text, number, date, duration, short, status } from "./format";
export type DetailView = {
  kind:
    | "responses"
    | "tools"
    | "turn"
    | "source"
    | "relations"
    | "models"
    | "usage";
  turn?: Turn;
};
export function Details({
  view,
  data,
  close,
  choose,
}: {
  view: DetailView;
  data: ReadySnapshot;
  close: () => void;
  choose: (id: string) => void;
}) {
  const [tab, setTab] = useState(view.kind === "tools" ? "tools" : "responses");
  const [displayPage, setDisplayPage] = useState(0);
  const [page, setPage] = useState(0),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  const [detail, setDetail] = useState(
    view.kind === "turn" ? null : data.thread,
  );
  const threadId = data.thread?.id;
  const records = ["turn", "responses", "tools"].includes(view.kind);
  useEffect(() => {
    if (!records || !threadId) return;
    let alive = true;
    setBusy(!detail || page !== displayPage);
    setError("");
    void refresh({
      scope: "thread",
      threadId,
      ...(view.turn ? { turnId: view.turn.id } : {}),
      responseOffset: page * 6,
      toolOffset: page * 6,
    })
      .then((next) => {
        if (alive) {
          if (next.state !== "ready" || !next.thread)
            throw new Error(next.message ?? text("unavailable"));
          setDetail(next.thread);
          setDisplayPage(page);
        }
      })
      .catch((e) => {
        if (alive) setError(String(e.message ?? e));
      })
      .finally(() => {
        if (alive) setBusy(false);
      });
    return () => {
      alive = false;
    };
  }, [threadId, view.turn?.id, page, tab, records, data.observedAt]);
  const title =
    view.kind === "turn"
      ? `${text("turns")} · ${short(view.turn!.id)}`
      : text(view.kind);
  return (
    <Dialog title={title} onClose={close} width={620}>
      <div className="insight-detail" aria-busy={busy}>
        {error && <Notice>{error}</Notice>}
        {view.kind === "source" && (
          <>
            <p>{text("sourceHelp")}</p>
            <dl>
              <dt>Connector</dt>
              <dd>{data.connectorVersion}</dd>
              <dt>{text("observed")}</dt>
              <dd>{date(data.observedAt)}</dd>
              {data.thread && (
                <>
                  <dt>{text("version")}</dt>
                  <dd>{data.thread.cliVersion ?? "—"}</dd>
                  <dt>{text("identity")}</dt>
                  <dd>{data.thread.id}</dd>
                  <dt>{text("origin")}</dt>
                  <dd>{data.thread.family}</dd>
                </>
              )}
            </dl>
            <Coverage
              issues={[
                ...new Set([...data.issues, ...(data.thread?.issues ?? [])]),
              ]}
            />
          </>
        )}
        {view.kind === "usage" && (
          <>
            <TokenBreakdown value={data.usage} />
            <p>{text("usageHelp")}</p>
            <small>
              {date(data.range.start)} — {date(data.range.end)}
            </small>
          </>
        )}
        {view.kind === "models" && (
          <>
            <p>{text("modelHelp")}</p>
            {data.models.map((m) => (
              <div
                className="insight-between insight-model-detail"
                key={m.name}
              >
                <span>{m.name}</span>
                <Value value={m.usage.total} />
              </div>
            ))}
          </>
        )}
        {view.kind === "relations" && data.thread && (
          <>
            <p>{text("relationsHelp")}</p>
            <HoverScope>
              {[
                ["parent", data.thread.parentId],
                ["fork", data.thread.forkedFromId],
                ...data.thread.children.map((id) => ["children", id]),
              ]
                .filter(([, id]) => id)
                .map(([label, id]) => (
                  <Button
                    variant="ghost"
                    data-hover-target={`${label}-${id}`}
                    className="insight-link-row"
                    key={`${label}-${id}`}
                    onClick={() => {
                      close();
                      choose(id!);
                    }}
                  >
                    <span>{text(label!)}</span>
                    <span>{short(id!)}</span>
                  </Button>
                ))}
            </HoverScope>
            {data.thread.compactions.map((c) => (
              <div className="insight-between" key={c.responseId}>
                <span>
                  {text("compactions")} · {short(c.responseId)}
                </span>
                <small>{date(c.at)}</small>
              </div>
            ))}
          </>
        )}
        {view.turn && (
          <>
            <TokenBreakdown value={view.turn.usage} />
            <dl>
              <dt>{text("status")}</dt>
              <dd>{status(view.turn.status)}</dd>
              <dt>{text("model")}</dt>
              <dd>{view.turn.model ?? "—"}</dd>
              <dt>{text("effort")}</dt>
              <dd>{view.turn.effort ?? "—"}</dd>
              <dt>{text("duration")}</dt>
              <dd>{duration(view.turn.durationMs)}</dd>
              <dt>{text("ttft")}</dt>
              <dd>{duration(view.turn.ttftMs)}</dd>
              <dt>{text("speed")}</dt>
              <dd>{number(view.turn.wholeTurnOutputTps)} tok/s</dd>
            </dl>
            <Help name="turn" />
          </>
        )}
        {records && (
          <>
            <div className="insight-section-heading">
              <SingleChoice
                label={text("source")}
                value={tab}
                onChange={(v) => {
                  setTab(v);
                  setPage(0);
                }}
                options={[
                  { value: "responses", label: text("responses") },
                  { value: "tools", label: text("tools") },
                ]}
              />
              <Help name={tab === "responses" ? "response" : "tool"} />
            </div>
            <motion.div
              key={`${tab}-${displayPage}`}
              className="insight-records"
              initial={{ opacity: 0 }}
              animate={{ opacity: busy ? 0.55 : 1 }}
            >
              {detail &&
                tab === "responses" &&
                detail.responses.slice(0, 6).map((r) => (
                  <Expandable
                    key={r.id}
                    title={
                      <>
                        <span>
                          <strong>{r.model ?? short(r.id)}</strong>
                          <small>
                            {date(r.at)} · {short(r.id)}
                          </small>
                        </span>
                        <span>
                          {number(
                            r.tokens.total ??
                              (r.tokens.input != null && r.tokens.output != null
                                ? r.tokens.input + r.tokens.output
                                : null),
                            true,
                          )}
                        </span>
                      </>
                    }
                  >
                    <TokenBreakdown value={r.tokens} />
                    <dl>
                      <dt>{text("effort")}</dt>
                      <dd>{r.effort ?? "—"}</dd>
                      <dt>{text("tier")}</dt>
                      <dd>{r.serviceTier ?? "—"}</dd>
                      <dt>{text("identity")}</dt>
                      <dd>{r.id}</dd>
                      <dt>{text("turns")}</dt>
                      <dd>{r.turnId ?? "—"}</dd>
                      <dt>{text("origin")}</dt>
                      <dd>{r.family}</dd>
                    </dl>
                    {!r.reliable && <Notice>{text("recordHelp")}</Notice>}
                  </Expandable>
                ))}
              {detail &&
                tab === "tools" &&
                detail.tools.slice(0, 6).map((tool) => (
                  <Expandable
                    key={tool.id}
                    title={
                      <>
                        <span>
                          <strong>{tool.name ?? short(tool.id)}</strong>
                          <small>{date(tool.at)}</small>
                        </span>
                        <span>
                          {tool.outputBytes == null
                            ? "—"
                            : `${number(tool.outputBytes, true)} B`}
                        </span>
                      </>
                    }
                  >
                    <dl>
                      <dt>{text("identity")}</dt>
                      <dd>{tool.id}</dd>
                      <dt>{text("turns")}</dt>
                      <dd>{tool.turnId ?? "—"}</dd>
                      <dt>{text("status")}</dt>
                      <dd>
                        {tool.status ??
                          text(
                            tool.outputBytes == null ? "pending" : "returned",
                          )}
                      </dd>
                      <dt>{text("duration")}</dt>
                      <dd>
                        {duration(
                          tool.startedAt != null && tool.completedAt != null
                            ? tool.completedAt - tool.startedAt
                            : null,
                        )}
                      </dd>
                      <dt>{text("bytes")}</dt>
                      <dd>{number(tool.outputBytes)} B</dd>
                    </dl>
                  </Expandable>
                ))}
              {(!detail ||
                (tab === "responses"
                  ? detail.responseCount
                  : detail.toolCount) === 0) && (
                <Empty>{busy ? text("loading") : text("empty")}</Empty>
              )}
            </motion.div>
            <Pager
              page={displayPage}
              count={
                detail
                  ? tab === "responses"
                    ? detail.responseCount
                    : detail.toolCount
                  : 0
              }
              size={6}
              change={setPage}
              busy={busy}
            />
          </>
        )}
      </div>
    </Dialog>
  );
}
