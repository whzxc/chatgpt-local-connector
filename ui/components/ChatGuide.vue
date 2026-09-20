<script setup lang="ts">
import { SquareArrowOutUpRight } from '@lucide/vue';
import { computed, ref } from 'vue';
import { useConnector } from '../composables/useConnector';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const emit = defineEmits<{ done: [] }>();
const { status, busy, run } = useConnector();
const step = ref(-1);
const online = computed(() => status.value?.tunnel.state === 'ready' && (status.value?.autoOpenCodex === false ? status.value?.core.appServer?.state === 'ready' : status.value?.core.desktop?.state === 'ready'));
const verified = computed(() => status.value?.core.chatgpt?.verifiedAt);
const prompt = computed(() => `请使用 Local Connector 插件调用 connector_verify，code 为 ${status.value?.core.chatgpt?.code || ''}。只验证连接，不创建任务。`);
const open = (url: string) => run('chatgpt-open', () => openUrl(url));
</script>
<template>
  <div class="chat-guide">
    <template v-if="verified">
      <div class="guide-symbol" aria-hidden="true">✓</div>
      <h1>连接验证已完成。</h1>
      <p class="page-subtitle">已收到远程工具调用。这是连通记录，不代表当前网页端安装状态。</p>
      <button class="primary" @click="emit('done')">回到连接主页 →</button>
      <p class="hint">验证时间 · {{new Date(verified).toLocaleString()}}</p>
    </template>
    <template v-else-if="step===-1">
      <div class="eyebrow">CHATGPT</div><h1>插件状态尚未验证。</h1>
      <p class="page-subtitle">本机还没有连通记录，不代表插件未安装。已添加的插件可以直接使用，无需重复接入。</p>
      <button class="primary" @click="emit('done')">已添加，返回概览 →</button>
      <div class="actions"><button class="text-button" @click="step=1">验证连接（可选）</button><button class="text-button" @click="step=0">尚未添加？查看添加步骤</button></div>
      <p class="hint">正常使用插件，成功的工具调用也会自动确认连通。</p>
    </template>
    <template v-else-if="!online">
      <div class="eyebrow">CONNECT CHATGPT</div><h1>先开启本机连接。</h1>
      <p class="page-subtitle">通道开启后即可使用已有插件，或进行可选的连接验证。</p>
      <button class="primary" @click="emit('done')">返回开启连接 →</button>
    </template>
    <template v-else>
      <div class="step-dots" aria-label="ChatGPT 接入进度"><i v-for="n in 2" :key="n" :class="{active:n===step+1,complete:n<step+1}"></i><span>{{step+1}} / 2 · 接入 ChatGPT</span></div>
      <template v-if="step===0">
        <h1>让 ChatGPT 找到这台电脑。</h1>
        <p class="page-subtitle">在网页端添加一次 Local Connector。</p>
        <p class="guide-instruction">Plugins → ＋ → 连接方式选择 Tunnel</p>
        <CopyField :value="status?.config.tunnelId || ''" label="添加插件所需的 Tunnel ID" />
        <div class="actions"><button class="primary" :disabled="!!busy" @click="open(status?.chatgptUrl || 'https://chatgpt.com/plugins')">打开 ChatGPT 插件页 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div>
        <details><summary>没有添加入口或找不到 Tunnel？</summary><p class="hint">在 ChatGPT「设置 → 安全与登录」开启开发者模式。若没有此选项，请联系工作区管理员；Tunnel 也需要关联当前工作区并授予使用权限。</p><button class="text-button" @click="open('https://developers.openai.com/plugins/deploy/connect-chatgpt')">查看官方指南 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></details>
        <div class="wizard-actions"><span class="hint">保持本机连接开启</span><button @click="step=1">已添加，验证连接 →</button></div>
      </template>
      <template v-else>
        <h1>发一句话，确认连接。</h1>
        <p class="page-subtitle">在新对话中选中 Local Connector，发送下方消息。</p>
        <CopyField :value="prompt" label="连接验证消息" />
        <div class="actions"><button class="primary" :disabled="!!busy" @click="open('https://chatgpt.com/')">打开 ChatGPT <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div>
        <p class="verification-wait" role="status"><span class="status-dot"></span> 等待验证调用，收到后会自动确认</p>
        <details><summary>没有收到验证？</summary><p class="hint">确认对话已启用 Local Connector，并发送完整消息。已有插件请先在插件页刷新工具列表，再开启新对话；本机连接需保持开启。</p></details>
        <div class="wizard-actions"><button class="text-button" @click="step=0">← 查看添加步骤</button></div>
      </template>
    </template>
  </div>
</template>
