import { useMemo, useState } from "react";
import { SingleChoice, Progress, StackedBarChart } from "../components/ui";
import { Empty } from "./components";
import { number, percent, text, money } from "./format";
import type { ReadySnapshot, Metrics } from "./bridge";

type Metric = "tokens" | "cost" | "requests";
const value = (usage: Metrics, metric: Metric) => metric === "tokens" ? usage.total ?? usage.knownTotal ?? 0 : metric === "cost" ? usage.estimatedUsd ?? 0 : usage.requests;

export function UsageChart({data}: {data: ReadySnapshot}) {
  const [metric, setMetric] = useState<Metric>("tokens");
  const [highlight, setHighlight] = useState<string>();
  const categories = useMemo(() => data.models.map((model, index) => ({
    ...model, key:`model${index}`, color:`var(--chart-${index % 8 + 1})`,
  })), [data.models]);
  const rows = useMemo(() => {
    const points = new Map(data.series.map(point => [data.range.days === 1 ? String(Date.parse(point.at)) : point.at, point.models]));
    const rows: Record<string, string | number>[] = [];
    const today = data.range.days === 1;
    const start = today ? data.range.start : Date.parse(`${data.range.startDay}T00:00:00Z`);
    const end = today ? data.range.end : Date.parse(`${data.range.endDay}T00:00:00Z`);
    const step = today ? 3600000 : 86400000;
    for (let at = start; at <= end && rows.length < 32; at += step) {
      const day = new Date(at).toISOString().slice(0,10);
      const label = today ? `${String(rows.length).padStart(2,"0")}:00` : day.slice(5).replace("-","/");
      const models = points.get(today ? String(at) : day) ?? [];
      const row: Record<string,string|number> = {label};
      for (const category of categories) {
        const model = models.find(model => model.name === category.name);
        if (model && model.usage.requests > 0) row[category.key] = value(model.usage, metric);
      }
      rows.push(row);
    }
    return rows;
  }, [data.series, data.range, categories, metric]);
  const ranked = [...categories].sort((a,b) => value(b.usage,metric)-value(a.usage,metric));
  const total = categories.reduce((sum,model) => sum+value(model.usage,metric), 0);
  const display = (n: number, compact = false) => metric === "cost" ? money(n, compact) : number(n, compact);
  return <section className={`insight-card insight-trend-card${metric === "cost" ? " insight-trend-cost" : ""}`}>
    <div className="insight-section-heading">
      <h2>{text("trend")}</h2>
      <SingleChoice compact label={text("chartMetric")} value={metric} onChange={v => setMetric(v as Metric)} options={[
        {value:"tokens",label:text("total")},{value:"cost",label:text("estimatedCost")},{value:"requests",label:text("requests")},
      ]} />
    </div>
    <div className="insight-trend-body">
      <StackedBarChart data={rows} series={categories} valueLabel={display} label={`${text("trend")} · ${data.range.timezone}`} highlighted={highlight} />
      <div className="insight-model-ranking">
        {ranked.map(model => {
          const share = total ? value(model.usage,metric)/total*100 : 0;
          const hit = model.usage.input && model.usage.cached != null ? model.usage.cached/model.usage.input*100 : null;
          return <div className="insight-ranked-model" key={model.key} tabIndex={0}
            onPointerEnter={() => setHighlight(model.key)} onPointerLeave={() => setHighlight(undefined)}
            onFocus={() => setHighlight(model.key)} onBlur={() => setHighlight(undefined)}>
            <div className="insight-between"><span className="insight-title"><i className="insight-swatch" style={{background:model.color}} />{model.name}<span className="insight-model-share">{percent(share)}</span></span><strong className={metric === "cost" ? "insight-cost" : undefined}>{metric === "cost" && model.usage.estimatedUsd == null ? "—" : display(value(model.usage,metric))}</strong></div>
            <Progress color={model.color} value={share} label={model.name} />
            <small>{text("requestCount",{n:number(model.usage.requests)})} · <span className="insight-cost">{money(model.usage.estimatedUsd)}</span> · {text("cacheHit")} <span className="insight-cache-hit">{hit == null ? "—" : percent(hit)}</span>{model.usage.unpricedRecords > 0 ? ` · ${text("partialCost")}` : ""}</small>
          </div>;
        })}
        {!ranked.length && <Empty />}
      </div>
    </div>
  </section>;
}
