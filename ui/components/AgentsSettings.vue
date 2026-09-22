<script setup lang="ts">
import ElasticPanel from './ElasticPanel.vue';
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { computed, onMounted, ref } from 'vue';
import { NButton, NSwitch } from 'naive-ui';
import { Bot, RefreshCw } from '@lucide/vue';
import { useAgents, icons, name, type Agent } from '../composables/useAgents';
const { agents, loaded, loading, saving, error, refresh, toggle } = useAgents();
const props=defineProps<{origin:{x:number;y:number;size:number}}>();
const open=ref(true);



const emit = defineEmits<{ close: [] }>();
const initialAgent: Agent = { agent: 'codex', displayName: 'Codex' };
const sorted = computed(() => [...agents.value]
  .sort((a, b) => Number(b.agent === 'codex') - Number(a.agent === 'codex') || Number(!!b.installed) - Number(!!a.installed) || name(a).localeCompare(name(b))));
const visible = computed(() => !loaded.value ? [initialAgent] : sorted.value);
onMounted(() => { void refresh(); });
</script>
<template>
  <ElasticPanel :show="open" title="Agents" :origin="props.origin" @close="open=false" @closed="emit('close')">
    <template #actions><NButton quaternary circle :disabled="loading || !!saving" :aria-label="t('refreshAgents')" :title="t('refreshAgents')" :aria-busy="loading" @click="refresh(true)"><template #icon><RefreshCw :size="20" :class="{spinning:loading}"/></template></NButton></template>
    <div v-for="agent in visible" :key="agent.agent" class="agent-row">
      <component :is="agent.installed ? 'label' : 'div'" :for="agent.installed ? `agent-${agent.agent}` : undefined" class="agent-info">
        <span v-if="icons[agent.agent]" class="agent-icon" v-html="icons[agent.agent]" aria-hidden="true" />
        <Bot v-else class="agent-fallback" :size="22" aria-hidden="true" />
        <span class="agent-copy">
          <span class="agent-name">{{ name(agent) }}<span v-if="agent.agent === 'codex'" class="agent-default">{{ t('default') }}</span></span>
          <span v-if="agent.installed && agent.version" class="agent-version">{{ agent.version }}</span>
        </span>
      </component>
      <NSwitch v-if="agent.installed" :id="`agent-${agent.agent}`"
        :aria-label="t('allowConnectorToUseValue', { agent: name(agent) })" :value="agent.agent === 'codex' || agent.enabled"
        :disabled="agent.agent === 'codex' || !!saving || loading || typeof agent.enabled !== 'boolean'"
        :title="agent.agent === 'codex' ? t('codexIsAlwaysEnabled') : t('allowConnectorToAcceptRequestsForThisAgent')"
        @update:value="toggle(agent, $event)" />
    </div>
    <p v-if="error" class="agent-error" role="alert">{{displayMessage(error)}}</p>
  </ElasticPanel>
</template>

<style scoped>
.spinning{animation:agents-spin 1s linear infinite}
@keyframes agents-spin{to{transform:rotate(360deg)}}
@media(prefers-reduced-motion:reduce){.spinning{animation:none}}

.agent-row { display: flex; align-items: center; gap: 16px; min-height: 66px; margin: 0; padding: 12px 0; }
.agent-row + .agent-row { border-top: 1px solid var(--line); }
.agent-info { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; margin: 0; }
.agent-icon { display: flex; width: 22px; height: 22px; flex-shrink: 0; }
.agent-icon :deep(svg) { width: 100%; height: 100%; }
.agent-fallback { flex-shrink: 0; }
.agent-copy { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.agent-name { font-size: 13px; font-weight: 500; line-height: 20px; }
.agent-default { color: var(--muted); font-weight: 400; }
.agent-version { font-size: 12px; line-height: 18px; color: var(--muted); overflow-wrap: anywhere; }

.agent-error { color: #a46651; font-size: 12px; padding: 0 0 10px; margin: 0; }
</style>
