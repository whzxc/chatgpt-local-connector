<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { api } from '../composables/useConnector';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
type Agent = { agent: string; protocol: string; status: string; version: string | null };
const agents = ref<Agent[]>([]);
const error = ref('');
const loading = ref(false);
const labels: Record<string, string> = { installed: '已安装', ready: '已就绪', unavailable: '不可用' };
async function refresh() {
  loading.value = true;
  try { agents.value = (await api<{ agents: Agent[] }>('agents')).agents; error.value = ''; }
  catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { loading.value = false; }
}
onMounted(refresh);
</script>
<template>
  <SettingsGroup title="Agents">
    <SettingsRow v-for="agent in agents" :key="agent.agent" :title="agent.agent === 'codex' ? 'Codex（默认）' : agent.agent === 'pi' ? 'Pi' : agent.agent === 'opencode' ? 'OpenCode' : agent.agent" :description="`${agent.protocol} · ${agent.version || '版本未知'}`">
      <span>{{ labels[agent.status] || agent.status }}</span>
    </SettingsRow>
    <SettingsRow title="自动发现本机 Agent" :description="error || '沿用各 Agent 已有配置和登录。已安装不表示模型服务已连接；Codex 保持默认。'">
      <button :disabled="loading" @click="refresh">{{ loading ? '检测中…' : '刷新' }}</button>
    </SettingsRow>
  </SettingsGroup>
</template>
