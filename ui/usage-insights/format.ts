import { locale, t } from "../i18n";
export const text = (key: string, params: Record<string, unknown> = {}) =>
  t(`insights.${key}`, params);
export const number = (value: number | null | undefined, compact = false) =>
  value == null
    ? "—"
    : new Intl.NumberFormat(compact ? "en-US" : locale.get(), {
        notation: compact ? "compact" : "standard",
        maximumFractionDigits: compact ? 2 : 1,
      }).format(value);
export const date = (value: number | string | null | undefined) =>
  value == null
    ? "—"
    : new Date(value).toLocaleString(locale.get(), {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
export const duration = (value: number | null | undefined) =>
  value == null
    ? "—"
    : value < 60000
      ? `${number(value / 1000)} s`
      : `${Math.floor(value / 60000)}m ${Math.floor((value % 60000) / 1000)}s`;
export const short = (id: string) => id.slice(0, 8);
export const status = (value: string) =>
  text(
    (
      {
        completed: "completed",
        interrupted: "interrupted",
        "running-at-last-event": "running",
      } as Record<string, string>
    )[value] ?? "unknown",
  );

export const percent = (value: number) =>
  value > 0 && value < 0.1
    ? "<0.1%"
    : `${new Intl.NumberFormat(locale.get(), { maximumFractionDigits: 2 }).format(value)}%`;
export const windowLabel = (value: string) => {
  const match = /^(\d+) min$/.exec(value);
  if (!match) return value;
  const minutes = Number(match[1]);
  const unit =
    minutes % 1440 === 0 ? "day" : minutes % 60 === 0 ? "hour" : "minute";
  const amount =
    unit === "day" ? minutes / 1440 : unit === "hour" ? minutes / 60 : minutes;
  return new Intl.NumberFormat(locale.get(), {
    style: "unit",
    unit,
    unitDisplay: "short",
  }).format(amount);
};

export const count = (kind: "Turns" | "Responses" | "Tools", value: number) =>
  text(`count${kind}${value === 1 ? "One" : ""}`, { n: number(value) });
