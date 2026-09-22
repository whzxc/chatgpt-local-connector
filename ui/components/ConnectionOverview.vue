<script setup lang="ts">
import { NTooltip } from 'naive-ui';
import { useElementSize, useMediaQuery } from '@vueuse/core';
import { computed, ref, watch, onMounted, onUnmounted } from 'vue';
import { Ellipsis, Plus, Power, LoaderCircle, CircleCheck, CircleDashed, CircleAlert, CirclePause, Bot } from '@lucide/vue';
import { t } from '../i18n';
import { openUrl } from '../platform';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { useAgents, icons, name, type Agent } from '../composables/useAgents';
import PlayfulMascot from './PlayfulMascot.vue';
import SourceIcon from './SourceIcon.vue';
import { controlSource } from '../controlSources';
import { controlSourceClick } from '../behavior';
import IngressDetailsDialog from './IngressDetailsDialog.vue';
import IngressDialog from './IngressDialog.vue';
import AgentsSettings from './AgentsSettings.vue';
const agentsDialog = ref(false);
const agentOrigin = ref({x:0,y:0,size:30});
function showAgents() {
  nodeTip.value=''; activeTip.value='';
  const rect=document.querySelector('.agents-more')?.getBoundingClientRect();
  if(rect) agentOrigin.value={x:rect.x,y:rect.y,size:rect.width};
  agentsDialog.value=true;
}
function closeAgents() {
  agentsDialog.value=false;
  requestAnimationFrame(()=>document.querySelector<HTMLButtonElement>('.agents-more')?.focus());
}
const { status, busy, loading, connectionError, run, refresh } = useConnector();
const inventory = useAgents();
const activeTip = ref('');
const nodeTip = ref('');
function openSource(source?: Ingress) {
  nodeTip.value = '';
  if (controlSourceClick.value === 'settings') { source ? view(source) : edit(undefined, true); return; }
  const url = controlSource(source?.controlSource || 'chatgpt')?.homeUrl;
  if (url) void run('open-platform', () => openUrl(url));
}
const agents = computed(() => inventory.orderedAgents.value.slice(0,4));
const sources = computed(() => [...(status.value?.ingresses || [])].sort((a,b) => Number(b.controlSource==='chatgpt')-Number(a.controlSource==='chatgpt') || a.name.localeCompare(b.name)));
const running = computed(() => sources.value.some(i => i.running));
const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)');
const progressing = computed(() => !!busy.value || sources.value.some(i => ['starting','stopping'].includes(i.state)));
const switchOn = ref<boolean>();
const sparks = ref<{ id: number; style: Record<string, string> }[]>([]);
let sparkPending = false, sparkId = 0, impactFrame = 0;
watch([() => status.value ? running.value : undefined, progressing], ([on, active]) => {
  if (active) {
    sparkPending = false;
    sparks.value = [];
    return;
  }
  if (on === undefined) return;
  sparkPending = on && switchOn.value === false;
  switchOn.value = on;
  if (!on) sparks.value = [];
}, { immediate: true });
function watchSwitchImpact(event: TransitionEvent) {
  if (event.target !== event.currentTarget || event.propertyName !== 'transform') return;
  cancelAnimationFrame(impactFrame);
  const knob = event.currentTarget as HTMLElement;
  const track = knob.parentElement;
  if (!track || !sparkPending || reducedMotion.value) return;
  const edge = track.clientWidth - knob.offsetWidth - knob.offsetLeft * 2;
  const observe = () => {
    if (!sparkPending || !switchOn.value || progressing.value || !knob.isConnected) return;
    const position = new DOMMatrixReadOnly(getComputedStyle(knob).transform).m41;
    // The spring easing reaches the edge before it finishes overshooting and settling.
    if (position >= edge - .5) burstSparks();
    else impactFrame = requestAnimationFrame(observe);
  };
  impactFrame = requestAnimationFrame(observe);
}
function stopImpactWatch(event: TransitionEvent) {
  if (event.target !== event.currentTarget || event.propertyName !== 'transform') return;
  cancelAnimationFrame(impactFrame);
}
function burstSparks() {
  if (!sparkPending) return;
  sparkPending = false;
  if (!switchOn.value || progressing.value || reducedMotion.value) return;
  const count = 5 + Math.floor(Math.random() * 3);
  sparks.value = Array.from({ length: count }, (_, index) => {
    // Spread origins across the right semicircle, with jitter within each segment.
    const angle = -Math.PI / 2 + (index + .15 + Math.random() * .7) / count * Math.PI;
    const distance = 30 + Math.random() * 55;
    return {
      id: ++sparkId,
      style: {
        '--spark-origin-x': `${50 + Math.cos(angle) * 50}%`,
        '--spark-origin-y': `${50 + Math.sin(angle) * 50}%`,
        '--spark-size': `${9 + Math.random() * 8}px`,
        '--spark-x': `${Math.cos(angle) * distance}px`,
        '--spark-y': `${Math.sin(angle) * distance}px`,
        '--spark-turn': `${60 + Math.random() * 180}deg`,
        '--spark-duration': `${650 + Math.random() * 250}ms`,
      },
    };
  });
}
function removeSpark(id: number) { sparks.value = sparks.value.filter(spark => spark.id !== id); }
const dialog = ref(false), editing = ref<Ingress>(), defaultChatGPT = ref(false);
const sourceOrigin = ref({x:20,y:20,size:44});
function closeSource() { dialog.value=false; requestAnimationFrame(()=>document.querySelector<HTMLButtonElement>('.add-source')?.focus()); }
function edit(ingress?: Ingress, chatgpt=false) {
  nodeTip.value=''; activeTip.value='';
  const rect=document.querySelector('.add-source')?.getBoundingClientRect();
  if(rect) sourceOrigin.value={x:rect.x,y:rect.y,size:rect.width}; editing.value=ingress; defaultChatGPT.value=chatgpt; dialog.value=true; }
const details = ref<Ingress>(), connectAfterSave = ref(false);
function view(ingress: Ingress, connect=false) { dialog.value=false; details.value=ingress; connectAfterSave.value=connect; }
function editDetails(ingress: Ingress) { details.value=undefined; edit(ingress); }
function sourceState(i: Ingress) { return !i.enabled ? t('disabled') : ['error','degraded'].includes(i.state) ? t('connectionError') : ['starting','stopping'].includes(i.state) ? t('connectingLabel') : i.running ? t('connected') : t('disconnected'); }
function agentState(a: Agent) {
  if (a.enabled === false) return t('disabled');
  if (a.agent==='codex') { const state=status.value?.autoOpenCodex===false ? status.value?.core.appServer?.state : status.value?.core.desktop?.state; if(state==='ready') return t('connected'); if(state==='error') return t('connectionError'); if(state==='connecting') return t('preparing'); }
  return a.status==='ready' ? t('connected') : a.installed && a.available ? t('agentAvailable') : t('connectionError');
}
async function openAgent(agent: Agent) { await run('open-agent',()=>api('agents/open','POST',{agent:agent.agent})); }
async function toggle() {
  if (!running.value && (!sources.value.length || sources.value.every(i => !i.config.configured))) { edit(sources.value[0],true); return; }
  await run('ingress-all',async()=> { const result=await api<{results:{id:string;ok:boolean;error?:string}[]}>(`ingress/${running.value?'stop-all':'start-all'}`,'POST'); await refresh(); const errors=result.results.filter(r=>!r.ok); if(errors.length) throw new Error(errors.map(r=>`${sources.value.find(i=>i.id===r.id)?.name || r.id}: ${r.error}`).join('\n')); });
}
let timer: ReturnType<typeof setInterval>;
onMounted(()=>{ void inventory.refresh(); timer=setInterval(()=>void inventory.refresh(),15000); });
onUnmounted(()=>{ clearInterval(timer); cancelAnimationFrame(impactFrame); });
const height = computed(()=>Math.max(320,Math.max(Math.max(1,sources.value.length)+1,(agents.value.length+1))*84));
const y = (index:number,count:number) => height.value/2+(index-(count-1)/2)*84;
const leftWires = ref<HTMLElement>();
const rightWires = ref<HTMLElement>();
const { width: leftWidth } = useElementSize(leftWires);
const { width: rightWidth } = useElementSize(rightWires);
const compactGraph = useMediaQuery('(max-width: 800px)');
const wireWidth = (right: boolean) => Math.max(1, right ? rightWidth.value : leftWidth.value);
// Distribute individual ports along the central ring, including utility branches.
function port(index: number, count: number, right = false) {
  const radius = compactGraph.value ? 80 : 130;
  const halfCore = compactGraph.value ? 62 : 95;
  const offset = count <= 1 ? 0 : (index / (count - 1) - .5) * Math.min((count - 1) * 24, 96);
  const reach = Math.sqrt(radius * radius - offset * offset) - halfCore;
  return { x: right ? reach : wireWidth(false) - reach, y: height.value / 2 + offset };
}
function wire(index: number, count: number, right = false) {
  const inner = port(index, count, right);
  const start = right ? inner : { x: 0, y: y(index, count) };
  const end = right ? { x: wireWidth(true), y: y(index, count) } : inner;
  const span = end.x - start.x;
  return { start, end, c1: start.x + span * .45, c2: start.x + span * .55 };
}
function path(index: number, count: number, right = false, gap = false) {
  const { start, end, c1, c2 } = wire(index, count, right);
  const point = (u: number) => ({
    x: (1-u)**3*start.x + 3*(1-u)**2*u*c1 + 3*(1-u)*u*u*c2 + u**3*end.x,
    y: start.y + (end.y-start.y)*(3*u*u-2*u*u*u),
  });
  const tangent = (u: number) => ({
    x: 3*(1-u)**2*(c1-start.x)+6*(1-u)*u*(c2-c1)+3*u*u*(end.x-c2),
    y: 6*u*(1-u)*(end.y-start.y),
  });
  const segment = (a: number, b: number) => {
    const p = point(a), q = point(b), da = tangent(a), db = tangent(b), span = (b-a)/3;
    return `M ${p.x} ${p.y} C ${p.x+span*da.x} ${p.y+span*da.y}, ${q.x-span*db.x} ${q.y-span*db.y}, ${q.x} ${q.y}`;
  };
  if (!gap) return segment(0, 1);
  // Omit the actual curve beneath the status icon, including on transparent windows.
  const center = right ? 2/3 : 1/3, marker = point(center);
  const edge = (bound: number) => {
    let near = center, far = bound;
    for (let n = 0; n < 20; n++) {
      const u = (near+far)/2, p = point(u);
      if (Math.hypot(p.x-marker.x, p.y-marker.y) < 13) near = u;
      else far = u;
    }
    return far;
  };
  return `${segment(0, edge(0))} ${segment(edge(1), 1)}`;
}
const portStyle = (index: number, count: number, right = false) => {
  const point = port(index, count, right);
  return { left: `${point.x}px`, top: `${point.y}px` };
};
function stateIcon(label: string) {
  if (label === t('connectionError')) return CircleAlert;
  if (label === t('disabled')) return CirclePause;
  if ([t('connectingLabel'), t('preparing')].includes(label)) return LoaderCircle;
  if ([t('connected'), t('agentAvailable')].includes(label)) return CircleCheck;
  return CircleDashed;
}
// Match the cubic wire, one third inward from each outer node.
const markerPoint = (index: number, count: number, right = false) => {
  const u = right ? 2 / 3 : 1 / 3;
  const blend = 3 * u * u - 2 * u * u * u;
  const { start, end, c1, c2 } = wire(index, count, right);
  const x = (1-u)**3 * start.x + 3 * (1-u)**2 * u * c1 + 3 * (1-u) * u*u * c2 + u**3 * end.x;
  const top = start.y + (end.y - start.y) * blend;
  return { x, y: top };
};
const marker = (index: number, count: number, right = false) => {
  const point = markerPoint(index, count, right);
  return { left: `${point.x}px`, top: `${point.y}px` };
};
</script>
<template>
  <section class="connection-core" :aria-label="t('connectionStatus')">
    <div class="core-toggle"><button class="connection-action" :class="{'is-on':switchOn,'is-busy':progressing}" :disabled="progressing || loading || !status" :aria-busy="progressing" :aria-label="progressing ? t('pleaseWait') : running ? t('disconnect') : t('connect')" @click="toggle"><span class="connection-switch-track" aria-hidden="true"><span class="connection-switch-state">{{ switchOn ? 'ON' : 'OFF' }}</span><span class="connection-knob" @transitionrun="watchSwitchImpact" @transitionend="stopImpactWatch" @transitioncancel="stopImpactWatch"><LoaderCircle v-if="progressing"/><Power v-else/></span></span></button><span class="connection-sparks" aria-hidden="true"><span v-for="spark in sparks" :key="spark.id" class="connection-spark" :style="spark.style" @animationend="removeSpark(spark.id)"/></span></div>
    <div class="connection-graph" :style="{height:`${height}px`}">
      <div class="graph-nodes sources">
        <NTooltip v-if="!sources.length" :show="!dialog && !agentsDialog && nodeTip==='default-source'" placement="top"><template #trigger><button class="graph-node" aria-label="ChatGPT" @mouseenter="nodeTip='default-source'" @mouseleave="nodeTip=''" @focus="nodeTip='default-source'" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="openSource()"><SourceIcon platform="chatgpt"/></button></template>ChatGPT</NTooltip>
        <NTooltip v-for="source in sources" :key="source.id" :show="!dialog && !agentsDialog && nodeTip==='source:'+source.id" placement="top"><template #trigger><button class="graph-node" :aria-label="source.name" :aria-disabled="controlSourceClick === 'service' && !controlSource(source.controlSource)?.homeUrl" @mouseenter="nodeTip='source:'+source.id" @mouseleave="nodeTip=''" @focus="nodeTip='source:'+source.id" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="openSource(source)"><SourceIcon :platform="source.controlSource"/></button></template>{{source.name}}</NTooltip>
        <NTooltip :show="!dialog && !agentsDialog && nodeTip==='add-source'" placement="top"><template #trigger><button class="graph-node add-source" :aria-label="t('addControlSource')" @mouseenter="nodeTip='add-source'" @mouseleave="nodeTip=''" @focus="nodeTip='add-source'" @blur="nodeTip=''" @keydown.esc="nodeTip=''" @click="edit()"><span class="source-icon"><Plus/></span></button></template>{{t('addControlSource')}}</NTooltip>
      </div>
      <div ref="leftWires" class="graph-wires left"><svg :viewBox="`0 0 ${wireWidth(false)} ${height}`" preserveAspectRatio="none" aria-hidden="true"><path v-for="(source,index) in sources" :key="source.id" :d="path(index,Math.max(1,sources.length)+1,false,true)" :class="{hovered:activeTip===source.id,live:source.running,failed:['error','degraded'].includes(source.state)}"/><path v-if="!sources.length" :d="path(0,2)"/><path :d="path(Math.max(1,sources.length),Math.max(1,sources.length)+1)" class="placeholder"/></svg><span v-for="(source,index) in sources" :key="'port:'+source.id" class="wire-port" :class="{live:source.running,failed:['error','degraded'].includes(source.state)}" :style="portStyle(index,Math.max(1,sources.length)+1)" aria-hidden="true"/><span v-if="!sources.length" class="wire-port" :style="portStyle(0,2)" aria-hidden="true"/><span class="wire-port utility-port" :style="portStyle(Math.max(1,sources.length),Math.max(1,sources.length)+1)" aria-hidden="true"/><NTooltip v-for="(source,index) in sources" :key="source.id" :show="!dialog && !agentsDialog && activeTip===source.id && stateIcon(sourceState(source))!==CircleCheck" placement="top"><template #trigger><button  type="button" class="wire-status" :class="{hovered:activeTip===source.id,live:source.running,failed:['error','degraded'].includes(source.state)}" :style="marker(index,Math.max(1,sources.length)+1)" :aria-label="`${source.name}: ${sourceState(source)} · ${t('viewConnectionDetails')}`" aria-haspopup="dialog" @click="view(source)" @mouseenter="activeTip=source.id" @mouseleave="activeTip=''" @focus="activeTip=source.id" @blur="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(sourceState(source))" :class="{spinning:stateIcon(sourceState(source))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{sourceState(source)}}<div class="status-hint">{{t('viewConnectionDetails')}}</div></div></NTooltip></div>
      <div class="graph-brain"><PlayfulMascot/></div>
      <div class="agent-side">
      <div ref="rightWires" class="graph-wires right" :aria-busy="inventory.loading.value && !inventory.loaded.value"><svg v-if="inventory.loading.value && !inventory.loaded.value" class="search-wires" viewBox="0 0 200 120" preserveAspectRatio="none" aria-hidden="true"><path v-for="n in 3" :key="n" :style="{'--drift-delay':(n-1)*-.8+'s'}" d="M 0 60 C 90 60, 110 36, 200 36"/></svg><svg :viewBox="`0 0 ${wireWidth(true)} ${height}`" preserveAspectRatio="none" aria-hidden="true"><path class="agent-wire" v-for="(agent,index) in agents" :key="agent.agent" :d="path(index,agents.length+1,true,true)" :class="{hovered:activeTip===agent.agent,live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}"/><path :d="path(agents.length,agents.length+1,true)" class="more-wire" :class="{hovered:nodeTip==='more-agents'}"/></svg><span v-for="(agent,index) in agents" :key="'port:'+agent.agent" class="wire-port" :class="{live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}" :style="portStyle(index,agents.length+1,true)" aria-hidden="true"/><span class="wire-port utility-port" :style="portStyle(agents.length,agents.length+1,true)" aria-hidden="true"/><NTooltip v-for="(agent,index) in agents" :key="agent.agent" :show="!dialog && !agentsDialog && activeTip===agent.agent && stateIcon(agentState(agent))!==CircleCheck" placement="top"><template #trigger><button  type="button" class="wire-status agent-marker" :class="{hovered:activeTip===agent.agent,live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}" :style="marker(index,agents.length+1,true)" :aria-label="`${name(agent)}: ${agentState(agent)} · ${t('manageAgents')}`" @click="showAgents" @mouseenter="activeTip=agent.agent" @mouseleave="activeTip=''" @focus="activeTip=agent.agent" @blur="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(agentState(agent))" :class="{spinning:stateIcon(agentState(agent))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{agentState(agent)}}<div class="status-hint">{{t('manageAgents')}}</div></div></NTooltip></div>
      <div class="graph-nodes agents"><NTooltip v-for="agent in agents" :key="agent.agent" :show="!dialog && !agentsDialog && nodeTip==='agent:'+agent.agent" placement="top"><template #trigger><button type="button" class="graph-node" @click="openAgent(agent)" :aria-label="name(agent)" tabindex="0" @mouseenter="nodeTip='agent:'+agent.agent" @mouseleave="nodeTip=''" @focus="nodeTip='agent:'+agent.agent" @blur="nodeTip=''" @keydown.esc="nodeTip=''"><span v-if="icons[agent.agent]" class="source-icon agent-brand" aria-hidden="true" v-html="icons[agent.agent]"/><Bot v-else aria-hidden="true"/></button></template><div>{{name(agent)}}<div v-if="agent.version" class="status-hint">{{agent.version}}</div></div></NTooltip><NTooltip placement="top"><template #trigger><button type="button" class="graph-node agents-more" data-page="agents" :aria-label="t('manageAgents')" @click="showAgents" @mouseenter="nodeTip='more-agents'" @mouseleave="nodeTip=''" @focus="nodeTip='more-agents'" @blur="nodeTip=''"><Ellipsis aria-hidden="true"/></button></template>{{t('manageAgents')}}</NTooltip></div>
      </div>
    </div>
    <p v-if="connectionError || inventory.error.value" class="graph-error" role="alert">{{connectionError || inventory.error.value}}</p>
  </section>
  <AgentsSettings v-if="agentsDialog" :origin="agentOrigin" @close="closeAgents"/>
  <IngressDialog v-if="dialog" :origin="sourceOrigin" :ingress="editing" :default-chat-g-p-t="defaultChatGPT" @close="closeSource" @saved="view($event, true)"/>
  <IngressDetailsDialog v-if="details" :ingress="details" :auto-connect="connectAfterSave" @close="details=undefined" @edit="editDetails"/>
</template>
<style scoped>
.connection-core{--graph-bg:var(--scene-color,light-dark(#e3eee8,#20352f));background:var(--graph-bg);border:1px solid light-dark(#cfdfd6,#354c43);border-radius:24px;padding:34px 32px 28px;overflow:hidden}.core-toggle{display:flex;align-items:center;flex-direction:column;gap:12px}.connection-graph{width:100%;max-width:1100px;align-self:center;grid-template-rows:minmax(0,1fr);margin-top:30px;display:grid;grid-template-columns:minmax(80px,.75fr) minmax(110px,1.25fr) 190px minmax(80px,1fr) minmax(110px,1fr);align-items:center}.graph-nodes{display:flex;flex-direction:column;justify-content:center;gap:40px;z-index:2}.graph-node{width:44px;height:44px;flex-shrink:0;display:flex;align-items:center;justify-content:center;padding:0;border-radius:12px;min-width:0;background:transparent}.graph-node{border:0;box-shadow:none}.graph-node:hover{background:transparent}.graph-node>:first-child{transition:transform 160ms ease}.graph-node:hover>:first-child{transform:scale(1.1)}.graph-node:focus-visible{outline:2px solid #87ad99;outline-offset:0}.sources{align-items:flex-end}.sources .graph-node{margin-right:4px;padding:0;justify-content:center;background:transparent;border:0;box-shadow:none}.sources .graph-node:hover{background:transparent}.sources .source-icon{transition:transform 160ms ease}.graph-node img,.source-icon{width:38px;height:38px;flex-shrink:0;border-radius:12px;display:grid;place-items:center;background:transparent}.graph-node img{object-fit:contain}.source-icon b{font-family:Georgia,serif;font-size:27px}.source-icon svg{width:23px;height:23px}.agent-brand :deep(svg){width:32px;height:32px}.add-source{color:var(--muted);background:transparent}.add-source .source-icon{background:transparent;border:1px dashed currentColor}.graph-brain{display:flex;flex-direction:column;align-items:center;gap:18px;position:relative;z-index:3}.graph-brain:before{content:'';position:absolute;width:260px;height:260px;border:1px solid light-dark(#cedfd470,#ffffff08);border-radius:50%;top:50%;left:50%;transform:translate(-50%,-50%);pointer-events:none}.graph-wires{height:100%;min-height:0;position:relative}.graph-wires>svg{position:absolute;inset:0;display:block}.graph-wires svg{width:100%;height:100%;overflow:visible}.graph-wires path{fill:none;stroke:light-dark(#abbfb4,#60796b);stroke-width:1.4;vector-effect:non-scaling-stroke}.graph-wires path.live{stroke:#5c9d80;stroke-width:2}.graph-wires path.failed{stroke:#be826b}.graph-wires path.placeholder{stroke-dasharray:4 5;opacity:.65}.wire-status{padding:0;border:0;box-shadow:none;cursor:pointer;position:absolute;transform:translate(-50%,-50%);width:26px;height:26px;display:grid;place-items:center;color:var(--muted);background:transparent;border-radius:50%}.wire-status svg{width:17px;height:17px}.wire-status.live{color:var(--green)}.wire-status.failed,.graph-error{color:#b66b54}.graph-error{font-size:12px}.agent-side{min-height:0;grid-template-rows:minmax(0,1fr);--agent-icon-size:38px;grid-column:4 / 6;display:grid;height:100%;grid-template-columns:1fr;align-items:center}.agent-side>.right{grid-area:1 / 1;width:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agent-side>.agents{grid-area:1 / 1;margin-left:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agents{align-items:flex-start;gap:46px}.agents .graph-node{width:38px;height:38px;margin-left:12px}@media(max-width:800px){.connection-core{padding:26px 16px}.connection-graph{grid-template-columns:minmax(64px,.75fr) minmax(88px,1.05fr) 124px minmax(60px,.8fr) minmax(92px,1fr)}.graph-brain :deep(.mascot){width:120px;height:120px}.graph-brain:before{width:160px;height:160px}.graph-node img,.source-icon{width:30px;height:30px}.agent-side{--agent-icon-size:30px}.agents{gap:54px}.agents .graph-node{width:30px;height:30px}}
.wire-port{position:absolute;width:8px;height:8px;box-sizing:border-box;transform:translate(-50%,-50%);border:1.5px solid light-dark(#8da69a,#789486);border-radius:50%;background:var(--graph-bg);box-shadow:0 0 0 3px var(--graph-bg);pointer-events:none;z-index:2}
.wire-port.live{border-color:var(--green)}
.wire-port.failed{border-color:#be826b}
.wire-port.utility-port{opacity:.65}
.agents .agents-more{border:1px dashed var(--muted);border-radius:50%;color:var(--ink)}
.spinning{animation:status-spin 1s linear infinite}
@keyframes status-spin{to{rotate:360deg}}
@media(prefers-reduced-motion:reduce){.spinning{animation:none}}

.sources .source-icon{width:44px;height:44px}.sources :deep(.platform-icon){width:38px;height:38px}.sources .graph-node:hover :deep(.platform-icon){transform:scale(1.1)}.sources .add-source .source-icon{width:34px;height:34px}.sources .add-source .source-icon>svg{width:20px;height:20px}.source-brand :deep(svg){width:40px;height:40px}.sources .source-icon>svg{width:30px;height:30px}
.wire-status:hover,.wire-status:focus-visible{z-index:5;background:var(--graph-bg)}
.status-hint{font-size:11px;opacity:.7}
@media(prefers-reduced-motion:reduce){.graph-node>:first-child{transition:none}}
.graph-wires .search-wires{position:absolute;top:50%;left:0;width:100%;height:100px;transform:translateY(-50%);pointer-events:none}
.search-wires path{stroke-linecap:round;animation:search-drift 2.8s ease-in-out infinite;animation-delay:var(--drift-delay);opacity:.55}
@keyframes search-drift{0%,100%{d:path("M 0 60 C 90 60, 110 36, 200 36");opacity:.3}50%{d:path("M 0 60 C 90 60, 110 84, 200 84");opacity:.75}}
@media(prefers-reduced-motion:reduce){.search-wires path{animation:none}}

.wire-status::before{content:'';position:absolute;inset:-12px;border-radius:50%}
.wire-status{transition:filter 180ms ease,box-shadow 180ms ease}
.wire-status:hover,.wire-status:focus-visible{filter:brightness(.82);box-shadow:0 5px 12px #15342a28}
.graph-wires path{transition:filter 180ms ease}
.graph-wires path.hovered{filter:brightness(.78) drop-shadow(0 5px 3px #15342a30)}
@media(prefers-reduced-motion:reduce){.wire-status,.graph-wires path{transition:none}}
</style>
