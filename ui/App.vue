<script setup lang="ts">
import { displayMessage } from './messages';
import { t } from './i18n';
import { defineAsyncComponent, ref, watch, nextTick, onMounted, onUnmounted } from 'vue';
import { LayoutDashboard, Logs, Settings, Download, Minus, Square, X, Maximize2, Minimize2 } from '@lucide/vue';
import { NButton } from 'naive-ui';
import { provideConnector } from './composables/useConnector';
import { isDesktop, notifyNative } from './platform';
import { startUpdateChecks, useAppUpdate } from './composables/useAppUpdate';
const AppUpdateDialog = defineAsyncComponent(() => import('./components/AppUpdateDialog.vue'));
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
const { status, feedback, notify } = provideConnector();
type Page = 'guide' | 'overview' | 'logs' | 'settings' | 'tasks';
const page = ref<Page>('overview');
const agentsOpen = ref(false);
const sheetWide = ref(false);
const workspace = ref<HTMLElement>();
watch(page, () => workspace.value?.scrollTo({ top: 0 }));
const mac = isDesktop && navigator.platform.toLowerCase().includes('mac');
const lastPage = ref<Page>('tasks');
async function navigate(next: Page) {
  if(next !== 'overview') { lastPage.value = next; sheetWide.value = false; }
  page.value = next;
  await nextTick();
  if(next === 'overview') document.querySelector<HTMLButtonElement>(`[data-page="${lastPage.value}"]`)?.focus();
  else document.querySelector<HTMLElement>('.capsule-title')?.focus();
}
function closePage(event: KeyboardEvent) {
  if(event.key === 'Escape' && page.value !== 'overview' && !document.querySelector('.n-modal, dialog[open]')) {
    event.preventDefault(); void navigate('overview');
  }
}
const unlisteners: (() => void)[] = [];
onMounted(async () => {
  if (!isDesktop) return;
  const { listen } = await import('@tauri-apps/api/event');
  unlisteners.push(await listen<string>('navigate', event => { if (['overview','settings','logs','tasks'].includes(event.payload)) navigate(event.payload as Page); }));
  unlisteners.push(await listen<string>('connection-error', event => notify(event.payload, true)));
});
onUnmounted(() => unlisteners.forEach(stop => stop()));
async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close') {
  const { getCurrentWindow } = await import('@tauri-apps/api/window'); await getCurrentWindow()[action]();
}
watch(() => status.value?.core.appServer.state, (n,old) => { if(old && n==='error' && n!==old) void notifyNative(t('localConnectionError'),t('checkDiagnosticsInSettings')).catch(()=>{}); });
let toastTimer: ReturnType<typeof setTimeout>;
watch(() => feedback.value.text, text => { clearTimeout(toastTimer); if(text && !feedback.value.error) toastTimer=setTimeout(()=>feedback.value.text='',4500); });
onUnmounted(()=>clearTimeout(toastTimer));
</script>
<template>
  <div class="app-scene" :class="{desktop:isDesktop,mac,'sheet-wide':sheetWide}" @keydown="closePage">
    <div v-if="isDesktop" class="window-drag-strip" data-tauri-drag-region>
      <div v-if="!mac" class="window-controls"><button :aria-label="t('minimize')" @click="windowAction('minimize')"><Minus /></button><button :aria-label="t('maximizeOrRestore')" @click="windowAction('toggleMaximize')"><Square /></button><button :aria-label="t('closeWindow')" @click="windowAction('close')"><X /></button></div>
    </div>
    <main class="home-workspace" :inert="page!=='overview' || agentsOpen">
      <ConnectionOverview @panel="agentsOpen=$event"/>
    </main>
    <div class="navigation-surface" :class="{expanded:page!=='overview'}" aria-hidden="true"/>
    <Transition name="capsule-page">
      <section v-if="page!=='overview'" class="page-sheet" :aria-label="page==='settings'?t('settings'):page==='tasks'?t('tasks'):t('records')">
        <main ref="workspace" class="workspace" :class="{'records-workspace':page==='logs'}">
          <ChatGuide v-if="page==='guide'" @done="navigate('overview')" @settings="navigate('settings')"/>
          <TasksPage v-else-if="page==='tasks'" :records="tasks.records.value" :error="tasks.error.value" @refresh="tasks.refresh"/>
          <RecordsPage v-else-if="page==='logs'"/>
          <SettingsPage v-else-if="status"/>
        </main>
        <NButton class="panel-width-toggle" quaternary circle :aria-label="t(sheetWide ? 'restorePanelWidth' : 'expandPanelWidth')" :title="t(sheetWide ? 'restorePanelWidth' : 'expandPanelWidth')" :aria-pressed="sheetWide" @click="sheetWide=!sheetWide"><template #icon><component :is="sheetWide ? Minimize2 : Maximize2" :size="20"/></template></NButton>
      </section>
    </Transition>
    <nav :inert="agentsOpen" class="navigation-capsule" :class="{expanded:page!=='overview'}" :aria-label="t('mainNavigation')">
      <template v-if="page!=='overview'"><button class="capsule-close" :aria-label="t('backToHome')" :title="t('backToHome')" @click="navigate('overview')"><X aria-hidden="true"/></button><h1 class="capsule-title" tabindex="-1">{{page==='settings'?t('settings'):page==='tasks'?t('tasks'):page==='logs'?t('records'):t('home')}}</h1></template>
      <div class="capsule-icons" :inert="page!=='overview'" :aria-hidden="page!=='overview'">
        <button data-page="tasks" class="tasks-nav" :aria-label="t('tasks')" :title="t('tasks')" @click="navigate('tasks')"><LayoutDashboard aria-hidden="true"/><span v-if="tasks.pending.value.length" class="task-count">{{tasks.pending.value.length}}</span></button>
        <button data-page="logs" :aria-label="t('records')" :title="t('records')" @click="navigate('logs')"><Logs aria-hidden="true"/></button>
        <button data-page="settings" :aria-label="t('settings')" :title="t('settings')" @click="navigate('settings')"><Settings aria-hidden="true"/></button>
      </div>
    </nav>
    <button v-if="appUpdate.available.value" class="update-shortcut" :aria-label="t('updateToValue', { version: appUpdate.update.value?.version })" :title="t('newVersionValueViewUpdate', { version: appUpdate.update.value?.version })" aria-haspopup="dialog" @click="appUpdate.showUpdate"><Download aria-hidden="true"/></button>
    <div v-if="feedback.text && feedback.error" class="status-banner warning app-feedback" role="alert"><span>{{displayMessage(feedback.text)}}</span><button class="ghost banner-dismiss" :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
    <AppUpdateDialog v-if="appUpdate.dialogOpen.value"/>
    <div v-if="feedback.text && !feedback.error" class="message" role="status">{{displayMessage(feedback.text)}}<button :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
  </div>
</template>
