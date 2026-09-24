<script setup lang="ts">
import { computed, inject, ref } from 'vue';
import ElasticPanel from './ElasticPanel.vue';
import AgentQuotaIcon from '../subscriptions/AgentQuotaIcon.vue';
import QuotaBubble from '../subscriptions/QuotaBubble.vue';
import { subscriptionSnapshotKey } from '../subscriptions/useSubscriptions';
import { name, type Agent } from '../composables/useAgents';
import { t } from '../i18n';
const props = defineProps<{agent:Agent;origin:{x:number;y:number;size:number}}>();
const emit = defineEmits<{close:[]}>();
const open = ref(true);
const snapshot = inject(subscriptionSnapshotKey);
const provider = computed(() => snapshot?.value?.settings.enabled ? snapshot.value.providers.find(p => p.agentId === props.agent.agent && p.eligible && p.selected) : undefined);
</script>
<template>
  <ElasticPanel headerless :show="open" :title="name(agent)" :origin="origin" :width="440" @close="open=false" @closed="emit('close')">
    <div class="agent-summary" :class="{'has-quota':provider?.windows.length}"><AgentQuotaIcon :agent="agent.agent"/><div><strong>{{name(agent)}}</strong><p v-if="agent.version || !agent.installed">{{agent.installed ? agent.version : t('agentNotInstalled')}}</p></div></div>
    <QuotaBubble v-if="provider" :provider="provider" embedded/>
  </ElasticPanel>
</template>
<style scoped>
.agent-summary{display:flex;align-items:center;gap:16px;margin-bottom:20px;--agent-quota-size:56px}.agent-summary.has-quota{--agent-quota-size:48px}.agent-summary p{margin:5px 0 0;color:var(--muted);font-size:12px}.agent-summary strong{font-size:15px}
</style>
