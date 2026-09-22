<script setup lang="ts">
import { t, language, languageOptions, setLanguage } from '../i18n';
import { NButton, NSwitch, NInput } from 'naive-ui';
import SingleChoice from './SingleChoice.vue';
import { theme, themeColor, themeColors, translucent } from '../theme';
import { controlSourceClick } from '../behavior';
import { Monitor, Sun, Moon } from '@lucide/vue';
import { useIntervalFn } from '@vueuse/core';
import { computed, onMounted, ref } from 'vue';
import { api, useConnector, type Service } from '../composables/useConnector';
import { isDesktop } from '../platform';
import AppUpdate from './AppUpdate.vue';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import '../settings.css';
const { status, busy, run, refresh, notify } = useConnector();
const proxyMode = ref(status.value?.config.proxyMode || 'system');
const proxyUrl = ref(status.value?.config.proxyUrl || '');
async function saveProxy() {
  if (proxyMode.value==='custom') {
    try { const url=new URL(proxyUrl.value.trim()); if(!['http:','https:'].includes(url.protocol)) throw new Error(); }
    catch { throw new Error(t('validProxyUrl')); }
  }
  await api('network', 'PUT', { proxyMode: proxyMode.value, proxyUrl: proxyMode.value === 'custom' ? proxyUrl.value.trim() : '' });
  await refresh();
  proxyUrl.value = status.value?.config.proxyUrl || '';
  notify(status.value?.connection?.running ? t('proxySavedReconnectToApplyNewDownloadsUse') : t('proxySettingsSaved'));
}
async function changeProxy() {
  if (proxyMode.value === 'custom') return;
  await run('network', saveProxy);
  proxyMode.value = status.value?.config.proxyMode || 'system';
}
const mac = navigator.platform.toLowerCase().includes('mac');
const appearance = ref({ showMenuBar: true, showDock: true });
const appearanceReady = ref(false);
onMounted(async () => {
  if (!mac || !isDesktop) return;
  try {
    appearance.value = await api<typeof appearance.value>('appearance');
    appearanceReady.value = true;
  } catch (error) { notify(error instanceof Error ? error.message : String(error), true); }
});
const displayPosition = computed(() => appearance.value.showMenuBar ? (appearance.value.showDock ? 'all' : 'menu') : 'dock');
async function setAppearance(position: 'all' | 'menu' | 'dock') {
  const next = { showMenuBar: position !== 'dock', showDock: position !== 'menu' };
  await run('appearance', async () => {
    appearance.value = await api<typeof appearance.value>('appearance', 'PUT', next);
  });
}
const service = ref<Service>();
const notifications = ref(localStorage.getItem('notifications') !== 'off');
const refreshService = () => api<Service>('service').then(value => service.value = value).catch(() => {});
onMounted(refreshService);
useIntervalFn(refreshService, 5000);
async function startup(enabled: boolean) {
  await run('startup', async () => { service.value = await api<Service>('service', 'POST', { enabled }); });
}
function preferences() {
  localStorage.setItem('notifications', notifications.value ? 'on' : 'off');
}
async function setAutoOpen(autoOpenCodex: boolean) {
  await run('task-settings', async () => { await api('task-settings', 'PUT', { autoOpenCodex }); await refresh(); });
}
async function setApproval(enabled: boolean) {
  await run('task-settings', async () => { await api('task-settings', 'PUT', { enabled }); await refresh(); });
}
</script>
<template>
  <div class="settings-preferences">


    <SettingsGroup :title="t('behavior')">
      <SettingsRow :title="t('clickControlSource')">
        <SingleChoice v-model:value="controlSourceClick" :label="t('clickControlSource')" :options="[{value:'service',label:t('goToService')},{value:'settings',label:t('connectionSettings')}] as const"/>
      </SettingsRow>
      <SettingsRow :title="t('automaticallyOpenCodexTasks')" :description="t('whenDisabledNewTasksRunInTheBackground')" control-id="settings-auto-open">
        <NSwitch id="settings-auto-open" :aria-label="t('automaticallyOpenCodexTasks')"  :value="status?.autoOpenCodex !== false" :disabled="!!busy || !status" @update:value="setAutoOpen" />
      </SettingsRow>
      <SettingsRow :title="t('taskApprovalMode')" :description="status?.taskApprovalEnabled ? t('askForConfirmationByDefaultTheCloudCan') : t('submitTaskRequestsImmediately')" control-id="settings-approval">
        <NSwitch id="settings-approval" :aria-label="t('taskApprovalMode')"  :value="!!status?.taskApprovalEnabled" :disabled="!!busy || !status" @update:value="setApproval" />
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup :title="t('general')">
      <SettingsRow :title="t('connectAtSystemSignIn')" :description="t('theConnectionKeepsRunningAfterTheWindowCloses')" control-id="settings-startup">
        <NSwitch id="settings-startup" :aria-label="t('connectAtSystemSignIn')"  :value="service?.enabled" :disabled="!!busy || !service?.supported" @update:value="startup" />
      </SettingsRow>
      <SettingsRow v-if="isDesktop" :title="t('connectionNotifications')" :description="t('notifyMeWhenTheConnectionNeedsAttention')" control-id="settings-notifications">
        <NSwitch id="settings-notifications" :aria-label="t('connectionNotifications')" v-model:value="notifications"  @update:value="preferences" />
      </SettingsRow>
      <SettingsRow :title="t('language')">
        <SingleChoice :value="language" :label="t('language')" :options="languageOptions" @update:value="setLanguage"/>
      </SettingsRow>
      <SettingsRow :title="t('appearance')">
        <SingleChoice v-model:value="theme" :label="t('appearance')" @update:value="preferences" :options="[{value:'system',label:t('system'),icon:Monitor},{value:'light',label:t('light'),icon:Sun},{value:'dark',label:t('dark'),icon:Moon}]"/>
      </SettingsRow>
      <SettingsRow :title="t('themeColor')">
        <SingleChoice appearance="swatches" v-model:value="themeColor" :label="t('themeColor')" :options="Object.entries(themeColors).map(([value, color]) => ({value,label:t(color.label),color:color.light}))"/>
      </SettingsRow>
      <SettingsRow title="Translucent">
        <NSwitch v-model:value="translucent" aria-label="Translucent"/>
      </SettingsRow>
      <SettingsRow v-if="mac" :title="t('showAppIn')">
        <SingleChoice :value="displayPosition" :disabled="!!busy || !appearanceReady" :label="t('showAppIn')" @update:value="setAppearance" :options="[{value:'all',label:t('all')},{value:'menu',label:t('menuBarOnly')},{value:'dock',label:t('dockOnly')}] as const"/>
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup :title="t('network')">
      <SettingsRow :title="t('proxy')" :description="t('useTheSystemProxyOrAProxyJust')">
        <SingleChoice v-model:value="proxyMode" :disabled="!!busy" :label="t('proxyMode')" @update:value="changeProxy" :options="[{value:'system',label:t('systemProxy')},{value:'direct',label:t('noProxy')},{value:'custom',label:t('custom')}] as const"/>
      </SettingsRow>
      <SettingsRow v-if="proxyMode === 'custom'" :title="t('proxyUrl')" :description="t('supportsHttpHttpsProxiesReconnectAfterSavingTo')" control-id="settings-proxy-url">
        <form class="proxy-form" @submit.prevent="run('network', saveProxy)">
          <NInput :input-props="{id:'settings-proxy-url','aria-label':t('proxyUrl')}" v-model:value="proxyUrl" placeholder="http://127.0.0.1:7890" :disabled="!!busy" />
          <NButton attr-type="submit" :disabled="!!busy" type="primary">{{ t('save') }}</NButton>
        </form>
      </SettingsRow>
    </SettingsGroup>

    <AppUpdate />
  </div>
</template>

<style scoped>
.proxy-form { display: flex; gap: 8px; max-width: 100%; }
.proxy-form .n-input { width: 220px; min-width: 0; }
@media (max-width: 600px) { .proxy-form .n-input { width: 190px; } }
</style>
