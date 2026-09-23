<script setup lang="ts">
import {quotaPace} from './pace';
import UsageHistory from './UsageHistory.vue';
import {ref,inject,onUnmounted,computed} from 'vue';
import {NButton,NText,NTooltip} from 'naive-ui';
import {RefreshCw,Flame} from '@lucide/vue';
import {openUrl} from '../platform';
import {usageLink} from './usageLinks';
import {subscriptionRefreshKey} from './useSubscriptions';
import {t} from '../i18n';
import {type ProviderSnapshot} from './types';
import {quotaLabel,quotaPercent,quotaValue,resetDisplay} from './displayPreferences';
import {windowLabel,subscriptionLabel,isFiveHour} from './labels';
import {brandIcon,quotaColor,resetText,preciseTime,errorLabels} from './presentation';
const emit=defineEmits<{popover:[points:[number,number][]]}>();
const props=defineProps<{provider:ProviderSnapshot;warningAt?:number;embedded?:boolean;detailPlacement?:'left'|'right'}>();
const refreshSubscription=inject(subscriptionRefreshKey);
const usageUrl=computed(()=>usageLink(props.provider));
async function openUsage(){if(!usageUrl.value)return;try{await openUrl(usageUrl.value);}catch(error){refreshError.value=String(error);}}
const requesting=ref(false),refreshError=ref('');
async function refresh() {
  if (!refreshSubscription || requesting.value || props.provider.refreshing) return;
  requesting.value=true;refreshError.value='';
  try { await refreshSubscription(props.provider.providerId); }
  catch (error) { refreshError.value=String(error); }
  finally { requesting.value=false; }
}
const readings=computed(()=>props.provider.windows.map(w=>({w,pace:quotaPace(w,props.provider,now.value)})));
const now=ref(Date.now());const timer=setInterval(()=>now.value=Date.now(),60000);onUnmounted(()=>clearInterval(timer));
</script>
<template><div class="quota-bubble" :class="{embedded}"><header><span class="provider-logo" :style="{maskImage: `url(${JSON.stringify(brandIcon(provider.agentId))})`}" aria-hidden="true"/><NText v-if="usageUrl" tag="a" class="subscription-link" :href="usageUrl" target="_blank" rel="noopener noreferrer" :draggable="false" style="color:inherit" :title="t('usageOpenWebsite')" @click.stop.prevent="openUsage">{{subscriptionLabel(provider)}}</NText><strong v-else>{{subscriptionLabel(provider)}}</strong><span v-if="provider.plan && provider.agentId!=='codex'" class="provider-plan">{{provider.plan}}</span><NButton v-if="refreshSubscription" class="quota-refresh" text size="small" style="color:var(--quota-muted,#8b8b90)" :loading="requesting || provider.refreshing" :disabled="requesting || provider.refreshing || !provider.eligible || !provider.selected" :aria-label="t('usageRefresh')" :title="t('usageRefresh')" @click.stop="refresh"><template #icon><RefreshCw :size="14"/></template></NButton></header><div v-for="{w,pace} in readings" :key="w.id" class="bubble-reading"><div class="reading-title"><span>{{provider.providerId==='antigravity' && w.scope ? w.scope+' · ' : ''}}{{windowLabel(w.label)}}</span><i v-if="w.id===provider.displayWindowId" :title="t('usageMainDisplay')"/><NTooltip v-if="pace?.label" :theme-overrides="embedded ? undefined : {color:'#29292c',textColor:'#f5f5f7'}"><template #trigger><span class="pace-warning"><Flame v-if="pace.flame" :size="12" :color="pace.color"/>{{pace.label}}</span></template>{{pace.tooltip}}</NTooltip></div><NTooltip :disabled="!pace" :theme-overrides="embedded ? undefined : {color:'#29292c',textColor:'#f5f5f7'}"><template #trigger><div class="reading-track" :aria-label="pace?.tooltip"><div :style="{width:`${quotaValue(w)}%`,background:pace?.color ?? quotaColor(w,provider,warningAt)}"/><span v-if="pace?.tick!=null" class="pace-tick" :style="{left:`clamp(1px, ${pace.tick}%, calc(100% - 1px))`}"/></div></template>{{pace?.tooltip}}</NTooltip><div class="reading-meta"><span>{{quotaLabel()}} {{quotaPercent(w)}}</span><span :title="preciseTime(w.resetsAt)">{{resetDisplay==='time' ? preciseTime(w.resetsAt) || '—' : resetText(w.resetsAt,now,isFiveHour(w.label))}}</span></div><p v-if="w.exhausted||provider.blockedPoolIds.includes(w.poolId)">{{t('usagePoolBlocked')}}</p></div><UsageHistory :rail="!embedded" :detail-placement="detailPlacement" @popover="emit('popover',$event)" v-if="provider.history || provider.availableResetCount!=null" :history="provider.history" :reset-count="provider.availableResetCount" :reset-credits="provider.resetCredits" :now="now"/><p v-if="refreshError" role="status">{{refreshError}}</p><p v-if="provider.refreshing">{{t('usageRefreshing')}}</p><p v-if="provider.pinUnavailable">{{t('usagePinUnavailable')}}</p><p v-if="provider.error" role="status">{{t(errorLabels[provider.error.code]||'usageUnavailable')}}</p><p v-else-if="!provider.windows.length && !provider.plan">{{t('usageWaiting')}}</p><p v-if="provider.accountBlocked">{{t('usageAccountBlocked')}}</p><p v-if="provider.state==='stale'" class="bubble-footnote">{{t('usageStale')}}</p></div></template>
<style scoped>
.quota-bubble{box-sizing:border-box;width:250px;padding:18px;display:flex;flex-direction:column;gap:14px;color:var(--quota-ink,#f5f5f7);font-family:ui-rounded,'SF Pro Rounded',-apple-system,sans-serif}.quota-bubble header{display:flex;align-items:center;gap:7px;min-height:19px;line-height:19px;font-size:14px}.provider-logo{width:16px;height:16px;flex:none;background:currentColor;mask-size:contain;mask-repeat:no-repeat;mask-position:center}.quota-bubble header strong,.subscription-link{font-weight:600;margin-right:auto}.subscription-link{text-decoration:none;cursor:pointer}.subscription-link:hover{text-decoration:underline}.subscription-link:focus-visible{outline:2px solid currentColor;outline-offset:3px;border-radius:2px}.quota-refresh{flex:none;width:20px;height:20px}.quota-bubble .provider-plan{max-width:76px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px;color:var(--quota-muted,#8b8b90)}.bubble-reading{display:flex;flex-direction:column;gap:7px;font-size:11.5px;line-height:14px;font-variant-numeric:tabular-nums}.reading-title{display:flex;align-items:center;gap:6px;min-height:14px}.reading-title i{width:4px;height:4px;border-radius:50%;background:var(--quota-muted,#888)}.reading-track{height:6px;flex:none;background:var(--quota-track,#29292c);border-radius:3px;position:relative}.reading-track div{height:100%;border-radius:3px}.reading-meta{display:flex;justify-content:space-between;gap:5px}.reading-meta>span:last-child{color:var(--quota-muted,#8b8b90);text-align:right}.quota-bubble p{margin:0;color:var(--quota-muted,#a3a3aa);font-size:12px;line-height:17px}.quota-bubble .bubble-footnote{font-size:11px}
.quota-bubble.embedded{width:100%;padding:0;--quota-ink:var(--ink);--quota-muted:var(--muted);--quota-track:var(--line)}
</style>

<style scoped>
.pace-warning{margin-left:auto;display:flex;align-items:center;gap:4px;color:var(--quota-muted,#8b8b90);white-space:nowrap}.reading-title>span:first-child{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.pace-tick{position:absolute;top:-2px;width:2px;height:10px;border-radius:1px;transform:translateX(-50%);background:var(--quota-muted,#8b8b90)}
</style>
