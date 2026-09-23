import { computed, inject, onMounted, onUnmounted, provide, ref, type InjectionKey, type Ref } from 'vue';
import { api } from '../api';
const key: InjectionKey<Ref<string[]>> = Symbol('agent-activity');
export function provideAgentActivity() {
  const active = ref<string[]>([]);
  provide(key, active);
  let stopped = false, timer: ReturnType<typeof setTimeout> | undefined, expiry: ReturnType<typeof setTimeout> | undefined;
  async function read() {
    expiry = setTimeout(() => { active.value = []; }, 10000);
    try {
      const result = await api<{activeAgents:string[]}>('agents/activity');
      if (!stopped) active.value = result.activeAgents;
    } catch { if (!stopped) active.value = []; }
    finally { clearTimeout(expiry); if (!stopped) timer = setTimeout(read, 2000); }
  }
  onMounted(read);
  onUnmounted(() => { stopped = true; clearTimeout(timer); clearTimeout(expiry); });
  return active;
}
export function useAgentActive(agent: () => string) {
  const active = inject(key);
  return computed(() => active?.value.includes(agent()) ?? false);
}
