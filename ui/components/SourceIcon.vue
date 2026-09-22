<script setup lang="ts">
import { Plus, Plug } from '@lucide/vue';
import { computed } from 'vue';
import { controlSource } from '../controlSources';
import { sourceVectors } from '../assets/brand-reserve/control-sources';
const props = defineProps<{ platform: string; add?: boolean }>();
const icon = computed(() => controlSource(props.platform)?.icon || '');
</script>
<template>
  <span class="platform-icon" :class="{ 'platform-add':add }" aria-hidden="true">
    <Plus v-if="add"/><span v-else-if="sourceVectors[icon]" v-html="sourceVectors[icon]"/><Plug v-else/>
  </span>
</template>
<style scoped>
.platform-icon{width:44px;height:44px;display:grid;place-items:center;color:inherit;fill:currentColor;flex-shrink:0}
.platform-icon span{display:contents}.platform-icon :deep(svg){width:100%;height:100%}
.platform-icon > :deep(svg){fill:none}
.platform-add{border:1px dashed currentColor;border-radius:12px;padding:8px;box-sizing:border-box}
</style>
