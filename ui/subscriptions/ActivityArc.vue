<script setup lang="ts">
import { t } from '../i18n';
</script>
<template><svg class="activity-arc" viewBox="0 0 48 48" role="img" :aria-label="t('agentTaskRunning')"><circle cx="24" cy="24" r="16" pathLength="100" stroke-dasharray="33 67"/></svg></template>
<style scoped>
.activity-arc{position:absolute;inset:0;width:100%;height:100%;pointer-events:none;overflow:visible;animation:agent-orbit 1.8s linear infinite}.activity-arc circle{fill:none;stroke:currentColor;stroke-width:1.8;stroke-linecap:round}
@keyframes agent-orbit{from{transform:rotate(-90deg)}to{transform:rotate(270deg)}}
@media(prefers-reduced-motion:reduce){.activity-arc{animation:none;transform:rotate(-90deg)}}
</style>
