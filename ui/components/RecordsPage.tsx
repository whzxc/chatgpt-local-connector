import { useState } from "react";
import { t, type MessageKey } from "../i18n";
import { connector } from "../state/connector";
import { Empty, Input, SingleChoice, Status } from "./ui";
export default function RecordsPage() {
  const { status } = connector.use();
  const [filter, setFilter] = useState("all"),
    [query, setQuery] = useState("");
  const internalMessage =
    /^(?:provided|supplied|run|invoking|invoke|initialized custom fxevent\.Logger|OnStart hook (?:executing|executed)|OnStop hook (?:executing|executed)|mcp channel route resolved|TunnelServiceClient created|control-plane route resolved|dispatcher channels registered|Skipping MCP probe for transport|admin ui enabled|tls trust summary|health server listening|harpoon enabled|starting control-plane poller|poller started|harpoon startup catalog digest|tunnel-client startup summary|Codex detected without Tunnel MCP plugin|tunnel metadata fetched|stdio MCP command started)$/i;
  const rows = (status?.logs || [])
    .flatMap((line, index) => {
      let entry: Record<string, unknown> = {};
      try {
        const value = JSON.parse(line);
        if (value && typeof value === "object" && !Array.isArray(value))
          entry = value;
      } catch {
        /* Plain log lines remain readable. */
      }
      const rawMessage = String(entry.msg ?? entry.message ?? line);
      const level = String(entry.level ?? "").toLowerCase();
      const error =
        /error|fatal|panic/.test(level) ||
        /\b(error|failed|failure)\b/i.test(rawMessage);
      const warning = /warn/.test(level) || /\bwarn(ing)?\b/i.test(rawMessage);
      // Never hide a warning or error just because its source is internal.
      if (
        !error &&
        !warning &&
        (/debug|trace/.test(level) ||
          internalMessage.test(rawMessage) ||
          /^(?:🩺 HEALTH URL:|🌐 WEB UI:)/.test(rawMessage))
      )
        return [];
      const message = rawMessage;
      const kind: MessageKey = error
        ? "error"
        : warning
          ? "warning"
          : /request|command/i.test(rawMessage)
            ? "request"
            : /tunnel|connect/i.test(rawMessage)
              ? "connection"
              : "info";
      const rawTime = entry.time ?? entry.timestamp;
      const date =
        rawTime === undefined
          ? undefined
          : new Date(rawTime as string | number);
      const validDate = date && !Number.isNaN(date.getTime());
      const time = validDate
        ? `${date.toLocaleTimeString("en-GB", { hour12: false })}.${String(date.getMilliseconds()).padStart(3, "0")}`
        : "—";
      const detailKeys = ["error", "reason", "status", "method", "duration"];
      const detail = detailKeys
        .flatMap((key) =>
          entry[key] === undefined
            ? []
            : [
                `${key}: ${typeof entry[key] === "object" ? JSON.stringify(entry[key]) : String(entry[key])}`,
              ],
        )
        .join(" · ");
      return [
        {
          index,
          time,
          timestamp: validDate
            ? date.toLocaleString("en-GB")
            : "No timestamp in the original record",
          kind,
          message,
          detail,
        },
      ];
    })
    .reverse();

  const visibleRows = rows.filter(
    (row) =>
      (filter === "all" || row.kind === filter) &&
      `${row.message} ${row.detail}`
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  return (
    <section>
      <div className="list-toolbar">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label={t("searchLogs")}
          placeholder={t("searchRecords")}
        />
        <SingleChoice
          label={t("logType")}
          value={filter}
          onChange={setFilter}
          options={[
            "all",
            "connection",
            "request",
            "info",
            "warning",
            "error",
          ].map((value) => ({ value, label: t(value) }))}
        />
        <span className="muted small">
          {t("recordsCount", { count: visibleRows.length })}
        </span>
      </div>
      <div className="log-table-scroll">
        <table className="log-table">
          <caption className="sr-only">{t("connectionLogContents")}</caption>
          <thead>
            <tr>
              <th>{t("time")}</th>
              <th>{t("type")}</th>
              <th>{t("content")}</th>
            </tr>
          </thead>
          <tbody>
            {visibleRows.map((row) => (
              <tr key={row.index}>
                <td>
                  <time title={row.timestamp}>{row.time}</time>
                </td>
                <td>
                  <Status
                    tone={
                      row.kind === "error"
                        ? "danger"
                        : row.kind === "warning"
                          ? "warning"
                          : "neutral"
                    }
                  >
                    {t(row.kind)}
                  </Status>
                </td>
                <td>
                  <span>{row.message}</span>
                  {row.detail && (
                    <details>
                      <summary>{t("details")}</summary>
                      <pre>{row.detail}</pre>
                    </details>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {!visibleRows.length && (
        <Empty>
          {t(rows.length ? "noMatchingRecords" : "noConnectionRecordsYet")}
        </Empty>
      )}
    </section>
  );
}
