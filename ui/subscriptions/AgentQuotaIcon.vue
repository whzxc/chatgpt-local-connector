<script setup lang="ts">
import { computed, inject } from 'vue';
import ActivityArc from './ActivityArc.vue';
import { useAgentActive } from '../composables/useAgentActivity';
import { Bot } from '@lucide/vue';
import { icons } from '../composables/useAgents';
import { subscriptionSnapshotKey } from './useSubscriptions';
import {quotaValue,quotaPercent,quotaLabel} from './displayPreferences';
import { readingStatus } from './presentation';
import { windowLabel } from './labels';
const props = defineProps<{ agent: string }>();
const snapshot = inject(subscriptionSnapshotKey);
const running = useAgentActive(() => props.agent);
const provider = computed(() => snapshot?.value?.providers.find(p => p.agentId === props.agent));
const weekly = computed(() => {
  const p = provider.value;
  if (!p || p.state !== 'ready' || p.error) return;
  const pool = p.windows.find(w => w.id === p.displayWindowId)?.poolId;
  // Explicit weekly periods only; monthly/session readings never stand in for a week.
  const windows = p.windows.filter(w => ['weekly', 'weekly_all', 'seven_day', '10080 min', '604800 s'].includes(w.label)
    && Number.isFinite(w.usedPercent) && (!w.resetsAt || Date.parse(w.resetsAt) > Date.now()));
  return windows.find(w => w.poolId === pool) || windows[0];
});
const label = computed(() => weekly.value ? `${windowLabel('weekly')} · ${quotaLabel()} ${quotaPercent(weekly.value)}` : undefined);
const color = computed(() => provider.value && weekly.value ? readingStatus(weekly.value, provider.value).color : undefined);
</script>
<template>
  <span class="agent-quota-icon" :class="{running,'has-quota':!!weekly}" :aria-busy="running" :aria-label="label" :role="label ? 'img' : undefined">
    <svg v-if="weekly" class="quota-outline" viewBox="0 0 48 48" aria-hidden="true">
      <circle class="quota-track" cx="24" cy="24" r="21"/>
      <circle cx="24" cy="24" r="21" pathLength="100" :stroke="color" :stroke-dasharray="`${quotaValue(weekly)} 100`" transform="rotate(-90 24 24)"/>
    </svg>
    <ActivityArc v-if="running"/>
    <span v-if="icons[agent]" class="agent-logo" v-html="icons[agent]" aria-hidden="true"/>
    <Bot v-else class="agent-logo" aria-hidden="true"/>
  </span>
</template>
<style scoped>
.agent-quota-icon{position:relative;display:inline-grid;place-items:center;width:var(--agent-quota-size,40px);height:var(--agent-quota-size,40px);flex-shrink:0;vertical-align:middle}
.agent-logo{display:flex;width:65%;height:65%;align-items:center;justify-content:center}.agent-logo :deep(svg){width:100%;height:100%}
.has-quota .agent-logo{width:55%;height:55%}
.running .agent-logo{width:45%;height:45%}
.quota-outline{position:absolute;inset:0;width:100%;height:100%;fill:none;stroke-width:var(--quota-ring-width);stroke-linecap:round;pointer-events:none}.quota-outline circle{vector-effect:non-scaling-stroke}.quota-track{stroke:var(--line)}
</style>
