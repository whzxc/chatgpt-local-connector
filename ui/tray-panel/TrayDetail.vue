<script setup lang="ts">
import {ref,onMounted,onUnmounted,nextTick} from 'vue';
import UsageDetail from '../subscriptions/UsageDetail.vue';
import type {UsageHistory,ProviderSnapshot} from '../subscriptions/types';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
type Payload={nativeChrome:boolean;request:string;content:{displayed:string;detailEntry?:{lines:string[]};period?:NonNullable<UsageHistory['periods']>[number];resetCredits?:ProviderSnapshot['resetCredits'];now:number}};
const data=ref<Payload>(),content=ref<HTMLElement>();
let disposed=false;const stops:(()=>void)[]=[];
async function hover(inside:boolean){await invoke('tray_detail',{action:'hover',payload:{inside}});}
onMounted(async()=>{
  const events=await Promise.all([
    listen<Payload>('tray-detail:content',async e=>{
      data.value=e.payload;await nextTick();
      if(content.value)content.value.scrollTop=0;
      await invoke('tray_detail',{action:'ready',payload:{request:e.payload.request,dark:document.documentElement.dataset.theme==='dark'||(document.documentElement.dataset.theme==='system'&&matchMedia('(prefers-color-scheme: dark)').matches),animate:!matchMedia('(prefers-reduced-motion: reduce)').matches,height:content.value?.scrollHeight??360}});
    }),

  ]);
  if(disposed)events.forEach(off=>off());else stops.push(...events);
});
onUnmounted(()=>{disposed=true;stops.forEach(off=>off());});
</script>
<template>
  <div ref="content" class="detail-body" :class="{fallback:data&&!data.nativeChrome}" @mouseenter="hover(true)" @mouseleave="hover(false)"><UsageDetail v-if="data" v-bind="data.content" native-tooltip/></div>
</template>
<style scoped>
.detail-body{box-sizing:border-box;padding:14px;max-height:100%;overflow:auto;scrollbar-width:thin;color:var(--ink)}
.fallback{background:var(--surface);border:1px solid var(--line);border-radius:14px}
</style>
