import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import {
  Info,
  ChevronDown,
} from "lucide-react";
import { Button, Icon, Tooltip, Popover } from "../components/ui";
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
type StatItem = {label: string; value: ReactNode};
export function useMetricFontSize() {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const grid = ref.current;
    if (!grid) return;
    const fit = () => {
      grid.style.removeProperty("--metric-font-size");
      const values = Array.from(grid.querySelectorAll<HTMLElement>(":scope > div > strong"));
      const base = values[0] && parseFloat(getComputedStyle(values[0]).fontSize);
      if (!base) return;
      const scale = Math.min(1, ...values.map(value => {
        const width = value.firstElementChild?.getBoundingClientRect().width ?? 0;
        return width > 0 ? Math.max(0, value.clientWidth - 2) / width : 1;
      }));
      grid.style.setProperty("--metric-font-size", `${Math.floor(base * scale * 10) / 10}px`);
    };
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(grid);
    return () => observer.disconnect();
  });
  return ref;
}
function StatsGrid({items}: {items: StatItem[]}) {
  const ref = useMetricFontSize();
  return <div className="insight-metric-grid" ref={ref}>{items.map(item =>
    <div className="insight-metric" key={item.label}><span>{item.label}</span><strong><span>{item.value}</span></strong></div>
  )}</div>;
}
const cacheHit = (value: Counts) => value.input && value.cached != null ? percent(value.cached / value.input * 100) : "—";
export function UsageStats({detail}: {detail: Detail}) {
  const value = detail.usage, turns = detail.turnCount;
  return <StatsGrid items={[
    {label:text("turns"),value:number(turns)},
    {label:text("requests"),value:number(value.requests)},
    {label:text("estimatedCost"),value:<span className="insight-cost">{money(value.estimatedUsd)}</span>},
    {label:text("total"),value:<Value value={value.total ?? value.knownTotal} />},
    {label:text("inputOutput"),value:<span className="insight-io"><Value value={value.input} /><span>/</span><Value value={value.output} /></span>},
    {label:text("cacheHit"),value:<span className="insight-cache-hit">{cacheHit(value)}</span>},
  ]} />;
}
export function Empty({ children }: { children?: ReactNode }) {
  return <div className="insight-empty">{children ?? text("empty")}</div>;
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
