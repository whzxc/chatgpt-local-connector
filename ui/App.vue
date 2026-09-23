<script setup lang="ts">
import { provideAgentActivity } from './composables/useAgentActivity';
import { useSubscriptions, subscriptionSnapshotKey } from './subscriptions/useSubscriptions';
import { requestedAgentSettings, requestedProvider } from './subscriptions/navigation';
import { displayMessage } from './messages';
import { t } from './i18n';
import { computed, defineAsyncComponent, provide, ref, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { LayoutDashboard, Logs, Settings, Download, X, Maximize2, Minimize2 } from '@lucide/vue';
import { NButton } from 'naive-ui';
import { provideConnector } from './composables/useConnector';
import { isDesktop, notifyNative } from './platform';
import { startUpdateChecks, useAppUpdate } from './composables/useAppUpdate';
import AppUpdateDialog from './components/AppUpdateDialog.vue';
import AgentDisplaySettings from './components/AgentDisplaySettings.vue';
import { panelPreferences } from './usage-rail/preferences';
import type { PanelPreferences } from './usage-rail/layout';
import { panelLayers } from './composables/panels';
provide(subscriptionSnapshotKey, useSubscriptions().snapshot);
provideAgentActivity();
const appUpdate = useAppUpdate();
let stopUpdateChecks = () => {};
onMounted(() => { stopUpdateChecks = startUpdateChecks(); });
onUnmounted(() => stopUpdateChecks());
import TasksPage from './components/TasksPage.vue';
import { useTasks } from './composables/useTasks';
const tasks = useTasks();
import RecordsPage from './components/RecordsPage.vue';
const SettingsPage = defineAsyncComponent(() => import('./components/SettingsPage.vue'));
import ChatGuide from './components/ChatGuide.vue';
import ConnectionOverview from './components/ConnectionOverview.vue';
const UsageRailPreview = import.meta.env.DEV ? defineAsyncComponent(() => import('./usage-rail/BrowserRailPreview.vue')) : null;
const { status, feedback, notify } = provideConnector();
type Page = 'guide' | 'overview' | 'logs' | 'settings' | 'tasks';
const page = ref<Page>('overview');
const panelOpen = computed(() => panelLayers.value.length > 0);
const sheetWide = ref(false);
const workspace = ref<HTMLElement>();
watch(page, () => workspace.value?.scrollTo({ top: 0 }));
const mac = isDesktop && navigator.platform.toLowerCase().includes('mac');
const browserRailPreview = import.meta.env.DEV && !isDesktop;
const lastPage = ref<Page>('tasks');
async function navigate(next: Page) {
  if(next !== 'overview') { lastPage.value = next; sheetWide.value = false; }
  page.value = next;
  await nextTick();
  if(next === 'overview') document.querySelector<HTMLButtonElement>(`[data-page="${lastPage.value}"]`)?.focus();
  else document.querySelector<HTMLElement>('.capsule-title')?.focus();
}
const sheetMotions = new WeakMap<Element, Animation>();
function cancelSheetMotion(element: Element) {
  sheetMotions.get(element)?.cancel();
  sheetMotions.delete(element);
  (element as HTMLElement).style.willChange = '';
}
function animateSheet(element: Element, done: () => void, opening: boolean) {
  cancelSheetMotion(element);
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) { done(); return; }
  const node = element as HTMLElement;
  const dock = document.querySelector<HTMLElement>('.navigation-surface')!.getBoundingClientRect();
  const rect = node.getBoundingClientRect();
  const collapsed = { transform: `translate(${dock.x-rect.x}px,${dock.y-rect.y}px) scale(${dock.width/rect.width},${dock.height/rect.height})`, opacity: 0 };
  const expanded = { transform: 'translate(0px,0px) scale(1,1)', opacity: 1 };
  node.style.willChange = 'transform, opacity';
  const motion = node.animate(opening ? [collapsed, expanded] : [expanded, collapsed], {
    duration: opening ? 480 : 220, easing: opening ? 'cubic-bezier(.22,1,.36,1)' : 'cubic-bezier(.4,0,.6,1)', fill: 'both',
  });
  sheetMotions.set(element, motion);
  void motion.finished.then(() => {
    if (sheetMotions.get(element) !== motion) return;
    cancelSheetMotion(element);
    done();
  }, () => {});
}
const enterSheet = (element: Element, done: () => void) => animateSheet(element, done, true);
const leaveSheet = (element: Element, done: () => void) => animateSheet(element, done, false);
function closePage(event: KeyboardEvent) {
  if(event.key === 'Escape' && page.value !== 'overview' && !panelOpen.value) {
    event.preventDefault(); void navigate('overview');
  }
}
const unlisteners: (() => void)[] = [];
onMounted(async () => {
  if (!isDesktop) return;
  const { listen } = await import('@tauri-apps/api/event');
  unlisteners.push(await listen<string>('navigate', event => { if (['overview','settings','logs','tasks'].includes(event.payload)) navigate(event.payload as Page); }));
  unlisteners.push(await listen<PanelPreferences>('usage-panel:preferences', event => { panelPreferences.value=event.payload; }));
  unlisteners.push(await listen('agents:settings', () => { requestedAgentSettings.value=true; }));
  unlisteners.push(await listen<string>('subscriptions:open', event => { requestedProvider.value=event.payload; navigate('overview'); }));
  unlisteners.push(await listen<string>('connection-error', event => notify(event.payload, true)));
});
onUnmounted(() => unlisteners.forEach(stop => stop()));
watch(() => status.value?.core.appServer.state, (n,old) => { if(old && n==='error' && n!==old) void notifyNative(t('localConnectionError'),t('checkDiagnosticsInSettings')).catch(()=>{}); });
let toastTimer: ReturnType<typeof setTimeout>;
watch(() => feedback.value.text, text => { clearTimeout(toastTimer); if(text && !feedback.value.error) toastTimer=setTimeout(()=>feedback.value.text='',4500); });
onUnmounted(()=>clearTimeout(toastTimer));
</script>
<template>
  <div class="app-scene" :class="{desktop:isDesktop,mac,'sheet-wide':sheetWide}" @keydown="closePage">
    <div v-if="isDesktop" class="window-drag-strip" data-tauri-drag-region/>
    <main class="home-workspace" :inert="page!=='overview' || panelOpen">
      <ConnectionOverview/>
    </main>
    <div class="navigation-surface" :class="{expanded:page!=='overview','has-update':appUpdate.visible.value}" aria-hidden="true"/>
    <Transition :css="false" @enter="enterSheet" @leave="leaveSheet" @enter-cancelled="cancelSheetMotion" @leave-cancelled="cancelSheetMotion">
      <section v-if="page!=='overview'" class="page-sheet" :inert="panelOpen" :aria-label="page==='settings'?t('settings'):page==='tasks'?t('tasks'):t('records')">
        <main ref="workspace" class="workspace" :class="{'records-workspace':page==='logs'}">
          <ChatGuide v-if="page==='guide'" @done="navigate('overview')" @settings="navigate('settings')"/>
          <TasksPage v-else-if="page==='tasks'" :records="tasks.records.value" :error="tasks.error.value" @refresh="tasks.refresh"/>
          <RecordsPage v-else-if="page==='logs'"/>
          <SettingsPage v-else-if="status"/>
        </main>
        <NButton class="panel-width-toggle" quaternary circle :aria-label="t(sheetWide ? 'restorePanelWidth' : 'expandPanelWidth')" :title="t(sheetWide ? 'restorePanelWidth' : 'expandPanelWidth')" :aria-pressed="sheetWide" @click="sheetWide=!sheetWide"><template #icon><component :is="sheetWide ? Minimize2 : Maximize2" :size="20"/></template></NButton>
      </section>
    </Transition>
    <nav :inert="panelOpen" class="navigation-capsule" :class="{expanded:page!=='overview','has-update':appUpdate.visible.value}" :aria-label="t('mainNavigation')">
      <template v-if="page!=='overview'"><button class="capsule-close" :aria-label="t('backToHome')" :title="t('backToHome')" @click="navigate('overview')"><X aria-hidden="true"/></button><h1 class="capsule-title" tabindex="-1">{{page==='settings'?t('settings'):page==='tasks'?t('tasks'):page==='logs'?t('records'):t('home')}}</h1></template>
      <div class="capsule-icons" :inert="page!=='overview'" :aria-hidden="page!=='overview'">
        <NButton v-if="appUpdate.visible.value" class="update-shortcut" quaternary circle type="success" :loading="appUpdate.active.value" :disabled="appUpdate.active.value || appUpdate.checking.value" :aria-label="t('updateToValue', { version: appUpdate.update.value?.version })" :title="t('updateToValue', { version: appUpdate.update.value?.version })" @click="appUpdate.installDirect()"><template #icon><Download aria-hidden="true"/></template></NButton>
        <button data-page="tasks" class="tasks-nav" :aria-label="t('tasks')" :title="t('tasks')" @click="navigate('tasks')"><LayoutDashboard aria-hidden="true"/><span v-if="tasks.pending.value.length" class="task-count">{{tasks.pending.value.length}}</span></button>
        <button data-page="logs" :aria-label="t('records')" :title="t('records')" @click="navigate('logs')"><Logs aria-hidden="true"/></button>
        <button data-page="settings" :aria-label="t('settings')" :title="t('settings')" @click="navigate('settings')"><Settings aria-hidden="true"/></button>
      </div>
    </nav>
    <div v-if="feedback.text && feedback.error" class="status-banner warning app-feedback" role="alert"><span>{{displayMessage(feedback.text)}}</span><button class="ghost banner-dismiss" :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
    <AppUpdateDialog/>
    <AgentDisplaySettings v-if="requestedAgentSettings" :origin="{x:0,y:0,size:28}" @close="requestedAgentSettings=false"/>
    <UsageRailPreview v-if="browserRailPreview"/>
    <div v-if="feedback.text && !feedback.error" class="message" role="status">{{displayMessage(feedback.text)}}<button :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
  </div>
</template>
