<script setup lang="ts">
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
      <span class="field-label">连接方式</span>
      <div class="mode-switch" role="group" aria-label="连接方式">
        <button type="button" :aria-pressed="!https" :class="{active:!https}" :disabled="disabled || !!busy" @click="form.connectionMode='tunnel'">OpenAI Tunnel</button>
        <button type="button" :aria-pressed="https" :class="{active:https}" :disabled="disabled || !!busy" @click="form.connectionMode='https'">HTTPS MCP</button>
      </div>
    </div>
    <div class="settings-fields connection-form-grid">
      <template v-if="https">
        <div class="full-field connection-mode-row">
          <span class="field-label">接入方式</span>
          <div class="mode-switch" role="group" aria-label="HTTPS 接入方式">
            <button v-for="option in [{value:'cloudflare',label:'Cloudflare'},{value:'ngrok',label:'ngrok'},{value:'custom',label:'自定义域名'}] as const" :key="option.value" type="button" :aria-pressed="form.httpsProvider===option.value" :class="{active:form.httpsProvider===option.value}" :disabled="disabled || !!busy" @click="form.httpsProvider=option.value">{{option.label}}</button>
          </div>
        </div>
        <div v-if="form.httpsProvider==='cloudflare'" class="full-field connection-mode-row">
          <span class="field-label">Cloudflare 模式</span>
          <div class="mode-switch" role="group" aria-label="Cloudflare 模式">
            <button v-for="option in [{value:'quick',label:'临时体验'},{value:'named',label:'固定域名'}] as const" :key="option.value" type="button" :aria-pressed="form.cloudflareMode===option.value" :class="{active:form.cloudflareMode===option.value}" :disabled="disabled || !!busy" @click="form.cloudflareMode=option.value">{{option.label}}</button>
          </div>
        </div>
        <p v-if="form.httpsProvider==='cloudflare' && !cloudflareNamed" class="hint full-field">无需账号或域名，连接后自动生成临时地址。重新连接后地址可能变化，需要更新 ChatGPT 中的连接；适合试用。</p>
        <div v-if="form.httpsProvider==='ngrok' || cloudflareNamed" class="full-field">
          <div class="credential-label"><label for="provider-token">{{cloudflareNamed ? 'Tunnel Token' : 'Authtoken'}}</label><a :href="tokenHelp" @click.prevent="run('provider-help',()=>openUrl(tokenHelp))">获取令牌 <SquareArrowOutUpRight aria-hidden="true" /></a></div>
          <div class="credential-input">
            <input id="provider-token" v-model="form[tokenField]" :type="tokenRevealed ? 'text' : 'password'" :readonly="disabled" :placeholder="hasToken ? '已保存；留空保持不变' : cloudflareNamed ? '粘贴 Tunnel Token，不含安装命令' : '粘贴 ngrok 账号的 Authtoken'" :required="!hasToken" autocomplete="new-password" spellcheck="false" />
            <button type="button" :disabled="!!busy" :aria-label="tokenRevealed ? '隐藏令牌' : '显示令牌'" :aria-pressed="tokenRevealed" @click="run('reveal-token',revealToken)"><EyeOff v-if="tokenRevealed" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button>
          </div>
        </div>
        <label v-if="!managed || cloudflareNamed" class="full-field">公网 MCP URL<input v-model="form.httpsUrl" type="url" placeholder="https://connector.example.com/mcp" :readonly="disabled" required /></label>
        <div v-if="cloudflareNamed" class="full-field connection-upstream">
          <span class="field-label">Cloudflare 路由的服务地址</span>
          <CopyField value="http://127.0.0.1:8787" label="Cloudflare Service URL" />
          <p class="hint">在 Cloudflare 的 Networking → Tunnels 创建通道，复制 Token 后保存并连接；再添加公开路由，将上述公网域名指向此服务地址。客户端由应用管理，无需运行安装命令。</p>
        </div>
        <div v-if="publicUrl" class="full-field connection-upstream">
          <span class="field-label">ChatGPT 接入地址</span><CopyField :value="publicUrl" label="公网 MCP URL" />
          <button type="button" class="text-button" :disabled="!!busy" @click="run('chatgpt-open',()=>openUrl(status?.chatgptUrl || 'https://chatgpt.com/plugins'))">前往 ChatGPT 添加连接 <SquareArrowOutUpRight class="external-icon" aria-hidden="true" /></button>
        </div>
      </template>
      <div v-else>
        <div class="credential-label"><label for="connection-tunnel">Tunnel ID</label><a href="https://platform.openai.com/settings/organization/tunnels" @click.prevent="run('tunnel-help',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">获取凭据 <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <input id="connection-tunnel" v-model="form.tunnelId" placeholder="tunnel_…" :readonly="disabled" required autocomplete="off" />
      </div>
      <div v-if="!https">
        <div class="credential-label"><label for="connection-secret">Runtime API Key</label><a href="https://platform.openai.com/settings/organization/api-keys" @click.prevent="run('api-key-help',()=>openUrl('https://platform.openai.com/settings/organization/api-keys'))">获取密钥 <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <div class="credential-input">
          <input id="connection-secret" v-model="form.apiKey" :type="revealed ? 'text' : 'password'" :readonly="disabled" :placeholder="hasSecret ? '已保存；留空保持不变' : '粘贴运行密钥'" autocomplete="new-password" spellcheck="false" :required="!hasSecret" />
          <button type="button" :disabled="!!busy" :aria-label="revealed ? '隐藏密钥' : '显示密钥'" :aria-pressed="revealed" @click="run('reveal-key',reveal)"><EyeOff v-if="revealed" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button>
        </div>
      </div>
      <template v-if="https">
        <div v-if="!managed" class="full-field connection-upstream"><span class="field-label">代理目标</span><CopyField :value="upstream" label="HTTP 反向代理目标" /></div>
        <details v-if="!managed" class="full-field connection-advanced" :open="advancedOpen" @toggle="advancedOpen=($event.target as HTMLDetailsElement).open" @invalid.capture="advancedOpen=true">
          <summary>高级设置</summary>
          <div class="settings-fields connection-form-grid">
            <label>本机监听 IP<input v-model="form.httpsHost" placeholder="127.0.0.1" :readonly="disabled" required /></label>
            <label>监听端口<input v-model.number="form.httpsPort" type="number" min="1" max="65535" :readonly="disabled" required /></label>
          </div>
        </details>
      </template>
      <p v-if="https" class="hint full-field connection-help"><template v-if="!managed">将公网 URL 转发到上述代理目标。</template>在 ChatGPT 选择 No authentication。</p>
    </div>
  </div>
</template>
