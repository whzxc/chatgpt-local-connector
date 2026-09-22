<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { NAlert, NButton, NCheckbox, NFormItem, NText, NTooltip } from 'naive-ui';
import ElasticPanel from './ElasticPanel.vue';
import SingleChoice from './SingleChoice.vue';
import { api } from '../composables/useConnector';
import { t, locale } from '../i18n';
import { identifyPolicy, normalizeSelection, presetPolicy, type ToolCatalog, type ToolMode, type ToolPolicy } from '../toolPolicy';
const props = defineProps<{ value?: ToolPolicy; disabled?: boolean }>();
const emit = defineEmits<{ 'update:value': [value: ToolPolicy] }>();
const catalog = ref<ToolCatalog>();
const mode = ref<ToolMode>(!props.value || props.value === 'all' ? 'all' : 'custom');
const selected = ref<string[]>([]);
const failure = ref(false);
const detailsOpen = ref(false);
const detailSelection = ref<string[]>([]);
const exposedTools = computed(() => catalog.value?.tools.filter(tool => mode.value === 'all' || selected.value.includes(tool.name)) ?? []);
const toolCount = computed(() => exposedTools.value.length);
const groupCount = computed(() => new Set(exposedTools.value.map(tool => tool.group)).size);
const detailTitle = computed(() => `${t('toolList')} · ${options.value.find(option => option.value === mode.value)?.label}`);
function openDetails() {
  detailSelection.value = mode.value === 'all' ? catalog.value!.tools.map(tool => tool.name) : [...selected.value];
  detailsOpen.value = true;
}
function applyDetails() {
  publish(detailSelection.value);
  detailsOpen.value = false;
}
const options = computed(() => [
  { value: 'all' as const, label: t('toolsAll') },
  { value: 'common' as const, label: t('toolsCommon') },
  { value: 'readOnly' as const, label: t('toolsReadOnly') },
  { value: 'custom' as const, label: t('toolsCustom') },
]);
async function load() {
  failure.value = false;
  try {
    catalog.value = await api<ToolCatalog>('tool-catalog');
    mode.value = identifyPolicy(props.value, catalog.value);
    selected.value = props.value && props.value !== 'all' ? [...props.value.allowlist] : catalog.value.tools.map(tool => tool.name);
  } catch { failure.value = true; }
}
onMounted(load);
function publish(names: string[]) {
  selected.value = normalizeSelection(names, catalog.value!);
  emit('update:value', { allowlist: [...selected.value] });
}
function choose(next: ToolMode) {
  mode.value = next;
  if (next === 'all') {
    selected.value = catalog.value!.tools.map(tool => tool.name);
    emit('update:value', 'all');
    return;
  }
  if (next !== 'custom') selected.value = (presetPolicy(next, catalog.value!) as { allowlist: string[] }).allowlist;
  publish(selected.value);
}
const groups = computed(() => catalog.value?.groups.map(group => ({ ...group, tools: catalog.value!.tools.filter(tool => tool.group === group.id && (mode.value === 'custom' || detailSelection.value.includes(tool.name))) })).filter(group => group.tools.length) ?? []);
// Required dependencies are visible and locked while an operation needs them.
const locked = computed(() => {
  const names = new Set(catalog.value?.required ?? []);
  for (const tool of catalog.value?.tools ?? []) if (detailSelection.value.includes(tool.name)) for (const name of tool.requires) names.add(name);
  return names;
});
function toggle(names: string[], checked: boolean) {
  detailSelection.value = normalizeSelection(checked ? [...detailSelection.value, ...names] : detailSelection.value.filter(name => !names.includes(name)), catalog.value!);
}
</script>
<template>
  <NFormItem :label="t('toolList')">
    <div class="tool-policy">
      <SingleChoice :value="mode" :options="options" :label="t('toolList')" :disabled="disabled || !catalog" @update:value="choose"/>
      <div v-if="catalog" class="tool-summary"><NText depth="3">{{t('toolSelectionCount', {groups:groupCount, count:toolCount})}}</NText><NButton text type="primary" :disabled="disabled" @click="openDetails">{{t('toolViewAll')}}</NButton></div>
      <NAlert v-if="failure" type="error" :show-icon="false">{{t('toolCatalogFailed')}} <NButton text @click="load">{{t('toolCatalogRetry')}}</NButton></NAlert>
    </div>
  </NFormItem>
  <ElasticPanel v-if="catalog" :show="detailsOpen" :width="600" :title="detailTitle" :busy="disabled" @close="detailsOpen=false">
    <div class="tool-details">
      <div v-for="group in groups" :key="group.id" class="tool-group">
        <NCheckbox v-if="mode==='custom'" :disabled="disabled" :checked="group.tools.every(tool=>detailSelection.includes(tool.name))" :indeterminate="group.tools.some(tool=>detailSelection.includes(tool.name)) && !group.tools.every(tool=>detailSelection.includes(tool.name))" @update:checked="toggle(group.tools.map(tool=>tool.name), $event)">{{group.label[locale==='zh-CN' ? 'zh-CN' : 'en']}}</NCheckbox>
        <NText v-else strong>{{group.label[locale==='zh-CN' ? 'zh-CN' : 'en']}}</NText>
        <div class="tool-items">
          <span v-for="tool in group.tools" :key="tool.name" class="tool-entry">
            <NCheckbox v-if="mode==='custom'" :checked="detailSelection.includes(tool.name)" :disabled="disabled || detailSelection.includes(tool.name) && locked.has(tool.name)" @update:checked="toggle([tool.name], $event)">
              <NTooltip :style="{maxWidth:'420px'}"><template #trigger><span class="tool-name">{{tool.name}}</span></template>{{tool.description}}</NTooltip>
            </NCheckbox>
            <NTooltip v-else :style="{maxWidth:'420px'}"><template #trigger><NText class="tool-name">{{tool.name}}</NText></template>{{tool.description}}</NTooltip>
          </span>
        </div>
      </div>
    </div>
    <template #footer>
      <NButton v-if="mode==='custom'" :disabled="disabled" @click="detailsOpen=false">{{t('cancel')}}</NButton>
      <NButton type="primary" :disabled="disabled" @click="mode==='custom' ? applyDetails() : detailsOpen=false">{{mode==='custom' ? t('toolSelectionDone') : t('close')}}</NButton>
    </template>
  </ElasticPanel>
</template>
<style scoped>
.tool-policy{display:grid;gap:8px;width:100%;min-width:0}.tool-summary{display:flex;align-items:center;gap:12px;flex-wrap:wrap;font-size:14px}.tool-details{display:grid;gap:16px}.tool-group{display:grid;gap:8px}.tool-items{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:6px 12px;padding-left:16px}.tool-entry{min-width:0}.tool-name{display:inline-block}
@media(max-width:420px){.tool-items{grid-template-columns:minmax(0,1fr)}}
</style>
