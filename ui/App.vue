<script setup lang="ts">
import { displayMessage } from './messages';
import { t } from './i18n';
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import { LayoutDashboard, Logs, Settings, Power, ArrowLeft, RefreshCw, Download, Info, TriangleAlert, CircleCheck, CircleDashed, CircleAlert, LoaderCircle, Minus, Square, X } from '@lucide/vue';
import { provideConnector, api } from './composables/useConnector';
import { isDesktop, notifyNative, openUrl } from './platform';
import logo from './assets/local-connector.png';
import PlayfulMascot from './components/PlayfulMascot.vue';
import chatgptLogo from './assets/chatgpt.png';
import codexLogo from './assets/codex.png';
import { startUpdateChecks, useAppUpdate } from './composables/useAppUpdate';
import AppUpdateDialog from './components/AppUpdateDialog.vue';
const appUpdate = useAppUpdate();
let stopUpdateChecks = () => {};
onMounted(() => { stopUpdateChecks = startUpdateChecks(); });
onUnmounted(() => stopUpdateChecks());
import TasksPage from './components/TasksPage.vue';
import { useTasks } from './composables/useTasks';
const tasks = useTasks();
import RecordsPage from './components/RecordsPage.vue';
import SettingsPage from './components/SettingsPage.vue';
import ChatGuide from './components/ChatGuide.vue';
const { status, busy, loading, connectionError, feedback, run, refresh, notify } = provideConnector();
type Page = 'guide' | 'overview' | 'logs' | 'settings' | 'tasks';
const page = ref<Page>('overview');
const workspace = ref<HTMLElement>();
watch(page, () => workspace.value?.scrollTo({ top: 0 }));
const mac = isDesktop && navigator.platform.toLowerCase().includes('mac');
const needsConfiguration = computed(() => {
  const config = status.value?.config;
  if (!config) return false;
  return config.connectionMode === 'https'
    ? (config.httpsProvider === 'ngrok' ? !config.hasNgrokAuthtoken : config.httpsProvider === 'cloudflare' ? config.cloudflareMode === 'named' && (!config.hasCloudflareToken || !config.httpsUrl) : !config.httpsUrl)
    : !config.tunnelId || !config.hasApiKey;
});
const state = computed(() => status.value?.tunnel.state || 'stopped');
const connected = computed(() => state.value === 'ready' && (status.value?.autoOpenCodex === false ? status.value?.core.appServer?.state === 'ready' : status.value?.core.desktop?.state === 'ready'));
const desktopState = computed(() => (status.value?.autoOpenCodex === false ? status.value?.core.appServer?.state : status.value?.core.desktop?.state) || 'unknown');
const desktopLabel = computed(() => ({ ready: t('ready'), running: t('open'), connecting: t('preparing'), disconnected: t('notReady'), unavailable: t('unavailable'), error: t('connectionError'), unknown: t('checking') })[desktopState.value] || t('checking'));
const verified = computed(() => !!status.value?.core.chatgpt?.verifiedAt);
const tunnelRunning = computed(() => status.value?.connection?.running ?? !['stopped','error'].includes(state.value));
const progressing = computed(() => ['connect','disconnect'].includes(busy.value) || ['starting','stopping'].includes(state.value));
const title = computed(() => needsConfiguration.value ? t('configureTheConnectionFirst') : progressing.value ? (busy.value === 'disconnect' ? t('disconnecting') : t('connecting')) : connected.value ? (verified.value ? t('connectedStartInChatgpt') : status.value?.config.connectionMode === 'https' ? t('localMcpIsRunningAwaitingPublicEndpointVerification') : t('localConnectionStartedAwaitingChatgptVerification')) : ['error','degraded'].includes(state.value) ? t('connectionErrorPleaseRetry') : t('connectChatgptToCodexOnThisComputer'));
const links = computed(() => {
  const pending = progressing.value;
  const failed = ['error', 'degraded'].includes(state.value);
  return [
    { ready: verified.value && state.value === 'ready', pending, failed, label: t('chatgptConnectorValue', { status: pending ? t('connectingLabel') : failed ? t('connectionError') : state.value !== 'ready' ? t('disconnected') : verified.value ? t('verified') : t('awaitingVerification') }) },
    { ready: connected.value, pending: pending || desktopState.value === 'connecting', failed: failed || desktopState.value === 'error' || desktopState.value === 'unavailable', label: t('connectorCodexValue', { status: pending ? t('connectingLabel') : state.value !== 'ready' ? t('disconnected') : desktopLabel.value }) },
  ];
});
function linkIcon(link: {ready: boolean; pending: boolean; failed: boolean}) {
  return link.pending ? LoaderCircle : link.failed ? CircleAlert : link.ready ? CircleCheck : CircleDashed;
}
function navigate(next: Page) { page.value = next; }
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
async function toggle() {
  if (needsConfiguration.value) { navigate('settings'); return; }
  const stop=tunnelRunning.value;
  await run(stop?'disconnect':'connect', async () => {
    await api(stop?'stop':'start','POST'); await refresh();
  });
}
const needsReconnect = computed(() => !!connectionError.value || ['error','degraded'].includes(state.value) || (tunnelRunning.value && desktopState.value !== 'ready'));
const connectionIssue = computed(() => {
  if (connectionError.value) return connectionError.value;
  if (status.value?.tunnel.error) return status.value.tunnel.error;
  // Present current product states rather than backend implementation details.
  return desktopState.value === 'unavailable' ? t('codexWasNotFoundInstallAndOpenIt') : t('codexIsNotReadyMakeSureTheApp');
});
async function reconnect() {
  await run('connect', async () => { await api('stop', 'POST'); await api('start', 'POST'); await refresh(); });
}


</script>
<template>
  <div class="app-scene" :class="{desktop:isDesktop,mac}">
    <header class="app-header" data-tauri-drag-region>
      <button v-if="page==='overview'||page==='guide'" class="brand ghost" @click="navigate('overview')" :aria-label="t('home')"><img :src="logo" alt=""/> <span>Local Connector</span></button>
      <div v-else class="header-page-title"><button class="ghost" :aria-label="t('backToHome')" @click="navigate('overview')"><ArrowLeft aria-hidden="true"/></button><h1>{{page==='settings'?t('settings'):page==='tasks'?t('tasks'):t('records')}}</h1></div>
      <nav class="top-nav" :aria-label="t('mainNavigation')">
        <button v-if="appUpdate.available.value" class="active" :aria-label="t('updateToValue', { version: appUpdate.update.value?.version })" :title="t('newVersionValueViewUpdate', { version: appUpdate.update.value?.version })" aria-haspopup="dialog" @click="appUpdate.showUpdate"><Download aria-hidden="true"/></button>
        <button class="tasks-nav" :class="{active:page==='tasks'}" :aria-pressed="page==='tasks'" :aria-label="t('tasks')" :title="t('tasks')" @click="navigate('tasks')"><LayoutDashboard aria-hidden="true"/><span v-if="tasks.pending.value.length" class="task-count">{{tasks.pending.value.length}}</span></button>
        <button :class="{active:page==='logs'}" :aria-pressed="page==='logs'" :aria-label="t('records')" :title="t('records')" @click="navigate('logs')"><Logs aria-hidden="true" /></button>
        <button :class="{active:page==='settings'}" :aria-pressed="page==='settings'" :aria-label="t('settings')" :title="t('settings')" @click="navigate('settings')"><Settings aria-hidden="true" /></button>
      </nav>
      <div v-if="isDesktop&&!mac" class="window-controls"><button :aria-label="t('minimize')" @click="windowAction('minimize')"><Minus /></button><button :aria-label="t('maximizeOrRestore')" @click="windowAction('toggleMaximize')"><Square /></button><button :aria-label="t('closeWindow')" @click="windowAction('close')"><X /></button></div>
    </header>
    <main ref="workspace" class="workspace" :class="{'records-workspace':page==='logs'}">
      <div v-if="feedback.text && feedback.error" class="status-banner warning" role="alert"><TriangleAlert aria-hidden="true"/><span>{{displayMessage(feedback.text)}}</span><button class="ghost banner-dismiss" :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
      <template v-if="page==='guide'"><ChatGuide @done="navigate('overview')" @settings="navigate('settings')"/></template>
      <template v-else-if="page==='overview'">
          <div v-if="status?.connection?.updateAvailable" class="status-banner" role="status"><Info aria-hidden="true"/><span>{{ t('connectionComponentUpdateAvailable') }}</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> {{ t('updateConnection') }}</button></div>
          <div v-if="needsReconnect && !progressing && !needsConfiguration" class="status-banner warning" role="status"><TriangleAlert aria-hidden="true"/><span>{{displayMessage(connectionIssue)}}</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> {{ t('reconnect') }}</button></div>
          <section class="connection-panel" :class="{connected,progressing}">
            <div class="connection-content"><h2>{{title}}</h2>
              <p v-if="needsConfiguration">{{ t('goToSettingsAndEnterYourConnectionDetails') }}</p>
              <button v-if="needsConfiguration" class="connection-action" @click="navigate('settings')"><span class="connection-knob"><Settings aria-hidden="true"/></span><span class="connection-action-label">{{ t('goToSettings') }}</span></button>
              <button v-else class="connection-action" :class="{'is-on':tunnelRunning,'is-busy':progressing}" :aria-label="progressing?t('switchingConnection'):tunnelRunning?t('disconnect'):t('connect')" :aria-busy="progressing" :disabled="!!busy || progressing || loading || !status" @click="toggle">
                <span class="connection-switch-track" aria-hidden="true"><span class="connection-knob"><LoaderCircle v-if="progressing"/><Power v-else/></span></span>
                <span class="connection-action-label">{{progressing?t('pleaseWait'):tunnelRunning?t('disconnect'):t('connect')}}</span>
                <span class="connection-action-light" aria-hidden="true"></span>
              </button>
            </div>
            <div class="connection-art"><PlayfulMascot/><span class="art-orbit" aria-hidden="true"></span><span class="art-orbit second" aria-hidden="true"></span></div>
            <section class="connection-path" :aria-label="t('connectionStatus')">
              <div class="path-node" :class="{ready:verified}"><a class="path-shortcut" href="https://chatgpt.com/" target="_blank" rel="noopener noreferrer" :aria-label="t('openChatgptWebsite')" :title="t('openChatgptWebsite')" @click="isDesktop && ($event.preventDefault(), run('chatgpt-open', () => openUrl('https://chatgpt.com/')))"><img class="product-logo" :src="chatgptLogo" alt=""/><strong>ChatGPT</strong></a></div>
              <button class="path-link path-link-action" :class="{ready:links[0].ready,pending:links[0].pending,failed:links[0].failed}" :aria-label="t('valueOpenSetupGuide', { status: links[0].label })" :title="t('valueViewSetupAndVerification', { status: links[0].label })" @click="navigate('guide')"><component :is="linkIcon(links[0])" aria-hidden="true"/><span v-if="!links[0].ready" class="path-link-caption">{{links[0].pending ? t('connectingLabel') : links[0].failed ? t('connectionError') : state==='ready' ? t('unverified') : t('disconnected')}}</span></button>
              <div class="path-node" :class="{ready:state==='ready'}"><img class="product-logo connector-logo" :src="logo" alt=""/><strong>Connector</strong></div>
              <span class="path-link" :class="{ready:links[1].ready,pending:links[1].pending,failed:links[1].failed}" role="img" :aria-label="links[1].label" :title="links[1].label"><component :is="linkIcon(links[1])" aria-hidden="true"/></span>
              <div class="path-node" :class="{ready:desktopState==='ready'}"><button type="button" class="path-shortcut" :aria-label="t('openCodexDesktop')" :title="t('openCodexDesktop')" :disabled="!!busy" @click="run('codex-open', async () => { await api('codex/login', 'POST'); })"><img class="product-logo" :src="codexLogo" alt=""/><strong>Codex</strong></button></div>
            </section>
          </section>


      </template>
      <template v-else-if="page==='tasks'"><TasksPage :records="tasks.records.value" :error="tasks.error.value" @refresh="tasks.refresh"/></template>
      <template v-else-if="page==='logs'">
        <RecordsPage/>
      </template>
      <template v-else>
        <SettingsPage v-if="status"/>
      </template>
    </main>
    <AppUpdateDialog/>
    <div v-if="feedback.text && !feedback.error" class="message" role="status">{{displayMessage(feedback.text)}}<button :aria-label="t('dismissMessage')" @click="feedback.text=''"><X aria-hidden="true" /></button></div>
  </div>
</template>
