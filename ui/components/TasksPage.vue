<script setup lang="ts">
import { X } from '@lucide/vue';
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { useClipboard, useIntervalFn, usePreferredReducedMotion, useResizeObserver } from '@vueuse/core';
import { api, useConnector } from '../composables/useConnector';
import type { TaskRecord, TaskRuntime } from '../composables/useTasks';
const props = defineProps<{ records: TaskRecord[]; error: string }>();
const emit = defineEmits<{ refresh: [] }>();
const { busy, run } = useConnector();
const runtimes = ref<Record<string, TaskRuntime>>({});
const { copy, copied } = useClipboard();
const runtimeLabels: Record<string,string> = { waiting:'等待交互', active:'运行中', idle:'空闲', systemError:'运行异常' };
const decisions: Record<string,string> = {pending:'等待确认', automatic:'自动执行', approved:'已批准', bypass:'已绕过', reject:'已拒绝'};
const kinds: Record<string,string> = {create:'创建任务',send:'发送输入',interrupt:'中断任务',native:'管理任务'};
const idOf = (r: TaskRecord) => r.threadId || r.task.threadId;
const projectOf = (r: TaskRecord) => r.task.project || r.task.directory || (idOf(r) && runtimes.value[idOf(r)!]?.project) || '';
const titleOf = (r: TaskRecord) => r.task.title || (idOf(r) && runtimes.value[idOf(r)!]?.title) || r.task.prompt.split('\n')[0]?.slice(0, 90) || kinds[r.task.kind] || '任务请求';
const time = (s: string) => new Date(s).toLocaleString('zh-CN', { hour12: false });
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
const createdTime = (s: string) => new Date(s).toLocaleString('zh-CN', { year:'2-digit', month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit', hour12:false });
function stateOf(g: (typeof groups.value)[number]) {
  const runtime = runtimes.value[g.id];
  if (runtime?.archived) return '已归档';
  if (!runtime || runtime.stale) return '';
  return runtimeLabels[runtime.runtimeStatus] || '';
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
  try { await copy(prompt); } catch { actionError.value = '复制失败，请手动选择正文复制。'; }
}
const actionError = ref('');
const openingTask = ref(false);
async function openTask(id: string) {
  if (openingTask.value) return;
  actionError.value = '';
  openingTask.value = true;
  try { await api('tasks/open', 'POST', { threadId: id }); }
  catch { if (selectedId.value === id) actionError.value = '无法打开 Codex，请确认 Codex 已启动后重试。'; }
  finally { openingTask.value = false; }
}
const selectedId = ref<string>();
const selected = computed(() => groups.value.find(g => g.id === selectedId.value));
const dialog = ref<HTMLDialogElement>();
const reducedMotion = usePreferredReducedMotion();
let source: HTMLElement | undefined;
let animations: Animation[] = [];
let closing = false;
let opening: Promise<void> = Promise.resolve();
const cards = ref<HTMLElement[]>([]);
useResizeObserver(cards, entries => {
  for (const { target } of entries) {
    const card = target as HTMLElement;
    card.parentElement!.style.gridRowEnd = `span ${Math.ceil(card.offsetHeight + 22)}`;
  }
});
function stopAnimations() { animations.forEach(a => a.cancel()); animations = []; }
async function morphCard(opening: boolean) {
  const el = dialog.value!;
  const sheet = el.querySelector<HTMLElement>('.task-focus-sheet')!;
  const origin = source!.getBoundingClientRect();
  const expanded = el.getBoundingClientRect();
  const preview = source!.cloneNode(true) as HTMLElement;
  preview.classList.remove('is-focused');
  preview.classList.add('task-morph-preview');
  preview.removeAttribute('aria-haspopup');
  preview.setAttribute('aria-hidden', 'true');
  preview.tabIndex = -1;
  preview.style.width = `${origin.width}px`;
  preview.style.height = `${origin.height}px`;
  el.append(preview);
  sheet.style.width = `${el.clientWidth}px`;
  sheet.style.height = `${el.clientHeight}px`;
  const frame = (r: DOMRect, radius: string) => ({left:`${r.left}px`, top:`${r.top}px`, width:`${r.width}px`, height:`${r.height}px`, margin:'0', borderRadius:radius});
  const compactStyle = getComputedStyle(source!);
  const expandedStyle = getComputedStyle(el);
  const compactFrame = { ...frame(origin, '22px'), borderColor:compactStyle.borderColor, boxShadow:compactStyle.boxShadow };
  const expandedFrame = { ...frame(expanded, '24px'), borderColor:expandedStyle.borderColor, boxShadow:expandedStyle.boxShadow };
  const duration = reducedMotion.value === 'reduce' ? 0 : 380;
  const options: KeyframeAnimationOptions = { duration, easing:'cubic-bezier(.22,1,.36,1)', fill:'both' };
  animations = [
    el.animate(opening ? [compactFrame, expandedFrame] : [expandedFrame, compactFrame], options),
    sheet.animate(opening ? [{opacity:0,offset:0},{opacity:0,offset:.2},{opacity:1,offset:1}] : [{opacity:1,offset:0},{opacity:0,offset:.45},{opacity:0,offset:1}], {duration,fill:'both'}),
    preview.animate(opening ? [{opacity:1,offset:0},{opacity:0,offset:.45},{opacity:0,offset:1}] : [{opacity:0,offset:0},{opacity:0,offset:.25},{opacity:1,offset:1}], {duration,fill:'both'}),
  ];
  try { await Promise.all(animations.map(a => a.finished)); } catch { /* Closing can interrupt opening. */ }
  preview.remove();
  sheet.style.width = ''; sheet.style.height = '';
}
async function focusCard(id: string, event: MouseEvent) {
  if (selectedId.value) return;
  actionError.value = '';
  source = event.currentTarget as HTMLElement;
  selectedId.value = id;
  await nextTick();
  const el = dialog.value!;
  el.showModal();
  el.querySelector<HTMLElement>('#task-focus-title')?.focus({ preventScroll: true });
  opening = morphCard(true);
  await opening;
  if (!closing) stopAnimations();
}
async function closeCard() {
  if (closing || !dialog.value) return;
  closing = true;
  await opening;
  if (!dialog.value) return;
  stopAnimations();
  dialog.value.classList.add('is-closing');
  await morphCard(false);
  dialog.value?.close();
  selectedId.value = undefined;
  stopAnimations();
  await nextTick();
  source?.focus({ preventScroll: true });
  closing = false;
}
onBeforeUnmount(() => { stopAnimations(); dialog.value?.close(); });
</script>
<template>
  <section class="tasks-page" aria-label="云端任务">
    <p v-if="error" class="status-banner warning" role="alert">{{error}}</p>
    <div class="task-grid">
      <div v-for="g in visible" :key="g.id" class="task-cell"><button ref="cards" class="task-document" :class="{'is-focused':selectedId===g.id}" aria-haspopup="dialog" @click="focusCard(g.id,$event)">
        <span class="task-document-heading"><strong class="task-document-title">{{titleOf(g.first)}}</strong><span v-if="g.first.task.model" class="task-tag">{{g.first.task.model}}</span><span v-if="g.first.task.effort" class="task-tag">{{g.first.task.effort}}</span></span>
        <time :datetime="g.first.createdAt">{{createdTime(g.first.createdAt)}}</time>
        <span v-if="g.first.task.prompt" class="task-document-preview">{{g.first.task.prompt}}</span>
        <span class="task-document-bottom"><span v-if="projectOf(g.first)" class="task-document-project" :title="projectOf(g.first)">{{projectName(g.first)}}</span><span v-if="stateOf(g)" class="task-document-state">{{stateOf(g)}}</span></span>
      </button></div>
    </div>
    <Teleport to="body">
      <dialog v-if="selected" ref="dialog" class="task-focus" aria-labelledby="task-focus-title" @cancel.prevent="closeCard" @click="($event.target===dialog) && closeCard()">
        <div class="task-focus-sheet">
          <header class="task-focus-header">
            <div><div class="task-document-heading"><h2 id="task-focus-title" tabindex="-1" autofocus>{{titleOf(selected.first)}}</h2><span v-if="selected.first.task.model" class="task-tag">{{selected.first.task.model}}</span><span v-if="selected.first.task.effort" class="task-tag">{{selected.first.task.effort}}</span></div><time class="task-document-time">{{createdTime(selected.first.createdAt)}}</time></div>
            <button class="task-focus-close" aria-label="关闭任务详情" @click="closeCard"><X aria-hidden="true"/></button>
          </header>
          <div class="task-focus-content">
          <p v-if="selected.first.executionOwner === 'connector'" class="hint">此任务由 Connector 后台执行；关闭连接会停止后台执行，不保证可在 Codex Desktop 中继续或中断。</p>
        <section v-for="r in [...selected.records].reverse()" :key="r.requestId" class="task-entry">
          <div class="task-entry-heading"><strong>{{kinds[r.task.kind] || '管理任务'}}</strong><span v-if="r.task.model" class="task-tag">{{r.task.model}}</span><span v-if="r.task.effort" class="task-tag">{{r.task.effort}}</span><time>{{time(r.createdAt)}}</time><button v-if="r.task.prompt" class="text-button" @click="copyPrompt(r.task.prompt)">{{copied?'已复制':'复制 Prompt'}}</button></div>
          <pre v-if="r.task.prompt" class="task-prompt">{{r.task.prompt}}</pre>
          <div v-if="decisions[r.approval.decision]" class="task-entry-meta"><span v-if="decisions[r.approval.decision]">{{decisions[r.approval.decision]}} · {{r.approval.source==='local'?'本机':'云端'}}</span></div>
          <p v-if="r.error?.message" class="status-banner warning">{{r.error.message}}</p>
          <div v-if="r.state==='awaiting-approval'" class="task-entry-actions"><button class="primary" :disabled="!!busy" @click="decide(r,'approve')">批准并提交</button><button :disabled="!!busy" @click="decide(r,'reject')">拒绝</button></div>
        </section>
          </div>
          <p v-if="actionError" class="status-banner warning" role="alert">{{actionError}}</p>
          <footer v-if="projectOf(selected.first) || stateOf(selected) || idOf(selected.first)" class="task-focus-footer"><span v-if="projectOf(selected.first)" class="task-document-project" :title="projectOf(selected.first)">{{projectName(selected.first)}}</span><span v-if="stateOf(selected)" class="task-document-state">{{stateOf(selected)}}</span><button v-if="idOf(selected.first) && !runtimes[selected.id]?.archived" class="text-button" :disabled="openingTask" @click="openTask(selected.id)">{{openingTask ? '正在打开…' : '在 Codex 中打开 ↗'}}</button></footer>
        </div>
      </dialog>
    </Teleport>
    <p v-if="!visible.length && !error" class="empty">暂无任务</p>
  </section>
</template>
