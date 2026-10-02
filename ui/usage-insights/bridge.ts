import { text } from "./format";
import { locale, resolveLocale } from "../i18n";
function applyHostContext(context: unknown) {
  applyMcpHostTheme(context);
  const host = context as { locale?: string } | undefined;
  if (host?.locale) {
    const next = resolveLocale([host.locale]);
    locale.set(next);
    document.documentElement.lang = next;
  }
}

import { applyMcpHostTheme } from "../theme";
type RpcResult = { _meta?: { usage?: Snapshot }; structuredContent?: Snapshot };
export type Counts = {
  input: number | null;
  cached: number | null;
  output: number | null;
  reasoning: number | null;
  total: number | null;
  knownTotal?: number | null;
  uncertainRecords?: number;
  records: number;
};
export type Task = {
  id: string;
  label: string;
  lastEventAt: number | null;
  period: Counts;
  lifetime: Counts;
  family: string;
  issues: string[];
  parentId: string | null;
  forkedFromId: string | null;
};
export type Turn = {
  id: string;
  startedAt: number | null;
  completedAt: number | null;
  durationMs: number | null;
  ttftMs: number | null;
  status: string;
  model: string | null;
  effort: string | null;
  usage: Counts;
  wholeTurnOutputTps: number | null;
  toolCount: number;
};
export type Response = {
  id: string;
  turnId: string | null;
  at: number;
  model: string | null;
  effort: string | null;
  serviceTier: string | null;
  tokens: Counts;
  family: string;
  reliable: boolean;
};
export type Tool = {
  id: string;
  turnId: string | null;
  name: string | null;
  at: number;
  outputBytes: number | null;
  startedAt: number | null;
  completedAt: number | null;
  status: string | null;
};
export type Detail = {
  id: string;
  usage: Counts;
  issues: string[];
  family: string;
  cliVersion: string | null;
  lastEventAt: number | null;
  turns: Turn[];
  turnCount: number;
  responses: Response[];
  responseCount: number;
  tools: Tool[];
  toolCount: number;
  compactions: { responseId: string; turnId: string | null; at: number }[];
  children: string[];
  parentId: string | null;
  forkedFromId: string | null;
};
export type Snapshot = {
  schemaVersion: number;
  scope: "global" | "thread";
  state: string;
  message?: string;
  connectorVersion: string;
  observedAt: string;
  binding: string;
  thread: Detail | null;
  tasks: Task[];
  taskCount: number;
  usage: Counts;
  models: { name: string; usage: Counts }[];
  daily: { day: string; usage: Counts }[];
  issues: string[];
  range: { start: number; end: number; days: number };
  quota: {
    state: string;
    observedAt: string | null;
    selected: boolean;
    windows: {
      id: string;
      poolId: string;
      label: string;
      usedPercent: number;
      resetsAt: string | null;
    }[];
  } | null;
};
let sequence = 0;
const pending = new Map<
  number,
  {
    resolve: (value: RpcResult) => void;
    reject: (error: Error) => void;
    timeout: ReturnType<typeof setTimeout>;
  }
>();
const listeners = new Set<(data: Snapshot) => void>();
export function onResult(listener: (data: Snapshot) => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
function rpc(method: string, params: unknown): Promise<RpcResult> {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(text("timeout")));
    }, 20000);
    pending.set(id, { resolve, reject, timeout });
    window.parent.postMessage({ jsonrpc: "2.0", id, method, params }, "*");
  });
}
function accept(result: RpcResult) {
  const data = result?._meta?.usage ?? result?.structuredContent;
  if (data?.schemaVersion === 1) listeners.forEach((fn) => fn(data));
}
window.addEventListener("message", (event) => {
  if (event.source !== window.parent || event.data?.jsonrpc !== "2.0") return;
  const message = event.data;
  const call = pending.get(message.id);
  if (call && !message.method) {
    pending.delete(message.id);
    clearTimeout(call.timeout);
    if (message.error)
      call.reject(new Error(message.error.message || text("rpcError")));
    else call.resolve(message.result);
    return;
  }
  if (message.method === "ui/notifications/tool-result") accept(message.params);
  if (message.method === "ui/notifications/host-context-changed")
    applyHostContext(message.params);
  if (message.method === "ui/resource-teardown") {
    for (const call of pending.values()) {
      clearTimeout(call.timeout);
      call.reject(new Error(text("closed")));
    }
    pending.clear();
    window.parent.postMessage(
      { jsonrpc: "2.0", id: message.id, result: {} },
      "*",
    );
  }
});
let initialization: Promise<void> | undefined;
export function initialize() {
  if (!initialization)
    initialization = rpc("ui/initialize", {
      appInfo: { name: "clc-usage", version: __CONNECTOR_VERSION__ },
      appCapabilities: {},
      protocolVersion: "2026-01-26",
    }).then(
      (result) => {
        applyHostContext((result as { hostContext?: unknown }).hostContext);
        window.parent.postMessage(
          { jsonrpc: "2.0", method: "ui/notifications/initialized" },
          "*",
        );
      },
      (error) => {
        initialization = undefined;
        throw error;
      },
    );
  return initialization;
}
export async function refresh(args: Record<string, unknown>) {
  const result = await rpc("tools/call", {
    name: "connector_usage_refresh",
    arguments: args,
  });
  const data = result._meta?.usage ?? result.structuredContent;
  if (data?.schemaVersion !== 1) throw new Error(text("schemaError"));
  return data;
}
declare const __CONNECTOR_VERSION__: string;
