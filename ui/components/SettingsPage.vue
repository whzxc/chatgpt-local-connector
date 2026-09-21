<script setup lang="ts">
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
const accessProvider = computed(() => ({ cloudflare: 'Cloudflare', ngrok: 'ngrok', custom: '自定义域名' })[connectionConfig.value.httpsProvider || 'custom']);
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
  notify(status.value?.connection?.running ? '代理已保存，重新连接后生效；新的下载使用新设置。' : '代理设置已保存。');
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
    notify('连接信息已保存，已按新参数重新连接。');
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(saved ? `连接信息已保存，但重新连接失败：${reason}` : `未能保存连接信息：${reason}`);
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
    <SettingsGroup title="连接">
      <SettingsRow title="连接信息" description="通道身份与密钥只保存在这台电脑上。">
        <div v-if="editingConnection" class="connection-edit-actions">
          <button type="button" :disabled="!!busy" @click="cancelConnection">取消</button>
          <button form="connection-settings" type="submit" :disabled="!!busy || !status" class="primary"><LoaderCircle v-if="busy==='config'" class="save-spinner" aria-hidden="true"/>{{busy==='config' ? '重连中…' : '保存并重连'}}</button>
        </div>
        <button v-else type="button" :disabled="!!busy || !status" aria-label="编辑连接信息" @click="editConnection"><Pencil aria-hidden="true"/>编辑</button>
      </SettingsRow>
      <form v-if="editingConnection" id="connection-settings" class="settings-connection-form" @submit.prevent="run('config', save)">
        <ConnectionFields :form="form" :disabled="!!busy" />
      </form>
      <dl v-else class="connection-summary">
        <div><dt>连接方式</dt><dd>{{connectionConfig.connectionMode === 'https' ? 'HTTPS MCP' : 'OpenAI Tunnel'}}</dd></div>
        <template v-if="connectionConfig.connectionMode === 'https'">
          <div><dt>接入方式</dt><dd>{{accessProvider}}</dd></div>
          <div class="connection-summary-address"><dt>接入地址</dt><dd><CopyField v-if="accessUrl" :value="accessUrl" label="ChatGPT 接入地址"/><span v-else class="hint">连接后生成</span></dd></div>
        </template>
      </dl>
    </SettingsGroup>

    <AgentsSettings />

    <SettingsGroup title="任务">
      <SettingsRow title="自动打开 Codex 任务" description="关闭后，新任务在后台执行。" control-id="settings-auto-open">
        <input id="settings-auto-open" class="settings-switch" type="checkbox" role="switch" :checked="status?.autoOpenCodex !== false" :disabled="!!busy || !status" @change="setAutoOpen" />
      </SettingsRow>
      <SettingsRow title="任务审批模式" :description="status?.taskApprovalEnabled ? '默认先确认；云端仍可批准或绕过。' : '收到任务请求后直接提交。'" control-id="settings-approval">
        <input id="settings-approval" class="settings-switch" type="checkbox" role="switch" :checked="!!status?.taskApprovalEnabled" :disabled="!!busy || !status" @change="setApproval(($event.target as HTMLInputElement).checked)" />
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup title="通用">
      <SettingsRow title="登录系统时开启连接" description="关闭窗口后，连接仍会继续运行。" control-id="settings-startup">
        <input id="settings-startup" class="settings-switch" type="checkbox" role="switch" :checked="service?.enabled" :disabled="!!busy || !service?.supported" @change="startup" />
      </SettingsRow>
      <SettingsRow v-if="isDesktop" title="连接异常通知" description="连接需要处理时提醒我。" control-id="settings-notifications">
        <input id="settings-notifications" v-model="notifications" class="settings-switch" type="checkbox" role="switch" @change="preferences" />
      </SettingsRow>
      <SettingsRow title="外观">
        <div class="settings-theme" role="group" aria-label="外观">
          <button v-for="option in [{value:'system',label:'跟随系统',icon:Monitor},{value:'light',label:'浅色',icon:Sun},{value:'dark',label:'深色',icon:Moon}]" :key="option.value" :aria-pressed="theme===option.value" :class="{active:theme===option.value}" @click="theme=option.value;preferences()"><component :is="option.icon" aria-hidden="true"/>{{option.label}}</button>
        </div>
      </SettingsRow>
      <SettingsRow v-if="mac" title="显示位置">
        <div class="mode-switch" role="group" aria-label="显示位置" :title="!isDesktop ? '请在桌面应用中设置' : undefined">
          <button v-for="option in [{value:'all',label:'全部'},{value:'menu',label:'仅菜单栏'},{value:'dock',label:'仅 Dock 栏'}] as const" :key="option.value" type="button" :aria-pressed="displayPosition===option.value" :class="{active:displayPosition===option.value}" :disabled="!!busy || !appearanceReady" @click="setAppearance(option.value)">{{option.label}}</button>
        </div>
      </SettingsRow>
    </SettingsGroup>

    <SettingsGroup title="网络">
      <SettingsRow title="代理" description="使用系统代理，或仅为 Local Connector 指定代理。">
        <div class="mode-switch" role="group" aria-label="代理方式">
          <button v-for="option in [{value:'system',label:'系统代理'},{value:'direct',label:'不使用代理'},{value:'custom',label:'自定义'}] as const" :key="option.value" type="button" :aria-pressed="proxyMode===option.value" :class="{active:proxyMode===option.value}" :disabled="!!busy" @click="proxyMode=option.value;changeProxy()">{{option.label}}</button>
        </div>
      </SettingsRow>
      <SettingsRow v-if="proxyMode === 'custom'" title="代理地址" description="支持 HTTP/HTTPS 代理。保存后重新连接生效。" control-id="settings-proxy-url">
        <form class="proxy-form" @submit.prevent="run('network', saveProxy)">
          <input id="settings-proxy-url" v-model="proxyUrl" type="url" required placeholder="http://127.0.0.1:7890" :disabled="!!busy" />
          <button type="submit" :disabled="!!busy" class="primary">保存</button>
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
