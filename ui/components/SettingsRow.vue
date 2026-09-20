<script setup lang="ts">
defineProps<{ title: string; description?: string; controlId?: string }>();
</script>

<template>
  <div class="settings-row">
    <div class="settings-row-copy">
      <component :is="controlId ? 'label' : 'span'" :for="controlId" class="settings-row-title">{{ title }}</component>
      <p v-if="description">{{ description }}</p>
    </div>
    <div class="settings-row-control"><slot /></div>
  </div>
</template>

<style scoped>
.settings-row { display: flex; align-items: center; justify-content: space-between; gap: 24px; min-height: 58px; padding: 12px 16px; position: relative; }
.settings-row + .settings-row::before { content: ''; position: absolute; top: 0; left: 16px; right: 16px; border-top: 1px solid var(--line); pointer-events: none; }
.settings-row-copy { min-width: 0; }
.settings-row-title { display: block; margin: 0; font-size: 13px; font-weight: 500; line-height: 20px; }
.settings-row-copy p { margin: 3px 0 0; color: var(--muted); font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }
.settings-row-control { display: flex; align-items: center; justify-content: flex-end; flex-shrink: 0; gap: 8px; }
@media (max-width: 480px) {
  .settings-row { gap: 12px; flex-wrap: wrap; }
  .settings-row-control { max-width: 100%; margin-left: auto; }
}
</style>
