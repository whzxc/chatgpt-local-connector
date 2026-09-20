<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ title: string }>();
const headingId = useId();
</script>

<template>
  <section class="settings-group" :aria-labelledby="headingId">
    <h2 :id="headingId">{{ title }}</h2>
    <div class="settings-group-body"><slot /></div>
  </section>
</template>

<style scoped>
.settings-group > h2 { margin: 0 0 8px 4px; font-size: 13px; line-height: 20px; font-weight: 600; }
.settings-group-body { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
</style>
