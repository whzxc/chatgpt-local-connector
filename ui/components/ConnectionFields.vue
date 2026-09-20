<script setup lang="ts">
import { Eye, EyeOff, SquareArrowOutUpRight } from '@lucide/vue';
import { computed, ref, onUnmounted, watch } from 'vue';
import { api, useConnector } from '../composables/useConnector';
import type { ConnectionForm } from '../composables/useConnectionForm';
import { openUrl } from '../platform';
const props = defineProps<{ form: ConnectionForm; disabled?: boolean; }>();
const { status, run, busy } = useConnector();
const revealed = ref(false);
const https = computed(() => props.form.connectionMode === 'https');
const secret = computed(() => https.value ? 'httpsApiKey' : 'apiKey');
const hasSecret = computed(() => https.value ? status.value?.config.hasHttpsApiKey : status.value?.config.hasApiKey);
watch(https, () => { revealed.value = false; });
onUnmounted(() => { props.form.apiKey = ''; props.form.httpsApiKey = ''; });
async function reveal() {
  if (revealed.value) { revealed.value = false; return; }
  if (!props.form[secret.value] && hasSecret.value) {
    const credentials = await api<{ apiKey: string; httpsApiKey: string }>('config/credentials');
    props.form[secret.value] = credentials[secret.value];
  }
  revealed.value = true;
}
async function generate() {
  props.form.httpsApiKey = (await api<{ key: string }>('config/key', 'POST')).key;
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
        <label class="full-field">公网 MCP URL<input v-model="form.httpsUrl" type="url" placeholder="https://connector.example.com/mcp" :readonly="disabled" required /></label>
        <label>本机监听 IP<input v-model="form.httpsHost" placeholder="127.0.0.1" :readonly="disabled" required /></label>
        <label>监听端口<input v-model.number="form.httpsPort" type="number" min="1" max="65535" :readonly="disabled" required /></label>
      </template>
      <div v-else>
        <div class="credential-label"><label for="connection-tunnel">Tunnel ID</label><a href="https://platform.openai.com/settings/organization/tunnels" @click.prevent="run('tunnel-help',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">获取凭据 <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <input id="connection-tunnel" v-model="form.tunnelId" placeholder="tunnel_…" :readonly="disabled" required autocomplete="off" />
      </div>
      <div v-if="https" class="full-field connection-mode-row">
        <span class="field-label">访问认证</span>
        <div class="mode-switch" role="group" aria-label="访问认证">
          <button type="button" :aria-pressed="!form.httpsRequireAuth" :class="{active:!form.httpsRequireAuth}" :disabled="disabled || !!busy" @click="form.httpsRequireAuth=false">无需认证</button>
          <button type="button" :aria-pressed="form.httpsRequireAuth" :class="{active:form.httpsRequireAuth}" :disabled="disabled || !!busy" @click="form.httpsRequireAuth=true">访问密钥</button>
        </div>
      </div>
      <div v-if="!https || form.httpsRequireAuth" :class="{'full-field':https}">
        <div class="credential-label"><label for="connection-secret">{{https ? 'MCP 访问密钥' : 'Runtime API Key'}}</label><button v-if="https" type="button" class="text-button" :disabled="disabled || !!busy" @click="run('generate-key',generate)">生成新密钥</button><a v-else href="https://platform.openai.com/settings/organization/api-keys" @click.prevent="run('api-key-help',()=>openUrl('https://platform.openai.com/settings/organization/api-keys'))">获取密钥 <SquareArrowOutUpRight aria-hidden="true" /></a></div>
        <div class="credential-input">
          <input id="connection-secret" v-model="form[secret]" :type="revealed ? 'text' : 'password'" :readonly="disabled" :placeholder="hasSecret ? '已保存；留空保持不变' : https ? '生成或填写至少 32 位随机密钥' : '粘贴运行密钥'" autocomplete="new-password" spellcheck="false" :required="!hasSecret" />
          <button type="button" :disabled="!!busy" :aria-label="revealed ? '隐藏密钥' : '显示密钥'" :aria-pressed="revealed" @click="run('reveal-key',reveal)"><EyeOff v-if="revealed" aria-hidden="true"/><Eye v-else aria-hidden="true"/></button>
        </div>
      </div>
      <p v-if="https" class="hint full-field connection-help">将公网 HTTPS /mcp 反向代理到本机监听地址的 HTTP /mcp（同机使用 127.0.0.1，跨设备使用局域网 IP）。{{form.httpsRequireAuth ? '保留 Authorization 请求头，在 ChatGPT 选择 API key（Bearer）并填写此密钥。' : '在 ChatGPT 选择 No authentication；如需限制访问，请在代理侧配置。'}}</p>
    </div>
  </div>
</template>
