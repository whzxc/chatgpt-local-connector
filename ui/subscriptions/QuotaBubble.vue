<script setup lang="ts">
import {quotaPace} from './pace';
import {cycleEstimate} from './cycleEstimate';
import UsageHistory from './UsageHistory.vue';
import {ref,inject,onUnmounted,computed} from 'vue';
import {NButton,NText} from 'naive-ui';
import {RefreshCw,Flame,TriangleAlert,CircleAlert} from '@lucide/vue';
import {openUrl} from '../platform';
import {usageLink} from './usageLinks';
import {subscriptionRefreshKey} from './useSubscriptions';
import {t} from '../i18n';
import {type ProviderSnapshot} from './types';
import {quotaLabel,quotaPercent,quotaValue,resetDisplay} from './displayPreferences';
import {windowLabel,subscriptionLabel,isFiveHour} from './labels';
import {brandIcon,quotaColor,resetText,preciseTime,errorLabels} from './presentation';
const emit=defineEmits<{popover:[points:[number,number][]]}>();
const props=defineProps<{provider:ProviderSnapshot;warningAt?:number;embedded?:boolean;valueTrigger?:boolean;detailPlacement?:'left'|'right';dismissKey?:number;}>();
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
const notices=computed(()=>{
  const p=props.provider, items:{message:string;error:boolean}[]=[];
  if(refreshError.value)items.push({message:refreshError.value,error:true});
  if(p.error)items.push({message:t(errorLabels[p.error.code]||'usageUnavailable'),error:!['no-subscription','no-limits-reported','unsupported-platform'].includes(p.error.code)});
  if(p.history?.error && p.history.error!=='history-unavailable')items.push({message:p.history.error,error:true});
  if(p.pinUnavailable)items.push({message:t('usagePinUnavailable'),error:false});
  if(p.accountBlocked)items.push({message:t('usageAccountBlocked'),error:true});
  for(const w of p.windows)if(w.exhausted||p.blockedPoolIds.includes(w.poolId))items.push({message:`${windowLabel(w.label)}：${t('usagePoolBlocked')}`,error:false});
  if(p.state==='stale')items.push({message:t('usageStale'),error:false});
  return items;
});
const hasError=computed(()=>notices.value.some(n=>n.error));
const details=computed(()=>notices.value.length?[{id:'notices',lines:notices.value.map(n=>n.message)}]:[]);
const readings=computed(()=>props.provider.windows.map(w=>({w,pace:quotaPace(w,props.provider,now.value),estimate:cycleEstimate(w,props.provider,now.value)})));
const now=ref(Date.now());const timer=setInterval(()=>now.value=Date.now(),60000);onUnmounted(()=>clearInterval(timer));
const refreshAge=computed(()=>{
  const timestamp=Date.parse(props.provider.observedAt??'');
  if(!Number.isFinite(timestamp))return '';
  const minutes=Math.max(0,Math.floor((now.value-timestamp)/60000));
  if(minutes<1)return '< 1m';
  if(minutes<60)return `${minutes}m ago`;
  if(minutes<1440)return `${Math.floor(minutes/60)}h ago`;
  return `${Math.floor(minutes/1440)}d ago`;
});
</script>
<template>
  <div class="quota-bubble" :class="{embedded}">
    <UsageHistory :rail="!embedded" :value-trigger="valueTrigger" :detail-placement="detailPlacement" @popover="emit('popover',$event)" :history="provider.history" :reset-count="provider.availableResetCount" :reset-credits="provider.resetCredits" :now="now" :dismiss-key="dismissKey" :details="details" v-slot="{register,hover,focus,expanded,displayed}">
      <header>
        <span class="provider-logo" :style="{maskImage: `url(${JSON.stringify(brandIcon(provider.agentId))})`}" aria-hidden="true"/>
        <span class="subscription-heading">
          <NText v-if="usageUrl" tag="a" class="subscription-link" :href="usageUrl" target="_blank" rel="noopener noreferrer" :draggable="false" style="color:inherit" @click.stop.prevent="openUsage">{{subscriptionLabel(provider)}}</NText>
          <strong v-else>{{subscriptionLabel(provider)}}</strong>
          <span v-if="provider.plan && provider.agentId!=='codex'" class="provider-plan">{{provider.plan}}</span>
          <span v-if="notices.length" :ref="el=>register('notices',el)" @mouseenter="hover('notices')" @mouseleave="hover('')" @focus="focus('notices')" @blur="focus('')" tabindex="0" :aria-expanded="expanded&&displayed==='notices'" class="subscription-notice" :class="{error:hasError}" role="img" :aria-label="notices.map(n=>n.message).join(' · ')"><CircleAlert v-if="hasError" :size="14"/><TriangleAlert v-else :size="14"/></span>
        </span>
        <span v-if="refreshAge" class="refresh-age">{{refreshAge}}</span>
        <NButton v-if="refreshSubscription" class="quota-refresh" text size="small" style="color:var(--quota-muted,var(--rail-muted))" :loading="requesting || provider.refreshing" :disabled="requesting || provider.refreshing || !provider.eligible || !provider.selected" :aria-label="t('usageRefresh')" @click.stop="refresh"><template #icon><RefreshCw :size="14"/></template></NButton>
      </header>
      <div v-for="{w,pace,estimate} in readings" :key="w.id" class="bubble-reading" :aria-label="[windowLabel(w.label),pace?.tooltip,...(estimate?.lines??[])].filter(Boolean).join(' · ')">
        <div class="reading-title">
          <span>{{provider.providerId==='antigravity' && w.scope ? w.scope+' · ' : ''}}{{windowLabel(w.label)}}</span>
          <span v-if="estimate" class="quota-estimate">≈ {{estimate.amount}}</span>
          <i v-if="w.id===provider.displayWindowId" role="img" :aria-label="t('usageMainDisplay')"/>
          <span v-if="pace?.label" class="pace-warning"><Flame v-if="pace.flame" :size="12" :color="pace.color"/>{{pace.label}}</span>
        </div>
        <div class="reading-track"><div :style="{width:`${quotaValue(w)}%`,background:pace?.color ?? quotaColor(w,provider,warningAt)}"/><span v-if="pace?.tick!=null" class="pace-tick" :style="{left:`clamp(1px, ${pace.tick}%, calc(100% - 1px))`}"/></div>
        <div class="reading-meta"><span>{{quotaLabel()}} {{quotaPercent(w)}}</span><span>{{resetDisplay==='time' ? preciseTime(w.resetsAt) || '—' : resetText(w.resetsAt,now,isFiveHour(w.label))}}</span></div>
      </div>
    </UsageHistory>
  </div>
</template>
<style scoped>
.quota-bubble{box-sizing:border-box;width:280px;padding:18px;display:flex;flex-direction:column;gap:14px;color:var(--quota-ink,var(--rail-ink));font-family:ui-rounded,'SF Pro Rounded',-apple-system,sans-serif}.quota-bubble header{display:flex;align-items:center;gap:7px;min-height:19px;line-height:19px;font-size:14px}.provider-logo{width:16px;height:16px;flex:none;background:currentColor;mask-size:contain;mask-repeat:no-repeat;mask-position:center}.quota-bubble header strong,.subscription-link{font-weight:600}.subscription-link{text-decoration:none;cursor:pointer}.subscription-link:hover{text-decoration:underline}.subscription-link:focus-visible{outline:2px solid currentColor;outline-offset:3px;border-radius:2px}.refresh-age{flex:none;white-space:nowrap;font-size:10px;font-weight:400;color:var(--quota-muted,var(--rail-muted));font-variant-numeric:tabular-nums}.quota-refresh{flex:none;width:20px;height:20px}.quota-bubble .provider-plan{max-width:76px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:11px;color:var(--quota-muted,var(--rail-muted))}.bubble-reading{display:flex;flex-direction:column;gap:7px;font-size:11.5px;line-height:14px;font-variant-numeric:tabular-nums}.reading-title{display:flex;flex-wrap:wrap;align-items:center;gap:6px;min-height:14px}.quota-estimate{white-space:nowrap;font-size:10.5px;color:var(--quota-muted,var(--rail-muted))}.reading-title i{flex:none;width:4px;height:4px;border-radius:50%;background:var(--quota-muted,var(--rail-muted))}.reading-track{height:6px;flex:none;background:var(--quota-track,var(--rail-track));border-radius:3px;position:relative}.reading-track div{height:100%;border-radius:3px}.reading-meta{display:flex;justify-content:space-between;gap:5px}.reading-meta>span:last-child{color:var(--quota-muted,var(--rail-muted));text-align:right}.quota-bubble p{margin:0;color:var(--quota-muted,var(--rail-muted));font-size:12px;line-height:17px}.subscription-heading{display:flex;align-items:center;gap:7px;min-width:0;margin-right:auto}.subscription-notice{display:inline-flex;align-items:center;flex:none;color:var(--usage-caution)}.subscription-notice.error{color:var(--usage-warning)}
.quota-bubble.embedded{width:100%;padding:0;--quota-ink:var(--ink);--quota-muted:var(--muted);--quota-track:var(--line)}
</style>

<style scoped>
.pace-warning{margin-left:auto;display:flex;align-items:center;gap:4px;color:var(--quota-muted,var(--rail-muted));white-space:nowrap}.reading-title>span:first-child{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.pace-tick{position:absolute;top:-2px;width:2px;height:10px;border-radius:1px;transform:translateX(-50%);background:var(--quota-muted,var(--rail-muted))}
</style>
