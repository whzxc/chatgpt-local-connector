<script setup lang="ts">
import { ref, inject, onMounted } from 'vue';
import { NAlert, NSwitch, NCheckboxGroup, NCheckbox } from 'naive-ui';
import { subscriptionSnapshotKey, subscriptionSettingsKey } from '../subscriptions/useSubscriptions';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import ElasticPanel from './ElasticPanel.vue';
import SingleChoice from './SingleChoice.vue';
import { quotaDisplay, resetDisplay, usagePeriods, usagePeriodLabels, maxVisibleAgents } from '../subscriptions/displayPreferences';
import { t } from '../i18n';
import { panelPreferences, readPanelPreferences, savePanelPreferences } from '../usage-rail/preferences';
import type { PanelPreferences } from '../usage-rail/layout';
defineProps<{ origin: { x: number; y: number; size: number } }>();
const emit = defineEmits<{ close: [] }>();
const open = ref(true);
const snapshot = inject(subscriptionSnapshotKey);
const saveSettings = inject(subscriptionSettingsKey);
const saving = ref(false), error = ref('');
async function setRefresh(value:string) {
  if (!saveSettings) return;
  saving.value=true;error.value='';
  try { await saveSettings({refreshMinutes:Number(value)}); }
  catch (e) { error.value=String(e); }
  finally { saving.value=false; }
}
onMounted(() => { readPanelPreferences().catch(e => error.value=String(e)); });
async function setAppearance(patch: Partial<PanelPreferences>) {
  saving.value=true;error.value='';
  try { await savePanelPreferences(patch); }
  catch(e) { error.value=String(e); }
  finally { saving.value=false; }
}

</script>
<template>
  <ElasticPanel :show="open" :title="t('agentDisplaySettings')" :origin="origin" :width="600" @close="open=false" @closed="emit('close')">
    <NAlert v-if="error" type="error">{{error}}</NAlert>
    <div class="agent-settings-groups">
    <SettingsGroup :title="t('usageAppearance')">
      <SettingsRow :title="t('usageShow')">
        <NSwitch :value="panelPreferences.visible !== false" :disabled="saving" :aria-label="t('usageShow')" @update:value="setAppearance({visible:$event})"/>
      </SettingsRow>
      <SettingsRow :title="t('usageSize')" :description="t('usageSizeDescription')">
        <SingleChoice :value="panelPreferences.size" :disabled="saving" :label="t('usageSize')" :options="[{value:'small',label:t('usageSmall')},{value:'standard',label:t('usageStandard')},{value:'large',label:t('usageLarge')}] as const" @update:value="setAppearance({size:$event})"/>
      </SettingsRow>
      <SettingsRow :title="t('usageSpacing')" :description="t('usageSpacingDescription')">
        <SingleChoice :value="panelPreferences.spacing" :disabled="saving" :label="t('usageSpacing')" :options="[{value:'compact',label:t('usageCompact')},{value:'standard',label:t('usageStandard')},{value:'roomy',label:t('usageRoomy')}] as const" @update:value="setAppearance({spacing:$event})"/>
      </SettingsRow>
      <SettingsRow :title="t('agentsMaxVisible')">
        <SingleChoice :value="String(maxVisibleAgents)" @update:value="maxVisibleAgents=Number($event)" :label="t('agentsMaxVisible')" :options="['4','6','8','10'].map(value=>({value,label:value}))"/>
      </SettingsRow>
    </SettingsGroup>
    <SettingsGroup :title="t('agentQuotaGroup')">
      <SettingsRow :title="t('agentQuotaDisplay')">
        <SingleChoice v-model:value="quotaDisplay" :label="t('agentQuotaDisplay')" :options="[{value:'remaining',label:t('usageRemaining')},{value:'used',label:t('usageUsed')}] as const"/>
      </SettingsRow>
      <SettingsRow :title="t('agentResetDisplay')">
        <SingleChoice v-model:value="resetDisplay" :label="t('agentResetDisplay')" :options="[{value:'countdown',label:t('usageCountdown')},{value:'time',label:t('usageResetTime')}] as const"/>
      </SettingsRow>
      <SettingsRow :title="t('usageAutoRefresh')">
        <SingleChoice :value="String(snapshot?.settings.refreshMinutes ?? 3)" :label="t('usageAutoRefresh')" :disabled="saving || !snapshot || !saveSettings" :options="['1','3','5','10'].map(value=>({value,label:`${value} min`}))" @update:value="setRefresh"/>
      </SettingsRow>
    </SettingsGroup>
    <SettingsGroup :title="t('usageGroup')">
      <SettingsRow :title="t('usageDuration')">
        <NCheckboxGroup v-model:value="usagePeriods" :aria-label="t('usageDuration')">
          <div class="usage-period-options"><NCheckbox v-for="(label,id) in usagePeriodLabels" :key="id" :value="id" :label="t(label)"/></div>
        </NCheckboxGroup>
      </SettingsRow>
    </SettingsGroup>

    </div>
  </ElasticPanel>
</template>

<style scoped>
.agent-settings-groups{display:flex;flex-direction:column;gap:24px}
.usage-period-options{display:flex;flex-wrap:wrap;gap:8px 16px}
</style>
