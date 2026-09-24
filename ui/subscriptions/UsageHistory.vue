<script setup lang="ts">
import { computed, ref, watch, watchEffect, nextTick, onMounted, onUnmounted } from 'vue';
import { useElementBounding, useResizeObserver, useWindowSize } from '@vueuse/core';
import { NButton, NPopover } from 'naive-ui';
import { usagePopoverTheme, usageValueButtonTheme } from '../components/UiProvider.vue';
import type { UsageHistory, ProviderSnapshot } from './types';
import { t } from '../i18n';
import { isDesktop } from '../platform';
import { usagePeriods, usagePeriodLabels as labels } from './displayPreferences';
import railMetrics from '../../shared/usage-panel.json';
import UsageDetail from './UsageDetail.vue';
import { useSpring } from '../usage-rail/spring';
const props=defineProps<{rail?:boolean;valueTrigger?:boolean;detailPlacement?:'left'|'right';dismissKey?:number;externalDetail?:string;history?:UsageHistory;resetCount?:number|null;resetCredits?:ProviderSnapshot['resetCredits'];now:number;details?:{id:string;lines:string[]}[]}>();
const emit=defineEmits<{popover:[points:[number,number][]]}>();
const expanded=ref(false), displayed=ref('');
const detailEntry=computed(()=>props.details?.find(item=>item.id===displayed.value));
function register(key:string,element:unknown){triggers.value[key]=element instanceof HTMLElement?element:undefined;}
const period=computed(()=>props.history?.periods?.find(p=>p.id===displayed.value));
const triggers=ref<Record<string,HTMLElement|undefined>>({});
const content=ref<HTMLElement>();
const detail=computed(()=>content.value?.closest<HTMLElement>('.history-popover'));
const trigger=computed(()=>{
  const element=triggers.value[displayed.value];
  return props.valueTrigger ? element?.querySelector<HTMLElement>('.history-toggle') ?? element : element;
});
const triggerBounds=useElementBounding(trigger), bounds=useElementBounding(detail);
const {height:viewportHeight}=useWindowSize();
const motion=useSpring([0,0,80],railMetrics.cardResponse,railMetrics.cardDamping,()=>bounds.update());
const fade=useSpring([1],.18,.9);
const popoverY=computed(()=>{const half=(motion.value.value[2]!+36)/2;return Math.max(half+8,Math.min(viewportHeight.value-half-8,motion.value.value[1]!));});
const horizontalGap=ref(28);
function geometry() {
  const el=trigger.value;if(!el)return;
  const r=el.getBoundingClientRect(), card=el.closest('.quota-bubble')?.getBoundingClientRect();
  const scale=card?card.width/railMetrics.cardWidth:1;
  horizontalGap.value=(railMetrics.cardGap+railMetrics.pointerWidth)*scale;
  const x=(props.rail||props.detailPlacement)&&card?(props.detailPlacement==='left'?card.left:card.right):r.left;
  const y=r.top+r.height/2;
  const height=Math.min(content.value?.offsetHeight||80,Math.max(0,Math.min(396,viewportHeight.value*.6+36)-36));
  return [x,y,height];
}
function retarget(){const next=geometry();if(next&&expanded.value&&!(props.externalDetail&&isDesktop))motion.to(next);}
useResizeObserver(content,retarget);
watch([triggerBounds.left,triggerBounds.top,triggerBounds.width,triggerBounds.height,viewportHeight],retarget);
let hoverTimer:ReturnType<typeof setTimeout>|undefined, hoverTarget='', disposeHover:(()=>void)|undefined, disposed=false;
async function show(key:string) {
  if(!key){expanded.value=false;return;}
  const opening=!expanded.value, switching=displayed.value!==key;
  displayed.value=key;
  // Keep the same popover mounted and retain its contents throughout leave.
  await nextTick();
  if(disposed||hoverTarget!==key)return;
  if(props.externalDetail&&isDesktop){
    const el=trigger.value;
    const r=(el?.querySelector('.history-toggle')??el)?.getBoundingClientRect();if(!r)return;
    const {invoke}=await import('@tauri-apps/api/core');
    const request=crypto.randomUUID();
    await invoke('tray_detail',{action:'show',payload:{request,owner:props.externalDetail,anchor:{x:r.x,y:r.y,width:r.width,height:r.height},content:{displayed:key,detailEntry:detailEntry.value,period:period.value,resetCredits:props.resetCredits,now:props.now}}});
    if(!disposed&&hoverTarget===key)expanded.value=true;
    else await invoke('tray_detail',{action:'hide',payload:{owner:props.externalDetail,request}});
    return;
  }
  const next=geometry();if(!next)return;
  if(opening)motion.jump(next);else motion.to(next);
  if(switching&&!opening){fade.jump([.35]);fade.to([1]);}
  expanded.value=true;
}
function schedule(key:string) {
  if(key===hoverTarget)return;
  hoverTarget=key;clearTimeout(hoverTimer);
  hoverTimer=setTimeout(()=>void show(key),key?(expanded.value?0:(props.externalDetail?400:120)):180);
}
async function closeExternal(){if(props.externalDetail&&isDesktop){const {invoke}=await import('@tauri-apps/api/core');await invoke('tray_detail',{action:'hide',payload:{owner:props.externalDetail}});}}
watch(expanded,value=>{if(!value)void closeExternal();},{flush:'sync'});
watch(()=>props.dismissKey,()=>{overInline='';overDetail=false;clearTimeout(hoverTimer);hoverTarget='';expanded.value=false;});
let overInline='',overDetail=false;
function hover(key:string){
  if(isDesktop&&props.externalDetail){overInline=key;schedule(key||(overDetail?displayed.value:''));}
  else if(!(isDesktop&&props.rail))schedule(key);
}
function focus(key:string){schedule(key);}
function nativeHover({x,y}:{x:number;y:number}) {
  const contains=(el:HTMLElement|null|undefined)=>{const r=el?.getBoundingClientRect();return r && x>=r.left && x<=r.right && y>=r.top && y<=r.bottom;};
  schedule(Object.keys(triggers.value).find(key=>contains(triggers.value[key])) ?? (expanded.value&&contains(detail.value)?displayed.value:''));
}
watchEffect(()=>{
  const {left,top,right,bottom,width,height}=bounds;
  emit('popover', expanded.value&&width.value&&height.value?[[left.value-24,top.value-24],[right.value+24,top.value-24],[right.value+24,bottom.value+24],[left.value-24,bottom.value+24]]:[]);
});
onMounted(async()=>{
  if(isDesktop&&props.externalDetail){
    const {listen}=await import('@tauri-apps/api/event');
    const off=await listen<{owner:string;inside:boolean;dismiss?:boolean}>('tray-detail:hover',e=>{
      if(e.payload.owner!==props.externalDetail)return;
      if(e.payload.dismiss){overInline='';overDetail=false;clearTimeout(hoverTimer);hoverTarget='';expanded.value=false;return;}
      overDetail=e.payload.inside;
      if(overDetail){clearTimeout(hoverTimer);hoverTarget=displayed.value;}
      else schedule(overInline);
    });
    if(disposed)off();else disposeHover=off;
    return;
  }
  if(!isDesktop||!props.rail)return;
  const {listen}=await import('@tauri-apps/api/event');
  const off=await listen<{x:number;y:number}>('usage-panel:hover',event=>nativeHover(event.payload));
  if(disposed)off();else disposeHover=off;
});
onUnmounted(()=>{void closeExternal();disposed=true;disposeHover?.();clearTimeout(hoverTimer);emit('popover',[]);});
const tokens=new Intl.NumberFormat(undefined,{notation:'compact',maximumFractionDigits:1});
const dollars=new Intl.NumberFormat(undefined,{style:'currency',currency:'USD',maximumFractionDigits:2});
// Keep the tail roots within the straight edge, clear of both rounded corners.
const arrowStyle = computed(() => {
  const verticalSpan=Math.max(4,Math.min(80,motion.value.value[2]!+36-40));
  const horizontalSpan=Math.max(4,Math.min(80,(bounds.width.value||250)-40));
  function tail(span:number,side:'left'|'right'|'top'|'bottom') {
    const depth=Math.min(20,span/2), half=span/2;
    const point=(x:number,y:number)=>side==='left'?`${20-x},${y}`:side==='right'?`${x},${y}`:side==='top'?`${y},${20-x}`:`${y},${x}`;
    return `path('M${point(0,0)} C${point(0,half*.72)} ${point(depth*.58,half*.78)} ${point(depth,half)} C${point(depth*.58,half*1.22)} ${point(0,span-half*.72)} ${point(0,span)} Z')`;
  }
  return {
    width: 'var(--usage-tail-width)', height: 'var(--usage-tail-height)',
    top: 'var(--usage-tail-top)', bottom: 'var(--usage-tail-bottom)',
    left: 'var(--usage-tail-left)', right: 'var(--usage-tail-right)',
    transform: 'var(--usage-tail-transform)', clipPath: 'var(--usage-tail-shape)',
    boxShadow: 'none',
    '--tail-vertical-span': `${verticalSpan}px`, '--tail-horizontal-span': `${horizontalSpan}px`,
    '--tail-anchor-y': `${motion.value.value[1]! - bounds.top.value}px`,
    '--tail-anchor-x': `${motion.value.value[0]! - bounds.left.value}px`,
    '--tail-left': tail(verticalSpan,'left'), '--tail-right': tail(verticalSpan,'right'),
    '--tail-top': tail(horizontalSpan,'top'), '--tail-bottom': tail(horizontalSpan,'bottom'),
  };
});

</script>
<template>
  <slot :register="register" :hover="hover" :focus="focus" :expanded="expanded" :displayed="displayed"/>
  <div class="usage-history" v-if="resetCount!=null || history?.periods?.some(p=>usagePeriods.includes(p.id))">
    <div v-if="resetCount!=null" :ref="el=>register('resets',el)" class="history-period" @mouseenter="!valueTrigger && hover('resets')" @mouseleave="!valueTrigger && hover('')">
      <span>{{t('usageResetCount')}}</span>
      <NButton text :theme-overrides="usageValueButtonTheme" class="history-toggle" @mouseenter="valueTrigger && hover('resets')" @mouseleave="valueTrigger && hover('')" @focus="focus('resets')" @blur="focus('')" :aria-expanded="expanded&&displayed==='resets'" @click.stop>
        <span>{{t('usageResetCountValue',{count:resetCount})}}</span>
      </NButton>
    </div>
    <template v-if="usagePeriods.length">
      <div v-for="row in history?.periods?.filter(p=>usagePeriods.includes(p.id))" :key="row.id" :ref="el=>register(row.id,el)" class="history-period" @mouseenter="!valueTrigger && hover(row.id)" @mouseleave="!valueTrigger && hover('')">
        <span>{{t(labels[row.id])}}</span>
        <NButton text :theme-overrides="usageValueButtonTheme" class="history-toggle" @mouseenter="valueTrigger && hover(row.id)" @mouseleave="valueTrigger && hover('')" @focus="focus(row.id)" @blur="focus('')" :aria-expanded="expanded&&displayed===row.id" @click.stop>
          <span>{{row.tokens?`${row.estimatedUsd==null?'—':dollars.format(row.estimatedUsd)} · ${tokens.format(row.tokens)} tokens`:'-'}}</span>
        </NButton>
      </div>
    </template>
  </div>
    <NPopover v-if="!externalDetail || !isDesktop" class="history-popover" scrollable arrow-class="usage-detail-arrow" :arrow-style="arrowStyle" trigger="manual" :x="motion.value.value[0]" :y="popoverY" :theme-overrides="usagePopoverTheme(!!rail,horizontalGap)" :placement="detailPlacement ?? (rail ? 'right' : 'left')" :show="expanded" display-directive="show" @mouseenter="hover(displayed)" @mouseleave="hover('')" :style="{width:'250px',height:`${motion.value.value[2]!+36}px`,boxSizing:'border-box',maxWidth:'calc(100vw - 32px)'}">
      <div ref="content" :style="{opacity:fade.value.value[0]}"><UsageDetail :displayed="displayed" :detail-entry="detailEntry" :period="period" :reset-credits="resetCredits" :now="now" :rail="rail"/></div>
    </NPopover>
</template>
<style scoped>
.usage-history{display:flex;flex-direction:column;gap:9px;font-size:12px;line-height:1.4}
.history-toggle{flex-shrink:0;font:inherit;color:inherit;padding:3px 6px;margin:-3px -6px;border-radius:7px}
.history-period{display:flex;justify-content:space-between;align-items:baseline;gap:10px;font-variant-numeric:tabular-nums;width:100%}
.history-period>span:last-child{text-align:right;flex-shrink:0}
.usage-history p{font-size:10px;margin:0;color:var(--quota-muted,var(--muted))}
</style>
<style>
[v-placement^="right"] .usage-detail-arrow{--usage-tail-width:20px;--usage-tail-height:var(--tail-vertical-span);--usage-tail-top:clamp(calc(20px + var(--tail-vertical-span)/2),var(--tail-anchor-y),calc(100% - 20px - var(--tail-vertical-span)/2));--usage-tail-bottom:auto;--usage-tail-right:0;--usage-tail-left:auto;--usage-tail-transform:translateY(-50%);--usage-tail-shape:var(--tail-left)}
[v-placement^="left"] .usage-detail-arrow{--usage-tail-width:20px;--usage-tail-height:var(--tail-vertical-span);--usage-tail-top:clamp(calc(20px + var(--tail-vertical-span)/2),var(--tail-anchor-y),calc(100% - 20px - var(--tail-vertical-span)/2));--usage-tail-bottom:auto;--usage-tail-left:0;--usage-tail-right:auto;--usage-tail-transform:translateY(-50%);--usage-tail-shape:var(--tail-right)}
[v-placement^="top"] .usage-detail-arrow{--usage-tail-width:var(--tail-horizontal-span);--usage-tail-height:20px;--usage-tail-left:clamp(calc(20px + var(--tail-horizontal-span)/2),var(--tail-anchor-x),calc(100% - 20px - var(--tail-horizontal-span)/2));--usage-tail-right:auto;--usage-tail-top:0;--usage-tail-bottom:auto;--usage-tail-transform:translateX(-50%);--usage-tail-shape:var(--tail-bottom)}
[v-placement^="bottom"] .usage-detail-arrow{--usage-tail-width:var(--tail-horizontal-span);--usage-tail-height:20px;--usage-tail-left:clamp(calc(20px + var(--tail-horizontal-span)/2),var(--tail-anchor-x),calc(100% - 20px - var(--tail-horizontal-span)/2));--usage-tail-right:auto;--usage-tail-bottom:0;--usage-tail-top:auto;--usage-tail-transform:translateX(-50%);--usage-tail-shape:var(--tail-top)}
</style>
