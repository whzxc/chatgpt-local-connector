import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import {
  Info,
  ChevronDown,
  ChevronRight,
  AlertCircle,
} from "lucide-react";
import { Button, Icon, Tooltip, Popover, Pagination } from "../components/ui";
import { t } from "../i18n";
import { text, number, percent, money } from "./format";
import type { Counts, Detail } from "./bridge";

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
type StatItem = {label: string; value: ReactNode; onClick?: () => void};
function StatsGrid({items}: {items: StatItem[]}) {
  return <div className="insight-metric-grid">{items.map(item => item.onClick
    ? <Button variant="ghost" className="insight-metric insight-metric-action" key={item.label} onClick={item.onClick}>
        <span>{item.label}<Icon icon={ChevronRight} /></span><strong>{item.value}</strong>
      </Button>
    : <div className="insight-metric" key={item.label}><span>{item.label}</span><strong>{item.value}</strong></div>
  )}</div>;
}
const cacheHit = (value: Counts) => value.input && value.cached != null ? percent(value.cached / value.input * 100) : "—";
export function UsageStats({detail, open}: {detail: Detail; open: (kind: "responses" | "tools" | "relations") => void}) {
  const value = detail.usage, turns = detail.turnCount;
  const relations = detail.children.length + detail.compactions.length + Number(!!detail.parentId) + Number(!!detail.forkedFromId);
  return <StatsGrid items={[
    {label:text("turns"),value:number(turns)},
    {label:text("requests"),value:number(value.requests)},
    {label:text("estimatedCost"),value:<span className="insight-cost">{money(value.estimatedUsd)}</span>},
    {label:text("total"),value:<Value value={value.total ?? value.knownTotal} />},
    {label:text("inputOutput"),value:<span className="insight-io"><Value value={value.input} /><span>/</span><Value value={value.output} /></span>},
    {label:text("cacheHit"),value:cacheHit(value)},
    {label:text("responses"),value:number(detail.responseCount),onClick:() => open("responses")},
    {label:text("tools"),value:number(detail.toolCount),onClick:() => open("tools")},
    {label:text("relations"),value:number(relations),onClick:() => open("relations")},
  ]} />;
}
export function TokenStats({value}: {value: Counts}) {
  return <StatsGrid items={[
    {label:text("total"),value:<Value value={value.total ?? value.knownTotal ?? (value.input != null && value.output != null ? value.input + value.output : null)} />},
    {label:text("input"),value:<Value value={value.input} />},
    {label:text("output"),value:<Value value={value.output} />},
    {label:text("cached"),value:<Value value={value.cached} />},
    {label:text("reasoning"),value:<Value value={value.reasoning} />},
    {label:text("cacheHit"),value:cacheHit(value)},
  ]} />;
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
      <Pagination
        current={page + 1}
        total={count}
        pageSize={size}
        disabled={busy}
        onChange={(next) => change(next - 1)}
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
