<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { NAlert, NButton, NForm, NFormItem } from 'naive-ui';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { t } from '../i18n';
import { displayMessage } from '../messages';
import ElasticPanel from './ElasticPanel.vue';
import CopyField from './CopyField.vue';
import OAuthGrants from './OAuthGrants.vue';

const props = defineProps<{ ingress: Ingress; origin?: {x:number;y:number;size:number;height?:number}; autoConnect?: boolean }>();
const open = ref(true);
const editing = ref(false);
const emit = defineEmits<{ close: []; edit: [ingress: Ingress] }>();
const { status, refresh } = useConnector();
const entry = computed(() => status.value?.ingresses.find(i => i.id === props.ingress.id) || props.ingress);
const https = computed(() => entry.value.transport === 'https');
const endpoint = computed(() => {
  if (!https.value) return entry.value.config.tunnelId;
  if (entry.value.url) return entry.value.url;
  const config = entry.value.config;
  if (config.httpsProvider === 'pinggy' && config.pinggyMode === 'named' || config.httpsProvider === 'localxpose' && config.localxposeMode === 'named') return config.httpsUrl;
  if (config.httpsProvider === 'custom' || config.httpsProvider === 'cloudflare' && config.cloudflareMode === 'named') return config.httpsUrl;
  if (config.httpsProvider === 'ngrok' && config.ngrokMode === 'named' && config.ngrokEndpoint) {
    const domain = config.ngrokEndpoint;
    return new URL(domain.includes('://') ? domain : `https://${domain}`).origin + '/mcp';
  }
  return '';
});
const reverseProxy = computed(() => {
  const config = entry.value.config;
  return `http://${config.httpsHost.includes(':') ? `[${config.httpsHost}]` : config.httpsHost}:${config.httpsPort}/mcp`;
});
const working = ref(false), reading = ref(false), error = ref(''), credentialError = ref(''), bearerToken = ref(''), apiKey = ref('');
const refreshingUrl = ref(false), oauthBusy = ref(false);
const connecting = computed(() => working.value || oauthBusy.value || entry.value.state === 'starting');
let disposed = false;
async function readCredentials() {
  if (!['bearer', 'openai'].includes(entry.value.auth)) return;
  reading.value = true;
  credentialError.value = '';
  try {
    const credentials = await api<{ bearerToken?: string; apiKey?: string }>(`ingress/${entry.value.id}/credentials`, 'POST');
    if (!disposed) { bearerToken.value = credentials.bearerToken || ''; apiKey.value = credentials.apiKey || ''; }
  } catch { if (!disposed) credentialError.value = t('credentialReadFailed'); }
  finally { if (!disposed) reading.value = false; }
}
async function connect() {
  if (connecting.value) return;
  working.value = true;
  error.value = '';
  try { await api(`ingress/${entry.value.id}/start`, 'POST'); }
  catch (e) { if (!disposed) error.value = e instanceof Error ? e.message : String(e); }
  finally { await refresh().catch(() => {}); if (!disposed) working.value = false; }
}
async function updateConnection(kind: 'url' | 'token') {
  if (connecting.value || reading.value) return;
  const id = entry.value.id;
  const restart = kind === 'url' ? entry.value.config.httpsProvider !== 'custom' : entry.value.running;
  working.value = true;
  refreshingUrl.value = restart;
  error.value = '';
  let stopped = false;
  try {
    if (restart) { await api(`ingress/${id}/stop`, 'POST'); stopped = true; }
    if (kind === 'token') {
      const result = await api<{ token: string }>(`ingress/${id}/token`, 'POST');
      if (!disposed) { bearerToken.value = result.token; credentialError.value = ''; }
    }
  } catch (e) { if (!disposed) error.value = e instanceof Error ? e.message : String(e); }
  finally {
    if (stopped) {
      try { await api(`ingress/${id}/start`, 'POST'); }
      catch (e) { if (!disposed) error.value = [error.value, e instanceof Error ? e.message : String(e)].filter(Boolean).join('\n'); }
    }
    await refresh().catch(e => { if (!disposed) error.value = String(e); });
    if (!disposed) { working.value = false; refreshingUrl.value = false; }
  }
}
onMounted(() => {
  void readCredentials();
  if (props.autoConnect) void connect();
});
onUnmounted(() => { disposed = true; bearerToken.value = ''; apiKey.value = ''; });
</script>

<template>
  <ElasticPanel :continuation="editing" :show="open" :origin="origin" :width="600" :title="t('connectionDetails') + ' · ' + entry.name" :busy="working || oauthBusy" @close="open=false" @closed="editing ? emit('edit',entry) : emit('close')">
    <NForm label-placement="top">
      <NFormItem :label="https ? 'MCP URL' : 'Tunnel ID'" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
        <template #label><span class="detail-heading"><span>{{https ? 'MCP URL' : 'Tunnel ID'}}</span><NButton v-if="https" :aria-label="t('reobtainMcpUrl')" text type="primary" size="tiny" :disabled="connecting || reading || !entry.enabled" @click="updateConnection('url')">{{t('reobtainMcpUrl')}}</NButton></span></template>
        <CopyField :value="refreshingUrl ? '' : endpoint" :label="https ? 'MCP URL' : 'Tunnel ID'" :loading="https && (refreshingUrl || !endpoint && connecting)" :placeholder="connecting ? t('fetchingMcpUrl') : t('generatedWhenConnected')"/>
      </NFormItem>
      <NFormItem v-if="entry.auth === 'bearer'" label="Bearer token" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
        <template #label><span class="detail-heading"><span>Bearer token</span><NButton :aria-label="t('generateNewToken')" text type="primary" size="tiny" :disabled="connecting || reading" @click="updateConnection('token')">{{t('generateNewToken')}}</NButton></span></template>
        <CopyField :value="bearerToken" label="Bearer token" password :placeholder="credentialError || ''"/>
      </NFormItem>
      <NFormItem v-if="entry.auth === 'openai'" label="Runtime API Key">
        <CopyField :value="apiKey" label="Runtime API Key" password :placeholder="credentialError || ''"/>
      </NFormItem>
      <OAuthGrants v-if="entry.auth === 'oauth'" :key="entry.id" :ingress-id="entry.id" :running="entry.running" :disabled="working" @busy="oauthBusy=$event"/>
      <NAlert v-if="credentialError" :show-icon="false" type="error">{{credentialError}} <NButton text :disabled="reading" @click="readCredentials">{{t('retry')}}</NButton></NAlert>
      <NFormItem v-if="https && (entry.config.httpsProvider === 'custom' || entry.config.httpsProvider === 'cloudflare' && entry.config.cloudflareMode === 'named')" :label="t('httpReverseProxyTarget')">
        <CopyField :value="reverseProxy" :label="t('httpReverseProxyTarget')"/>
      </NFormItem>
      <NAlert v-if="error || entry.error" :show-icon="false" type="error">{{displayMessage(error || entry.error)}}</NAlert>
    </NForm>
    <template #footer>
      <NButton :disabled="connecting" @click="editing=true; open=false">{{t('edit')}}</NButton>
      <span class="action-spacer"/>
      <NButton v-if="!entry.running && !connecting" :disabled="!entry.enabled" @click="connect">{{t('connect')}}</NButton>
      <NButton type="primary" :disabled="working || oauthBusy" @click="open=false">{{t('done')}}</NButton>
    </template>
  </ElasticPanel>
</template>

<style scoped>.detail-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;width:100%}</style>
