<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { SquareArrowOutUpRight } from '@lucide/vue';
import { computed } from 'vue';
import { api, useConnector } from '../composables/useConnector';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const emit = defineEmits<{ done: []; settings: [] }>();
const { status, busy, run, refresh, connectionError } = useConnector();
const https = computed(() => status.value?.config.connectionMode === 'https');
const endpoint = computed(() => https.value ? status.value?.connection?.mcpUrl || '' : status.value?.config.tunnelId || '');
// These values are copied into ChatGPT; keep model-facing text locale-independent.
const description = computed(() => `Connect to this computer via ${https.value ? 'HTTPS MCP' : 'Tunnel'}`);
const online = computed(() => !connectionError.value && status.value?.tunnel.state === 'ready');
const verified = computed(() => status.value?.core.chatgpt?.challengeVerifiedAt);
const code = computed(() => status.value?.core.chatgpt?.code || '');
const prompt = computed(() => `Use the Local Connector plugin to call connector_verify with code ${code.value}. Only verify the connection; do not create a task.`);
const open = (url: string) => run('chatgpt-open', () => openUrl(url));
const reset = () => run('chatgpt-verify', async () => { await api('verification/reset', 'POST'); await refresh(); });
</script>
<template>
  <div class="chat-guide">
    <h1>{{ t('connectChatgpt') }}</h1>
    <div v-if="!online" class="guide-notice" role="status">
      <span>{{displayMessage(connectionError || status?.tunnel.error) || t('startTheLocalConnectionFirst')}}</span>
      <button v-if="!status?.config.configured" class="text-button" @click="emit('settings')">{{ t('configureConnection') }}</button>
      <button v-else class="text-button" :disabled="!!busy || ['starting','installing','connecting'].includes(status?.tunnel.state || '')" @click="run('connect', async () => { await api('start', 'POST'); await refresh(); })">{{ t('connect') }}</button>
    </div>
    <ol class="guide-steps">
      <li><span class="guide-step-number">1</span><div><div class="guide-step-heading"><strong>{{ t('enableDeveloperMode') }}</strong><button class="text-button" :disabled="!!busy" @click="open('https://chatgpt.com/settings/security')">{{ t('openSettings') }} <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div><p>{{ t('settingsSecuritySignInDeveloperMode') }}</p></div></li>
      <li><span class="guide-step-number">2</span><div>
        <div class="guide-step-heading"><strong>Create MCP App</strong><button class="text-button" :disabled="!!busy" @click="open('https://chatgpt.com/plugins')">{{ t('openPlugins') }} <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button></div>
        <p>Plugins → Add / ＋ → Create MCP App</p>
        <dl class="guide-fields">
          <div><dt>Name</dt><dd><CopyField value="Local Connector" :label="t('suggestedName')" /></dd></div>
          <div><dt>Description</dt><dd><CopyField :value="description" :label="t('suggestedDescription')" /></dd></div>
          <div><dt>Connection</dt><dd>{{https ? 'Server URL' : 'Tunnel'}}</dd></div>
          <div v-if="https"><dt>Server URL</dt><dd><CopyField :value="endpoint" :label="t('chatgptConnectionUrl')" /></dd></div>
          <div v-else><dt>Tunnel</dt><dd><span>{{ t('selectTheCurrentTunnel') }}</span><small v-if="endpoint">{{endpoint}}</small></dd></div>
          <div><dt>Authentication</dt><dd>{{ (status?.ingresses.find(i => i.id === 'default') ?? status?.ingresses[0])?.auth === 'bearer' ? 'Bearer (configure with CLI)' : 'No Authentication' }}</dd></div>
        </dl>
      </div></li>
      <li><span class="guide-step-number">3</span><div><strong>{{ t('sendVerificationMessage') }}</strong><p>{{ t('selectTheConnectionInANewConversationAnd') }}</p><CopyField v-if="code" :value="prompt" :label="t('connectionVerificationMessage')" multiline />
        <p v-if="online && verified" class="guide-result" role="status">{{ t('verificationRequestReceived') }} <button class="text-button" :disabled="!!busy" @click="reset">{{ t('verifyAgain') }}</button></p>
      </div></li>
    </ol>
  </div>
</template>
<style scoped>
h1{font-size:26px;line-height:1.35;letter-spacing:-.6px;margin-bottom:26px}
.guide-steps{padding:0;margin:0;list-style:none;display:grid;gap:24px}
.guide-steps>li{display:flex;gap:14px;align-items:flex-start}
.guide-step-number{display:grid;place-items:center;flex:0 0 26px;height:26px;border-radius:8px;background:color-mix(in srgb,var(--accent) 8%,transparent);color:var(--accent);font-size:12px}
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
