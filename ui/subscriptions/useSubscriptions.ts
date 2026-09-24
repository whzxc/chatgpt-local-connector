import { inject, onMounted, onUnmounted, provide, ref, type InjectionKey, type Ref } from 'vue';
import { presentSubscription } from './response';
import { api } from '../api';
import { isDesktop } from '../platform';
import type { Snapshot, Settings } from './types';
export const subscriptionSnapshotKey: InjectionKey<Ref<Snapshot | undefined>> = Symbol('subscriptions');
export const subscriptionRefreshKey: InjectionKey<(providerId: string) => Promise<void>> = Symbol('subscription-refresh');
export const subscriptionSettingsKey: InjectionKey<(patch: Partial<Settings>) => Promise<void>> = Symbol('subscription-settings');
const subscriptionsKey: InjectionKey<{
  snapshot: Ref<Snapshot | undefined>;
  error: Ref<string>;
  accept: (next: Snapshot) => void;
  read: () => Promise<void>;
  refresh: (providerId: string) => Promise<void>;
}> = Symbol('subscription-state');
export function useSubscriptions() {
  const inherited = inject(subscriptionsKey, undefined);
  if (inherited) return inherited;
  const snapshot = ref<Snapshot>(); const error = ref('');
  let unlisten: (() => void) | undefined; let timer: ReturnType<typeof setInterval> | undefined; let stopped = false;
  let stream: EventSource | undefined;
  const retired = new Set<string>();
  function accept(next: Snapshot) {
    if (stopped || retired.has(next.instanceId)) return;
    if (snapshot.value?.instanceId === next.instanceId && snapshot.value.revision > next.revision) return;
    if (snapshot.value && snapshot.value.instanceId !== next.instanceId) retired.add(snapshot.value.instanceId);
    snapshot.value = {...next, providers:next.providers.map(presentSubscription)}; error.value = '';
  }
  async function read() { const before=snapshot.value; try { accept(await api<Snapshot>('subscriptions')); } catch (e) { if (!stopped && snapshot.value===before) { error.value = String(e); if (before) snapshot.value = {...before, providers:before.providers.map(p=>({...p, state:p.state==='ready'?'stale':p.state, refreshing:false}))}; } } }
  async function refresh(providerId: string) {
    await api('subscriptions/refresh', 'POST', { providerId });
    if (stopped) return;
    accept(await api<Snapshot>('subscriptions'));

  }
  provide(subscriptionRefreshKey, refresh);
  provide(subscriptionSettingsKey, async patch => {
    if (!snapshot.value) throw new Error('Subscription settings unavailable');
    accept(await api<Snapshot>('subscriptions/settings', 'PUT', {...snapshot.value.settings, ...patch}));
  });
  onMounted(async () => {
    if (isDesktop) { const { listen } = await import('@tauri-apps/api/event'); const off = await listen<Snapshot>('subscriptions:changed', e => accept(e.payload)); if (stopped) off(); else unlisten = off; }
    if (stopped) return;
    if (!isDesktop) {
      stream = new EventSource('/api/subscriptions/events');
      stream.addEventListener('snapshot', event => accept(JSON.parse((event as MessageEvent).data) as Snapshot));
    }
    await read();
    if (stopped) return;
    timer = setInterval(() => { if (document.visibilityState === 'visible' && (!stream || stream.readyState !== EventSource.OPEN)) void read(); }, 30000);
  });
  onUnmounted(() => { stopped = true; stream?.close(); unlisten?.(); clearInterval(timer); });
  const subscriptions = { snapshot, error, accept, read, refresh };
  provide(subscriptionsKey, subscriptions);
  return subscriptions;
}
