<script setup lang="ts">
import { maxVisibleAgents } from '../subscriptions/displayPreferences';
import { NButton, NTooltip } from 'naive-ui';
import { openUrl } from '../platform';
import githubIcon from '../assets/agents/copilot.svg?raw';
import { useElementSize, useMediaQuery } from '@vueuse/core';
import { computed, inject, ref, watch, onMounted, onUnmounted } from 'vue';
import { ChartNoAxesCombined, Ellipsis, Plus, Power, LoaderCircle, CircleCheck, CircleDashed, CircleAlert, CirclePause } from '@lucide/vue';
import { t } from '../i18n';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { useAgents, name, type Agent } from '../composables/useAgents';
import PlayfulMascot from './PlayfulMascot.vue';
import SourceIcon from './SourceIcon.vue';
import IngressDetailsDialog from './IngressDetailsDialog.vue';
import IngressDialog from './IngressDialog.vue';
import AgentsSettings from './AgentsSettings.vue';
import AgentDetails from './AgentDetails.vue';
import AgentQuotaIcon from '../subscriptions/AgentQuotaIcon.vue';
import { subscriptionSnapshotKey } from '../subscriptions/useSubscriptions';
import { requestedProvider } from '../subscriptions/navigation';
const emit = defineEmits<{ usage: [event: MouseEvent] }>();
const subscriptions = inject(subscriptionSnapshotKey);
const selectedAgent = ref<Agent>();
const detailOrigin = ref<{x:number;y:number;size:number}>();
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
const { status, busy, loading, connectionError, run, refresh, notify } = useConnector();
const openRepository = () => openUrl('https://github.com/whzxc/chatgpt-local-connector').catch(cause => notify(String(cause), true));
const inventory = useAgents();
const activeTip = ref('');
const nodeTip = ref('');
function openSource(source: Ingress | undefined, event: MouseEvent) {
  nodeTip.value = '';
  source ? view(source, false, event) : edit(undefined, true, event);
}
const agents = computed(() => {
  const railOrder = subscriptions?.value?.providers.filter(p => p.eligible && p.selected).map(p => p.agentId) || [];
  return inventory.orderedAgents.value.filter(a => a.installed && a.enabled)
    .sort((a, b) => {
      const ai = railOrder.indexOf(a.agent), bi = railOrder.indexOf(b.agent);
      return (ai < 0 ? Infinity : ai) - (bi < 0 ? Infinity : bi);
    })
    .slice(0, [4,6,8,10].includes(maxVisibleAgents.value) ? maxVisibleAgents.value : 4);
});
const sources = computed(() => [...(status.value?.ingresses || [])].sort((a,b) => Number(b.controlSource==='chatgpt')-Number(a.controlSource==='chatgpt') || a.name.localeCompare(b.name)));
const running = computed(() => sources.value.some(i => i.running));
const connectionTransition = ref<'connecting' | 'stopping'>();
const mascotState = computed(() => {
  if (connectionError.value) return 'unavailable';
  if (!status.value) return 'connecting';
  const enabled = sources.value.filter(source => source.enabled);
  // Only connection operations affect the mascot, not unrelated UI actions.
  if (connectionTransition.value) return connectionTransition.value;
  if (enabled.some(source => source.state === 'stopping')) return 'stopping';
  if (enabled.some(source => source.state === 'starting')) return 'connecting';
  if (enabled.some(source => ['error', 'degraded'].includes(source.state))) return running.value ? 'degraded' : 'error';
  if (!running.value) return 'offline';
  if (status.value.core.activeTurns > 0 || status.value.core.activeWrites > 0) return 'working';
  return 'connected';
});
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
function edit(ingress?: Ingress, chatgpt=false, event?: MouseEvent) {
  nodeTip.value=''; activeTip.value='';
  const rect=((event?.currentTarget as HTMLElement | undefined) ?? document.querySelector('.add-source'))?.getBoundingClientRect();
  if(rect) sourceOrigin.value={x:rect.x,y:rect.y,size:rect.width}; editing.value=ingress; defaultChatGPT.value=chatgpt; dialog.value=true; }
const details = ref<Ingress>(), connectAfterSave = ref(false);
const sourceDetailOrigin = ref<{x:number;y:number;size:number;height:number}>();
function view(ingress: Ingress, connect=false, event?: MouseEvent) {
  nodeTip.value=''; activeTip.value='';
  const rect=(event?.currentTarget as HTMLElement | undefined)?.getBoundingClientRect();
  sourceDetailOrigin.value=rect ? {x:rect.x,y:rect.y,size:rect.width,height:rect.height} : undefined;
  dialog.value=false; details.value=ingress; connectAfterSave.value=connect;
}
function editDetails(ingress: Ingress) { details.value=undefined; edit(ingress); }
function sourceState(i: Ingress) { return !i.enabled ? t('disabled') : ['error','degraded'].includes(i.state) ? t('connectionError') : ['starting','stopping'].includes(i.state) ? t('connectingLabel') : i.running ? t('connected') : t('disconnected'); }
function agentState(a: Agent) {
  if (a.enabled === false) return t('disabled');
  if (a.agent==='codex') { const state=status.value?.autoOpenCodex===false ? status.value?.core.appServer?.state : status.value?.core.desktop?.state; if(state==='ready') return t('connected'); if(state==='error') return t('connectionError'); if(state==='connecting') return t('preparing'); }
  return a.status==='ready' ? t('connected') : a.installed && a.available ? t('agentAvailable') : t('connectionError');
}
function openAgent(agent: Agent, event?: MouseEvent) {
  nodeTip.value=''; activeTip.value='';
  const rect=(event?.currentTarget as HTMLElement | undefined)?.getBoundingClientRect();
  detailOrigin.value=rect ? {x:rect.x,y:rect.y,size:rect.width} : undefined;
  selectedAgent.value=agent;
}
watch([requestedProvider, () => subscriptions?.value, inventory.orderedAgents], () => {
  const id=requestedProvider.value;
  if(id===undefined)return;
  if(!id){requestedProvider.value=undefined;showAgents();return;}
  const provider=subscriptions?.value?.providers.find(p=>p.providerId===id);
  const agent=inventory.orderedAgents.value.find(a=>a.agent===provider?.agentId) ?? (provider ? {agent:provider.agentId,displayName:provider.name,installed:provider.eligible} : undefined);
  if(agent){requestedProvider.value=undefined;openAgent(agent);}
}, {immediate:true});
async function toggle() {
  if (busy.value) return;
  if (!running.value && (!sources.value.length || sources.value.every(i => !i.config.configured))) { edit(sources.value[0],true); return; }
  connectionTransition.value = running.value ? 'stopping' : 'connecting';
  try {
    await run('ingress-all',async()=> { const result=await api<{results:{id:string;ok:boolean;error?:string}[]}>(`ingress/${running.value?'stop-all':'start-all'}`,'POST'); await refresh(); const errors=result.results.filter(r=>!r.ok); if(errors.length) throw new Error(errors.map(r=>`${sources.value.find(i=>i.id===r.id)?.name || r.id}: ${r.error}`).join('\n')); });
  } finally { connectionTransition.value = undefined; }
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
const core = ref<HTMLElement>();
const { width: coreWidth, height: coreHeight } = useElementSize(core, { width: 1000, height: 650 }, { box: 'border-box' });
const ringRadius = computed(() => Math.max(80, Math.min(130, coreWidth.value * .13, coreHeight.value * .2)));
const coreColumnWidth = computed(() => 124 + (ringRadius.value - 80) * 1.32);
const graphSizeStyle = computed(() => ({
  '--graph-ring-radius': `${ringRadius.value}px`,
  '--graph-core-width': `${coreColumnWidth.value}px`,
}));
const ringOffset = ref({ x: 0, y: 0 });
function pullRing(position: { x: number; y: number }) {
  ringOffset.value = { x: position.x / 3, y: position.y / 3 };
}
const ringStyle = computed(() => ({
  '--ring-offset-x': `${ringOffset.value.x}px`,
  '--ring-offset-y': `${ringOffset.value.y}px`,
}));
const wireWidth = (right: boolean) => Math.max(1, right ? rightWidth.value : leftWidth.value);
// Distribute individual ports along the central ring, including utility branches.
function port(index: number, count: number, right = false) {
  const radius = ringRadius.value;
  const halfCore = coreColumnWidth.value / 2;
  const offset = count <= 1 ? 0 : (index / (count - 1) - .5) * Math.min((count - 1) * 24, 96);
  const reach = Math.sqrt(radius * radius - offset * offset) - halfCore;
  return { x: (right ? reach : wireWidth(false) - reach) + ringOffset.value.x, y: height.value / 2 + offset + ringOffset.value.y };
}
function wire(index: number, count: number, right = false) {
  const inner = port(index, count, right);
  const start = right ? inner : { x: 0, y: y(index, count) };
  const end = right ? { x: wireWidth(true), y: y(index, count) } : inner;
  const span = end.x - start.x;
  return { start, end, c1: start.x + span * .45, c2: start.x + span * .55 };
}
function wirePoint(curve: ReturnType<typeof wire>, u: number) {
  const { start, end, c1, c2 } = curve;
  return {
    x: (1-u)**3*start.x + 3*(1-u)**2*u*c1 + 3*(1-u)*u*u*c2 + u**3*end.x,
    y: start.y + (end.y-start.y)*(3*u*u-2*u*u*u),
  };
}
// The horizontal status positions define a shared circle in graph coordinates.
function markerParameter(curve: ReturnType<typeof wire>, right: boolean) {
  const rightOrigin = wireWidth(false) + coreColumnWidth.value;
  const leftAnchor = wirePoint(wire(0, 1), 1/3).x;
  const rightAnchor = rightOrigin + wirePoint(wire(0, 1, true), 2/3).x;
  const centerX = (leftAnchor + rightAnchor)/2;
  const radius = (rightAnchor - leftAnchor)/2;
  let inner = right ? 0 : 1, outer = right ? 1 : 0;
  for (let n = 0; n < 30; n++) {
    const u = (inner + outer)/2;
    const point = wirePoint(curve, u);
    const distance = Math.hypot(point.x + (right ? rightOrigin : 0) - centerX, point.y - height.value/2);
    if (distance < radius) inner = u;
    else outer = u;
  }
  return (inner + outer)/2;
}
function path(index: number, count: number, right = false, gap = false) {
  const curve = wire(index, count, right);
  const { start, end, c1, c2 } = curve;
  const point = (u: number) => wirePoint(curve, u);
  const tangent = (u: number) => ({
    x: 3*(1-u)**2*(c1-start.x)+6*(1-u)*u*(c2-c1)+3*u*u*(end.x-c2),
    y: 6*u*(1-u)*(end.y-start.y),
  });
  const segment = (a: number, b: number) => {
    const p = point(a), q = point(b), da = tangent(a), db = tangent(b), span = (b-a)/3;
    return `M ${p.x} ${p.y} C ${p.x+span*da.x} ${p.y+span*da.y}, ${q.x-span*db.x} ${q.y-span*db.y}, ${q.x} ${q.y}`;
  };
  if (!gap) return segment(0, 1);
  // Keep the curve gap centered on the same circle intersection as the icon.
  const center = markerParameter(curve, right), marker = point(center);
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
const markerPoint = (index: number, count: number, right = false) => {
  const curve = wire(index, count, right);
  return wirePoint(curve, markerParameter(curve, right));
};
const marker = (index: number, count: number, right = false) => {
  const point = markerPoint(index, count, right);
  return { left: `${point.x}px`, top: `${point.y}px` };
};
</script>
<template>
  <section ref="core" class="connection-core" :style="graphSizeStyle" :aria-label="t('connectionStatus')">
    <div class="core-toggle"><button class="connection-action" :class="{'is-on':switchOn,'is-busy':progressing}" :disabled="progressing || loading || !status" :aria-busy="progressing" :aria-label="progressing ? t('pleaseWait') : running ? t('disconnect') : t('connect')" @click="toggle"><span class="connection-switch-track" aria-hidden="true"><span class="connection-switch-state">{{ switchOn ? 'ON' : 'OFF' }}</span><span class="connection-knob" @transitionrun="watchSwitchImpact" @transitionend="stopImpactWatch" @transitioncancel="stopImpactWatch"><LoaderCircle v-if="progressing"/><Power v-else/></span></span></button><span class="connection-sparks" aria-hidden="true"><span v-for="spark in sparks" :key="spark.id" class="connection-spark" :style="spark.style" @animationend="removeSpark(spark.id)"/></span></div>
    <div class="connection-graph" :style="{height:`${height}px`}">
      <div class="graph-nodes sources">
        <NTooltip v-if="!sources.length" :show="!dialog && !agentsDialog && !selectedAgent && nodeTip==='default-source'" placement="right"><template #trigger><button data-panel-anchor class="graph-node" aria-label="ChatGPT" @mouseenter="nodeTip='default-source'" @mouseleave="nodeTip=''" @keydown.esc="nodeTip=''" @click="openSource(undefined, $event)"><SourceIcon platform="chatgpt"/></button></template>ChatGPT</NTooltip>
        <NTooltip v-for="source in sources" :key="source.id" :show="!dialog && !agentsDialog && !selectedAgent && nodeTip==='source:'+source.id" placement="right"><template #trigger><button data-panel-anchor class="graph-node" :aria-label="source.name" @mouseenter="nodeTip='source:'+source.id" @mouseleave="nodeTip=''" @keydown.esc="nodeTip=''" @click="openSource(source, $event)"><SourceIcon :platform="source.controlSource"/></button></template>{{source.name}}</NTooltip>
        <NTooltip :show="!dialog && !agentsDialog && !selectedAgent && nodeTip==='add-source'" placement="right"><template #trigger><button data-panel-anchor class="graph-node add-source" :aria-label="t('addControlSource')" @mouseenter="nodeTip='add-source'" @mouseleave="nodeTip=''" @keydown.esc="nodeTip=''" @click="edit(undefined, false, $event)"><span class="source-icon"><Plus/></span></button></template>{{t('addControlSource')}}</NTooltip>
      </div>
      <div ref="leftWires" class="graph-wires left"><svg :viewBox="`0 0 ${wireWidth(false)} ${height}`" preserveAspectRatio="none" aria-hidden="true"><path v-for="(source,index) in sources" :key="source.id" :d="path(index,Math.max(1,sources.length)+1,false,true)" :class="{hovered:activeTip===source.id,live:source.running,failed:['error','degraded'].includes(source.state)}"/><path v-if="!sources.length" :d="path(0,2)"/><path :d="path(Math.max(1,sources.length),Math.max(1,sources.length)+1)" class="placeholder"/></svg><span v-for="(source,index) in sources" :key="'port:'+source.id" class="wire-port" :class="{live:source.running,failed:['error','degraded'].includes(source.state)}" :style="portStyle(index,Math.max(1,sources.length)+1)" aria-hidden="true"/><span v-if="!sources.length" class="wire-port" :style="portStyle(0,2)" aria-hidden="true"/><span class="wire-port utility-port" :style="portStyle(Math.max(1,sources.length),Math.max(1,sources.length)+1)" aria-hidden="true"/><NTooltip v-for="(source,index) in sources" :key="source.id" :show="!dialog && !agentsDialog && !selectedAgent && activeTip===source.id && stateIcon(sourceState(source))!==CircleCheck" placement="right"><template #trigger><button  type="button" data-panel-anchor class="wire-status" :class="{hovered:activeTip===source.id,live:source.running,failed:['error','degraded'].includes(source.state)}" :style="marker(index,Math.max(1,sources.length)+1)" :aria-label="`${source.name}: ${sourceState(source)} · ${t('viewConnectionDetails')}`" aria-haspopup="dialog" @click="view(source, false, $event)" @mouseenter="activeTip=source.id" @mouseleave="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(sourceState(source))" :class="{spinning:stateIcon(sourceState(source))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{sourceState(source)}}<div class="status-hint">{{t('viewConnectionDetails')}}</div></div></NTooltip></div>
      <div class="graph-brain" :style="ringStyle">
        <PlayfulMascot :state="mascotState" @displacement="pullRing"/>
        <NButton class="graph-ring-action graph-github" circle :bordered="false" :aria-label="'GitHub · whzxc/chatgpt-local-connector'" @click="openRepository">
          <span class="github-icon" v-html="githubIcon" aria-hidden="true"/>
        </NButton>
        <NButton data-panel-anchor class="graph-ring-action graph-usage" circle :bordered="false" :aria-label="t('trayUsage')" aria-haspopup="dialog" @click="emit('usage', $event)">
          <ChartNoAxesCombined :size="22" aria-hidden="true"/>
        </NButton>
      </div>
      <div class="agent-side">
      <div ref="rightWires" class="graph-wires right" :aria-busy="inventory.loading.value && !inventory.loaded.value"><svg v-if="inventory.loading.value && !inventory.loaded.value" class="search-wires" viewBox="0 0 200 120" preserveAspectRatio="none" aria-hidden="true"><path v-for="n in 3" :key="n" :style="{'--drift-delay':(n-1)*-.8+'s'}" d="M 0 60 C 90 60, 110 36, 200 36"/></svg><svg :viewBox="`0 0 ${wireWidth(true)} ${height}`" preserveAspectRatio="none" aria-hidden="true"><path class="agent-wire" v-for="(agent,index) in agents" :key="agent.agent" :d="path(index,agents.length+1,true,true)" :class="{hovered:activeTip===agent.agent,live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}"/><path :d="path(agents.length,agents.length+1,true)" class="more-wire" :class="{hovered:nodeTip==='more-agents'}"/></svg><span v-for="(agent,index) in agents" :key="'port:'+agent.agent" class="wire-port" :class="{live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}" :style="portStyle(index,agents.length+1,true)" aria-hidden="true"/><span class="wire-port utility-port" :style="portStyle(agents.length,agents.length+1,true)" aria-hidden="true"/><NTooltip v-for="(agent,index) in agents" :key="agent.agent" :show="!dialog && !agentsDialog && !selectedAgent && activeTip===agent.agent && stateIcon(agentState(agent))!==CircleCheck" placement="left"><template #trigger><button  type="button" data-panel-anchor class="wire-status agent-marker" :class="{hovered:activeTip===agent.agent,live:[t('connected'),t('agentAvailable')].includes(agentState(agent))}" :style="marker(index,agents.length+1,true)" :aria-label="`${name(agent)}: ${agentState(agent)} · ${t('manageAgents')}`" @click="showAgents" @mouseenter="activeTip=agent.agent" @mouseleave="activeTip=''" @keydown.esc="activeTip=''"><component :is="stateIcon(agentState(agent))" :class="{spinning:stateIcon(agentState(agent))===LoaderCircle}" aria-hidden="true"/></button></template><div>{{agentState(agent)}}<div class="status-hint">{{t('manageAgents')}}</div></div></NTooltip></div>
      <div class="graph-nodes agents"><NTooltip v-for="agent in agents" :key="agent.agent" :show="!dialog && !agentsDialog && !selectedAgent && nodeTip==='agent:'+agent.agent" placement="left"><template #trigger><button type="button" data-panel-anchor class="graph-node" @click="openAgent(agent,$event)" aria-haspopup="dialog" :aria-label="name(agent)" tabindex="0" @mouseenter="nodeTip='agent:'+agent.agent" @mouseleave="nodeTip=''" @keydown.esc="nodeTip=''"><AgentQuotaIcon :agent="agent.agent"/></button></template><div>{{name(agent)}}<div v-if="agent.version" class="status-hint">{{agent.version}}</div></div></NTooltip><NTooltip :show="!dialog && !agentsDialog && !selectedAgent && nodeTip==='more-agents'" placement="left"><template #trigger><button type="button" data-panel-anchor class="graph-node agents-more" data-page="agents" :aria-label="t('manageAgents')" @click="showAgents" @mouseenter="nodeTip='more-agents'" @mouseleave="nodeTip=''"><Ellipsis aria-hidden="true"/></button></template>{{t('manageAgents')}}</NTooltip></div>
      </div>
    </div>
    <p v-if="connectionError || inventory.error.value" class="graph-error" role="alert">{{connectionError || inventory.error.value}}</p>
  </section>
  <AgentDetails v-if="selectedAgent" :agent="selectedAgent" :origin="detailOrigin" @close="selectedAgent=undefined"/>
  <AgentsSettings v-if="agentsDialog" :origin="agentOrigin" @close="closeAgents"/>
  <IngressDialog v-if="dialog" :origin="sourceOrigin" :ingress="editing" :default-chat-g-p-t="defaultChatGPT" @close="closeSource" @saved="view($event, true)"/>
  <IngressDetailsDialog v-if="details" :origin="sourceDetailOrigin" :ingress="details" :auto-connect="connectAfterSave" @close="details=undefined" @edit="editDetails"/>
</template>
<style scoped>
.graph-github{--ring-action-x:-.5}.graph-usage{--ring-action-x:.5}
.graph-brain .graph-ring-action{position:absolute;z-index:4;left:calc(50% + var(--graph-ring-radius)*var(--ring-action-x) + var(--ring-offset-x));top:calc(50% + var(--graph-ring-radius)*.8660254 + var(--ring-offset-y));box-sizing:border-box;width:40px;height:40px;border:var(--graph-line-width) solid var(--accent);border-radius:50%;background:var(--graph-bg);color:var(--accent);opacity:0;pointer-events:none;transform:translate(-50%,-50%) scale(0);transform-origin:center;transition:opacity 120ms ease,transform 320ms cubic-bezier(.2,.8,.2,1.15)}
.graph-brain:hover .graph-ring-action,.graph-brain:focus-within .graph-ring-action{opacity:1;pointer-events:auto;transform:translate(-50%,-50%) scale(1)}
.github-icon{display:flex;width:22px;height:22px;align-items:center;justify-content:center}.github-icon :deep(svg){width:100%;height:100%}
/* Keep the mascot above persistent content and below the shell overlays (29+). */
.graph-brain :deep(.mascot){z-index:20}
@media(prefers-reduced-motion:reduce){.graph-brain .graph-ring-action{transition:none}}
@media(hover:none){.graph-brain .graph-ring-action{opacity:1;pointer-events:auto;transform:translate(-50%,-50%)}}

.connection-core{--graph-line-width:2px;--graph-bg:var(--canvas);background:var(--graph-bg);border:0;border-radius:0;padding:34px 32px 28px;overflow:hidden}.core-toggle{display:flex;align-items:center;flex-direction:column;gap:12px}.connection-graph{width:100%;max-width:1100px;align-self:center;grid-template-rows:minmax(0,1fr);margin-top:30px;display:grid;grid-template-columns:minmax(80px,.75fr) minmax(110px,1.25fr) var(--graph-core-width) minmax(80px,1fr) minmax(110px,1fr);align-items:center}.graph-nodes{display:flex;flex-direction:column;justify-content:center;gap:40px;z-index:2}.graph-node{width:44px;height:44px;flex-shrink:0;display:flex;align-items:center;justify-content:center;padding:0;border-radius:12px;min-width:0;background:transparent}.graph-node{border:0;box-shadow:none}.graph-node:hover{background:transparent}.graph-node>:first-child{transition:transform 160ms ease}.graph-node:hover>:first-child{transform:scale(1.1)}.graph-node:focus-visible{outline:2px solid var(--accent);outline-offset:0}.sources{align-items:flex-end}.sources .graph-node{margin-right:4px;padding:0;justify-content:center;background:transparent;border:0;box-shadow:none}.sources .graph-node:hover{background:transparent}.sources .source-icon{transition:transform 160ms ease}.graph-node img,.source-icon{width:38px;height:38px;flex-shrink:0;border-radius:12px;display:grid;place-items:center;background:transparent}.graph-node img{object-fit:contain}.source-icon b{font-family:Georgia,serif;font-size:27px}.source-icon svg{width:23px;height:23px}.agent-brand :deep(svg){width:32px;height:32px}.add-source{color:var(--muted);background:transparent}.add-source .source-icon{background:transparent;border:1px dashed currentColor}.graph-brain{display:flex;flex-direction:column;align-items:center;gap:18px;position:relative}.graph-brain:before{content:'';position:absolute;z-index:1;box-sizing:content-box;width:calc(var(--graph-ring-radius)*2 - var(--graph-line-width));height:calc(var(--graph-ring-radius)*2 - var(--graph-line-width));border:var(--graph-line-width) solid var(--accent);background:var(--accent-soft);border-radius:50%;top:calc(50% + var(--ring-offset-y));left:calc(50% + var(--ring-offset-x));transform:translate(-50%,-50%);pointer-events:auto}.graph-wires{height:100%;min-height:0;position:relative}.graph-wires>svg{position:absolute;inset:0;display:block}.graph-wires svg{width:100%;height:100%;overflow:visible}.graph-wires path{fill:none;stroke:var(--muted);stroke-width:1.4;vector-effect:non-scaling-stroke}.graph-wires path.live{stroke:var(--accent);stroke-width:var(--graph-line-width)}.graph-wires path.failed{stroke:var(--danger)}.graph-wires path.placeholder{stroke-dasharray:4 5;opacity:.65}.wire-status{padding:0;border:0;box-shadow:none;cursor:pointer;position:absolute;transform:translate(-50%,-50%);width:26px;height:26px;display:grid;place-items:center;color:var(--muted);background:transparent;border-radius:50%}.wire-status svg{width:17px;height:17px}.wire-status.live{color:var(--accent)}.wire-status.failed,.graph-error{color:var(--danger)}.graph-error{font-size:12px}.agent-side{min-height:0;grid-template-rows:minmax(0,1fr);--agent-icon-size:48px;grid-column:4 / 6;display:grid;height:100%;grid-template-columns:1fr;align-items:center}.agent-side>.right{grid-area:1 / 1;width:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agent-side>.agents{grid-area:1 / 1;margin-left:calc(75% - (var(--agent-icon-size) + 12px)/2)}.agents{align-items:flex-start;gap:36px}.agents .graph-node{width:48px;height:48px;margin-left:12px;--agent-quota-size:48px}@media(max-width:800px){.connection-core{padding:26px 16px}.connection-graph{grid-template-columns:minmax(64px,.75fr) minmax(88px,1.05fr) var(--graph-core-width) minmax(60px,.8fr) minmax(92px,1fr)}.graph-node img,.source-icon{width:30px;height:30px}.agent-side{--agent-icon-size:44px}.agents{gap:40px}.agents .graph-node{width:44px;height:44px;--agent-quota-size:44px}}
.wire-port{position:absolute;width:8px;height:8px;box-sizing:border-box;transform:translate(-50%,-50%);border:1.5px solid var(--muted);border-radius:50%;background:var(--graph-bg);pointer-events:none;z-index:2}
.wire-port.live{border-color:var(--accent)}
.wire-port.failed{border-color:var(--danger)}
.wire-port.utility-port{border-color:color-mix(in srgb,var(--muted) 65%,var(--graph-bg))}
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
.wire-status:hover,.wire-status:focus-visible{filter:brightness(.82);box-shadow:0 5px 12px color-mix(in srgb,var(--black) 15.69%,transparent)}
.graph-wires path{transition:filter 180ms ease}
.graph-wires path.hovered{filter:brightness(.78) drop-shadow(0 5px 3px color-mix(in srgb,var(--black) 18.82%,transparent))}
@media(prefers-reduced-motion:reduce){.wire-status,.graph-wires path{transition:none}}
</style>
