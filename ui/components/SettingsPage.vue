<script setup lang="ts">
import { Monitor, Sun, Moon } from '@lucide/vue';
import { useIntervalFn } from '@vueuse/core';
import { computed, onMounted, ref } from 'vue';
import { api, useConnector, type Service } from '../composables/useConnector';
import { isDesktop } from '../platform';
import AppUpdate from './AppUpdate.vue';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import '../settings.css';
import ConnectionFields from './ConnectionFields.vue';
import { useConnectionForm } from '../composables/useConnectionForm';
const { status, busy, editable, run, refresh, notify } = useConnector();
const { form, save: saveConnection } = useConnectionForm();
const saveLabel = computed(() => busy.value === 'config' ? '保存中…' : editable.value ? '保存' : status.value?.tunnel.state === 'ready' ? '已连接' : status.value?.tunnel.state === 'starting' ? '连接中…' : '暂不可编辑');
const saveHint = computed(() => !editable.value ? '关闭连接后可保存' : '保存连接信息');
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
const service = ref<Service>();
const theme = ref(localStorage.getItem('theme') || 'system');
const notifications = ref(localStorage.getItem('notifications') !== 'off');
const refreshService = () => api<Service>('service').then(value => service.value = value).catch(() => {});
onMounted(refreshService);
useIntervalFn(refreshService, 5000);
async function save() {
  if (!status.value) return;
  await saveConnection();
  await refresh(); notify('连接信息已保存。');
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
        <button form="connection-settings" type="submit" :disabled="!!busy || !editable || !status" :title="saveHint" class="primary">{{saveLabel}}</button>
      </SettingsRow>
      <form id="connection-settings" class="settings-connection-form" @submit.prevent="run('config', save)">
        <ConnectionFields :form="form" :disabled="!!busy || !editable" />
      </form>
    </SettingsGroup>

    <SettingsGroup title="网络">
      <SettingsRow title="代理" description="使用系统代理，或仅为 Local Connector 指定代理。" control-id="settings-proxy">
        <select id="settings-proxy" v-model="proxyMode" :disabled="!!busy" @change="changeProxy">
          <option value="system">跟随系统</option><option value="direct">不使用代理</option><option value="custom">自定义</option>
        </select>
      </SettingsRow>
      <SettingsRow v-if="proxyMode === 'custom'" title="代理地址" description="支持 HTTP/HTTPS 代理。保存后重新连接生效。" control-id="settings-proxy-url">
        <form class="proxy-form" @submit.prevent="run('network', saveProxy)">
          <input id="settings-proxy-url" v-model="proxyUrl" type="url" required placeholder="http://127.0.0.1:7890" :disabled="!!busy" />
          <button type="submit" :disabled="!!busy" class="primary">保存</button>
        </form>
      </SettingsRow>
    </SettingsGroup>

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
    </SettingsGroup>

    <AppUpdate />
    <footer class="settings-version">Local Connector <span v-if="status">{{status.version}}</span></footer>
  </div>
</template>

<style scoped>
.proxy-form { display: flex; gap: 8px; max-width: 100%; }
.proxy-form input { width: 260px; min-width: 0; }
@media (max-width: 600px) { .proxy-form input { width: 190px; } }
</style>
