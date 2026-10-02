import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import {
  Info,
  ChevronLeft,
  ChevronRight,
  ChevronDown,
  AlertCircle,
} from "lucide-react";
import { Button, Icon, IconButton, Tooltip, Popover } from "../components/ui";
import { t } from "../i18n";
import { text, number, percent } from "./format";
import type { Counts } from "./bridge";

export function Help({
  name,
  children,
}: {
  name: string;
  children?: ReactNode;
}) {
  const content =
    children ??
    text(
      `${({ models: "model", turns: "turn", tasks: "task" } as Record<string, string>)[name] ?? name}Help`,
    );
  return (
    <Popover
      hover
      label={text(name)}
      trigger={
        <Button
          variant="ghost"
          className="icon-button insight-help"
          aria-label={text(name)}
        >
          <Icon icon={Info} />
        </Button>
      }
    >
      <div className="insight-help-content">{content}</div>
    </Popover>
  );
}
export function Heading({
  name,
  children,
}: {
  name: string;
  children?: ReactNode;
}) {
  return (
    <div className="insight-section-heading">
      <div className="insight-title">
        <h2>{text(name)}</h2>
        <Help name={name} />
      </div>
      {children}
    </div>
  );
}
export function Value({
  value,
  compact = true,
}: {
  value: number | null | undefined;
  compact?: boolean;
}) {
  return (
    <Tooltip text={value == null ? text("unknown") : number(value)}>
      <span tabIndex={0} className="insight-value">
        {number(value, compact)}
      </span>
    </Tooltip>
  );
}
export function TokenBreakdown({ value }: { value: Counts }) {
  const input = value.input,
    output = value.output;
  const total = input != null && output != null ? input + output : null;
  const ratio = total && input != null ? (input / total) * 100 : null;
  return (
    <div className="insight-token">
      <div className="insight-section-heading">
        <div className="insight-title">
          <h2>{text("total")}</h2>
          <Help name="token" />
        </div>
        <div className="insight-token-total">
          <Value value={value.total ?? value.knownTotal} />
          {value.total == null && value.knownTotal != null && (
            <small>{text("known")}</small>
          )}
        </div>
      </div>
      <div
        className="insight-token-track"
        role="img"
        aria-label={`${text("input")} ${number(input)}, ${text("output")} ${number(output)}`}
      >
        {ratio != null && (
          <>
            <motion.div
              className="insight-input"
              initial={false}
              animate={{ width: `${ratio}%` }}
            />
            <motion.div
              className="insight-output"
              initial={false}
              animate={{ width: `${100 - ratio}%` }}
            />
          </>
        )}
      </div>
      <div className="insight-ratios">
        <span>{ratio == null ? "—" : percent(ratio)}</span>
        <span>{ratio == null ? "—" : percent(100 - ratio)}</span>
      </div>
      {(["input", "output"] as const).map((kind) => (
        <div className="insight-token-group" key={kind}>
          <div className="insight-between">
            <span className="insight-title">
              <i className={`insight-swatch insight-${kind}`} />
              {text(kind)}
            </span>
            <strong>
              <Value value={value[kind]} />
            </strong>
          </div>
          <div className="insight-between insight-subset">
            <span className="insight-title">
              ↳ {text(kind === "input" ? "cached" : "reasoning")}
              <Help name={kind === "input" ? "cached" : "reasoning"} />
            </span>
            <Value value={value[kind === "input" ? "cached" : "reasoning"]} />
          </div>
        </div>
      ))}
    </div>
  );
}
export function Pager({
  page,
  count,
  size,
  change,
  busy = false,
}: {
  page: number;
  count: number;
  size: number;
  change: (n: number) => void;
  busy?: boolean;
}) {
  return (
    <div className="insight-pager">
      <IconButton
        icon={ChevronLeft}
        label={text("previous")}
        disabled={busy || page === 0}
        onClick={() => change(page - 1)}
      />
      <span aria-live="polite">
        {count
          ? `${page * size + 1}–${Math.min((page + 1) * size, count)} / ${number(count)}`
          : "0 / 0"}
      </span>
      <IconButton
        icon={ChevronRight}
        label={text("next")}
        disabled={busy || (page + 1) * size >= count}
        onClick={() => change(page + 1)}
      />
    </div>
  );
}
export function Empty({ children }: { children?: ReactNode }) {
  return <div className="insight-empty">{children ?? text("empty")}</div>;
}
export function Coverage({ issues }: { issues: string[] }) {
  return (
    <div className="insight-coverage">
      {issues.map((issue) => (
        <div className="insight-issue" key={issue}>
          <Icon icon={AlertCircle} />
          <span>
            {t(`insights.issue.${issue}`) === `insights.issue.${issue}`
              ? text("fallbackIssue")
              : t(`insights.issue.${issue}`)}
          </span>
        </div>
      ))}
    </div>
  );
}
export function Expandable({
  title,
  children,
}: {
  title: ReactNode;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div className="insight-record">
      <Button
        variant="ghost"
        className="insight-record-trigger"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        {title}
        <motion.span animate={{ rotate: open ? 180 : 0 }}>
          <Icon icon={ChevronDown} />
        </motion.span>
      </Button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            className="insight-collapse"
          >
            <div className="insight-record-body">{children}</div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
