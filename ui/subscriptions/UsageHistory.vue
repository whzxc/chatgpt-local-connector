<script setup lang="ts">
import { computed, ref, watchEffect, onUnmounted } from 'vue';
import { useElementBounding } from '@vueuse/core';
import { NButton, NProgress, NPopover } from 'naive-ui';
import { usagePopoverTheme, usageValueButtonTheme } from '../components/UiProvider.vue';
import type { UsageHistory, ProviderSnapshot } from './types';
import { t } from '../i18n';
import { usagePeriods, usagePeriodLabels as labels } from './displayPreferences';
import { preciseTime, resetText } from './presentation';
import railMetrics from '../../shared/usage-panel.json';
import { bubbleShape, path } from '../usage-rail/geometry';
const props=defineProps<{rail?:boolean;detailPlacement?:'left'|'right';history?:UsageHistory;resetCount?:number|null;resetCredits?:ProviderSnapshot['resetCredits'];now:number}>();
const expanded = ref('');
const arrowStyle = {
  width: 'var(--usage-tail-width)', height: 'var(--usage-tail-height)',
  top: 'var(--usage-tail-top)', bottom: 'var(--usage-tail-bottom)',
  left: 'var(--usage-tail-left)', right: 'var(--usage-tail-right)',
  transform: 'var(--usage-tail-transform)', clipPath: 'var(--usage-tail-shape)',
  boxShadow: 'none',
  '--tail-left': `path('${path(bubbleShape(20,0,250,120,'left',60,1))}')`,
  '--tail-right': `path('${path(bubbleShape(-250,0,250,120,'right',60,1))}')`,
  '--tail-top': `path('${path(bubbleShape(0,20,120,250,'top',60,1))}')`,
  '--tail-bottom': `path('${path(bubbleShape(0,-250,120,250,'bottom',60,1))}')`,
};
const horizontalGap=ref(40);
function alignPopover(event:Event) {
  if(!props.rail)return;
  const trigger=event.currentTarget as HTMLElement;
  const card=trigger.closest('.quota-bubble')?.getBoundingClientRect();
  if(!card)return;
  const rect=trigger.getBoundingClientRect(), scale=card.width/railMetrics.cardWidth;
  const inset=props.detailPlacement==='left' ? rect.left-card.left : card.right-rect.right;
  horizontalGap.value=inset+(railMetrics.cardGap+railMetrics.pointerWidth)*scale;
}
const emit=defineEmits<{popover:[points:[number,number][]]}>();
const details=ref<Record<string,HTMLElement|undefined>>({});
const detail=computed(()=>details.value[expanded.value]?.closest<HTMLElement>('.history-popover') ?? details.value[expanded.value]);
const bounds=useElementBounding(detail);
watchEffect(()=>{
  const {left,top,right,bottom,width,height}=bounds;
  emit('popover', expanded.value && width.value && height.value ? [[left.value-24,top.value-24],[right.value+24,top.value-24],[right.value+24,bottom.value+24],[left.value-24,bottom.value+24]] : []);
});
onUnmounted(()=>emit('popover',[]));
const tokens=new Intl.NumberFormat(undefined,{notation:'compact',maximumFractionDigits:1});
const dollars=new Intl.NumberFormat(undefined,{style:'currency',currency:'USD',maximumFractionDigits:2});
</script>
<template>
  <div class="usage-history" v-if="resetCount!=null || usagePeriods.length">
    <div v-if="resetCount!=null" class="history-period">
      <span>{{t('usageResetCount')}}</span>
      <NPopover class="history-popover" scrollable arrow-class="usage-detail-arrow" :arrow-style="arrowStyle" trigger="hover" :delay="120" :duration="180" :keep-alive-on-hover="true" :theme-overrides="usagePopoverTheme(!!rail,horizontalGap)" :placement="rail ? detailPlacement ?? 'right' : 'top'" :show="expanded==='resets'" @update:show="expanded=$event?'resets':expanded==='resets'?'':expanded" :style="{maxWidth:'calc(100vw - 32px)',maxHeight:'min(396px, calc(60vh + 36px))'}">
      <template #trigger><NButton text :theme-overrides="usageValueButtonTheme" class="history-toggle" @mouseenter="alignPopover" @focus="alignPopover" :aria-expanded="expanded==='resets'" @click.stop>
        <span>{{t('usageResetCountValue',{count:resetCount})}}</span>
      </NButton></template>
      <div :ref="el=>details.resets=el as HTMLElement|undefined" class="history-detail" :aria-label="t('usageExpires')">
        <div v-for="(credit,index) in resetCredits" :key="index" class="history-period expiry-row">
          <span><span class="credit-number">{{index+1}}</span>{{preciseTime(credit.expiresAt)||t('usageExpiryUnknown')}}</span>
          <span>{{resetText(credit.expiresAt,now)}}</span>
        </div>
        <span v-if="!resetCredits?.length">{{t('usageExpiryUnknown')}}</span>
      </div>
      </NPopover>
    </div>
    <template v-if="usagePeriods.length">
      <div v-for="period in history?.periods?.filter(p=>usagePeriods.includes(p.id))" :key="period.id" class="history-period">
        <span>{{t(labels[period.id])}}</span>
        <NPopover class="history-popover" scrollable arrow-class="usage-detail-arrow" :arrow-style="arrowStyle" trigger="hover" :delay="120" :duration="180" :keep-alive-on-hover="true" :theme-overrides="usagePopoverTheme(!!rail,horizontalGap)" :placement="rail ? detailPlacement ?? 'right' : 'top'" :show="expanded===period.id" @update:show="expanded=$event?period.id:expanded===period.id?'':expanded" :style="{maxWidth:'calc(100vw - 32px)',maxHeight:'min(396px, calc(60vh + 36px))'}">
        <template #trigger><NButton text :theme-overrides="usageValueButtonTheme" class="history-toggle" @mouseenter="alignPopover" @focus="alignPopover" :aria-expanded="expanded===period.id" @click.stop>
          <span v-if="period.tokens" :title="period.tokens.toLocaleString()+' tokens'">{{period.estimatedUsd==null?'—':dollars.format(period.estimatedUsd)}} · {{tokens.format(period.tokens)}} tokens</span>
          <span v-else>{{t('usageNoHistory')}}</span>
        </NButton></template>
        <div :ref="el=>details[period.id]=el as HTMLElement|undefined" class="history-detail" :aria-label="t(labels[period.id])">
          <strong>{{t(labels[period.id])}}</strong>
          <div v-for="model in period.models" :key="model.model" class="model-reading">
            <div class="history-period"><strong>{{model.model}}</strong><span class="model-share">{{period.tokens ? Math.round(model.tokens/period.tokens*100) : 0}}%</span></div>
            <NProgress type="line" :percentage="period.tokens ? model.tokens/period.tokens*100 : 0" :show-indicator="false" :height="6" color="#00e68c" :rail-color="rail?'#29292c':'var(--line)'"/>
            <div class="history-period model-meta"><span>{{model.estimatedUsd==null?'—':dollars.format(model.estimatedUsd)}}</span><span :title="model.tokens.toLocaleString()+' tokens'">{{tokens.format(model.tokens)}} tokens</span></div>
          </div>
          <span v-if="!period.models?.length">{{t('usageNoHistory')}}</span>
        </div>
        </NPopover>
      </div>
      <p v-if="history?.error" role="status">{{t('usageHistoryUnavailable')}}</p>
    </template>
  </div>
</template>
<style scoped>
.usage-history{display:flex;flex-direction:column;gap:9px;font-size:12px;line-height:1.4}
.history-toggle{flex-shrink:0;font:inherit;color:inherit;padding:3px 6px;margin:-3px -6px;border-radius:7px;transition:filter 120ms}.history-toggle:hover,.history-toggle:focus-visible{font-weight:600;filter:brightness(1.18)}
.history-period{display:flex;justify-content:space-between;align-items:baseline;gap:10px;font-variant-numeric:tabular-nums;width:100%}
.history-period>span:last-child{text-align:right;flex-shrink:0}
.history-detail{display:flex;flex-direction:column;gap:14px;width:214px;max-width:calc(100vw - 68px);font-size:11.5px;line-height:14px;font-family:ui-rounded,'SF Pro Rounded',-apple-system,sans-serif}
.history-detail>strong{font-size:14px;line-height:19px;font-weight:600}
.model-reading{display:flex;flex-direction:column;gap:7px}
.model-reading strong{overflow-wrap:anywhere;min-width:0;font-weight:400}
.model-meta{font-size:11.5px}.model-meta>span:last-child,.model-share{opacity:.65}
.credit-number{display:inline-grid;place-items:center;min-width:18px;height:18px;border-radius:50%;margin-right:8px;background:#008aff;color:white;font-size:10px}
.expiry-row{font-size:11px;gap:6px}.expiry-row>span:first-child{min-width:0;overflow-wrap:anywhere}
.usage-history p{font-size:10px;margin:0;color:var(--quota-muted,var(--muted))}
</style>
<style>
[v-placement^="right"] .usage-detail-arrow{--usage-tail-width:20px;--usage-tail-height:120px;--usage-tail-top:50%;--usage-tail-bottom:auto;--usage-tail-right:0;--usage-tail-left:auto;--usage-tail-transform:translateY(-50%);--usage-tail-shape:var(--tail-left)}
[v-placement^="left"] .usage-detail-arrow{--usage-tail-width:20px;--usage-tail-height:120px;--usage-tail-top:50%;--usage-tail-bottom:auto;--usage-tail-left:0;--usage-tail-right:auto;--usage-tail-transform:translateY(-50%);--usage-tail-shape:var(--tail-right)}
[v-placement^="top"] .usage-detail-arrow{--usage-tail-width:120px;--usage-tail-height:20px;--usage-tail-left:50%;--usage-tail-right:auto;--usage-tail-top:0;--usage-tail-bottom:auto;--usage-tail-transform:translateX(-50%);--usage-tail-shape:var(--tail-bottom)}
[v-placement^="bottom"] .usage-detail-arrow{--usage-tail-width:120px;--usage-tail-height:20px;--usage-tail-left:50%;--usage-tail-right:auto;--usage-tail-bottom:0;--usage-tail-top:auto;--usage-tail-transform:translateX(-50%);--usage-tail-shape:var(--tail-top)}
</style>
