<script setup lang="ts">
import { t, language, languageOptions, setLanguage } from '../i18n';
import { Monitor, Sun, Moon, Pencil, LoaderCircle } from '@lucide/vue';
import { useIntervalFn } from '@vueuse/core';
import { computed, onMounted, ref } from 'vue';
import { api, useConnector, type Service } from '../composables/useConnector';
import { isDesktop } from '../platform';
import AppUpdate from './AppUpdate.vue';
import AgentsSettings from './AgentsSettings.vue';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import '../settings.css';
import ConnectionFields from './ConnectionFields.vue';
import CopyField from './CopyField.vue';
import { useConnectionForm } from '../composables/useConnectionForm';
const { status, busy, run, refresh, notify } = useConnector();
const { form, save: saveConnection, reset: resetConnection } = useConnectionForm();
const editingConnection = ref(false);
const connectionConfig = computed(() => status.value!.config);
const accessProvider = computed(() => ({ cloudflare: 'Cloudflare', ngrok: 'ngrok', custom: t('customDomain') })[connectionConfig.value.httpsProvider || 'custom']);
const accessUrl = computed(() => status.value?.connection?.mcpUrl || (connectionConfig.value.httpsProvider === 'custom' || (connectionConfig.value.httpsProvider === 'cloudflare' && connectionConfig.value.cloudflareMode === 'named') ? connectionConfig.value.httpsUrl : '') || '');
function editConnection() {
  resetConnection();
  editingConnection.value = true;
}
function cancelConnection() {
  editingConnection.value = false;
  resetConnection();
}
const proxyMode = ref(status.value?.config.proxyMode || 'system');
const proxyUrl = ref(status.value?.config.proxyUrl || '');
async function saveProxy() {
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
const theme = ref(localStorage.getItem('theme') || 'system');
const notifications = ref(localStorage.getItem('notifications') !== 'off');
const refreshService = () => api<Service>('service').then(value => service.value = value).catch(() => {});
onMounted(refreshService);
useIntervalFn(refreshService, 5000);
async function save() {
  if (!status.value) return;
  let saved = false;
  try {
    await api('stop', 'POST');
    await saveConnection();
    saved = true;
    await api('start', 'POST');
    await refresh();
    editingConnection.value = false;
    resetConnection();
    notify(t('connectionDetailsSavedAndReconnectedWithTheNew'));
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(saved ? t('connectionDetailsSavedButReconnectingFailedValue', { error: reason }) : t('unableToSaveConnectionDetailsValue', { error: reason }));
  } finally {
    await refresh().catch(() => {});
  }
}
async function startup(event: Event) {
  const input = event.target as HTMLInputElement;
  const enabled = input.checked;
  input.checked = !!service.value?.enabled;
  await run('startup', async () => { service.value = await api<Service>('service', 'POST', { enabled }); });
  input.checked = !!service.value?.enabled;
}
function preferences() {
  localStorage.setItem('theme', theme.value); document.documentElement.dataset.theme = theme.value;
  localStorage.setItem('notifications', notifications.value ? 'on' : 'off');
}
async function setAutoOpen(event: Event) {
  const input = event.target as HTMLInputElement;
  const autoOpenCodex = input.checked;
  input.checked = status.value?.autoOpenCodex !== false;
  await run('task-settings', async () => { await api('task-settings', 'PUT', { autoOpenCodex }); await refresh(); });
  input.checked = status.value?.autoOpenCodex !== false;
}
async function setApproval(enabled: boolean) {
  await run('task-settings', async () => { await api('task-settings', 'PUT', { enabled }); await refresh(); });
}
</script>
<template>
  <div class="settings-preferences">
    <SettingsGroup :title="t('connection')">
      <SettingsRow :title="t('connectionDetails')" :description="t('tunnelIdentityAndKeysAreStoredOnlyOn')">
        <div v-if="editingConnection" class="connection-edit-actions">
          <button type="button" :disabled="!!busy" @click="cancelConnection">{{ t('cancel') }}</button>
          <button form="connection-settings" type="submit" :disabled="!!busy || !status" class="primary"><LoaderCircle v-if="busy==='config'" class="save-spinner" aria-hidden="true"/>{{busy==='config' ? t('reconnecting') : t('saveAndReconnect')}}</button>
        </div>
        <button v-else type="button" :disabled="!!busy || !status" :aria-label="t('editConnectionDetails')" @click="editConnection"><Pencil aria-hidden="true"/>{{ t('edit') }}</button>
      </SettingsRow>
      <form v-if="editingConnection" id="connection-settings" class="settings-connection-form" @submit.prevent="run('config', save)">
        <ConnectionFields :form="form" :disabled="!!busy" />
      </form>
      <dl v-else class="connection-summary">
        <div><dt>{{ t('connectionMethod') }}</dt><dd>{{connectionConfig.connectionMode === 'https' ? 'HTTPS MCP' : 'OpenAI Tunnel'}}</dd></div>
        <template v-if="connectionConfig.connectionMode === 'https'">
          <div><dt>{{ t('provider') }}</dt><dd>{{accessProvider}}</dd></div>
          <div class="connection-summary-address"><dt>{{ t('connectionUrl') }}</dt><dd><CopyField v-if="accessUrl" :value="accessUrl" :label="t('chatgptConnectionUrl')"/><span v-else class="hint">{{ t('generatedWhenConnected') }}</span></dd></div>
        </template>
      </dl>
    </SettingsGroup>

    <AgentsSettings />

    <SettingsGroup :title="t('tasks')">
      <SettingsRow :title="t('automaticallyOpenCodexTasks')" :description="t('whenDisabledNewTasksRunInTheBackground')" control-id="settings-auto-open">
        <input id="settings-auto-open" class="settings-switch" type="checkbox" role="switch" :checked="status?.autoOpenCodex !== false" :disabled="!!busy || !status" @change="setAutoOpen" />
      </SettingsRow>
      <SettingsRow :title="t('taskApprovalMode')" :description="status?.taskApprovalEnabled ? t('askForConfirmationByDefaultTheCloudCan') : t('submitTaskRequestsImmediately')" control-id="settings-approval">
        <input id="settings-approval" class="settings-switch" type="checkbox" role="switch" :checked="!!status?.taskApprovalEnabled" :disabled="!!busy || !status" @change="setApproval(($event.target as HTMLInputElement).checked)" />
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup :title="t('general')">
      <SettingsRow :title="t('connectAtSystemSignIn')" :description="t('theConnectionKeepsRunningAfterTheWindowCloses')" control-id="settings-startup">
        <input id="settings-startup" class="settings-switch" type="checkbox" role="switch" :checked="service?.enabled" :disabled="!!busy || !service?.supported" @change="startup" />
      </SettingsRow>
      <SettingsRow v-if="isDesktop" :title="t('connectionNotifications')" :description="t('notifyMeWhenTheConnectionNeedsAttention')" control-id="settings-notifications">
        <input id="settings-notifications" v-model="notifications" class="settings-switch" type="checkbox" role="switch" @change="preferences" />
      </SettingsRow>
      <SettingsRow :title="t('language')">
        <div id="settings-language" class="mode-switch" role="group" :aria-label="t('language')">
          <button v-for="option in languageOptions" :key="option.value" type="button" :aria-pressed="language===option.value" :class="{active:language===option.value}" @click="setLanguage(option.value)">{{option.label}}</button>
        </div>
      </SettingsRow>
      <SettingsRow :title="t('appearance')">
        <div class="settings-theme" role="group" :aria-label="t('appearance')">
          <button v-for="option in [{value:'system',label:t('system'),icon:Monitor},{value:'light',label:t('light'),icon:Sun},{value:'dark',label:t('dark'),icon:Moon}]" :key="option.value" :aria-pressed="theme===option.value" :class="{active:theme===option.value}" @click="theme=option.value;preferences()"><component :is="option.icon" aria-hidden="true"/>{{option.label}}</button>
        </div>
      </SettingsRow>
      <SettingsRow v-if="mac" :title="t('showAppIn')">
        <div class="mode-switch" role="group" :aria-label="t('showAppIn')" :title="!isDesktop ? t('configureThisInTheDesktopApp') : undefined">
          <button v-for="option in [{value:'all',label:t('all')},{value:'menu',label:t('menuBarOnly')},{value:'dock',label:t('dockOnly')}] as const" :key="option.value" type="button" :aria-pressed="displayPosition===option.value" :class="{active:displayPosition===option.value}" :disabled="!!busy || !appearanceReady" @click="setAppearance(option.value)">{{option.label}}</button>
        </div>
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup :title="t('network')">
      <SettingsRow :title="t('proxy')" :description="t('useTheSystemProxyOrAProxyJust')">
        <div class="mode-switch" role="group" :aria-label="t('proxyMode')">
          <button v-for="option in [{value:'system',label:t('systemProxy')},{value:'direct',label:t('noProxy')},{value:'custom',label:t('custom')}] as const" :key="option.value" type="button" :aria-pressed="proxyMode===option.value" :class="{active:proxyMode===option.value}" :disabled="!!busy" @click="proxyMode=option.value;changeProxy()">{{option.label}}</button>
        </div>
      </SettingsRow>
      <SettingsRow v-if="proxyMode === 'custom'" :title="t('proxyUrl')" :description="t('supportsHttpHttpsProxiesReconnectAfterSavingTo')" control-id="settings-proxy-url">
        <form class="proxy-form" @submit.prevent="run('network', saveProxy)">
          <input id="settings-proxy-url" v-model="proxyUrl" type="url" required placeholder="http://127.0.0.1:7890" :disabled="!!busy" />
          <button type="submit" :disabled="!!busy" class="primary">{{ t('save') }}</button>
        </form>
      </SettingsRow>
    </SettingsGroup>

    <AppUpdate />
    <footer class="settings-version">Local Connector <span v-if="status">{{status.version}}</span></footer>
  </div>
</template>

<style scoped>
.proxy-form { display: flex; gap: 8px; max-width: 100%; }
.proxy-form input { width: 220px; min-width: 0; }
@media (max-width: 600px) { .proxy-form input { width: 190px; } }
</style>
