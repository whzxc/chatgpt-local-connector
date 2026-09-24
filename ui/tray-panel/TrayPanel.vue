<script setup lang="ts">
import {computed,onMounted,onUnmounted,ref} from 'vue';
import {NAlert,NButton,NEmpty,NSpin} from 'naive-ui';
import {ChevronRight} from '@lucide/vue';
import QuotaBubble from '../subscriptions/QuotaBubble.vue';
import {useSubscriptions} from '../subscriptions/useSubscriptions';
import {api} from '../api';
import {isDesktop} from '../platform';
import {t} from '../i18n';
import {brandIcon} from '../subscriptions/presentation';
import {copyShareCard} from './shareCard';
const subscriptions=useSubscriptions();
const side=ref<'left'|'right'>('left'), dismissKey=ref(0),busy=ref(false),error=ref('');
const connection=ref(false),version=ref(''),copied=ref(false);
const content=ref<HTMLElement>(),footer=ref<HTMLElement>();
let resizeObserver:ResizeObserver|undefined,resizeFrame=0,lastHeight=0;
function resizePanel(){
  cancelAnimationFrame(resizeFrame);
  resizeFrame=requestAnimationFrame(async()=>{
    if(!content.value||!footer.value)return;
    const height=Math.ceil(content.value.getBoundingClientRect().height+footer.value.getBoundingClientRect().height+2);
    if(!isDesktop||height===lastHeight)return;
    lastHeight=height;
    try{const {invoke}=await import('@tauri-apps/api/core');await invoke('tray_panel_resize',{height});}
    catch(e){lastHeight=0;error.value=String(e);}
  });
}
const providerCards=new Map<string,HTMLElement>();
let copyTimer:ReturnType<typeof setTimeout>|undefined;
function registerCard(id:string,el:unknown){if(el instanceof HTMLElement)providerCards.set(id,el);else providerCards.delete(id);}
const providers=computed(()=>subscriptions.snapshot.value?.settings.enabled?subscriptions.snapshot.value.providers.filter(p=>p.selected&&p.eligible):[]);
async function nativeAction(action:string){
  if(isDesktop){const {invoke}=await import('@tauri-apps/api/core');await invoke('tray_action',{action});}
  else if(['overview','settings','tasks','logs'].includes(action))window.location.href='/';
}
async function readStatus(){
  const status=await api<{connection:{running:boolean};version:string}>('status');
  connection.value=status.connection.running;version.value=status.version;
}
async function select(key:string){
  error.value='';
  try{await nativeAction(key);}catch(e){error.value=String(e);}
}
async function share(id:string){
  error.value='';copied.value=false;busy.value=true;
  try{
    const card=providerCards.get(id),provider=providers.value.find(p=>p.providerId===id);
    if(!card||!provider)throw new Error(t('trayEmpty'));
    await copyShareCard(card,brandIcon(provider.agentId));
    copied.value=true;clearTimeout(copyTimer);copyTimer=setTimeout(()=>copied.value=false,4000);
  }catch(e){error.value=String(e);}finally{busy.value=false;}
}
async function openOptions(event:MouseEvent){
  if(!isDesktop||busy.value)return;
  error.value='';dismissKey.value++;
  const rect=(event.currentTarget as HTMLElement).getBoundingClientRect();
  let menu:import('@tauri-apps/api/menu').Menu|undefined;
  try{
    await readStatus();
    const {Menu}=await import('@tauri-apps/api/menu');
    const {LogicalPosition}=await import('@tauri-apps/api/dpi');
    menu=await Menu.new({items:[
      {id:'status',text:`${t('trayConnectionStatus')}：${t(connection.value?'trayConnected':'trayDisconnected')}`,action:()=>void select('overview')},
      {id:'settings',text:t('traySettings'),accelerator:'CmdOrCtrl+,',action:()=>void select('settings')},
      {item:'Separator'},
      {id:'share',text:t('trayShareScreenshot'),enabled:providers.value.length>0,items:providers.value.map(p=>({id:`share:${p.providerId}`,text:p.name,action:()=>void share(p.providerId)}))},
      {id:'updates',text:t('checkForUpdates')+'…',action:()=>void select('updates')},
      {item:'Separator'},
      {item:{About:{name:'Local Connector',version:version.value,license:'MIT',website:'https://github.com/whzxc/chatgpt-local-connector'}},text:t('trayAbout')},
      {id:'quit',text:t('trayQuit'),accelerator:'CmdOrCtrl+Q',action:()=>void select('quit')},
    ]});
    await nativeAction('menu-open');
    await menu.popup(new LogicalPosition(rect.left,rect.top));
  }catch(e){error.value=String(e);}finally{
    await nativeAction('menu-close').catch(e=>error.value=String(e));
    await menu?.close();
  }
}
async function opened(){dismissKey.value++;await Promise.all([subscriptions.read(),readStatus().catch(e=>error.value=String(e))]);}
function dismiss(){void nativeAction('hide');}
function keydown(e:KeyboardEvent){if(e.key==='Escape'){e.preventDefault();dismiss();}}
let off:(()=>void)|undefined,disposed=false;
onMounted(async()=>{resizeObserver=new ResizeObserver(resizePanel);if(content.value)resizeObserver.observe(content.value);if(footer.value)resizeObserver.observe(footer.value);resizePanel();document.addEventListener('keydown',keydown);if(isDesktop){const {listen}=await import('@tauri-apps/api/event');const stop=await listen<{side:'left'|'right'}>('tray-panel:open',e=>{side.value=e.payload.side;void opened();});if(disposed)stop();else off=stop;}await readStatus().catch(e=>error.value=String(e));});
onUnmounted(()=>{resizeObserver?.disconnect();cancelAnimationFrame(resizeFrame);clearTimeout(copyTimer);disposed=true;off?.();document.removeEventListener('keydown',keydown);});
</script>
<template>
  <div class="tray-surface" @mousedown.self="dismiss">
    <section class="tray-card" :class="side" :aria-label="t('trayUsage')">
      <div class="tray-scroll" @scroll="dismissKey++">
        <div ref="content" class="tray-content">
        <NAlert v-if="error || subscriptions.error.value" type="error" :show-icon="false">{{error || subscriptions.error.value}}</NAlert>
        <NAlert v-if="copied" type="success" :show-icon="false" role="status">{{t('trayScreenshotCopied')}}</NAlert>
        <NSpin v-if="!subscriptions.snapshot.value && !subscriptions.error.value" size="small"/>
        <div v-for="provider in providers" :key="provider.providerId" :ref="el=>registerCard(provider.providerId,el)" class="provider-card"><QuotaBubble :provider="provider" embedded :external-detail="provider.providerId" :detail-placement="side" :dismiss-key="dismissKey"/></div>
        <NEmpty v-if="subscriptions.snapshot.value&&!providers.length" :description="t('trayEmpty')"><template #extra><NButton size="small" @click="select('settings')">{{t('traySettings')}}</NButton></template></NEmpty>
        </div>
      </div>
      <footer ref="footer"><span class="app-version">Local Connector {{version}}</span><NButton text size="small" class="connection-menu" :loading="busy" :disabled="!isDesktop" :aria-label="`${t(connection?'trayConnected':'trayDisconnected')} · ${t('trayOptions')}`" aria-haspopup="menu" @click="openOptions"><span>{{t(connection?'trayConnected':'trayDisconnected')}}</span><ChevronRight :size="12" aria-hidden="true"/></NButton></footer>
    </section>
  </div>
</template>
<style scoped>
.tray-surface{position:fixed;inset:0;color:var(--ink)}
.tray-card{position:absolute;top:0;max-height:100%;width:380px;max-width:100%;box-sizing:border-box;display:flex;flex-direction:column;background:var(--surface);border:1px solid var(--line);border-radius:22px;overflow:hidden}.tray-card.left{right:0}.tray-card.right{left:0}
.tray-scroll{min-height:0;overflow:auto;scrollbar-width:thin}.tray-content{padding:14px;display:flex;flex-direction:column;gap:14px}.provider-card{background:var(--soft);border-radius:18px;padding:14px;flex:none}footer{padding:8px 14px;min-height:22px;gap:12px;display:flex;justify-content:space-between;align-items:center;border-top:1px solid var(--line);flex:none;font-size:11px;color:var(--muted)}.app-version{white-space:nowrap}.connection-menu{flex:none;font-size:11px;color:var(--muted);gap:4px}
</style>
