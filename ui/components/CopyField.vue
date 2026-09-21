<script setup lang="ts">
import { t } from '../i18n';
import { ref } from "vue";
import { useClipboard } from "@vueuse/core";
import { useConnector } from "../composables/useConnector";
const props = defineProps<{ value: string; label: string; multiline?: boolean }>();
const { copy, copied } = useClipboard({
  source: () => props.value,
  legacy: true,
});
const { notify } = useConnector();
const field = ref<HTMLInputElement | HTMLTextAreaElement>();
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
  <div class="copy-row" :class="{ 'copy-multiline': multiline }">
    <textarea v-if="multiline" ref="field" :value="value" readonly :aria-label="label" rows="3" /><input v-else ref="field" :value="value" readonly :aria-label="label" /><button
      type="button"
      @click="copyValue"
    >
      {{ copied ? t('copied') : t('copy') }}
    </button>
  </div>
</template>

<style scoped>
.copy-multiline{align-items:flex-start}.copy-multiline textarea{flex:1;min-width:0;width:100%;resize:none;font:inherit;font-size:12px;line-height:1.7;padding:9px 12px}
</style>
