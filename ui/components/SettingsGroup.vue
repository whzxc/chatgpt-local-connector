<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ title: string }>();
const headingId = useId();
</script>

<template>
  <section class="settings-group" :aria-labelledby="headingId">
    <div class="settings-group-heading"><h2 :id="headingId">{{ title }}</h2><slot name="heading-actions" /></div>
    <div class="settings-group-body"><slot /></div>
  </section>
</template>

<style scoped>
.settings-group-heading { display: flex; align-items: center; justify-content: space-between; gap: 6px; margin: 0 0 8px 4px; min-height: 22px; }
.settings-group-heading h2 { margin: 0; font-size: 13px; line-height: 20px; font-weight: 600; }
.settings-group-body { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
</style>
