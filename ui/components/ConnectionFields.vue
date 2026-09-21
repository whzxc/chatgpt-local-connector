<script setup lang="ts">
import { computed, onUnmounted } from 'vue';
import { NFormItem, NInput, NButton, NTooltip, NTag, NText } from 'naive-ui';
import { t } from '../i18n';
import SingleChoice from './SingleChoice.vue';
import { required } from '../formRules';
import { useConnector, type Config } from '../composables/useConnector';
import type { ConnectionForm } from '../composables/connectionForm';
import { openUrl } from '../platform';
import CopyField from './CopyField.vue';
const props = defineProps<{ form: ConnectionForm; config?: Config; disabled?: boolean; allowTunnel: boolean; recommendTunnel?: boolean }>();
const { run } = useConnector();
const https = computed(() => props.form.connectionMode === 'https');
const named = computed(() => props.form.httpsProvider === 'cloudflare' && props.form.cloudflareMode === 'named');
const tokenField = computed(() => named.value ? 'cloudflareToken' : 'ngrokAuthtoken');
const hasToken = computed(() => named.value ? props.config?.hasCloudflareToken : props.config?.hasNgrokAuthtoken);
const tokenHelp = computed(() => named.value ? 'https://dash.cloudflare.com/' : 'https://dashboard.ngrok.com/get-started/your-authtoken');
onUnmounted(() => { props.form.apiKey=''; props.form.cloudflareToken=''; props.form.ngrokAuthtoken=''; });
</script>
<template>
  <NFormItem v-if="allowTunnel" :label="t('connectionMethod')" path="connectionMode">
    <SingleChoice v-model:value="form.connectionMode" :label="t('connectionMethod')" :disabled="disabled" :options="[{label:'OpenAI Secure Tunnel',value:'tunnel'},{label:'HTTPS MCP',value:'https'}]">
      <template #option="{option}">{{option.label}}<NTooltip v-if="recommendTunnel && option.value==='tunnel'" trigger="hover"><template #trigger><NTag class="recommend-tag" size="small" type="success" round tabindex="0">{{t('recommended')}}</NTag></template><span class="recommend-help">{{t('secureTunnelRecommendation')}}</span></NTooltip></template>
    </SingleChoice>
  </NFormItem>
  <template v-if="!https">
    <NFormItem path="tunnelId" :rule="required()" :show-require-mark="false" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
      <template #label><span class="credential-heading"><span>Tunnel ID <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getCredentials')" @click="run('tunnel-help',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">{{t('getCredentials')}}</NButton></span></template>
      <NInput v-model:value="form.tunnelId" :input-props="{'aria-label':'Tunnel ID'}" :disabled="disabled" placeholder="tunnel_…"/>
    </NFormItem>
    <NFormItem path="apiKey" :rule="config?.hasApiKey ? undefined : required()" :show-require-mark="false" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
      <template #label><span class="credential-heading"><span>Runtime API Key <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getKey')" @click="run('api-key-help',()=>openUrl('https://platform.openai.com/settings/organization/api-keys'))">{{t('getKey')}}</NButton></span></template>
      <NInput v-model:value="form.apiKey" type="password" show-password-on="click" :disabled="disabled" :placeholder="config?.hasApiKey ? t('savedLeaveBlankToKeep') : t('pasteRuntimeKey')" :input-props="{autocomplete:'new-password','aria-label':'Runtime API Key'}"/>
    </NFormItem>
  </template>
  <template v-else>
    <NFormItem :label="t('provider') + (allowTunnel ? '' : ' - HTTPS MCP')" path="httpsProvider"><SingleChoice v-model:value="form.httpsProvider" :label="t('provider')" :disabled="disabled" :options="[{label:'Cloudflare',value:'cloudflare'},{label:'ngrok',value:'ngrok'},{label:t('customDomain'),value:'custom'}]"/></NFormItem>
    <NFormItem v-if="form.httpsProvider==='cloudflare'" :label="t('cloudflareMode')" path="cloudflareMode"><SingleChoice v-model:value="form.cloudflareMode" :label="t('cloudflareMode')" :disabled="disabled" :options="[{label:t('quickTrial'),value:'quick'},{label:t('fixedDomain'),value:'named'}]"/></NFormItem>
    <NFormItem v-if="named || form.httpsProvider==='ngrok'" :path="tokenField" :rule="hasToken ? undefined : required()" :show-require-mark="false" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}"><template #label><span class="credential-heading"><span>{{named ? 'Tunnel Token' : 'Authtoken'}} <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getToken')" @click="run('provider-help',()=>openUrl(tokenHelp))">{{t('getToken')}}</NButton></span></template><NInput v-model:value="form[tokenField]" type="password" show-password-on="click" :disabled="disabled" :placeholder="hasToken ? t('savedLeaveBlankToKeep') : t('getToken')" :input-props="{autocomplete:'new-password','aria-label':named ? 'Tunnel Token' : 'Authtoken'}"/></NFormItem>
    <slot name="mcp-url"/>
    <div v-if="config && (named || form.httpsProvider==='custom')" class="listener-options">

      <CopyField :value="`http://${form.httpsHost.includes(':') ? `[${form.httpsHost}]` : form.httpsHost}:${form.httpsPort}/mcp`" :label="t('httpReverseProxyTarget')"/>
    </div>
  </template>
</template>
<style scoped>.recommend-tag{margin-left:8px;vertical-align:middle}.recommend-help{display:block;max-width:320px;line-height:1.6}.listener-options{margin:0 0 24px}.credential-heading{display:flex;width:100%;align-items:center;justify-content:space-between;gap:16px}</style>
