<script setup lang="ts">
import { displayMessage } from '../messages';
import { t, locale } from '../i18n';
import ElasticPanel from './ElasticPanel.vue';
import { computed, ref, watch } from 'vue';
import { useClipboard, useIntervalFn, useResizeObserver } from '@vueuse/core';
import { api, useConnector } from '../composables/useConnector';
import type { TaskRecord, TaskRuntime } from '../composables/useTasks';
const props = defineProps<{ records: TaskRecord[]; error: string }>();
const emit = defineEmits<{ refresh: [] }>();
const { busy, run } = useConnector();
const runtimes = ref<Record<string, TaskRuntime>>({});
const { copy, copied } = useClipboard();
const runtimeLabels = computed<Record<string, string>>(() => ({ waiting:t('awaitingInteraction'), active:t('running'), idle:t('idle'), systemError:t('runtimeError') }));
const decisions = computed<Record<string, string>>(() => ({pending:t('awaitingConfirmation'), automatic:t('automaticExecution'), approved:t('approved'), bypass:t('bypassed'), reject:t('rejected')}));
const kinds = computed<Record<string, string>>(() => ({create:t('createTask'),send:t('sendInput'),interrupt:t('interruptTask'),native:t('manageTask')}));
const idOf = (r: TaskRecord) => r.threadId || r.task.threadId;
const projectOf = (r: TaskRecord) => r.task.project || r.task.directory || (idOf(r) && runtimes.value[idOf(r)!]?.project) || '';
const titleOf = (r: TaskRecord) => r.task.title || (idOf(r) && runtimes.value[idOf(r)!]?.title) || r.task.prompt.split('\n')[0]?.slice(0, 90) || kinds.value[r.task.kind] || t('taskRequest');
const time = (s: string) => new Date(s).toLocaleString(locale.value, { hour12: false });
const groups = computed(() => {
  const result = new Map<string, TaskRecord[]>();
  for (const r of props.records) {
    const key = idOf(r) || r.requestId;
    const list = result.get(key) || [];
    list.push(r); result.set(key, list);
  }
  return [...result].map(([id, records]) => ({ id, records, first: records.find(r => r.task.kind === 'create') || records[records.length - 1]!, latest: records[0]! })).sort((a,b) => b.first.createdAt.localeCompare(a.first.createdAt));
});
const visible = computed(() => groups.value);
watch(() => props.records, records => {
  for (const r of records) {
    const id = idOf(r);
    if (id && r.runtime && !runtimes.value[id]) runtimes.value[id] = r.runtime;
  }
}, { immediate:true });
const projectName = (r: TaskRecord) => projectOf(r).replace(/[\\/]+$/, '').split(/[\\/]/).pop() || projectOf(r);
const createdTime = (s: string) => new Date(s).toLocaleString(locale.value, { year:'2-digit', month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit', hour12:false });
function stateOf(g: (typeof groups.value)[number]) {
  const runtime = runtimes.value[g.id];
  if (runtime?.archived) return t('archived');
  if (!runtime || runtime.stale) return '';
  return runtimeLabels.value[runtime.runtimeStatus] || '';
}
let checking = false;
async function refreshRuntime() {
  if (checking) return;
  checking = true;
  const ids = visible.value.map(g => idOf(g.first)).filter((id): id is string => !!id);
  try {
    // Keep owner snapshot reads bounded; failed refreshes retain the last observed state.
    for (let i = 0; i < ids.length; i += 4) await Promise.all(ids.slice(i, i + 4).map(async id => {
      try { runtimes.value[id] = await api<TaskRuntime>(`tasks/runtime/${encodeURIComponent(id)}`); }
      catch { const old = runtimes.value[id]; runtimes.value[id] = old ? { ...old, stale: true } : { runtimeStatus:'unknown', observedAt:'', stale:true }; }
    }));
  } finally { checking = false; }
}
watch(() => visible.value.map(g => g.id).join(','), refreshRuntime, { immediate:true });
useIntervalFn(refreshRuntime, 10000);
async function decide(r: TaskRecord, action: string) {
  await run('task-decision', async () => {
    await api('tasks/decision', 'POST', { requestId:r.requestId, action });
    emit('refresh');
  });
}
async function copyPrompt(prompt: string) {
  actionError.value = '';
  try { await copy(prompt); } catch { actionError.value = t('copyFailedSelectAndCopyTheTextManually'); }
}
const actionError = ref('');
const openingTask = ref(false);
async function openTask(id: string) {
  if (openingTask.value) return;
  actionError.value = '';
  openingTask.value = true;
  try { await api('tasks/open', 'POST', { threadId: id }); }
  catch { if (selectedId.value === id) actionError.value = t('unableToOpenCodexMakeSureCodexIs'); }
  finally { openingTask.value = false; }
}
const selectedId = ref<string>();
const selected = computed(() => groups.value.find(g => g.id === selectedId.value));
const detailOpen = ref(false);
const origin = ref({x:0,y:0,size:1,height:1});
const cards = ref<HTMLElement[]>([]);
useResizeObserver(cards, entries => {
  for (const { target } of entries) {
    const card = target as HTMLElement;
    card.parentElement!.style.gridRowEnd = `span ${Math.ceil(card.offsetHeight + 22)}`;
  }
});
function focusCard(id: string, event: MouseEvent) {
  actionError.value = '';
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  origin.value = { x: rect.x, y: rect.y, size: rect.width, height: rect.height };
  selectedId.value = id;
  detailOpen.value = true;
}
</script>
<template>
  <section class="tasks-page" :aria-label="t('cloudTasks')">
    <p v-if="error" class="status-banner warning" role="alert">{{displayMessage(error)}}</p>
    <div class="task-grid">
      <div v-for="g in visible" :key="g.id" class="task-cell"><button ref="cards" class="task-document" aria-haspopup="dialog" @click="focusCard(g.id,$event)">
        <span class="task-document-heading"><strong class="task-document-title">{{titleOf(g.first)}}</strong><span v-if="g.first.task.model" class="task-tag">{{g.first.task.model}}</span><span v-if="g.first.task.effort" class="task-tag">{{g.first.task.effort}}</span></span>
        <time :datetime="g.first.createdAt">{{createdTime(g.first.createdAt)}}</time>
        <span v-if="g.first.task.prompt" class="task-document-preview">{{g.first.task.prompt}}</span>
        <span class="task-document-bottom"><span v-if="projectOf(g.first)" class="task-document-project" :title="projectOf(g.first)">{{projectName(g.first)}}</span><span v-if="stateOf(g)" class="task-document-state">{{stateOf(g)}}</span></span>
      </button></div>
    </div>
    <ElasticPanel v-if="selected" :show="detailOpen" :origin="origin" :title="titleOf(selected.first)" :width="720" :busy="!!busy || openingTask" @close="detailOpen=false" @closed="selectedId=undefined">
      <div class="task-document-heading"><span v-if="selected.first.task.model" class="task-tag">{{selected.first.task.model}}</span><span v-if="selected.first.task.effort" class="task-tag">{{selected.first.task.effort}}</span><time>{{createdTime(selected.first.createdAt)}}</time></div>
          <div class="task-focus-content">
          <p v-if="selected.first.executionOwner === 'connector'" class="hint">{{ t('connectorRunsThisTaskInTheBackgroundDisconnecting') }}</p>
        <section v-for="r in [...selected.records].reverse()" :key="r.requestId" class="task-entry">
          <div class="task-entry-heading"><strong>{{kinds[r.task.kind] || t('manageTask')}}</strong><span v-if="r.task.model" class="task-tag">{{r.task.model}}</span><span v-if="r.task.effort" class="task-tag">{{r.task.effort}}</span><time>{{time(r.createdAt)}}</time><button v-if="r.task.prompt" class="text-button" @click="copyPrompt(r.task.prompt)">{{copied?t('copied'):t('copyPrompt')}}</button></div>
          <pre v-if="r.task.prompt" class="task-prompt">{{r.task.prompt}}</pre>
          <div v-if="decisions[r.approval.decision]" class="task-entry-meta"><span v-if="decisions[r.approval.decision]">{{decisions[r.approval.decision]}} · {{r.approval.source==='local'?t('local'):t('cloud')}}</span></div>
          <p v-if="r.error?.message" class="status-banner warning">{{displayMessage(r.error.message)}}</p>
          <div v-if="r.state==='awaiting-approval'" class="task-entry-actions"><button class="primary" :disabled="!!busy" @click="decide(r,'approve')">{{ t('approveAndSubmit') }}</button><button :disabled="!!busy" @click="decide(r,'reject')">{{ t('reject') }}</button></div>
        </section>
          </div>
          <p v-if="actionError" class="status-banner warning" role="alert">{{displayMessage(actionError)}}</p>
          <template v-if="projectOf(selected.first) || stateOf(selected) || idOf(selected.first)" #footer><div class="task-focus-footer"><span v-if="projectOf(selected.first)" class="task-document-project" :title="projectOf(selected.first)">{{projectName(selected.first)}}</span><span v-if="stateOf(selected)" class="task-document-state">{{stateOf(selected)}}</span><button v-if="idOf(selected.first) && !runtimes[selected.id]?.archived" class="text-button" :disabled="openingTask" @click="openTask(selected.id)">{{openingTask ? t('opening') : t('openInCodex')}}</button></div></template>
    </ElasticPanel>
    <p v-if="!visible.length && !error" class="empty">{{ t('noTasksYet') }}</p>
  </section>
</template>
