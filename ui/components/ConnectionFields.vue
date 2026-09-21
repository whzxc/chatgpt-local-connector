<script setup lang="ts">
import { t } from '../i18n';
import { Eye, EyeOff, SquareArrowOutUpRight } from '@lucide/vue';
import { computed, ref, onUnmounted, watch } from 'vue';
import { api, useConnector } from '../composables/useConnector';
import type { ConnectionForm } from '../composables/useConnectionForm';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const props = defineProps<{ form: ConnectionForm; disabled?: boolean; }>();
const { status, run, busy } = useConnector();
const revealed = ref(false);
const advancedOpen = ref(false);
const tokenRevealed = ref(false);
const cloudflareNamed = computed(() => props.form.httpsProvider === 'cloudflare' && props.form.cloudflareMode === 'named');
const tokenField = computed(() => cloudflareNamed.value ? 'cloudflareToken' : 'ngrokAuthtoken');
const hasToken = computed(() => cloudflareNamed.value ? status.value?.config.hasCloudflareToken : status.value?.config.hasNgrokAuthtoken);
const tokenHelp = computed(() => cloudflareNamed.value ? 'https://dash.cloudflare.com/' : 'https://dashboard.ngrok.com/get-started/your-authtoken');
const managed = computed(() => props.form.httpsProvider !== 'custom');
const publicUrl = computed(() => status.value?.config.connectionMode === 'https' && status.value.config.httpsProvider === props.form.httpsProvider && (props.form.httpsProvider !== 'cloudflare' || status.value.config.cloudflareMode === props.form.cloudflareMode) ? status.value.connection?.mcpUrl || '' : '');
async function revealToken() {
  if (!tokenRevealed.value && !props.form[tokenField.value] && hasToken.value) {
    props.form[tokenField.value] = (await api<{ ngrokAuthtoken: string; cloudflareToken: string }>('config/credentials'))[tokenField.value];
  }
  tokenRevealed.value = !tokenRevealed.value;
}
const upstream = computed(() => {
  const host = props.form.httpsHost;
  return `http://${host.includes(':') ? `[${host}]` : host}:${props.form.httpsPort}/mcp`;
});
const https = computed(() => props.form.connectionMode === 'https');
const hasSecret = computed(() => status.value?.config.hasApiKey);
watch(() => [props.form.connectionMode, props.form.httpsProvider, props.form.cloudflareMode], () => { tokenRevealed.value = false; revealed.value = false; advancedOpen.value = false; });
onUnmounted(() => { props.form.apiKey = ''; props.form.ngrokAuthtoken = ''; props.form.cloudflareToken = ''; });
async function reveal() {
  if (revealed.value) { revealed.value = false; return; }
  if (!props.form.apiKey && hasSecret.value) {
    const credentials = await api<{ apiKey: string }>('config/credentials');
    props.form.apiKey = credentials.apiKey;
  }
  revealed.value = true;
}
</script>
<template>
  <div class="connection-fields">
    <div class="connection-mode-row">
      <span class="field-label">{{ t('connectionMethod') }}</span>
      <div class="mode-switch" role="group" :aria-label="t('connectionMethod')">
        <button type="button" :aria-pressed="!https" :class="{active:!https}" :disabled="disabled || !!busy" @click="form.connectionMode='tunnel'">OpenAI Tunnel</button>
        <button type="button" :aria-pressed="https" :class="{active:https}" :disabled="disabled || !!busy" @click="form.connectionMode='https'">HTTPS MCP</button>
      </div>
    </div>
    <div class="settings-fields connection-form-grid">
      <template v-if="https">
        <div class="full-field connection-mode-row">
          <span class="field-label">{{ t('provider') }}</span>
          <div class="mode-switch" role="group" :aria-label="t('httpsProvider')">
            <button v-for="option in [{value:'cloudflare',label:'Cloudflare'},{value:'ngrok',label:'ngrok'},{value:'custom',label:t('customDomain')}] as const" :key="option.value" type="button" :aria-pressed="form.httpsProvider===option.value" :class="{active:form.httpsProvider===option.value}" :disabled="disabled || !!busy" @click="form.httpsProvider=option.value">{{option.label}}</button>
          </div>
        </div>
        <div v-if="form.httpsProvider==='cloudflare'" class="full-field connection-mode-row">
          <span class="field-label">{{ t('cloudflareMode') }}</span>
          <div class="mode-switch" role="group" :aria-label="t('cloudflareMode')">
            <button v-for="option in [{value:'quick',label:t('quickTrial')},{value:'named',label:t('fixedDomain')}] as const" :key="option.value" type="button" :aria-pressed="form.cloudflareMode===option.value" :class="{active:form.cloudflareMode===option.value}" :disabled="disabled || !!busy" @click="form.cloudflareMode=option.value">{{option.label}}</button>
          </div>
        </div>
        <p v-if="form.httpsProvider==='cloudflare' && !cloudflareNamed" class="hint full-field">{{ t('noAccountOrDomainRequiredATemporaryUrl') }}</p>
        <div v-if="form.httpsProvider==='ngrok' || cloudflareNamed" class="full-field">
          <div class="credential-label"><label for="provider-token">{{cloudflareNamed ? 'Tunnel Token' : 'Authtoken'}}</label><a :href="tokenHelp" @click.prevent="run('provider-help',()=>openUrl(tokenHelp))">{{ t('getToken') }} <SquareArrowOutUpRight aria-hidden="true" /></a></div>
          <div class="credential-input">
            <input id="provider-token" v-model="form[tokenField]" :type="tokenRevealed ? 'text' : 'password'" :readonly="disabled" :placeholder="hasToken ? t('savedLeaveBlankToKeep') : cloudflareNamed ? t('pasteTheTunnelTokenWithoutTheInstallCommand') : t('pasteYourNgrokAccountAuthtoken')" :required="!hasToken" autocomplete="new-password" spellcheck="false" />
            <button type="button" :disabled="!!busy" :aria-label="tokenRevealed ? t('hideToken') : t('showToken')" :aria-pressed="tokenRevealed" @click="run('reveal-token',revealToken)"><EyeOff v-if="tokenRevealed" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button>
          </div>
        </div>
        <label v-if="!managed || cloudflareNamed" class="full-field">{{ t('publicMcpUrl') }}<input v-model="form.httpsUrl" type="url" placeholder="https://connector.example.com/mcp" :readonly="disabled" required /></label>
        <div v-if="cloudflareNamed" class="full-field connection-upstream">
          <span class="field-label">{{ t('cloudflareRouteServiceUrl') }}</span>
          <CopyField value="http://127.0.0.1:8787" label="Cloudflare Service URL" />
          <p class="hint">{{ t('createATunnelInCloudflareUnderNetworkingTunnels') }}</p>
        </div>
        <div v-if="publicUrl" class="full-field connection-upstream">
          <span class="field-label">{{ t('chatgptConnectionUrl') }}</span><CopyField :value="publicUrl" :label="t('publicMcpUrl')" />
          <button type="button" class="text-button" :disabled="!!busy" @click="run('chatgpt-open',()=>openUrl(status?.chatgptUrl || 'https://chatgpt.com/plugins'))">{{ t('addConnectionInChatgpt') }} <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button>
        </div>
      </template>
      <div v-else>
        <div class="credential-label"><label for="connection-tunnel">Tunnel ID</label><a href="https://platform.openai.com/settings/organization/tunnels" @click.prevent="run('tunnel-help',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">{{ t('getCredentials') }} <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <input id="connection-tunnel" v-model="form.tunnelId" placeholder="tunnel_…" :readonly="disabled" required autocomplete="off" />
      </div>
      <div v-if="!https">
        <div class="credential-label"><label for="connection-secret">Runtime API Key</label><a href="https://platform.openai.com/settings/organization/api-keys" @click.prevent="run('api-key-help',()=>openUrl('https://platform.openai.com/settings/organization/api-keys'))">{{ t('getKey') }} <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <div class="credential-input">
          <input id="connection-secret" v-model="form.apiKey" :type="revealed ? 'text' : 'password'" :readonly="disabled" :placeholder="hasSecret ? t('savedLeaveBlankToKeep') : t('pasteRuntimeKey')" autocomplete="new-password" spellcheck="false" :required="!hasSecret" />
          <button type="button" :disabled="!!busy" :aria-label="revealed ? t('hideKey') : t('showKey')" :aria-pressed="revealed" @click="run('reveal-key',reveal)"><EyeOff v-if="revealed" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button>
        </div>
      </div>
      <template v-if="https">
        <div v-if="!managed" class="full-field connection-upstream"><span class="field-label">{{ t('proxyTarget') }}</span><CopyField :value="upstream" :label="t('httpReverseProxyTarget')" /></div>
        <details v-if="!managed" class="full-field connection-advanced" :open="advancedOpen" @toggle="advancedOpen=($event.target as HTMLDetailsElement).open" @invalid.capture="advancedOpen=true">
          <summary>{{ t('advancedSettings') }}</summary>
          <div class="settings-fields connection-form-grid">
            <label>{{ t('localListeningIp') }}<input v-model="form.httpsHost" placeholder="127.0.0.1" :readonly="disabled" required /></label>
            <label>{{ t('listeningPort') }}<input v-model.number="form.httpsPort" type="number" min="1" max="65535" :readonly="disabled" required /></label>
          </div>
        </details>
      </template>
      <p v-if="https && !managed" class="hint full-field connection-help">{{ t('forwardThePublicUrlToTheProxyTarget') }}</p>
    </div>
  </div>
</template>
