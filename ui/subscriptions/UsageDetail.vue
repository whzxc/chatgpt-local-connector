<script setup lang="ts">
import {NProgress} from 'naive-ui';
import type {UsageHistory,ProviderSnapshot} from './types';
import {t} from '../i18n';
import {usagePeriodLabels as labels} from './displayPreferences';
import {preciseTime,resetText} from './presentation';
defineProps<{displayed:string;detailEntry?:{lines:string[]};period?:NonNullable<UsageHistory['periods']>[number];resetCredits?:ProviderSnapshot['resetCredits'];now:number;rail?:boolean}>();
const tokens=new Intl.NumberFormat(undefined,{notation:'compact',minimumFractionDigits:2,maximumFractionDigits:2});
const dollars=new Intl.NumberFormat(undefined,{style:'currency',currency:'USD',maximumFractionDigits:2});
</script>
<template>
      <div  class="history-detail" :aria-label="displayed==='resets'?t('usageExpires'):period?t(labels[period.id]):undefined">
        <template v-if="detailEntry"><div v-for="line in detailEntry.lines" :key="line">{{line}}</div></template>
        <template v-else-if="displayed==='resets'">
          <div v-for="(credit,index) in resetCredits" :key="index" class="history-period expiry-row">
            <span><span class="credit-number">{{index+1}}</span>{{preciseTime(credit.expiresAt)||t('usageExpiryUnknown')}}</span>
            <span>{{resetText(credit.expiresAt,now)}}</span>
          </div>
          <span v-if="!resetCredits?.length">{{t('usageExpiryUnknown')}}</span>
        </template>
        <template v-else-if="period">
          <strong>{{t(labels[period.id])}}</strong>
          <div v-for="model in period.models" :key="model.model" class="model-reading">
            <div class="history-period"><strong>{{model.model}}</strong><span class="model-share">{{period.tokens ? Math.round(model.tokens/period.tokens*100) : 0}}%</span></div>
            <NProgress type="line" :percentage="period.tokens ? model.tokens/period.tokens*100 : 0" :show-indicator="false" :height="6" color="var(--accent)" :rail-color="rail?'var(--rail-track)':'var(--line)'"/>
            <div class="history-period model-meta"><span>{{model.estimatedUsd==null?'—':dollars.format(model.estimatedUsd)}}</span><span :title="model.tokens.toLocaleString()+' tokens'">{{tokens.format(model.tokens)}} tokens</span></div>
          </div>
          <span v-if="!period.models?.length">-</span>
        </template>
      </div>
</template>
<style scoped>
.history-period{display:flex;justify-content:space-between;align-items:baseline;gap:10px;font-variant-numeric:tabular-nums;width:100%}
.history-period>span:last-child{text-align:right;flex-shrink:0}
.history-detail{display:flex;flex-direction:column;gap:14px;width:214px;max-width:100%;font-size:11.5px;line-height:14px;font-family:ui-rounded,'SF Pro Rounded',-apple-system,sans-serif}
.history-detail>strong{font-size:14px;line-height:19px;font-weight:600}
.model-reading{display:flex;flex-direction:column;gap:7px}
.model-reading strong{overflow-wrap:anywhere;min-width:0;font-weight:400}
.model-meta{font-size:11.5px}.model-meta>span:last-child,.model-share{opacity:.65}
.credit-number{display:inline-grid;place-items:center;min-width:18px;height:18px;border-radius:50%;margin-right:8px;background:var(--accent);color:var(--white);font-size:10px}
.expiry-row{font-size:11px;gap:6px}.expiry-row>span:first-child{min-width:0;overflow-wrap:anywhere}

</style>
