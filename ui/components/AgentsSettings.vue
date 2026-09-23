<script setup lang="ts">
import { openUrl } from '../platform';
import ElasticPanel from './ElasticPanel.vue';
import AgentDetails from './AgentDetails.vue';
import AgentDisplaySettings from './AgentDisplaySettings.vue';
import AgentQuotaIcon from '../subscriptions/AgentQuotaIcon.vue';
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { computed, onMounted, ref } from 'vue';
import { NButton, NText, NSwitch } from 'naive-ui';
import { RefreshCw, Settings } from '@lucide/vue';
import { useAgents, agentLinks, name, type Agent } from '../composables/useAgents';
const selectedAgent=ref<Agent>();
const displaySettings=ref(false),settingsOrigin=ref({x:0,y:0,size:28});
function showDisplaySettings(event:MouseEvent){const rect=(event.currentTarget as HTMLElement).getBoundingClientRect();settingsOrigin.value={x:rect.x,y:rect.y,size:rect.width};displaySettings.value=true;}
const detailOrigin=ref({x:0,y:0,size:40});
function showAgent(agent:Agent,event:MouseEvent){const rect=(event.currentTarget as HTMLElement).getBoundingClientRect();detailOrigin.value={x:rect.x,y:rect.y,size:rect.width};selectedAgent.value=agent;}
const { orderedAgents, moveAgent, loaded, loading, saving, error, refresh, toggle } = useAgents();
function adapterFailure(error: string) {
  if (error === 'CLAUDE_ADAPTER_DOWNLOAD_FAILED') return t('agentAdapterDownloadFailed');
  if (error === 'CLAUDE_ADAPTER_CHECKSUM_FAILED') return t('agentAdapterChecksumFailed');
  if (error === 'CLAUDE_ADAPTER_EXTRACT_FAILED') return t('agentAdapterExtractFailed');
  if (error === 'CLAUDE_ADAPTER_PROBE_FAILED') return t('agentAdapterProbeFailed');
  return t('agentAdapterSetupFailed');
}
const props=defineProps<{origin:{x:number;y:number;size:number}}>();
const open=ref(true);



const emit = defineEmits<{ close: [] }>();
const initialAgent: Agent = { agent: 'codex', displayName: 'Codex' };
const visible = computed(() => !loaded.value ? [initialAgent] : orderedAgents.value);
const dragging = ref('');
const dropTarget = ref('');
function dragStart(event: DragEvent, id: string) {
  dragging.value = id;
  if (event.dataTransfer) { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData('text/plain', id); }
}
function finishDrag() { dragging.value = ''; dropTarget.value = ''; }
function drop(id: string) { if (dragging.value) moveAgent(dragging.value, id); finishDrag(); }
function moveWithKey(id: string, direction: number) {
  const index = visible.value.findIndex(agent => agent.agent === id);
  const target = visible.value[index + direction];
  if (target) moveAgent(id, target.agent);
}
onMounted(() => { void refresh(); });
</script>
<template>
  <ElasticPanel :show="open" title="Agents" :origin="props.origin" @close="open=false" @closed="emit('close')">
    <template #title-actions><NButton quaternary circle size="small" :aria-label="t('agentDisplaySettings')" :title="t('agentDisplaySettings')" @click="showDisplaySettings"><template #icon><Settings :size="18"/></template></NButton></template>
    <template #actions><NButton quaternary circle :disabled="loading || !!saving" :aria-label="t('refreshAgents')" :title="t('refreshAgents')" :aria-busy="loading" @click="refresh(true)"><template #icon><RefreshCw :size="20" :class="{spinning:loading}"/></template></NButton></template>
    <div class="agent-grid">
      <article v-for="agent in visible" :key="agent.agent" class="agent-card" :class="{ dragging: dragging===agent.agent, 'drop-target': dropTarget===agent.agent && dragging!==agent.agent }"
        :tabindex="loaded ? 0 : -1" :aria-label="t('reorderAgent', { agent: name(agent) })"
        @keydown.up.self.prevent="moveWithKey(agent.agent,-1)" @keydown.left.self.prevent="moveWithKey(agent.agent,-1)"
        @keydown.down.self.prevent="moveWithKey(agent.agent,1)" @keydown.right.self.prevent="moveWithKey(agent.agent,1)"
        :draggable="loaded" @dragstart="dragStart($event,agent.agent)" @dragend="finishDrag" @dragover.prevent="dropTarget=agent.agent" @drop.prevent="drop(agent.agent)">
          <NSwitch v-if="agent.installed" size="small" class="agent-switch" :id="`agent-${agent.agent}`" :aria-label="t('allowConnectorToUseValue', { agent: name(agent) })" :value="agent.enabled === true"
            :loading="agent.adapter?.state === 'preparing'" :disabled="agent.adapter?.state === 'preparing' || !!saving || loading || typeof agent.enabled !== 'boolean'"
            :title="t('allowConnectorToAcceptRequestsForThisAgent')" @update:value="toggle(agent, $event)" />
        <div class="agent-info">
          <button class="agent-icon" type="button" :aria-label="name(agent)" aria-haspopup="dialog" @click="showAgent(agent,$event)"><AgentQuotaIcon :agent="agent.agent"/></button>
          <span class="agent-copy"><NText v-if="agentLinks[agent.agent]" tag="a" class="agent-name agent-link" :href="agentLinks[agent.agent]" target="_blank" rel="noopener noreferrer" :draggable="false" @click.prevent="openUrl(agentLinks[agent.agent]).catch(cause => error=String(cause))">{{name(agent)}}</NText><span v-else class="agent-name">{{name(agent)}}</span><span v-if="!agent.installed || agent.version" class="agent-version">{{agent.installed ? agent.version : t('agentNotInstalled')}}</span></span>
          <span v-if="agent.installed && agent.adapter && agent.adapter.state !== 'ready'" class="agent-adapter" :role="agent.adapter.state === 'failed' ? 'alert' : 'status'">{{t(agent.adapter.state === 'preparing' ? 'agentAdapterPreparing' : agent.adapter.state === 'failed' ? 'agentAdapterFailed' : 'agentAdapterMissing')}}<span v-if="agent.adapter.error"> {{adapterFailure(agent.adapter.error)}}</span></span>
        </div>
      </article>
    </div>
    <p v-if="error" class="agent-error" role="alert">{{displayMessage(error)}}</p>
  </ElasticPanel>
  <AgentDisplaySettings v-if="displaySettings" :origin="settingsOrigin" @close="displaySettings=false"/>
  <AgentDetails v-if="selectedAgent" :agent="selectedAgent" :origin="detailOrigin" @close="selectedAgent=undefined"/>
</template>

<style scoped>
.spinning{animation:agents-spin 1s linear infinite}
@keyframes agents-spin{to{transform:rotate(360deg)}}
@media(prefers-reduced-motion:reduce){.spinning{animation:none}}

.agent-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:10px;padding:4px 0 12px}
.agent-card{border:1px solid var(--line);border-radius:14px;background:var(--surface);padding:12px;min-width:0;display:grid;grid-template-columns:40px minmax(0,1fr) auto;column-gap:10px;row-gap:4px;align-items:center;align-content:center;transition:border-color 160ms,box-shadow 160ms}
.agent-card[draggable="true"]{cursor:grab}
.agent-card.dragging{opacity:.5}
.agent-card.drop-target{border-color:var(--green);box-shadow:0 0 0 2px var(--green)}
.agent-switch{grid-column:3;grid-row:1}
.agent-card:focus-visible{outline:2px solid var(--green);outline-offset:2px}
.agent-info{display:contents}
.agent-icon{display:flex;width:40px;height:40px;padding:0;border:0;background:transparent;box-shadow:none;cursor:pointer;grid-column:1;grid-row:1 / 3;align-self:center}
.agent-icon :deep(svg){width:100%;height:100%}
.agent-copy{display:contents}
.agent-name{grid-column:2;grid-row:1;min-width:0;font-size:13px;font-weight:500;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.agent-link{text-decoration:none;cursor:pointer}.agent-link:hover{text-decoration:underline;color:var(--green)}
.agent-version{grid-column:2 / 4;grid-row:2;min-width:0;font-size:11px;color:var(--muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
@media(max-width:480px){.agent-grid{grid-template-columns:1fr}}

.agent-adapter{grid-column:2 / 4;grid-row:3;font-size:11px;color:var(--muted);overflow-wrap:anywhere}
.agent-error { color: #a46651; font-size: 12px; padding: 0 0 10px; margin: 0; }
</style>
