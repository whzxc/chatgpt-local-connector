import { AlertCircle, ChevronRight, CircleHelp } from "lucide-react";
import { Button, Icon, Input, DataTable, Pagination, Tooltip } from "../components/ui";
import { Empty } from "./components";
import { text, number, date, short, percent } from "./format";
import type { ReadySnapshot, TaskSelection } from "./bridge";
import type { GuideSection } from "./guide";
import QuotaBubble from "../subscriptions/QuotaBubble";
import { presentSubscription } from "../subscriptions/response";
import { UsageChart } from "./usage-chart";
import { money } from "./format";

export function Overview({ data, choose, guide, refreshQuota, selection, selectTasks }: {
  data: ReadySnapshot;
  selection: TaskSelection;
  selectTasks: (value: TaskSelection) => void;
  choose: (id: string) => void;
  guide: (section: GuideSection) => void;
  refreshQuota: () => Promise<void>;
}) {
  const tasks = data.tasks;
  const filtered = !!selection.search;
  const quota = data.quota ? presentSubscription(data.quota) : null;
  const usage = data.usage;
  const hit = usage.input != null && usage.input > 0 && usage.cached != null ? usage.cached / usage.input * 100 : null;
  return (<>
    <div className="insight-summary-grid">
      <section className="insight-card insight-account">
        {quota ? <QuotaBubble provider={quota} showHistory={false} link={false} detailPlacement="right" onRefresh={refreshQuota} /> :
          <Empty><Button variant="ghost" onClick={() => guide("start")}>{text("noQuota")}<Icon icon={CircleHelp} /></Button></Empty>}
      </section>
      <section className="insight-card insight-stat-grid" aria-label={text("usage")}>
        <div className="insight-stat">
          <div className="insight-title"><span>{text("total")}</span></div>
          <strong>{number(usage.total ?? usage.knownTotal, true)}</strong>
          {usage.total == null && usage.knownTotal != null && <small>{text("known")}</small>}
        </div>
        <div className="insight-stat">
          <div className="insight-title"><span>{text("requests")}</span></div>
          <strong>{number(usage.requests)}</strong>
        </div>
        <div className="insight-stat">
          <div className="insight-title"><span>{text("estimatedCost")}</span></div>
          <strong className="insight-cost">{money(usage.estimatedUsd)}</strong>{usage.unpricedRecords > 0 && <small>{text("partialCost")}</small>}
        </div>
        <div className="insight-stat">
          <div className="insight-title"><span>{text("cacheHit")}</span></div>
          <strong>{hit == null ? "—" : percent(hit)}</strong>
        </div>
      </section>
    </div>
    <UsageChart data={data} />
      <section className="insight-card insight-task-list">
        <div className="insight-section-heading">
          <div className="insight-title">
            <h2>{text("tasks")}</h2>
            <span className="insight-count">{number(data.taskMatchCount)}</span>
          </div>
          <div className="insight-task-actions">
            <Input
              className="insight-task-search"
              maxLength={200}
              aria-label={text("search")}
              placeholder={text("search")}
              value={selection.search}
              onChange={(e) => selectTasks({...selection, search: e.target.value, page: 1})}
            />
          </div>
        </div>
        <DataTable
          rows={tasks}
          onOpen={(task) => choose(task.id)}
          empty={<Empty>{text(filtered ? "noMatch" : "empty")}</Empty>}
          columns={[
            {
              title: text("title"), key: "label", width: 260,
              render: (task) => (
                <span className="insight-task-name">
                  <button className="insight-task-link" title={task.label ?? task.id} onClick={(event) => { event.stopPropagation(); choose(task.id); }}>
                    {task.label ?? `${text("task")} ${short(task.id)}`}
                  </button>
                  {task.issues.length > 0 && <Tooltip text={text("uncertain")}><span><Icon icon={AlertCircle} /></span></Tooltip>}
                </span>
              ),
            },
            { title: text("turns"), key: "turns", width: 75, align: "right", render: (task) => number(task.period.turns) },
            { title: text("requests"), key: "requests", width: 85, align: "right", render: (task) => number(task.period.requests) },
            { title: text("total"), key: "tokens", width: 100, align: "right", render: (task) => number(task.period.total, true) },
            { title: text("input"), key: "input", width: 100, align: "right", render: (task) => number(task.period.input, true) },
            { title: text("output"), key: "output", width: 100, align: "right", render: (task) => number(task.period.output, true) },
            { title: text("cacheHit"), key: "cacheHit", width: 110, align: "right", render: (task) => task.period.input && task.period.cached != null ? percent(task.period.cached / task.period.input * 100) : "—" },
            { title: text("estimatedCost"), key: "cost", width: 115, align: "right", render: (task) => money(task.period.estimatedUsd) },
            { title: text("updated"), key: "updated", width: 130, render: (task) => <small>{date(task.lastEventAt)}</small> },
            { key: "open", width: 36, render: () => <Icon icon={ChevronRight} /> },
          ]}
        />
        <Pagination current={data.taskPage} pageSize={10} total={data.taskMatchCount}
          onChange={(page) => selectTasks({...selection, page})} />
        {!tasks.length && (
          <Empty>
            {filtered ? (
              <Button variant="ghost" onClick={() => { selectTasks({search: "", page: 1}); }}>{text("clearFilters")}</Button>
            ) : <Button variant="ghost" onClick={() => guide("start")}>{text("guide.start")}</Button>}
          </Empty>
        )}
      </section>
    </>
  );
}
