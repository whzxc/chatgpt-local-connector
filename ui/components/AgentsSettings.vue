<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { computed, onMounted } from 'vue';
import { NButton, NSwitch } from 'naive-ui';
import { Bot } from '@lucide/vue';
import { ref } from 'vue';
import { useAgents, icons, name, type Agent } from '../composables/useAgents';
const { agents, loaded, loading, saving, error, refresh, toggle } = useAgents();
import SettingsGroup from './SettingsGroup.vue';
const expanded = ref(false);
const initialAgent: Agent = { agent: 'codex', displayName: 'Codex' };
const sorted = computed(() => [...agents.value]
  .sort((a, b) => Number(b.agent === 'codex') - Number(a.agent === 'codex') || Number(!!b.installed) - Number(!!a.installed) || name(a).localeCompare(name(b))));
const collapsed = computed(() => sorted.value.filter(agent => agent.installed).slice(0, 3));
const visible = computed(() => !loaded.value ? [initialAgent] : expanded.value ? sorted.value : collapsed.value);
const canExpand = computed(() => loaded.value && sorted.value.length > collapsed.value.length);
onMounted(refresh);
</script>
<template>
  <SettingsGroup title="Agents">
    <template #heading-actions>
      <NButton class="agent-refresh" :disabled="loading || !!saving" :aria-label="t('refreshAgents')" :title="t('refreshAgents')" @click="refresh">
        {{ loading ? t('refreshing') : t('refresh') }}
      </NButton>
    </template>
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
    <div v-if="canExpand" class="agent-expand">
      <NButton :aria-expanded="expanded" @click="expanded = !expanded">{{ expanded ? t('collapseList') : t('showAllValue', { count: sorted.length }) }}</NButton>
    </div>
    <p v-if="error" class="agent-error" role="alert">{{displayMessage(error)}}</p>
  </SettingsGroup>
</template>
<style scoped>
.agent-row { display: flex; align-items: center; gap: 16px; min-height: 66px; margin: 0 16px; padding: 12px 0; }
.agent-row + .agent-row { border-top: 1px solid var(--line); }
.agent-info { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; margin: 0; }
.agent-icon { display: flex; width: 22px; height: 22px; flex-shrink: 0; }
.agent-icon :deep(svg) { width: 100%; height: 100%; }
.agent-fallback { flex-shrink: 0; }
.agent-copy { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.agent-name { font-size: 13px; font-weight: 500; line-height: 20px; }
.agent-default { color: var(--muted); font-weight: 400; }
.agent-version { font-size: 12px; line-height: 18px; color: var(--muted); overflow-wrap: anywhere; }
.agent-refresh { height: 28px; font-size: 12px; }
.agent-expand { display: flex; justify-content: center; margin: 0 16px; padding: 10px 0; border-top: 1px solid var(--line); }
.agent-error { color: #a46651; font-size: 12px; padding: 0 16px 10px; margin: 0; }
</style>
