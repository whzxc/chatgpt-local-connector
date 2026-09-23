<script setup lang="ts" generic="T extends string">
import { NRadioGroup, NRadioButton, NSelect, type SelectOption } from 'naive-ui';
import { computed, ref, h, type Component } from 'vue';
import { useElementSize } from '@vueuse/core';
const props = defineProps<{ value: T; options: ReadonlyArray<{ value:T; label:string; disabled?:boolean; icon?:Component; color?:string }>; appearance?:'default'|'swatches'; disabled?:boolean; label:string }>();
const container = ref<HTMLElement>();
const track = ref<HTMLElement>();
const { width: availableWidth } = useElementSize(container);
const { width: trackWidth } = useElementSize(track, undefined, { box: 'border-box' });
const dropdown = computed(() => props.options.length > 5 || availableWidth.value > 0 && trackWidth.value > availableWidth.value + 1);
function renderLabel(option: SelectOption) {
  const item = props.options.find(item => item.value === option.value);
  return h('span', { style: { display: 'inline-flex', alignItems: 'center', gap: '5px' } }, [item?.icon ? h(item.icon, { style: { width: '16px', height: '16px', objectFit: 'contain', flexShrink: 0 }, 'aria-hidden': true }) : null, String(option.label ?? '')]);
}
const emit = defineEmits<{ 'update:value':[value:T] }>();
</script>
<template>
  <NRadioGroup v-if="appearance==='swatches'" class="color-choices" :value="value" :disabled="disabled" :aria-label="label" @update:value="emit('update:value',$event as T)">
    <NRadioButton v-for="option in options" :key="option.value" class="color-choice" :class="{selected:value===option.value}" :style="{'--swatch':option.color}" :value="option.value" :disabled="disabled || option.disabled" :aria-label="option.label" :title="option.label">
      <span class="color-dot" aria-hidden="true"/>
    </NRadioButton>
  </NRadioGroup>
  <div v-else ref="container" class="choice-layout">
    <div v-if="options.length <= 5" ref="track" class="choice-measure" :class="{concealed:dropdown}" :inert="dropdown" :aria-hidden="dropdown">
    <NRadioGroup class="choice-track" :class="{compact:options.length===5}" :value="value" :disabled="disabled" :aria-label="label" @update:value="emit('update:value',$event as T)">
      <NRadioButton class="choice-option" :class="{selected:value===option.value,'is-disabled':disabled || option.disabled}" v-for="option in options" :key="option.value" :value="option.value" :disabled="disabled || option.disabled"><component :is="option.icon" v-if="option.icon" class="choice-icon" aria-hidden="true"/><slot name="option" :option="option">{{option.label}}</slot></NRadioButton>
    </NRadioGroup>
    </div>
    <NSelect v-if="dropdown" :value="value" :options="[...options]" :render-label="renderLabel" :disabled="disabled" :input-props="{'aria-label':label}" @update:value="emit('update:value',$event as T)"/>
  </div>
</template>
<style scoped>
.choice-layout{position:relative;width:100%;min-width:0}
.choice-measure{width:max-content}
.choice-measure.concealed{position:absolute;visibility:hidden;pointer-events:none;top:0;left:0}
.color-choices{display:inline-flex;align-items:center;gap:6px;padding:3px;max-width:100%;flex-wrap:wrap}
.color-choice.color-choice{display:inline-flex;align-items:center;justify-content:center;width:28px;height:28px;min-width:28px;padding:0;border:1px solid var(--line);border-radius:50%;background:var(--surface);line-height:1;box-shadow:none;transition:box-shadow 160ms ease,transform 160ms ease}
.color-dot{display:block;width:20px;height:20px;border-radius:50%;background:var(--swatch)}
.color-choice.selected{border-color:var(--accent);box-shadow:0 0 0 1px var(--accent)}
.color-choice:not(.n-radio-button--disabled):hover{transform:translateY(-1px)}
.color-choice:focus-within{outline:2px solid var(--ink);outline-offset:5px}
@media(prefers-reduced-motion:reduce){.color-choice.color-choice{transition:none}}

.choice-track.choice-track{display:inline-flex;height:auto;min-height:38px;align-items:center;max-width:100%;padding:4px;gap:2px;border-radius:12px;background:var(--accent-soft);vertical-align:middle;isolation:isolate}
.choice-option.choice-option{border:0;border-radius:9px;min-width:64px;text-align:center;flex:1 1 auto;white-space:nowrap;transition:background-color 160ms ease,box-shadow 160ms ease,color 160ms ease}
.choice-track.compact .choice-option{padding-inline:10px}
.choice-option.selected{box-shadow:0 1px 3px color-mix(in srgb,var(--black) 7.84%,transparent),0 0 0 1px light-dark(color-mix(in srgb,var(--black) 5.1%,transparent),color-mix(in srgb,var(--white) 4.71%,transparent))}
.choice-option:not(.selected):not(.is-disabled):hover{background:var(--hover-surface)}
.choice-option:focus-within{outline:2px solid var(--accent);outline-offset:1px;z-index:1}
.choice-option.is-disabled{cursor:default}
.choice-icon{display:inline-flex;width:16px;height:16px;object-fit:contain;vertical-align:middle;margin-right:5px;flex-shrink:0}
@media(prefers-reduced-motion:reduce){.choice-option.choice-option{transition:none}}
</style>
