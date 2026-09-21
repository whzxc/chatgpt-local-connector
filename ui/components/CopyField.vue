<script setup lang="ts">
import { t } from '../i18n';
import { NInput, NButton, type InputInst } from "naive-ui";
import { ref } from "vue";
import { useClipboard } from "@vueuse/core";
import { useConnector } from "../composables/useConnector";
const props = defineProps<{ value: string; label: string; multiline?: boolean }>();
const { copy, copied } = useClipboard({
  source: () => props.value,
  legacy: true,
});
const { notify } = useConnector();
const field = ref<InputInst>();
async function copyValue() {
  try {
    await copy();
  } catch {
    field.value?.select();
    notify(t('copyFailedCopyTheSelectedTextManually'), true);
  }
}
</script>
<template><div class="copy-control"><NInput ref="field" :value="value" :type="multiline ? 'textarea' : 'text'" :autosize="multiline ? {minRows:3,maxRows:6} : undefined" readonly :input-props="{'aria-label':label}"/><NButton @click="copyValue">{{copied ? t('copied') : t('copy')}}</NButton></div></template>
<style scoped>.copy-control{display:flex;align-items:flex-start;gap:8px;width:100%}.copy-control .n-input{flex:1;min-width:0}</style>
