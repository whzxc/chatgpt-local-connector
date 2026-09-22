<script setup lang="ts" generic="T extends string">
import { NRadioGroup, NRadioButton, NSelect } from 'naive-ui';
import type { Component } from 'vue';
defineProps<{ value: T; options: ReadonlyArray<{ value:T; label:string; disabled?:boolean; icon?:Component; color?:string }>; appearance?:'default'|'swatches'; disabled?:boolean; label:string }>();
const emit = defineEmits<{ 'update:value':[value:T] }>();
</script>
<template>
  <NRadioGroup v-if="appearance==='swatches'" class="color-choices" :value="value" :disabled="disabled" :aria-label="label" @update:value="emit('update:value',$event as T)">
    <NRadioButton v-for="option in options" :key="option.value" class="color-choice" :class="{selected:value===option.value}" :style="{'--swatch':option.color}" :value="option.value" :disabled="disabled || option.disabled" :aria-label="option.label" :title="option.label">
      <span class="color-dot" aria-hidden="true"/>
    </NRadioButton>
  </NRadioGroup>
  <NRadioGroup class="choice-track" v-else-if="options.length <= 4" :value="value" :disabled="disabled" :aria-label="label" @update:value="emit('update:value',$event as T)">
    <NRadioButton class="choice-option" :class="{selected:value===option.value,'is-disabled':disabled || option.disabled}" v-for="option in options" :key="option.value" :value="option.value" :disabled="disabled || option.disabled"><component :is="option.icon" v-if="option.icon" class="choice-icon" aria-hidden="true"/><slot name="option" :option="option">{{option.label}}</slot></NRadioButton>
  </NRadioGroup>
  <NSelect v-else :value="value" :options="[...options]" :disabled="disabled" :input-props="{'aria-label':label}" @update:value="emit('update:value',$event as T)"/>
</template>
<style scoped>
.color-choices{display:inline-flex;align-items:center;gap:6px;padding:3px;max-width:100%;flex-wrap:wrap}
.color-choice.color-choice{display:inline-flex;align-items:center;justify-content:center;width:28px;height:28px;min-width:28px;padding:0;border:1px solid var(--line);border-radius:50%;background:var(--surface);line-height:1;box-shadow:none;transition:box-shadow 160ms ease,transform 160ms ease}
.color-dot{display:block;width:20px;height:20px;border-radius:50%;background:var(--swatch)}
.color-choice.selected{border-color:var(--swatch);box-shadow:0 0 0 1px var(--swatch)}
.color-choice:not(.n-radio-button--disabled):hover{transform:translateY(-1px)}
.color-choice:focus-within{outline:2px solid var(--ink);outline-offset:5px}
@media(prefers-reduced-motion:reduce){.color-choice.color-choice{transition:none}}

.choice-track.choice-track{display:inline-flex;height:auto;min-height:38px;align-items:center;max-width:100%;padding:4px;gap:2px;border-radius:12px;background:var(--accent-soft,light-dark(#eef1ed,#26372f));vertical-align:middle;isolation:isolate}
.choice-option.choice-option{border:0;border-radius:9px;min-width:64px;text-align:center;flex:1 1 auto;white-space:nowrap;transition:background-color 160ms ease,box-shadow 160ms ease,color 160ms ease}
.choice-option.selected{box-shadow:0 1px 3px #142e2114,0 0 0 1px light-dark(#1935250d,#ffffff0c)}
.choice-option:not(.selected):not(.is-disabled):hover{background:light-dark(#ffffff70,#ffffff08)}
.choice-option:focus-within{outline:2px solid var(--primary-color,#63927d);outline-offset:1px;z-index:1}
.choice-option.is-disabled{cursor:default}
.choice-icon{width:14px;height:14px;vertical-align:middle;margin-right:6px}
@media(prefers-reduced-motion:reduce){.choice-option.choice-option{transition:none}}
</style>
