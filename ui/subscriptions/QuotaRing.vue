<script setup lang="ts">
import QuotaTimeTick from './QuotaTimeTick.vue';
import ActivityArc from './ActivityArc.vue';
import { useAgentActive } from '../composables/useAgentActivity';
import { computed,watch } from 'vue';
import {type ProviderSnapshot} from './types';
import {quotaValue,quotaPercent,quotaLabel} from './displayPreferences';
import { brandIcon,quotaColor } from './presentation';
import { useSpring } from '../usage-rail/spring';
import m from '../../shared/usage-panel.json';
const props=defineProps<{provider:ProviderSnapshot;warningAt?:number;showPercentage?:boolean}>();
const running=useAgentActive(()=>props.provider.agentId);
const current=computed(()=>props.provider.windows.find(w=>w.id===props.provider.displayWindowId));
const left=computed(()=>quotaValue(current.value));
const progress=useSpring([left.value??0],m.readingResponse,m.readingDamping);
watch(left,v=>{if(v===undefined)progress.jump([0]);else progress.to([v]);});
</script>
<template><span class="quota-ring" :aria-busy="running" :class="{stale:provider.state==='stale',unknown:left===undefined}" :style="{'--quota-color':quotaColor(current,provider,warningAt)}"><span class="quota-dial"><svg viewBox="0 0 36 36" aria-hidden="true"><circle cx="18" cy="18" r="18" class="track"/><circle v-if="left!==undefined" cx="18" cy="18" r="18" class="value" transform="rotate(-90 18 18)" :stroke-dasharray="`${Math.max(0,Math.min(100,progress.value.value[0]!))/100*113.097} 113.097`"/><QuotaTimeTick :provider="provider" :window="current" :center="18" :radius="18"/></svg><ActivityArc v-if="running"/><img :src="brandIcon(provider.agentId)" alt=""/></span><span v-if="showPercentage!==false" class="quota-number" :aria-label="`${quotaLabel()} ${quotaPercent(current)}`">{{quotaPercent(current)}}</span></span></template>
<style scoped>
.quota-ring{display:flex;align-items:center;flex-direction:column;gap:6px;width:40px;color:var(--rail-ink);font-variant-numeric:tabular-nums}.quota-dial{width:36px;height:36px;position:relative}.quota-dial>svg:not(.activity-arc){width:36px;height:36px;overflow:visible}circle{fill:none;stroke-width:var(--quota-ring-width)}.track{stroke:var(--rail-track)}.value{stroke:var(--quota-color);stroke-linecap:round}.quota-dial img{position:absolute;inset:10px;width:16px;height:16px;filter:brightness(0) invert(1)}.quota-number{font-family:ui-rounded,'SF Pro Rounded',-apple-system,sans-serif;font-size:13px;font-weight:500;line-height:16px}.stale{opacity:.55}.unknown .quota-number{color:var(--rail-muted)}
</style>
