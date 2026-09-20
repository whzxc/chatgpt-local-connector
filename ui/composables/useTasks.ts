import { computed, onMounted, ref } from 'vue';
import { useIntervalFn } from '@vueuse/core';
import { api } from './useConnector';
import { notifyNative } from '../platform';

export interface TaskRecord {
  requestId: string;
  executionOwner?: 'desktop' | 'connector';
  threadId?: string;
  turnId?: string;
  runtime?: TaskRuntime;
  state: string;
  createdAt: string;
  updatedAt: string;
  approval: { decision: string; source: string; at: string };
  error?: { message?: string };
  task: {
    detailsRecorded?: boolean;
    kind: string; title?: string; prompt: string; input: unknown;
    project?: string; directory?: string; threadId?: string;
    model?: string; effort?: string; settings: unknown;
  };
}
export interface TaskRuntime {
  archived?: boolean;
  runtimeStatus: string;
  title?: string;
  project?: string;
  observedAt: string;
  stale?: boolean;
}
export function useTasks() {
  const records = ref<TaskRecord[]>([]);
  const error = ref('');
  const pending = computed(() => records.value.filter(r => r.state === 'awaiting-approval'));
  let refreshing: Promise<void> | undefined;
  let seen: Set<string> | undefined;
  function refresh() {
    if (refreshing) return refreshing;
    refreshing = api<{ records: TaskRecord[] }>('tasks').then(data => {
      const ids = new Set(data.records.filter(r => r.state === 'awaiting-approval').map(r => r.requestId));
      if (seen && [...ids].some(id => !seen!.has(id))) void notifyNative('有新的任务请求待审批', '可在 Connector 或 ChatGPT 中确认，也可按用户意图直接执行。').catch(() => {});
      seen = ids;
      records.value = data.records;
      error.value = '';
    }).catch(e => { error.value = e instanceof Error ? e.message : String(e); })
      .finally(() => { refreshing = undefined; });
    return refreshing;
  }
  onMounted(refresh);
  useIntervalFn(refresh, 5000);
  return { records, pending, error, refresh };
}
