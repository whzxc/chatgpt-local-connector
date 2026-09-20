<script setup lang="ts">
import { SquareArrowOutUpRight } from '@lucide/vue';
import { computed, ref, watch } from 'vue';
import { useClipboard } from '@vueuse/core';
import { api, useConnector } from '../composables/useConnector';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const emit = defineEmits<{ done: []; settings: [] }>();
const { status, busy, run, refresh, connectionError, notify } = useConnector();
const { copy } = useClipboard({ legacy: true });
const verifying = ref(false);
const https = computed(() => status.value?.config.connectionMode === 'https');
const endpoint = computed(() => https.value ? status.value?.connection?.mcpUrl || '' : status.value?.config.tunnelId || '');
const online = computed(() => !connectionError.value && status.value?.tunnel.state === 'ready');
const verified = computed(() => status.value?.core.chatgpt?.challengeVerifiedAt);
const code = computed(() => status.value?.core.chatgpt?.code || '');
const prompt = computed(() => `请使用 Local Connector 插件调用 connector_verify，code 为 ${code.value}。只验证连接，不创建任务。`);
const step = computed(() => !online.value ? 1 : verified.value ? 4 : verifying.value ? 3 : 2);
watch(endpoint, () => { verifying.value = false; });
const open = (url: string) => run('chatgpt-open', () => openUrl(url));
async function copyAndOpen(value: string, url: string) {
  // A clipboard failure must not prevent navigation; the visible field remains available.
  try { await copy(value); notify('已复制，前往 ChatGPT 粘贴即可。'); }
  catch { notify('自动复制失败，请使用页面中的复制框。', true); }
  await openUrl(url);
}
function verify(fresh = false) {
  return run('chatgpt-verify', async () => {
    if (fresh) await api('verification/reset', 'POST');
    await refresh();
    if (!online.value || !code.value) throw new Error('请先恢复本机连接。');
    verifying.value = true;
    await copyAndOpen(prompt.value, 'https://chatgpt.com/');
  });
}
</script>
<template>
  <div class="chat-guide">
    <div class="step-dots" aria-label="ChatGPT 接入进度"><i v-for="n in 3" :key="n" :class="{active:n===step,complete:n<step}"></i><span>{{step===4 ? '接入已验证' : `${step} / 3 · 接入 ChatGPT`}}</span></div>
    <p v-if="connectionError" class="error-detail" role="alert">无法读取本机状态：{{connectionError}}</p>
    <template v-if="!online">
      <h1>1. 准备本机连接。</h1>
      <p class="page-subtitle">应用会自动准备连接组件。开启后，这里会继续下一步。</p>
      <p v-if="status?.tunnel.error" class="error-detail" role="alert">{{status.tunnel.error}}</p>
      <button v-if="!status?.config.configured" class="primary" @click="emit('settings')">前往配置连接 →</button>
      <button v-else class="primary" :disabled="!!busy || ['starting','installing','connecting'].includes(status?.tunnel.state || '')" @click="run('connect', async () => { await api('start', 'POST'); await refresh(); })">开启连接</button>
      <p v-if="status?.core.chatgpt?.verifiedAt" class="hint">有历史连通记录；当前连接恢复前不能确认可用。</p>
    </template>
    <template v-else-if="verified">
      <div class="guide-symbol" aria-hidden="true">✓</div>
      <h1>连接验证已完成。</h1>
      <p class="page-subtitle">已收到匹配验证码的工具调用。若由 ChatGPT 发起，可在对话中确认工具结果。</p>
      <p class="hint">验证时间 · {{new Date(verified).toLocaleString()}}。此记录不代表网页安装状态或 Codex 任务已完成；完整任务验收请继续让 Codex 按配置指南执行。</p>
      <div class="actions"><button class="primary" @click="emit('done')">回到连接主页 →</button><button :disabled="!!busy" @click="verify(true)">重新验证</button></div>
    </template>
    <template v-else-if="!verifying">
      <h1>2. 在 ChatGPT 添加这台电脑。</h1>
      <p class="page-subtitle">已添加过？直接验证即可，无需重复创建。</p>
      <p class="guide-instruction">首次使用：设置 → 安全与登录 → Developer Mode，然后打开 Plugins → Add（或 ＋）→ Create MCP App。名称填写 Local Connector，描述填写「连接这台电脑的项目与 Codex 任务」。</p>
      <p class="guide-instruction">{{https ? '连接填写下方地址，认证选择 No authentication。' : '连接选择 Tunnel，选取对应通道或粘贴下方 ID。'}}</p>
      <CopyField :value="endpoint" :label="https ? 'ChatGPT 接入地址' : 'ChatGPT 接入 ID'" />
      <p v-if="https && (status?.config.httpsProvider==='custom' || status?.config.cloudflareMode==='named')" class="hint">请先完成自己的公开路由配置。本机就绪不代表公网地址可达。</p>
      <div class="actions"><button class="primary" :disabled="!!busy || !endpoint" @click="run('chatgpt-open', () => copyAndOpen(endpoint, 'https://chatgpt.com/plugins'))">复制{{https ? '地址' : 'ID'}}并打开 Plugins <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button><button :disabled="!!busy || !code" @click="verify()">已有连接，开始验证 →</button></div>
      <p class="hint">打开页面不会自动安装。提供已登录网页给本机 Codex 后，它可尝试代填和点击；登录、本人授权（如 I understand and want to continue）及浏览器安全确认仍由你完成。</p>
      <details><summary>没有入口或发现工具失败？</summary><p class="hint">检查当前账号和工作区是否允许 Developer Mode；Tunnel 需关联当前工作区并授予 Read + Use 权限。已有连接打开详情选择 Refresh，然后新建对话。</p><button class="text-button" @click="open('https://developers.openai.com/plugins/deploy/connect-chatgpt')">查看官方指南</button></details>
    </template>
    <template v-else>
      <h1>3. 发送验证消息。</h1>
      <p class="page-subtitle">在 ChatGPT 新对话的工具菜单选中 Local Connector，粘贴并发送。CLC 会自动接收结果，无需点击“已完成”。</p>
      <CopyField :value="prompt" label="连接验证消息" />
      <div class="actions"><button class="primary" :disabled="!!busy || !code" @click="verify()">复制消息并打开 ChatGPT <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div>
      <p class="verification-wait" role="status"><span class="status-dot"></span> 等待匹配验证码的工具调用…</p>
      <details><summary>没有收到验证？</summary><p class="hint">确认已选用连接，并实际调用 connector_verify。地址变更后需在 ChatGPT 更新连接；工具有变化时到 Plugins 详情选择 Refresh。本机状态无法判断 ChatGPT 是否已添加插件。</p><button class="text-button" @click="open('https://chatgpt.com/plugins')">打开 Plugins 检查连接</button></details>
      <div class="wizard-actions"><button class="text-button" @click="verifying=false">← 查看添加步骤</button></div>
    </template>
  </div>
</template>
