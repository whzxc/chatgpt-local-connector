<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { api } from '../composables/useConnector';
import { Bot } from '@lucide/vue';
import codexIcon from '../assets/agents/openai.svg?raw';
import piIcon from '../assets/agents/pi.svg?raw';
import opencodeIcon from '../assets/agents/opencode.svg?raw';
import SettingsGroup from './SettingsGroup.vue';
type Agent = { agent: string; protocol: string; installed: boolean; enabled: boolean; version: string | null };
const agents = ref<Agent[]>([]);
const error = ref('');
const loading = ref(false);
const saving = ref('');
const visible = computed(() => agents.value
  .filter(agent => agent.agent !== 'pi' || agent.installed)
  .sort((a, b) => Number(b.agent === 'codex') - Number(a.agent === 'codex')));
const icons: Record<string, string> = { codex: codexIcon, pi: piIcon, opencode: opencodeIcon };
const name = (agent: Agent) => ({ codex: 'Codex', pi: 'Pi', opencode: 'OpenCode' }[agent.agent] || agent.agent);
async function refresh() {
  loading.value = true;
  try { agents.value = (await api<{ agents: Agent[] }>('agents')).agents; error.value = ''; }
  catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { loading.value = false; }
}
async function toggle(agent: Agent, event: Event) {
  const input = event.target as HTMLInputElement;
  const enabled = input.checked;
  input.checked = agent.enabled;
  saving.value = agent.agent;
  try {
    const result = await api<{ enabled: boolean }>('agents', 'PUT', { agent: agent.agent, enabled });
    agent.enabled = result.enabled;
    error.value = '';
  } catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { saving.value = ''; }
}
onMounted(refresh);
</script>
<template>
  <SettingsGroup title="Agents">
    <template #heading-actions>
      <button class="agent-refresh" :disabled="loading || !!saving" aria-label="刷新 Agents" title="刷新 Agents" @click="refresh">
        {{ loading ? '刷新中…' : '刷新' }}
      </button>
    </template>
    <div v-for="agent in visible" :key="agent.agent" class="agent-row">
      <label :for="`agent-${agent.agent}`" class="agent-info">
        <span v-if="icons[agent.agent]" class="agent-icon" v-html="icons[agent.agent]" aria-hidden="true" />
        <Bot v-else class="agent-fallback" :size="22" aria-hidden="true" />
        <span class="agent-copy">
          <span class="agent-name">{{ name(agent) }}<span v-if="agent.agent === 'codex'" class="agent-default">（默认）</span></span>
          <span class="agent-version">{{ agent.protocol }} · {{ agent.version || (agent.installed ? '版本未知' : '未安装') }}</span>
        </span>
      </label>
      <input :id="`agent-${agent.agent}`" class="settings-switch" type="checkbox" role="switch"
        :aria-label="`允许 Connector 使用 ${name(agent)}`" :checked="agent.agent === 'codex' || agent.enabled"
        :disabled="agent.agent === 'codex' || !!saving || loading || typeof agent.enabled !== 'boolean'"
        :title="agent.agent === 'codex' ? 'Codex 始终开启' : '允许 Connector 接受此 Agent 的请求'"
        @change="toggle(agent, $event)" />
    </div>
    <p v-if="error" class="agent-error" role="alert">{{ error }}</p>
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
.agent-error { color: #a46651; font-size: 12px; padding: 0 16px 10px; margin: 0; }
</style>
