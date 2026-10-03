import type { ProviderSnapshot } from "../subscriptions/types";
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
export type TaskSelection = { page: number; search: string };
export type Task = {
  id: string;
  label: string | null;
  lastEventAt: number | null;
  period: Metrics & { turns: number | null };
  lifetime: Counts;
  family: string;
  issues: string[];
  parentId: string | null;
  forkedFromId: string | null;
};
export type ContextObservation = { id: string; at: number; input: number | null; model: string | null; contextWindow: number | null };
export type Compaction = { durationMs?: number | null; responseId: string; turnId: string | null; at: number; before: ContextObservation | null; after: ContextObservation | null };
export type Turn = {
  contextWindow: number | null;
  contextStart: ContextObservation | null;
  contextEnd: ContextObservation | null;
  id: string;
  prompt: string | null;
  promptImages: number;
  startedAt: number | null;
  completedAt: number | null;
  durationMs: number | null;
  ttftMs: number | null;
  status: string;
  model: string | null;
  effort: string | null;
  usage: Metrics;
  wholeTurnOutputTps: number | null;
  toolCount: number;
};
export type Response = {
  context: ContextObservation;
  previousContext: ContextObservation | null;
  id: string;
  estimatedUsd: number | null;
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
export type AssistantMessage = { id: string; turnId: string; at: number; text: string };
export type Reasoning = { id: string; turnId: string; at: number; text: string; durationMs?: number };
export type Detail = {
  reasoning: Reasoning[];
  messages: AssistantMessage[];
  toolPreviews: Record<string, string>;
  contentError: string | null;
  id: string;
  label: string | null;
  usage: Metrics;
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
  compactions: Compaction[];
  children: string[];
  parentId: string | null;
  forkedFromId: string | null;
};
type SnapshotBase = {
  schemaVersion: number;
  scope: "global" | "thread";
  connectorVersion: string | null;
  message?: string;
};
export type Snapshot =
  | ReadySnapshot
  | (SnapshotBase & {
      state: "collecting" | "incompatible" | "unavailable";
    });
export type Metrics = Counts & { estimatedUsd: number | null; unpricedRecords: number; requests: number };
export type ToolDetail = { id: string; input: unknown; output: unknown };
export type ReadySnapshot = SnapshotBase & {
  toolDetail?: ToolDetail;
  state: "ready";
  observedAt: string;
  binding: string;
  thread: Detail | null;
  tasks: Task[];
  taskCount: number;
  taskMatchCount: number;
  taskPage: number;
  usage: Metrics;
  models: { name: string; usage: Metrics }[];
  daily: { day: string; usage: Metrics }[];
  series: { at: string; models: { name: string; usage: Metrics }[] }[];
  issues: string[];
  range: { start: number; end: number; days: number; startDay: string; endDay: string; timezone: string };
  quota: ProviderSnapshot | null;
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
export type UsageRoute = { scope: "global" | "thread"; threadId: string };
let browserNavigation = false;
const routeListeners = new Set<(route: UsageRoute) => void>();
export function onRoute(listener: (route: UsageRoute) => void) {
  routeListeners.add(listener);
  return () => { routeListeners.delete(listener); };
}
export function navigate(threadId: string) {
  if (browserNavigation) window.parent.postMessage({ jsonrpc: "2.0", method: "clc/notifications/navigate", params: { scope: threadId ? "thread" : "global", threadId } }, "*");
}
function rpc(method: string, params: unknown, timeoutMs = 20000): Promise<RpcResult> {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(text("timeout")));
    }, timeoutMs);
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
  if (browserNavigation && message.method === "clc/notifications/route-changed") {
    const route = message.params;
    if ((route?.scope === "global" || route?.scope === "thread") && typeof route.threadId === "string") routeListeners.forEach(listener => listener(route));
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
        browserNavigation = !!(result as { hostCapabilities?: { experimental?: { usageNavigation?: boolean } } }).hostCapabilities?.experimental?.usageNavigation;
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
export type PluginUpdate = {
  currentVersion?: string; installedVersion?: string; version?: string;
  available?: boolean; installable?: boolean; reloadRequired?: boolean; enabled?: boolean;
};
export async function pluginUpdate(action: "check" | "update", force = false, version?: string) {
  await initialize();
  const result = await rpc("tools/call", {
    name: action === "check" ? "connector_plugin_check" : "connector_plugin_update",
    arguments: action === "check" ? { force } : { version },
  }, action === "update" ? 330000 : 130000);
  return result.structuredContent as unknown as PluginUpdate;
}
declare const __CONNECTOR_VERSION__: string;
