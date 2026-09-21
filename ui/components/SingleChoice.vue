<script setup lang="ts" generic="T extends string">
import { NRadioGroup, NRadioButton, NSelect } from 'naive-ui';
import type { Component } from 'vue';
defineProps<{ value: T; options: ReadonlyArray<{ value:T; label:string; disabled?:boolean; icon?:Component }>; disabled?:boolean; label:string }>();
const emit = defineEmits<{ 'update:value':[value:T] }>();
</script>
<template>
  <NRadioGroup class="choice-track" v-if="options.length <= 4" :value="value" :disabled="disabled" :aria-label="label" @update:value="emit('update:value',$event as T)">
    <NRadioButton class="choice-option" :class="{selected:value===option.value,'is-disabled':disabled || option.disabled}" v-for="option in options" :key="option.value" :value="option.value" :disabled="disabled || option.disabled"><component :is="option.icon" v-if="option.icon" class="choice-icon" aria-hidden="true"/><slot name="option" :option="option">{{option.label}}</slot></NRadioButton>
  </NRadioGroup>
  <NSelect v-else :value="value" :options="[...options]" :disabled="disabled" :input-props="{'aria-label':label}" @update:value="emit('update:value',$event as T)"/>
</template>
<style scoped>
.choice-track.choice-track{display:inline-flex;height:auto;min-height:38px;align-items:center;max-width:100%;padding:4px;gap:2px;border-radius:12px;background:light-dark(#eef1ed,#26372f);vertical-align:middle;isolation:isolate}
.choice-option.choice-option{border:0;border-radius:9px;min-width:64px;text-align:center;flex:1 1 auto;white-space:nowrap;transition:background-color 160ms ease,box-shadow 160ms ease,color 160ms ease}
.choice-option.selected{box-shadow:0 1px 3px #142e2114,0 0 0 1px light-dark(#1935250d,#ffffff0c)}
.choice-option:not(.selected):not(.is-disabled):hover{background:light-dark(#ffffff70,#ffffff08)}
.choice-option:focus-within{outline:2px solid var(--primary-color,#63927d);outline-offset:1px;z-index:1}
.choice-option.is-disabled{cursor:default}
.choice-icon{width:14px;height:14px;vertical-align:middle;margin-right:6px}
@media(prefers-reduced-motion:reduce){.choice-option.choice-option{transition:none}}
</style>
