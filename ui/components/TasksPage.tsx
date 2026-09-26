import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ExternalLink, Copy } from "lucide-react";
import { t, locale } from "../i18n";
import { api } from "../api";
import { copyText } from "../platform";
import { displayMessage } from "../messages";
import { connector, run, notify } from "../state/connector";
import {
  tasks,
  refreshTasks,
  type TaskRecord,
  type TaskRuntime,
} from "../state/tasks";
import { useInterval } from "../state/hooks";
import { Button, Empty, IconButton, Loading, Notice, Dialog } from "./ui";
const idOf = (r: TaskRecord) => r.threadId || r.task.threadId;
export default function TasksPage() {
  const { records, error, loading } = tasks.use(),
    { busy } = connector.use();
  const [runtimes, setRuntimes] = useState<Record<string, TaskRuntime>>({}),
    [selectedId, setSelectedId] = useState<string>(),
    [opening, setOpening] = useState(false),
    [actionError, setActionError] = useState("");
  const groups = useMemo(() => {
    const result = new Map<string, TaskRecord[]>();
    for (const r of records) {
      const key = idOf(r) || r.requestId;
      result.set(key, [...(result.get(key) || []), r]);
    }
    return [...result]
      .map(([id, records]) => ({
        id,
        records,
        first: records.find((r) => r.task.kind === "create") || records.at(-1)!,
      }))
      .sort((a, b) => b.first.createdAt.localeCompare(a.first.createdAt));
  }, [records]);
  const runtime = (r: TaskRecord) => runtimes[idOf(r) || ""] || r.runtime;
  const titleOf = (r: TaskRecord) =>
    r.task.title ||
    runtime(r)?.title ||
    r.task.prompt.split("\n")[0]?.slice(0, 90) ||
    t("taskRequest");
  const projectOf = (r: TaskRecord) =>
    r.task.project || r.task.directory || runtime(r)?.project || "";
  const projectName = (r: TaskRecord) =>
    projectOf(r)
      .replace(/[\\/]+$/, "")
      .split(/[\\/]/)
      .pop() || "";
  const stateOf = (group: (typeof groups)[number]) =>
    group.records.some((r) => r.state === "awaiting-approval")
      ? "awaitingConfirmation"
      : runtime(group.first)?.archived
        ? "archived"
        : !runtime(group.first) || runtime(group.first)?.stale
          ? "unknown"
          : {
              waiting: "awaitingInteraction",
              active: "running",
              idle: "idle",
              systemError: "runtimeError",
            }[runtime(group.first)!.runtimeStatus] || "unknown";
  const visible = groups;
  const selected = groups.find((g) => g.id === selectedId);
  const reading = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  async function refreshRuntime() {
    if (reading.current) return;
    reading.current = true;
    const ids = groups
      .map((g) => idOf(g.first))
      .filter((id): id is string => !!id);
    for (let i = 0; i < ids.length; i += 4)
      await Promise.all(
        ids.slice(i, i + 4).map(async (id) => {
          try {
            const next = await api<TaskRuntime>(
              `tasks/runtime/${encodeURIComponent(id)}`,
            );
            if (alive.current) setRuntimes((old) => ({ ...old, [id]: next }));
          } catch {
            if (alive.current)
              setRuntimes((old) => ({
                ...old,
                [id]: {
                  ...(old[id] || { runtimeStatus: "unknown", observedAt: "" }),
                  stale: true,
                },
              }));
          }
        }),
      );
    reading.current = false;
  }
  const ids = groups.map((g) => g.id).join(",");
  useEffect(() => {
    void refreshRuntime();
  }, [ids]);
  useInterval(() => void refreshRuntime(), 10000);
  function closeDetail() {
    const id = selectedId;
    setSelectedId(undefined);
    requestAnimationFrame(() =>
      document
        .querySelector<HTMLButtonElement>(
          `[data-task-id="${CSS.escape(id || "")}"]`,
        )
        ?.focus(),
    );
  }
  const grid = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    let frame = 0;
    const cards = [
      ...(grid.current?.querySelectorAll<HTMLElement>(".task-document") ?? []),
    ];
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const heights = cards.map((card) => Math.ceil(card.offsetHeight + 22));
        cards.forEach((card, index) => {
          card.parentElement!.style.gridRowEnd = `span ${heights[index]}`;
        });
      });
    });
    cards.forEach((node) => observer.observe(node));
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [ids]);
  const time = (value: string) =>
    new Date(value).toLocaleString(locale.get(), {
      year: "2-digit",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  const detail = selected && (
    <Dialog
      title={titleOf(selected.first)}
      width={720}
      busy={!!busy || opening}
      onClose={closeDetail}
      footer={
        <div className="task-focus-footer">
          <span
            className="task-document-project"
            title={projectOf(selected.first)}
          >
            {projectName(selected.first)}
          </span>
          <span className="task-document-state">{t(stateOf(selected))}</span>
          {idOf(selected.first) && !runtime(selected.first)?.archived && (
            <IconButton
              icon={ExternalLink}
              label={t("openInCodex")}
              busy={opening}
              onClick={() => {
                setOpening(true);
                setActionError("");
                void api("tasks/open", "POST", { threadId: selected.id })
                  .catch(() =>
                    setActionError(t("unableToOpenCodexMakeSureCodexIs")),
                  )
                  .finally(() => setOpening(false));
              }}
            />
          )}
        </div>
      }
    >
      <div className="task-document-heading">
        {selected.first.task.model && (
          <span className="task-tag">{selected.first.task.model}</span>
        )}
        {selected.first.task.effort && (
          <span className="task-tag">{selected.first.task.effort}</span>
        )}
        <time>{time(selected.first.createdAt)}</time>
      </div>
      <div className="task-focus-content">
        {selected.first.executionOwner === "connector" && (
          <p className="muted small">
            {t("connectorRunsThisTaskInTheBackgroundDisconnecting")}
          </p>
        )}
        {[...selected.records].reverse().map((r) => (
          <section className="task-entry" key={r.requestId}>
            <header>
              <strong>
                {t(
                  {
                    create: "createTask",
                    send: "sendInput",
                    interrupt: "interruptTask",
                    native: "manageTask",
                  }[r.task.kind] || "manageTask",
                )}
              </strong>
              <time>{time(r.createdAt)}</time>
              {r.task.prompt && (
                <IconButton
                  icon={Copy}
                  label={t("copyPrompt")}
                  onClick={() =>
                    void copyText(r.task.prompt)
                      .then(() => notify(t("copied")))
                      .catch(() =>
                        setActionError(
                          t("copyFailedSelectAndCopyTheTextManually"),
                        ),
                      )
                  }
                />
              )}
            </header>
            <pre className="task-prompt">{r.task.prompt}</pre>
            <p className="muted small">
              {t(
                {
                  pending: "awaitingConfirmation",
                  automatic: "automaticExecution",
                  approved: "approved",
                  bypass: "bypassed",
                  reject: "rejected",
                }[r.approval.decision] || r.approval.decision,
              )}{" "}
              · {t(r.approval.source === "local" ? "local" : "cloud")}
              {r.task.model ? ` · ${r.task.model}` : ""}
              {r.task.effort ? ` · ${r.task.effort}` : ""}
            </p>
            {r.error?.message && (
              <Notice>{displayMessage(r.error.message)}</Notice>
            )}
            {r.state === "awaiting-approval" && (
              <div className="actions">
                {["approve", "reject"].map((action) => (
                  <Button
                    key={action}
                    variant={action === "approve" ? "primary" : "secondary"}
                    disabled={!!busy}
                    onClick={() =>
                      void run("task-decision", async () => {
                        await api("tasks/decision", "POST", {
                          requestId: r.requestId,
                          action,
                        });
                        await refreshTasks();
                      })
                    }
                  >
                    {t(action === "approve" ? "approveAndSubmit" : "reject")}
                  </Button>
                ))}
              </div>
            )}
          </section>
        ))}
        {actionError && <Notice>{actionError}</Notice>}
      </div>
    </Dialog>
  );
  return (
    <section className="tasks-page">
      {error && (
        <Notice
          action={
            <Button onClick={() => void refreshTasks()}>{t("retry")}</Button>
          }
        >
          {displayMessage(error)}
        </Notice>
      )}
      {loading ? (
        <Loading />
      ) : (
        <div ref={grid} className="task-grid">
          {visible.map((g) => (
            <div className="task-cell" key={g.id}>
              <button
                type="button"
                className="task-document"
                data-task-id={g.id}
                aria-haspopup="dialog"
                onClick={() => {
                  setSelectedId(g.id);
                  setActionError("");
                }}
              >
                <span className="task-document-heading">
                  <strong className="task-document-title">
                    {titleOf(g.first)}
                  </strong>
                  {g.first.task.model && (
                    <span className="task-tag">{g.first.task.model}</span>
                  )}
                  {g.first.task.effort && (
                    <span className="task-tag">{g.first.task.effort}</span>
                  )}
                </span>
                <time dateTime={g.first.createdAt}>
                  {time(g.first.createdAt)}
                </time>
                {g.first.task.prompt && (
                  <span className="task-document-preview">
                    {g.first.task.prompt}
                  </span>
                )}
                <span className="task-document-bottom">
                  {projectOf(g.first) && (
                    <span
                      className="task-document-project"
                      title={projectOf(g.first)}
                    >
                      {projectName(g.first)}
                    </span>
                  )}
                  <span className="task-document-state">{t(stateOf(g))}</span>
                </span>
              </button>
            </div>
          ))}
        </div>
      )}
      {detail}
      {!loading && !visible.length && !error && (
        <Empty>{t(groups.length ? "noMatchingTasks" : "noTasksYet")}</Empty>
      )}
    </section>
  );
}
