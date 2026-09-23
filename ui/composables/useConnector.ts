import type { ToolPolicy } from '../toolPolicy';
import { t } from '../i18n';
import type { CoreSnapshot } from '../types';
import { onUnmounted } from 'vue';
import { isDesktop } from '../platform';
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
  httpsProvider: "cloudflare" | "ngrok" | "pinggy" | "localxpose" | "custom";
  hasNgrokAuthtoken: boolean;
  ngrokMode: "quick" | "named";
  ngrokEndpoint: string;
  pinggyMode: "quick" | "named";
  hasPinggyToken: boolean;
  localxposeMode: "quick" | "named";
  hasLocalxposeAccessToken: boolean;
  localxposeRegion: "us" | "eu" | "ap";
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
export interface Ingress {
  toolPolicy?: ToolPolicy;
  discoveredUrls?: string[];
  id: string; name: string; controlSource: string; transport: string;
  state: string; running: boolean; error: string; enabled: boolean; auth: string;
  config: Config; url: string; verification?: { verifiedAt?: string; challengeVerifiedAt?: string };
}
export interface Status {
  ingresses: Ingress[];
  ingressSummary: { running: number; ready: number; total: number };
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

export { api } from '../api';
import { api } from '../api';

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
        error instanceof Error ? error.message : typeof error === "string" ? error : t('operationFailedPleaseRetry'),
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
          error instanceof Error ? error.message : typeof error === "string" ? error : t('unableToConnectToTheManagementService');
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
  if (!connector) throw new Error(t('missingConnectorContext'));
  return connector;
}
