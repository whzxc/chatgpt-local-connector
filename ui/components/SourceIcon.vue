<script setup lang="ts">
import { Plus, Plug } from '@lucide/vue';
import chatgptLogo from '../assets/brand-reserve/control-sources/chatgpt/app.png';
import notion from '../assets/brand-reserve/control-sources/notion/mono.svg?raw';
import slack from '../assets/brand-reserve/control-sources/slack/color.svg?raw';
defineProps<{ platform: string; add?: boolean }>();
const icons: Record<string,string> = {notion,slack};
</script>
<template>
  <span class="platform-icon" :class="{ 'platform-add':add }" aria-hidden="true">
    <Plus v-if="add"/><img v-else-if="platform==='chatgpt'" :src="chatgptLogo" alt=""/><span v-else-if="icons[platform]" v-html="icons[platform]"/><Plug v-else/>
  </span>
</template>
<style scoped>
.platform-icon{width:44px;height:44px;display:grid;place-items:center;color:inherit;flex-shrink:0}
.platform-icon img{width:100%;height:100%;object-fit:contain;border-radius:10px}.platform-icon span{display:contents}.platform-icon :deep(svg){width:100%;height:100%}
.platform-icon > :deep(svg){fill:none}
.platform-add{border:1px dashed currentColor;border-radius:12px;padding:8px;box-sizing:border-box}
</style>
