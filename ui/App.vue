<script setup lang="ts">
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import { Logs, Settings, Pause, Play, ArrowRight, ArrowLeft, RefreshCw, Info, TriangleAlert, CircleCheck, CircleDashed, CircleAlert, LoaderCircle, Minus, Square, X } from '@lucide/vue';
import { provideConnector, api } from './composables/useConnector';
import { isDesktop, isDevelopment, notifyNative } from './platform';
import logo from './assets/local-connector.png';
import chatgptLogo from './assets/chatgpt.png';
import codexLogo from './assets/codex.png';
import SetupWizard from './components/SetupWizard.vue';
import { startUpdateChecks, useAppUpdate } from './composables/useAppUpdate';
const appUpdate = useAppUpdate();
let stopUpdateChecks = () => {};
onMounted(() => { stopUpdateChecks = startUpdateChecks(); });
onUnmounted(() => stopUpdateChecks());
import RecordsPage from './components/RecordsPage.vue';
import SettingsPage from './components/SettingsPage.vue';
import ChatGuide from './components/ChatGuide.vue';
const { status, busy, loading, connectionError, feedback, run, refresh, notify } = provideConnector();
type Page = 'guide' | 'overview' | 'logs' | 'settings';
const page = ref<Page>('overview');
const workspace = ref<HTMLElement>();
watch(page, () => workspace.value?.scrollTo({ top: 0 }));
const mac = isDesktop && navigator.platform.toLowerCase().includes('mac');
const setup = computed(() => !!status.value && (!status.value.config.tunnelId || !status.value.config.hasApiKey));
const state = computed(() => status.value?.tunnel.state || 'stopped');
const connected = computed(() => state.value === 'ready' && status.value?.core.desktop?.state === 'ready');
const desktopState = computed(() => status.value?.core.desktop?.state || 'unknown');
const desktopLabel = computed(() => ({ ready: '已就绪', running: '已打开', connecting: '准备中', disconnected: '未就绪', unavailable: '不可用', error: '连接异常', unknown: '检测中' })[desktopState.value] || '检测中');
const verified = computed(() => !!status.value?.core.chatgpt?.verifiedAt);
const tunnelRunning = computed(() => status.value?.connection?.running ?? !['stopped','error'].includes(state.value));
const progressing = computed(() => ['connect','disconnect'].includes(busy.value) || ['starting','stopping'].includes(state.value));
const title = computed(() => progressing.value ? (busy.value === 'disconnect' ? '正在关闭连接…' : '正在建立连接…') : connected.value ? (verified.value ? '已连接，从 ChatGPT 开始。' : '通道已就绪，等待 ChatGPT 接入。') : ['error','degraded'].includes(state.value) ? '连接异常，请重试。' : '连接 ChatGPT 与本机 Codex。');
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
function navigate(next: Page) { page.value = next === 'overview' && setup.value ? 'guide' : next; }
watch(setup, needed => { if (needed) page.value = 'guide'; else if (page.value === 'guide') page.value = 'overview'; });
const unlisteners: (() => void)[] = [];
onMounted(async () => {
  if (!isDesktop) return;
  const { listen } = await import('@tauri-apps/api/event');
  unlisteners.push(await listen<string>('navigate', event => { if (['overview','settings','logs'].includes(event.payload)) navigate(event.payload as Page); }));
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
      <div v-else class="header-page-title"><button class="ghost" aria-label="返回首页" @click="navigate('overview')"><ArrowLeft aria-hidden="true"/></button><h1>{{page==='settings'?'设置':'记录'}}</h1></div>
      <nav class="top-nav" aria-label="主导航">
        <span v-if="isDevelopment" class="development-badge" title="页面实时更新，只读取后台；不会重启服务或修改连接。">开发预览 · 只读</span>
        <button :class="{active:page==='logs'}" :aria-pressed="page==='logs'" aria-label="记录" title="记录" @click="navigate('logs')"><Logs aria-hidden="true" /></button>
        <button :class="{active:page==='settings'}" :aria-pressed="page==='settings'" aria-label="设置" title="设置" @click="navigate('settings')"><Settings aria-hidden="true" /></button>
      </nav>
      <div v-if="isDesktop&&!mac" class="window-controls"><button aria-label="最小化" @click="windowAction('minimize')"><Minus /></button><button aria-label="最大化或还原" @click="windowAction('toggleMaximize')"><Square /></button><button aria-label="关闭窗口" @click="windowAction('close')"><X /></button></div>
    </header>
    <main ref="workspace" class="workspace" :class="{'records-workspace':page==='logs'}">
      <div v-if="appUpdate.available.value && page!=='settings'" class="status-banner" role="status"><Info aria-hidden="true"/><span>Local Connector {{appUpdate.update.value?.version}} 可更新</span><button class="text-button" @click="navigate('settings')">查看更新</button></div>
      <div v-if="feedback.text && feedback.error" class="status-banner warning" role="alert"><TriangleAlert aria-hidden="true"/><span>{{feedback.text}}</span><button class="ghost banner-dismiss" aria-label="关闭提示" @click="feedback.text=''"><X aria-hidden="true"/></button></div>
      <template v-if="page==='guide'"><SetupWizard v-if="setup"/><ChatGuide v-else @done="navigate('overview')"/></template>
      <template v-else-if="page==='overview'">
          <div v-if="status?.connection?.updateAvailable" class="status-banner" role="status"><Info aria-hidden="true"/><span>连接组件有更新</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> 更新连接</button></div>
          <div v-if="needsReconnect && !progressing" class="status-banner warning" role="status"><TriangleAlert aria-hidden="true"/><span>{{connectionIssue}}</span><button class="text-button" :disabled="!!busy" @click="reconnect"><RefreshCw aria-hidden="true"/> 重新连接</button></div>
          <section class="connection-panel" :class="{connected,progressing}">
            <div class="connection-content"><h2>{{title}}</h2>
              <button class="primary connection-action" :disabled="progressing || loading" @click="toggle"><Pause v-if="tunnelRunning" aria-hidden="true" /><Play v-else aria-hidden="true" />{{progressing?'请稍候…':tunnelRunning?'关闭连接':'开启连接'}}</button>
            </div>
            <div class="connection-art" aria-hidden="true"><img :src="logo"/><span class="art-orbit"></span><span class="art-orbit second"></span></div>
            <section class="connection-path" aria-label="连接状态">
              <div class="path-node" :class="{ready:verified}"><img class="product-logo" :src="chatgptLogo" alt=""/><strong>ChatGPT</strong><button v-if="!verified" class="text-button" @click="navigate('guide')">接入引导 <ArrowRight aria-hidden="true"/></button></div>
              <span class="path-link" :class="{ready:links[0].ready,pending:links[0].pending,failed:links[0].failed}" role="img" :aria-label="links[0].label" :title="links[0].label"><component :is="linkIcon(links[0])" aria-hidden="true"/></span>
              <div class="path-node" :class="{ready:state==='ready'}"><img class="product-logo connector-logo" :src="logo" alt=""/><strong>Connector</strong></div>
              <span class="path-link" :class="{ready:links[1].ready,pending:links[1].pending,failed:links[1].failed}" role="img" :aria-label="links[1].label" :title="links[1].label"><component :is="linkIcon(links[1])" aria-hidden="true"/></span>
              <div class="path-node" :class="{ready:desktopState==='ready'}"><img class="product-logo" :src="codexLogo" alt=""/><strong>Codex</strong></div>
            </section>
          </section>

      </template>
      <template v-else-if="page==='logs'">
        <RecordsPage/>
      </template>
      <template v-else>
        <SettingsPage/>
      </template>
    </main>
    <div v-if="feedback.text && !feedback.error" class="message" role="status">{{feedback.text}}<button aria-label="关闭提示" @click="feedback.text=''"><X aria-hidden="true" /></button></div>
  </div>
</template>
