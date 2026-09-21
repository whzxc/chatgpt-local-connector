<script setup lang="ts">
import { SquareArrowOutUpRight } from '@lucide/vue';
import { computed } from 'vue';
import { api, useConnector } from '../composables/useConnector';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const emit = defineEmits<{ done: []; settings: [] }>();
const { status, busy, run, refresh, connectionError } = useConnector();
const https = computed(() => status.value?.config.connectionMode === 'https');
const endpoint = computed(() => https.value ? status.value?.connection?.mcpUrl || '' : status.value?.config.tunnelId || '');
const description = computed(() => `通过 ${https.value ? 'HTTPS MCP' : 'Tunnel'} 连接到本地设备`);
const online = computed(() => !connectionError.value && status.value?.tunnel.state === 'ready');
const verified = computed(() => status.value?.core.chatgpt?.challengeVerifiedAt);
const code = computed(() => status.value?.core.chatgpt?.code || '');
const prompt = computed(() => `请使用 Local Connector 插件调用 connector_verify，code 为 ${code.value}。只验证连接，不创建任务。`);
const open = (url: string) => run('chatgpt-open', () => openUrl(url));
const reset = () => run('chatgpt-verify', async () => { await api('verification/reset', 'POST'); await refresh(); });
</script>
<template>
  <div class="chat-guide">
    <h1>接入 ChatGPT</h1>
    <div v-if="!online" class="guide-notice" role="status">
      <span>{{connectionError || status?.tunnel.error || '请先开启本机连接'}}</span>
      <button v-if="!status?.config.configured" class="text-button" @click="emit('settings')">配置连接 →</button>
      <button v-else class="text-button" :disabled="!!busy || ['starting','installing','connecting'].includes(status?.tunnel.state || '')" @click="run('connect', async () => { await api('start', 'POST'); await refresh(); })">开启连接</button>
    </div>
    <ol class="guide-steps">
      <li><span class="guide-step-number">1</span><div><div class="guide-step-heading"><strong>开启开发者模式</strong><button class="text-button" :disabled="!!busy" @click="open('https://chatgpt.com/settings/security')">打开设置 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div><p>设置 → 安全与登录 → Developer Mode</p></div></li>
      <li><span class="guide-step-number">2</span><div>
        <div class="guide-step-heading"><strong>Create MCP App</strong><button class="text-button" :disabled="!!busy" @click="open('https://chatgpt.com/plugins')">打开 Plugins <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div>
        <p>Plugins → Add / ＋ → Create MCP App</p>
        <dl class="guide-fields">
          <div><dt>Name</dt><dd><CopyField value="Local Connector" label="推荐名称" /></dd></div>
          <div><dt>Description</dt><dd><CopyField :value="description" label="推荐描述" /></dd></div>
          <div><dt>Connection</dt><dd>{{https ? 'Server URL' : 'Tunnel'}}</dd></div>
          <div v-if="https"><dt>Server URL</dt><dd><CopyField :value="endpoint" label="ChatGPT 接入地址" /></dd></div>
          <div v-else><dt>Tunnel</dt><dd><span>选择当前通道</span><small v-if="endpoint">{{endpoint}}</small></dd></div>
          <div><dt>Authentication</dt><dd>No Authentication</dd></div>
        </dl>
      </div></li>
      <li><span class="guide-step-number">3</span><div><strong>发送验证消息</strong><p>新对话中选用连接，发送以下消息</p><CopyField v-if="code" :value="prompt" label="连接验证消息" multiline />
        <p v-if="online && verified" class="guide-result" role="status">✓ 已收到验证请求 <button class="text-button" :disabled="!!busy" @click="reset">重新验证</button></p>
      </div></li>
    </ol>
  </div>
</template>
<style scoped>
h1{font-size:26px;line-height:1.35;letter-spacing:-.6px;margin-bottom:26px}
.guide-steps{padding:0;margin:0;list-style:none;display:grid;gap:24px}
.guide-steps>li{display:flex;gap:14px;align-items:flex-start}
.guide-step-number{display:grid;place-items:center;flex:0 0 26px;height:26px;border-radius:8px;background:color-mix(in srgb,var(--green) 8%,transparent);color:var(--green);font-size:12px}
.guide-steps li>div{flex:1;min-width:0}
.guide-steps strong{font-size:14px;font-weight:550;line-height:26px}
.guide-steps p{font-size:12px;color:var(--muted);line-height:1.7;margin:3px 0 9px}
.guide-step-heading{display:flex;align-items:center;justify-content:space-between;gap:12px}
.guide-step-heading button{font-size:12px}
.guide-fields{display:grid;gap:12px;margin:12px 0 0;font-size:12px}
.guide-fields>div{display:grid;grid-template-columns:100px minmax(0,1fr);gap:12px;align-items:center}
.guide-fields dt{color:var(--muted)}.guide-fields dd{margin:0;min-width:0}
.guide-fields small{display:block;font-size:10px;color:var(--muted);overflow-wrap:anywhere;margin-top:4px}
.guide-notice{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:22px;font-size:12px;color:var(--muted)}
.guide-result{display:flex;align-items:center;gap:12px}.guide-result button{font-size:12px}
@media(max-width:600px){h1{font-size:24px}.guide-fields>div{grid-template-columns:90px minmax(0,1fr);gap:8px}}
</style>
