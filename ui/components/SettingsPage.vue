<script setup lang="ts">
import { Monitor, Sun, Moon, Eye, EyeOff, SquareArrowOutUpRight } from '@lucide/vue';
import { useIntervalFn } from '@vueuse/core';
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue';
import { api, useConnector, type Service } from '../composables/useConnector';
import { isDesktop, isDevelopment, openUrl } from '../platform';
import AppUpdate from './AppUpdate.vue';
const { status, busy, editable, run, refresh, notify } = useConnector();
const form = reactive({ tunnelId: '', apiKey: '' });
watch(() => status.value?.config, config => { if (config && !form.tunnelId) form.tunnelId = config.tunnelId; }, { immediate: true });
const showKey = ref(false);
const readingKey = ref(false);
const saveLabel = computed(() => busy.value === 'config' ? '保存中…' : isDevelopment ? '只读预览' : !editable.value ? '连接中' : '保存');
const saveHint = computed(() => isDevelopment ? '开发预览不修改连接信息' : !editable.value ? '关闭连接后可保存' : '保存连接信息');
async function toggleKey() {
  if (showKey.value) { showKey.value = false; return; }
  readingKey.value = true;
  try {
    if (!form.apiKey && status.value?.config.hasApiKey) {
      // Read only on explicit reveal, never via periodic status or localStorage.
      const value = isDesktop && !isDevelopment
        ? await api<{ apiKey: string }>('config/credentials')
        : await fetch('/api/config/credentials', { headers: { 'X-CLC-Request': '1' }, cache: 'no-store' }).then(async response => {
          if (!response.ok) throw new Error('无法读取已保存的密钥，请重试。');
          return response.json() as Promise<{ apiKey: string }>;
        });
      form.apiKey = value.apiKey;
    }
    showKey.value = true;
  } catch { notify('无法读取已保存的密钥，请重试。', true); }
  finally { readingKey.value = false; }
}
onUnmounted(() => { form.apiKey = ''; showKey.value = false; });
const service = ref<Service>();
const theme = ref(localStorage.getItem('theme') || 'system');
const notifications = ref(localStorage.getItem('notifications') !== 'off');
const refreshService = () => api<Service>('service').then(value => service.value = value).catch(() => {});
onMounted(refreshService);
useIntervalFn(refreshService, 5000);
async function save() {
  if (!status.value) return;
  const { tunnelBinary, codexBinary, autoStart } = status.value.config;
  await api('config', 'PUT', { tunnelBinary, codexBinary, autoStart, tunnelId: form.tunnelId.trim(), apiKey: form.apiKey.trim() });
  form.apiKey = ''; showKey.value = false; await refresh(); notify('连接信息已保存。');
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
async function setApproval(enabled: boolean) {
  await run('task-settings', async () => { await api('task-settings', 'PUT', { enabled }); await refresh(); });
}
</script>
<template>
  <div class="settings-page">
    <section class="settings-block">
      <div class="card-head"><div><h2>连接信息</h2><p>通道身份与密钥只保存在这台电脑上。</p></div><button form="connection-settings" type="submit" :disabled="!!busy || !editable || !status || isDevelopment" :title="saveHint" class="primary small-button">{{saveLabel}}</button></div>
      <form id="connection-settings" @submit.prevent="run('config', save)">
        <div class="settings-fields">
          <div>
            <div class="credential-label"><label for="tunnel-id">Tunnel ID</label><a href="https://platform.openai.com/settings/organization/tunnels" target="_blank" rel="noopener noreferrer" @click.prevent="openUrl('https://platform.openai.com/settings/organization/tunnels')">获取 Tunnel ID <SquareArrowOutUpRight aria-hidden="true" /></a></div>
            <input id="tunnel-id" v-model="form.tunnelId" :readonly="!!busy || !editable || isDevelopment" autocomplete="off" required />
          </div>
          <div>
            <div class="credential-label"><label for="api-key">Runtime API Key</label><a href="https://platform.openai.com/settings/organization/api-keys" target="_blank" rel="noopener noreferrer" @click.prevent="openUrl('https://platform.openai.com/settings/organization/api-keys')">获取 API Key <SquareArrowOutUpRight aria-hidden="true" /></a></div>
            <div class="credential-input"><input id="api-key" v-model="form.apiKey" :readonly="!!busy || !editable || isDevelopment" :type="showKey ? 'text' : 'password'" autocomplete="off" spellcheck="false" :placeholder="status?.config.hasApiKey ? '••••••••••••••••' : '填写运行密钥'" /><button type="button" :disabled="readingKey" :aria-label="showKey ? '隐藏 API Key' : '显示 API Key'" :aria-pressed="showKey" @click="toggleKey"><EyeOff v-if="showKey" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button></div>
          </div>
        </div>
      </form>
    </section>
    <section class="settings-block">
      <h2>偏好</h2>
      <label class="preference-row"><span><strong>任务审批模式</strong><small>{{status?.taskApprovalEnabled?'默认先确认；云端仍可批准或绕过。':'收到任务请求后直接提交。'}} 用户意图始终优先。</small></span><input type="checkbox" role="switch" aria-label="任务审批模式" :checked="!!status?.taskApprovalEnabled" :disabled="!!busy || !status || isDevelopment" @change="setApproval(($event.target as HTMLInputElement).checked)"/></label>
      <label class="preference-row"><span><strong>登录系统时开启连接</strong><small>登录后自动开启连接；关闭窗口不影响连接。</small></span><input type="checkbox" role="switch" :checked="service?.enabled" :disabled="!!busy || !service?.supported" @change="startup" /></label>
      <div class="preference-row"><span><strong>外观</strong><small>选择你习惯的界面颜色。</small></span><div class="theme-options" role="group" aria-label="外观"><button v-for="option in [{value:'system',label:'跟随系统',icon:Monitor},{value:'light',label:'浅色',icon:Sun},{value:'dark',label:'深色',icon:Moon}]" :key="option.value" :aria-label="option.label" :title="option.label" :aria-pressed="theme===option.value" :class="{active:theme===option.value}" @click="theme=option.value;preferences()"><component :is="option.icon" aria-hidden="true"/></button></div></div>
      <label v-if="isDesktop" class="preference-row"><span><strong>连接异常通知</strong><small>连接需要处理时提醒我。</small></span><input v-model="notifications" type="checkbox" role="switch" @change="preferences" /></label>
    </section>
    <AppUpdate />
    <footer class="settings-about"><span>Local Connector <span v-if="status">{{status.version}}</span></span></footer>
  </div>
</template>
