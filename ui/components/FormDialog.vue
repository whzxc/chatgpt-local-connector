<script setup lang="ts">
import { NModal } from 'naive-ui';
defineProps<{ show: boolean; title: string; busy?: boolean }>();
const emit = defineEmits<{ close: [] }>();
</script>
<template>
  <NModal :show="show" preset="card" :title="title" class="form-dialog" :closable="!busy" :mask-closable="!busy" :close-on-esc="!busy" :bordered="false" @update:show="!$event && emit('close')">
    <div class="form-dialog-body"><slot/></div>
    <template v-if="$slots.footer" #footer><div class="form-dialog-actions"><slot name="footer"/></div></template>
  </NModal>
</template>
<style>
.form-dialog{width:min(600px,calc(100vw - 32px));max-height:calc(100dvh - 48px)}
.form-dialog-body{max-height:calc(100dvh - 240px);overflow:auto;overscroll-behavior:contain;padding:2px 4px}
.form-dialog-actions{display:flex;align-items:center;justify-content:flex-end;flex-wrap:wrap;gap:8px}
.form-dialog-actions .action-spacer{flex:1}
.form-dialog .n-form-item:last-child{margin-bottom:0}
.form-help-link{font-size:12px}
</style>
