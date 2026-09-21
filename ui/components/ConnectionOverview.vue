<script setup lang="ts">
import { NTooltip } from 'naive-ui';
import { computed, defineAsyncComponent, ref, onMounted, onUnmounted } from 'vue';
import { Plus, Power, LoaderCircle, CircleCheck, CircleDashed, CircleAlert, CirclePause, Bot } from '@lucide/vue';
import { t } from '../i18n';
import { openUrl } from '../platform';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { useAgents, icons, name, type Agent } from '../composables/useAgents';
import PlayfulMascot from './PlayfulMascot.vue';
import SourceIcon from './SourceIcon.vue';
const IngressDialog = defineAsyncComponent(() => import('./IngressDialog.vue'));
const emit = defineEmits<{ settings: [] }>();
const { status, busy, loading, connectionError, run, refresh } = useConnector();
const inventory = useAgents();
const activeTip = ref('');
const nodeTip = ref('');
const platformUrls: Record<string,string> = { chatgpt:'https://chatgpt.com/', notion:'https://www.notion.so/', slack:'https://app.slack.com/' };
function openPlatform(platform: string) { const url=platformUrls[platform]; if(url) void run('open-platform',()=>openUrl(url)); }
const agents = computed(() => inventory.agents.value.filter(a => a.installed && a.available !== false).sort((a,b) => Number(b.agent==='codex')-Number(a.agent==='codex') || name(a).localeCompare(name(b))));
const sources = computed(() => [...(status.value?.ingresses || [])].sort((a,b) => Number(b.controlSource==='chatgpt')-Number(a.controlSource==='chatgpt') || a.name.localeCompare(b.name)));
const running = computed(() => sources.value.some(i => i.running));
const progressing = computed(() => !!busy.value || sources.value.some(i => ['starting','stopping'].includes(i.state)));
const dialog = ref(false), editing = ref<Ingress>(), defaultChatGPT = ref(false);
function edit(ingress?: Ingress, chatgpt=false) { editing.value=ingress; defaultChatGPT.value=chatgpt; dialog.value=true; }
function sourceState(i: Ingress) { return !i.enabled ? t('disabled') : ['error','degraded'].includes(i.state) ? t('connectionError') : ['starting','stopping'].includes(i.state) ? t('connectingLabel') : i.running ? t('connected') : t('disconnected'); }
function agentState(a: Agent) {
  if (a.enabled === false) return t('disabled');
  if (a.agent==='codex') { const state=status.value?.autoOpenCodex===false ? status.value?.core.appServer?.state : status.value?.core.desktop?.state; if(state==='ready') return t('connected'); if(state==='error') return t('connectionError'); if(state==='connecting') return t('preparing'); }
  return a.status==='ready' ? t('connected') : a.installed && a.available ? t('agentAvailable') : t('connectionError');
}
async function openAgent(agent: Agent) { await run('open-agent',()=>api('agents/open','POST',{agent:agent.agent})); }
async function toggle() {
  if (!running.value && (!sources.value.length || sources.value.every(i => !i.config.configured))) { edit(sources.value[0],true); return; }
  await run('ingress-all',async()=> { const result=await api<{id:string;ok:boolean;error?:string}[]>(`ingress/${running.value?'stop-all':'start-all'}`,'POST'); await refresh(); const errors=Array.isArray(result) ? result.filter(r=>!r.ok) : []; if(errors.length) throw new Error(errors.map(r=>`${sources.value.find(i=>i.id===r.id)?.name || r.id}: ${r.error}`).join('\n')); });
}
let timer: ReturnType<typeof setInterval>;
onMounted(()=>{ void inventory.refresh(); timer=setInterval(()=>void inventory.refresh(),15000); });
onUnmounted(()=>clearInterval(timer));
const height = computed(()=>Math.max(320,Math.max(Math.max(1,sources.value.length)+1,agents.value.length)*84));
const y = (index:number,count:number) => height.value/2+(index-(count-1)/2)*84;
const path = (index:number,count:number,right=false) => { const endpoint=y(index,count); return right ? `M 0 ${height.value/2} C 90 ${height.value/2}, 110 ${endpoint}, 200 ${endpoint}` : `M 0 ${endpoint} C 90 ${endpoint}, 110 ${height.value/2}, 200 ${height.value/2}`; };
function stateIcon(label: string) {
  if (label === t('connectionError')) return CircleAlert;
  if (label === t('disabled')) return CirclePause;
  if ([t('connectingLabel'), t('preparing')].includes(label)) return LoaderCircle;
  if ([t('connected'), t('agentAvailable')].includes(label)) return CircleCheck;
  return CircleDashed;
}
// Match the cubic wire, one third inward from each outer node.
const marker = (index: number, count: number, right = false) => {
  const u = right ? 2 / 3 : 1 / 3;
  const blend = 3 * u * u - 2 * u * u * u;
  const x = 3 * (1-u)**2 * u * 90 + 3 * (1-u) * u*u * 110 + u**3 * 200;
  const endpoint = y(index,count), center = height.value / 2;
  const top = right ? center + (endpoint-center)*blend : endpoint + (center-endpoint)*blend;
  return { left: `${x/2}%`, top: `${top}px`, '--unfold-y': `${(center-top)*11/14}px` };
};
const unfold = (index: number, count: number) => ({ '--unfold-y': `${((count-1)/2-index)*66}px` });
</script>
<template>
  <section class="connection-core" :aria-label="t('connectionStatus')">
    <div class="core-toggle"><button class="connection-action" :class="{'is-on':running,'is-busy':progressing}" :disabled="progressing || loading || !status" :aria-busy="progressing" @click="toggle"><span class="connection-switch-track" aria-hidden="true"><span class="connection-knob"><LoaderCircle v-if="progressing"/><Power v-else/></span></span><span class="connection-action-label">{{ progressing ? t('pleaseWait') : running ? t('disconnect') : t('connect') }}</span><span class="connection-action-light" aria-hidden="true"/></button></div>
    <div class="connection-graph" :style="{height:`${height}px`}">
      <div class="graph-nodes sources">
        <NTooltip v-if="!sources.length" :show="nodeTip==='default-source'" placement="top"><template #trigger><button class="graph-node" aria-label="ChatGPT" @mouseenter="nodeTip='default-source'" @mouseleave="nodeTip=''" @focus="nodeTip='default-source'" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="openPlatform('chatgpt')"><SourceIcon platform="chatgpt"/></button></template>ChatGPT</NTooltip>
        <NTooltip v-for="source in sources" :key="source.id" :show="nodeTip==='source:'+source.id" placement="top"><template #trigger><button class="graph-node" :aria-label="source.name" :aria-disabled="!platformUrls[source.controlSource]" @mouseenter="nodeTip='source:'+source.id" @mouseleave="nodeTip=''" @focus="nodeTip='source:'+source.id" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="openPlatform(source.controlSource)"><SourceIcon :platform="source.controlSource"/></button></template>{{source.name}}</NTooltip>
        <NTooltip :show="nodeTip==='add-source'" placement="top"><template #trigger><button class="graph-node add-source" :aria-label="t('addControlSource')" @mouseenter="nodeTip='add-source'" @mouseleave="nodeTip=''" @focus="nodeTip='add-source'" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="edit()"><span class="source-icon"><Plus/></span></button></template>{{t('addControlSource')}}</NTooltip>
      </div>
      <div class="graph-wires left"><svg :viewBox="`0 0 200 ${height}`" preserveAspectRatio="none" aria-hidden="true"><path v-for="(source,index) in sources" :key="source.id" :d="path(index,Math.max(1,sources.length)+1)" :class="{live:source.running,failed:['error','degraded'].includes(source.state)}"/><path v-if="!sources.length" :d="path(0,2)"/><path :d="path(Math.max(1,sources.length),Math.max(1,sources.length)+1)" class="placeholder"/></svg><NTooltip v-for="(source,index) in sources" :key="source.id" :show="activeTip===source.id && stateIcon(sourceState(source))!==CircleCheck" placement="top"><template #trigger><button  type="button" class="wire-status" :class="{live:source.running,failed:['error','degraded'].includes(source.state)}" :style="marker(index,Math.max(1,sources.length)+1)" :aria-label="`${source.name}: ${sourceState(source)} · ${t('editConnectionDetails')}`" aria-haspopup="dialog" @click="edit(source)" @mouseenter="activeTip=source.id" @mouseleave="activeTip=''" @focus="activeTip=source.id" @blur="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(sourceState(source))" :class="{spinning:stateIcon(sourceState(source))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{sourceState(source)}}<div class="status-hint">{{t('editConnectionDetails')}}</div></div></NTooltip></div>
      <div class="graph-brain"><PlayfulMascot/></div>
      <div class="agent-side">
      <div class="graph-wires right" :aria-busy="inventory.loading.value && !inventory.loaded.value"><svg v-if="inventory.loading.value && !inventory.loaded.value" class="search-wires" viewBox="0 0 200 120" preserveAspectRatio="none" aria-hidden="true"><path v-for="n in 3" :key="n" :style="{'--drift-delay':(n-1)*-.8+'s'}" d="M 0 60 C 90 60, 110 36, 200 36"/></svg><svg :viewBox="`0 0 200 ${height}`" preserveAspectRatio="none" aria-hidden="true"><path class="agent-wire" v-for="(agent,index) in agents" :key="agent.agent" :d="path(index,agents.length,true)" :class="{live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}"/></svg><NTooltip v-for="(agent,index) in agents" :key="agent.agent" :show="activeTip===agent.agent && stateIcon(agentState(agent))!==CircleCheck" placement="top"><template #trigger><button  type="button" class="wire-status agent-marker" :class="{live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}" :style="marker(index,agents.length,true)" :aria-label="`${name(agent)}: ${agentState(agent)} · ${t('settings')}`" @click="emit('settings')" @mouseenter="activeTip=agent.agent" @mouseleave="activeTip=''" @focus="activeTip=agent.agent" @blur="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(agentState(agent))" :class="{spinning:stateIcon(agentState(agent))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{agentState(agent)}}<div class="status-hint">{{t('settings')}}</div></div></NTooltip></div>
      <div class="graph-nodes agents"><NTooltip v-for="(agent,index) in agents" :key="agent.agent" :show="nodeTip==='agent:'+agent.agent" placement="top"><template #trigger><button type="button" class="graph-node" @click="openAgent(agent)" :style="unfold(index,agents.length)" :aria-label="name(agent)" tabindex="0" @mouseenter="nodeTip='agent:'+agent.agent" @mouseleave="nodeTip=''" @focus="nodeTip='agent:'+agent.agent" @blur="nodeTip=''" @keydown.esc="nodeTip=''"><span v-if="icons[agent.agent]" class="source-icon agent-brand" aria-hidden="true" v-html="icons[agent.agent]"/><Bot v-else aria-hidden="true"/></button></template><div>{{name(agent)}}<div v-if="agent.version" class="status-hint">{{agent.version}}</div></div></NTooltip><p v-if="!agents.length && !inventory.loading.value" class="hint">{{t('noInstalledAgents')}}</p></div>
      </div>
    </div>
    <p v-if="connectionError || inventory.error.value" class="graph-error" role="alert">{{connectionError || inventory.error.value}}</p>
  </section>
  <IngressDialog v-if="dialog" :ingress="editing" :default-chat-g-p-t="defaultChatGPT" @close="dialog=false"/>
</template>
<style scoped>
.connection-core{--graph-bg:light-dark(#e3eee8,#20352f);background:var(--graph-bg);border:1px solid light-dark(#cfdfd6,#354c43);border-radius:24px;padding:34px 32px 28px;overflow:hidden}.core-toggle{display:flex;align-items:center;flex-direction:column;gap:12px}.connection-graph{margin-top:30px;display:grid;grid-template-columns:minmax(110px,1fr) minmax(80px,1fr) 190px minmax(80px,1fr) minmax(110px,1fr);align-items:center}.graph-nodes{display:flex;flex-direction:column;justify-content:center;gap:40px;z-index:2}.graph-node{width:44px;height:44px;flex-shrink:0;display:flex;align-items:center;justify-content:center;padding:0;border-radius:12px;min-width:0;background:transparent}.graph-node{border:0;box-shadow:none}.graph-node:hover{background:transparent}.graph-node>:first-child{transition:transform 160ms ease}.graph-node:hover>:first-child{transform:scale(1.1)}.graph-node:focus-visible{outline:2px solid #87ad99;outline-offset:0}.sources{align-items:flex-end}.sources .graph-node{margin-right:4px;padding:0;justify-content:center;background:transparent;border:0;box-shadow:none}.sources .graph-node:hover{background:transparent}.sources .source-icon{transition:transform 160ms ease}.graph-node img,.source-icon{width:38px;height:38px;flex-shrink:0;border-radius:12px;display:grid;place-items:center;background:transparent}.graph-node img{object-fit:contain}.source-icon b{font-family:Georgia,serif;font-size:27px}.source-icon svg{width:23px;height:23px}.agent-brand :deep(svg){width:32px;height:32px}.add-source{color:var(--muted);background:transparent}.add-source .source-icon{background:transparent;border:1px dashed currentColor}.graph-brain{display:flex;flex-direction:column;align-items:center;gap:18px;position:relative;z-index:1}.graph-brain:before{content:'';position:absolute;width:260px;height:260px;border:1px solid light-dark(#cedfd470,#ffffff08);border-radius:50%;top:-34px;pointer-events:none}.graph-wires{height:100%;position:relative}.graph-wires svg{width:100%;height:100%;overflow:visible}.graph-wires path{fill:none;stroke:light-dark(#abbfb4,#60796b);stroke-width:1.4;vector-effect:non-scaling-stroke}.graph-wires path.live{stroke:#5c9d80;stroke-width:2}.graph-wires path.failed{stroke:#be826b}.graph-wires path.placeholder{stroke-dasharray:4 5;opacity:.65}.wire-status{padding:0;border:0;box-shadow:none;cursor:pointer;position:absolute;transform:translate(-50%,-50%);width:26px;height:26px;display:grid;place-items:center;color:var(--muted);background:var(--graph-bg);border-radius:50%}.wire-status svg{width:17px;height:17px}.wire-status.live{color:light-dark(#3b7860,#9cc6ab)}.wire-status.failed,.graph-error{color:#b66b54}.graph-error{font-size:12px}.agent-side{--agent-icon-size:38px;grid-column:4 / 6;display:grid;height:100%;grid-template-columns:1fr;align-items:center}.agent-side>.right{grid-area:1 / 1;width:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agent-side>.agents{grid-area:1 / 1;margin-left:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agents{align-items:flex-start;gap:46px}.agents .graph-node{width:38px;height:38px;margin-left:12px}@media(max-width:800px){.connection-core{padding:26px 16px}.connection-graph{grid-template-columns:minmax(92px,1fr) minmax(60px,.8fr) 124px minmax(60px,.8fr) minmax(92px,1fr)}.graph-brain :deep(.mascot){width:120px;height:120px}.graph-brain:before{width:160px;height:160px;top:-15px}.graph-node img,.source-icon{width:30px;height:30px}.agent-side{--agent-icon-size:30px}.agents{gap:54px}.agents .graph-node{width:30px;height:30px}}
.agents .graph-node,.agent-marker{animation:agent-unfold 700ms cubic-bezier(.22,1,.36,1) both}.agent-wire{transform-origin:0 50%;animation:wire-unfold 700ms cubic-bezier(.22,1,.36,1) both}.spinning{animation:status-spin 1s linear infinite}
@keyframes agent-unfold{from{translate:0 var(--unfold-y);opacity:.35}to{translate:0 0;opacity:1}}
@keyframes wire-unfold{from{transform:scaleY(.2142857);opacity:.35}to{transform:scaleY(1);opacity:1}}
@keyframes status-spin{to{rotate:360deg}}
@media(prefers-reduced-motion:reduce){.agents .graph-node,.agent-marker,.agent-wire,.spinning{animation:none}}

.sources .source-icon{width:44px;height:44px}.sources :deep(.platform-icon){transform:scale(1.25)}.sources .graph-node:hover :deep(.platform-icon){transform:scale(1.35)}.sources .add-source .source-icon{width:34px;height:34px}.sources .add-source .source-icon>svg{width:20px;height:20px}.source-brand :deep(svg){width:40px;height:40px}.sources .source-icon>svg{width:30px;height:30px}
.wire-status:hover,.wire-status:focus-visible{z-index:5;background:var(--graph-bg)}
.status-hint{font-size:11px;opacity:.7}
@media(prefers-reduced-motion:reduce){.graph-node>:first-child{transition:none}}
.graph-wires .search-wires{position:absolute;top:50%;left:0;width:100%;height:100px;transform:translateY(-50%);pointer-events:none}
.search-wires path{stroke-linecap:round;animation:search-drift 2.8s ease-in-out infinite;animation-delay:var(--drift-delay);opacity:.55}
@keyframes search-drift{0%,100%{d:path("M 0 60 C 90 60, 110 36, 200 36");opacity:.3}50%{d:path("M 0 60 C 90 60, 110 84, 200 84");opacity:.75}}
@media(prefers-reduced-motion:reduce){.search-wires path{animation:none}}
</style>
