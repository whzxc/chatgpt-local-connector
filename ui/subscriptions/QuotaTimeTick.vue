<script setup lang="ts">
import { computed } from 'vue';
import { useNow } from '@vueuse/core';
import { quotaPace } from './pace';
import type { ProviderSnapshot, QuotaWindow } from './types';
const props = defineProps<{ provider: ProviderSnapshot; window?: QuotaWindow; center: number; radius: number }>();
const now = useNow({ interval: 60000 });
const tick = computed(() => props.window ? quotaPace(props.window, props.provider, now.value.getTime())?.tick : undefined);
</script>
<template>
  <line v-if="tick != null" class="quota-time-tick" :x1="center" :x2="center" :y1="center-radius-2" :y2="center-radius+2"
    :transform="`rotate(${tick*3.6} ${center} ${center})`" aria-hidden="true"/>
</template>
<style scoped>
.quota-time-tick{stroke:var(--rail-muted,var(--muted));stroke-width:1.5;stroke-linecap:round;pointer-events:none}
</style>
