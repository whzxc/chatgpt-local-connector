<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import { LayoutDashboard, Logs, Settings, Pause, Play, ArrowRight, ArrowLeft, RefreshCw, Info, TriangleAlert, CircleCheck, CircleDashed, CircleAlert, LoaderCircle, Minus, Square, X } from '@lucide/vue';
import { provideConnector, api } from './composables/useConnector';
import { isDesktop, notifyNative, openUrl } from './platform';
import logo from './assets/local-connector.png';
import PlayfulMascot from './components/PlayfulMascot.vue';
import chatgptLogo from './assets/chatgpt.png';
import codexLogo from './assets/codex.png';
import { startUpdateChecks, useAppUpdate } from './composables/useAppUpdate';
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
const desktopLabel = computed(() => ({ ready: '已就绪', running: '已打开', connecting: '准备中', disconnected: '未就绪', unavailable: '不可用', error: '连接异常', unknown: '检测中' })[desktopState.value] || '检测中');
const verified = computed(() => !!status.value?.core.chatgpt?.verifiedAt);
const tunnelRunning = computed(() => status.value?.connection?.running ?? !['stopped','error'].includes(state.value));
const progressing = computed(() => ['connect','disconnect'].includes(busy.value) || ['starting','stopping'].includes(state.value));
const title = computed(() => needsConfiguration.value ? '请先配置连接。' : progressing.value ? (busy.value === 'disconnect' ? '正在关闭连接…' : '正在建立连接…') : connected.value ? (verified.value ? '已连接，从 ChatGPT 开始' : status.value?.config.connectionMode === 'https' ? '本机 MCP 已开启，等待公网验证。' : '本机连接已启动，等待 ChatGPT 验证。') : ['error','degraded'].includes(state.value) ? '连接异常，请重试。' : '连接 ChatGPT 与本机 Codex。');
const links = computed(() => {
  const pending = progressing.value;
  const failed = ['error', 'degraded'].includes(state.value);
  return [
    { ready: verified.value && state.value === 'ready', pending, failed, label: `ChatGPT — Connector：${pending ? '连接中' : failed ? '连接异常' : state.value !== 'ready' ? '未连接' : verified.value ? '已验证' : '等待验证'}` },
    { ready: connected.value, pending: pending || desktopState.value === 'connecting', failed: failed || desktopState.value === 'error' || desktopState.value === 'unavailable', label: `Connector — Codex：${pending ? '连接中' : state.value !== 'ready' ? '未连接' : desktopLabel.value}` },
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
watch(() => status.value?.core.appServer.state, (n,old) => { if(old && n==='error' && n!==old) void notifyNative('本机连接异常','请在设置中查看诊断').catch(()=>{}); });
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
  return desktopState.value === 'unavailable' ? '未找到 Codex，请先安装并打开应用。' : 'Codex 尚未就绪，请确认应用已打开后重试。';
});
async function reconnect() {
  await run('connect', async () => { await api('stop', 'POST'); await api('start', 'POST'); await refresh(); });
}


</script>
<template>
  <div class="app-scene" :class="{desktop:isDesktop,mac}">
    <header class="app-header" data-tauri-drag-region>
      <button v-if="page==='overview'||page==='guide'" class="brand ghost" @click="navigate('overview')" aria-label="首页"><img :src="logo" alt=""/> <span>Local Connector</span></button>
      <div v-else class="header-page-title"><button class="ghost" aria-label="返回首页" @click="navigate('overview')"><ArrowLeft aria-hidden="true"/></button><h1>{{page==='settings'?'设置':page==='tasks'?'任务':'记录'}}</h1></div>
      <nav class="top-nav" aria-label="主导航">
        <button class="tasks-nav" :class="{active:page==='tasks'}" :aria-pressed="page==='tasks'" aria-label="任务" title="任务" @click="navigate('tasks')"><LayoutDashboard aria-hidden="true"/><span v-if="tasks.pending.value.length" class="task-count">{{tasks.pending.value.length}}</span></button>
        <button :class="{active:page==='logs'}" :aria-pressed="page==='logs'" aria-label="记录" title="记录" @click="navigate('logs')"><Logs aria-hidden="true" /></button>
        <button :class="{active:page==='settings'}" :aria-pressed="page==='settings'" aria-label="设置" title="设置" @click="navigate('settings')"><Settings aria-hidden="true" /></button>
      </nav>
      <div v-if="isDesktop&&!mac" class="window-controls"><button aria-label="最小化" @click="windowAction('minimize')"><Minus /></button><button aria-label="最大化或还原" @click="windowAction('toggleMaximize')"><Square /></button><button aria-label="关闭窗口" @click="windowAction('close')"><X /></button></div>
    </header>
    <main ref="workspace" class="workspace" :class="{'records-workspace':page==='logs'}">
      <div v-if="appUpdate.available.value && page!=='settings'" class="status-banner" role="status"><Info aria-hidden="true"/><span>Local Connector {{appUpdate.update.value?.version}} 可更新</span><button class="text-button" @click="navigate('settings')">查看更新</button></div>
      <div v-if="feedback.text && feedback.error" class="status-banner warning" role="alert"><TriangleAlert aria-hidden="true"/><span>{{feedback.text}}</span><button class="ghost banner-dismiss" aria-label="关闭提示" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
      <template v-if="page==='guide'"><ChatGuide @done="navigate('overview')"/></template>
      <template v-else-if="page==='overview'">
          <div v-if="status?.connection?.updateAvailable" class="status-banner" role="status"><Info aria-hidden="true"/><span>连接组件有更新</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> 更新连接</button></div>
          <div v-if="needsReconnect && !progressing && !needsConfiguration" class="status-banner warning" role="status"><TriangleAlert aria-hidden="true"/><span>{{connectionIssue}}</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> 重新连接</button></div>
          <section class="connection-panel" :class="{connected,progressing}">
            <div class="connection-content"><h2>{{title}}</h2>
              <p v-if="needsConfiguration">前往设置，填写连接信息后即可开启连接。</p>
              <button v-if="needsConfiguration" class="primary connection-action" @click="navigate('settings')"><Settings aria-hidden="true" />前往设置</button>
              <button v-else class="primary connection-action" :disabled="progressing || loading || !status" @click="toggle"><Pause v-if="tunnelRunning" aria-hidden="true" /><Play v-else aria-hidden="true" />{{progressing?'请稍候…':tunnelRunning?'关闭连接':'开启连接'}}</button>
            </div>
            <div class="connection-art"><PlayfulMascot/><span class="art-orbit" aria-hidden="true"></span><span class="art-orbit second" aria-hidden="true"></span></div>
            <section class="connection-path" aria-label="连接状态">
              <div class="path-node" :class="{ready:verified}"><a class="path-shortcut" href="https://chatgpt.com/" target="_blank" rel="noopener noreferrer" aria-label="打开 ChatGPT 网页" title="打开 ChatGPT 网页" @click="isDesktop && ($event.preventDefault(), run('chatgpt-open', () => openUrl('https://chatgpt.com/')))"><img class="product-logo" :src="chatgptLogo" alt=""/><strong>ChatGPT</strong></a><button v-if="!verified && !needsConfiguration && status" class="text-button" @click="navigate('guide')">接入引导 <ArrowRight aria-hidden="true"/></button></div>
              <span class="path-link" :class="{ready:links[0].ready,pending:links[0].pending,failed:links[0].failed}" role="img" :aria-label="links[0].label" :title="links[0].label"><component :is="linkIcon(links[0])" aria-hidden="true"/></span>
              <div class="path-node" :class="{ready:state==='ready'}"><img class="product-logo connector-logo" :src="logo" alt=""/><strong>Connector</strong></div>
              <span class="path-link" :class="{ready:links[1].ready,pending:links[1].pending,failed:links[1].failed}" role="img" :aria-label="links[1].label" :title="links[1].label"><component :is="linkIcon(links[1])" aria-hidden="true"/></span>
              <div class="path-node" :class="{ready:desktopState==='ready'}"><button type="button" class="path-shortcut" aria-label="打开 Codex Desktop" title="打开 Codex Desktop" :disabled="!!busy" @click="run('codex-open', async () => { await api('codex/login', 'POST'); })"><img class="product-logo" :src="codexLogo" alt=""/><strong>Codex</strong></button></div>
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
    <div v-if="feedback.text && !feedback.error" class="message" role="status">{{feedback.text}}<button aria-label="关闭提示" @click="feedback.text=''"><X aria-hidden="true" /></button></div>
  </div>
</template>
