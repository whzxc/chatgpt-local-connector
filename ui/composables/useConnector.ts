import type { CoreSnapshot } from '../types';
import { onUnmounted } from 'vue';
import { desktopRequest, isDesktop } from '../platform';
import {
  computed,
  inject,
  onMounted,
  provide,
  ref,
  type InjectionKey,
} from "vue";
import { useIntervalFn } from "@vueuse/core";

export interface Config {
  proxyMode: "system" | "direct" | "custom";
  proxyUrl: string;
  connectionMode: "tunnel" | "https";
  httpsProvider: "cloudflare" | "ngrok" | "custom";
  hasNgrokAuthtoken: boolean;
  cloudflareMode: "quick" | "named";
  hasCloudflareToken: boolean;
  httpsUrl: string;
  httpsHost: string;
  httpsPort: number;
  configured: boolean;
  tunnelId: string;
  tunnelBinary: string;
  codexBinary: string;
  autoStart: boolean;
  hasApiKey: boolean;
}
export interface Status {
  taskApprovalEnabled: boolean;
  autoOpenCodex: boolean;
  core: CoreSnapshot;
  connection?: { running: boolean; updateAvailable: boolean; mcpUrl: string };
  deviceName: string;
  platform: string;
  version: string;
  connector: { state: string; error?: string };
  tunnel: { state: string; error?: string };
  config: Config;
  logs: string[];
  chatgptUrl?: string;
}
export interface Login {
  state: string;
  message?: string;
  userCode?: string;
  verificationUrl?: string;
}
export interface Service {
  supported: boolean;
  enabled: boolean;
  message?: string;
}
export interface Diagnostics {
  checks: { name: string; status: string; message?: string }[];
  summary?: string;
}

export async function api<T = unknown>(
  path: string,
  method = "GET",
  body?: unknown,
): Promise<T> {
  if (isDesktop) return desktopRequest<T>(path, method, body);
  const response = await fetch(`/api/${path}`, {
    method,
    headers: { "Content-Type": "application/json", "X-CLC-Request": "1" },
    ...(method !== "GET" ? { body: JSON.stringify(body ?? {}) } : {}),
  });
  let data;
  try {
    data = await response.json();
  } catch {
    throw new Error("服务返回了无法识别的响应，请确认管理服务已启动。");
  }
  if (!response.ok)
    throw new Error(data.error || data.message || "请求失败，请重试。");
  return data as T;
}

function createConnector() {
  const status = ref<Status>();
  const busy = ref("");
  const loading = ref(true);
  const connectionError = ref("");
  const feedback = ref({ text: "", error: false });
  const editable = computed(
    () =>
      !!status.value &&
      ["stopped", "error"].includes(status.value.tunnel.state),
  );
  let refreshing: Promise<void> | undefined;
  function notify(text: string, error = false) {
    feedback.value = { text, error };
  }
  async function run(label: string, action: () => Promise<void>) {
    if (busy.value) return;
    busy.value = label;
    try {
      await action();
    } catch (error) {
      notify(
        error instanceof Error ? error.message : typeof error === "string" ? error : "操作失败，请重试。",
        true,
      );
    } finally {
      busy.value = "";
    }
  }
  function refresh(): Promise<void> {
    if (refreshing) return refreshing;
    refreshing = (async () => {
      try {
        status.value = await api<Status>("status");
        connectionError.value = "";
      } catch (error) {
        connectionError.value =
          error instanceof Error ? error.message : typeof error === "string" ? error : "无法连接管理服务";
        throw error;
      } finally {
        refreshing = undefined;
        loading.value = false;
      }
    })();
    return refreshing;
  }
  onMounted(() => {
    void refresh().catch(() => {});
  });
  let stream: EventSource | undefined;
  onMounted(() => {
    if (!isDesktop) { stream = new EventSource('/api/events'); stream.addEventListener('snapshot', event => { status.value = JSON.parse((event as MessageEvent).data) as Status; connectionError.value = ""; }); }
  });
  onUnmounted(() => stream?.close());
  useIntervalFn(() => {
    if (isDesktop) void refresh().catch(() => {});
  }, 2000);
  return {
    status,
    busy,
    loading,
    connectionError,
    feedback,
    editable,
    notify,
    run,
    refresh,
  };
}
const connectorKey: InjectionKey<ReturnType<typeof createConnector>> =
  Symbol("connector");
export function provideConnector() {
  const connector = createConnector();
  provide(connectorKey, connector);
  return connector;
}
export function useConnector() {
  const connector = inject(connectorKey);
  if (!connector) throw new Error("缺少 Connector 上下文");
  return connector;
}
