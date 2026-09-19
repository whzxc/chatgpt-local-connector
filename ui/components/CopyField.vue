<script setup lang="ts">
import { ref } from "vue";
import { useClipboard } from "@vueuse/core";
import { useConnector } from "../composables/useConnector";
const props = defineProps<{ value: string; label: string }>();
const { copy, copied } = useClipboard({
  source: () => props.value,
  legacy: true,
});
const { notify } = useConnector();
const field = ref<HTMLInputElement>();
async function copyValue() {
  try {
    await copy();
  } catch {
    field.value?.select();
    notify("复制失败，请复制已选中的内容。", true);
  }
}
</script>
<template>
  <div class="copy-row">
    <input ref="field" :value="value" readonly :aria-label="label" /><button
      type="button"
      @click="copyValue"
    >
      {{ copied ? "已复制" : "复制" }}
    </button>
  </div>
</template>
