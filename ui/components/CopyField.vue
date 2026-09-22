<script setup lang="ts">
import { t } from '../i18n';
import { NInput, NButton, NTooltip, NSpin, type InputInst } from "naive-ui";
import { Copy, Check, Eye, EyeOff } from '@lucide/vue';
import { ref } from "vue";
import { useClipboard } from "@vueuse/core";
import { useConnector } from "../composables/useConnector";
const props = defineProps<{ value: string; label: string; multiline?: boolean; password?: boolean; placeholder?: string; disabled?: boolean; loading?: boolean }>();
const { copy, copied } = useClipboard({
  source: () => props.value,
  legacy: true,
});
const { notify } = useConnector();
const field = ref<InputInst>();
const revealed = ref(false);
async function copyValue() {
  try {
    await copy();
  } catch {
    field.value?.select();
    notify(t('copyFailedCopyTheSelectedTextManually'), true);
  }
}
</script>
<template>
  <NInput ref="field" class="copy-control" :value="value" :type="multiline ? 'textarea' : password && !revealed ? 'password' : 'text'" :placeholder="placeholder" :disabled="disabled" :autosize="multiline ? {minRows:3,maxRows:6} : undefined" readonly :input-props="{'aria-label':label, autocomplete:password ? 'off' : undefined}">
    <template #suffix><span class="copy-actions"><NTooltip v-if="password && !multiline"><template #trigger><NButton text :disabled="disabled || !value" :aria-label="revealed ? t('hideSecret') : t('showSecret')" :aria-pressed="revealed" @click="revealed = !revealed"><template #icon><Eye v-if="revealed" :size="16"/><EyeOff v-else :size="16"/></template></NButton></template>{{revealed ? t('hideSecret') : t('showSecret')}}</NTooltip><NSpin v-if="loading" :size="16"/><NTooltip v-else><template #trigger><NButton text :disabled="disabled || !value" :aria-label="copied ? t('copied') : t('copy')" @click="copyValue"><template #icon><Check v-if="copied" :size="16"/><Copy v-else :size="16"/></template></NButton></template>{{copied ? t('copied') : t('copy')}}</NTooltip></span></template>
  </NInput>
</template>
<style scoped>.copy-control{width:100%;min-width:0}.copy-actions{display:inline-flex;align-items:center;gap:10px}</style>
