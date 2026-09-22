<script setup lang="ts">
import { computed, onUnmounted } from 'vue';
import { NFormItem, NInput, NButton, NTooltip, NTag, NText } from 'naive-ui';
import { t } from '../i18n';
import SingleChoice from './SingleChoice.vue';
import { providerIcons } from '../assets/brand-reserve/providers';
import { required, domainRule } from '../formRules';
import { useConnector, type Config } from '../composables/useConnector';
import type { ConnectionForm } from '../composables/connectionForm';
import { openUrl } from '../platform';
const props = defineProps<{ form: ConnectionForm; config?: Config; disabled?: boolean; allowTunnel: boolean; recommendTunnel?: boolean }>();
const { run } = useConnector();
const https = computed(() => props.form.connectionMode === 'https');
const named = computed(() => props.form.httpsProvider === 'cloudflare' && props.form.cloudflareMode === 'named');
const modeField = computed(() => ({cloudflare:'cloudflareMode',ngrok:'ngrokMode',pinggy:'pinggyMode'} as const)[props.form.httpsProvider as 'cloudflare'|'ngrok'|'pinggy']);
const credential = computed(() => {
  switch(props.form.httpsProvider) {
    case 'cloudflare': return {field:'cloudflareToken' as const, saved:props.config?.hasCloudflareToken, label:'Tunnel Token', help:'https://dash.cloudflare.com/'};
    case 'pinggy': return {field:'pinggyToken' as const, saved:props.config?.hasPinggyToken, label:'Token', help:'https://dashboard.pinggy.io/'};
    case 'localxpose': return {field:'localxposeAccessToken' as const, saved:props.config?.hasLocalxposeAccessToken, label:'Access Token', help:'https://localxpose.io/dashboard/access'};
    default: return {field:'ngrokAuthtoken' as const, saved:props.config?.hasNgrokAuthtoken, label:'Authtoken', help:'https://dashboard.ngrok.com/get-started/your-authtoken'};
  }
});
const needsToken = computed(() => named.value || props.form.httpsProvider==='ngrok' || props.form.httpsProvider==='localxpose' || props.form.httpsProvider==='pinggy' && props.form.pinggyMode==='named');
onUnmounted(() => { props.form.apiKey=''; props.form.cloudflareToken=''; props.form.ngrokAuthtoken=''; props.form.pinggyToken=''; props.form.localxposeAccessToken=''; });
</script>
<template>
  <NFormItem v-if="allowTunnel" :label="t('connectionMethod')" path="connectionMode">
    <SingleChoice v-model:value="form.connectionMode" :label="t('connectionMethod')" :disabled="disabled" :options="[{label:'OpenAI Secure Tunnel',value:'tunnel'},{label:'HTTPS MCP',value:'https'}]">
      <template #option="{option}"><span class="connection-option"><span>{{option.label}}</span><NTooltip v-if="recommendTunnel && option.value==='tunnel'" trigger="hover"><template #trigger><NTag class="recommend-tag" size="tiny" type="success" round tabindex="0">{{t('recommended')}}</NTag></template><span class="recommend-help">{{t('secureTunnelRecommendation')}}</span></NTooltip></span></template>
    </SingleChoice>
  </NFormItem>
  <template v-if="!https">
    <NFormItem path="tunnelId" :rule="required()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
      <template #label><span class="credential-heading"><span>Tunnel ID <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getCredentials')" @click="run('tunnel-help',()=>openUrl('https://platform.openai.com/settings/organization/tunnels'))">{{t('getCredentials')}}</NButton></span></template>
      <NInput v-model:value="form.tunnelId" :input-props="{'aria-label':'Tunnel ID'}" :disabled="disabled" placeholder="tunnel_…"/>
    </NFormItem>
    <NFormItem path="apiKey" :rule="config?.hasApiKey ? undefined : required()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
      <template #label><span class="credential-heading"><span>Runtime API Key <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getKey')" @click="run('api-key-help',()=>openUrl('https://platform.openai.com/settings/organization/api-keys'))">{{t('getKey')}}</NButton></span></template>
      <NInput v-model:value="form.apiKey" type="password" show-password-on="click" :disabled="disabled" :placeholder="config?.hasApiKey ? t('savedLeaveBlankToKeep') : t('pasteRuntimeKey')" :input-props="{autocomplete:'new-password','aria-label':'Runtime API Key'}"/>
    </NFormItem>
  </template>
  <template v-else>
    <NFormItem :label="t('provider') + (allowTunnel ? '' : ' - HTTPS MCP')" path="httpsProvider"><SingleChoice v-model:value="form.httpsProvider" :label="t('provider')" :disabled="disabled" :options="[{label:'ngrok',value:'ngrok',icon:providerIcons.ngrok},{label:'Cloudflare',value:'cloudflare',icon:providerIcons.cloudflare},{label:'Pinggy',value:'pinggy',icon:providerIcons.pinggy},{label:'LocalXpose',value:'localxpose',icon:providerIcons.localxpose},{label:t('customDomain'),value:'custom',icon:providerIcons.custom}]"/></NFormItem>
    <NFormItem v-if="modeField" :label="t('domainMode')" :path="modeField"><SingleChoice v-model:value="form[modeField]" :label="t('domainMode')" :disabled="disabled" :options="[{label:t('fixedDomain'),value:'named'},{label:t('temporaryDomain'),value:'quick'}]"/></NFormItem>
    <NFormItem v-if="needsToken" :path="credential.field" :rule="credential.saved ? undefined : required()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}"><template #label><span class="credential-heading"><span>{{credential.label}} <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getToken')" @click="run('provider-help',()=>openUrl(credential.help))">{{t('getToken')}}</NButton></span></template><NInput v-model:value="form[credential.field]" type="password" show-password-on="click" :disabled="disabled" :placeholder="credential.saved ? t('savedLeaveBlankToKeep') : t('getToken')" :input-props="{autocomplete:'new-password','aria-label':credential.label}"/></NFormItem>
    <NFormItem v-if="form.httpsProvider==='ngrok' && form.ngrokMode==='named'" :label="t('domain')" path="domain" :rule="domainRule()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
      <template #label><span class="credential-heading"><span>{{t('domain')}} <NText type="error">*</NText></span><NButton text type="primary" :aria-label="t('getDomain')" size="tiny" @click="run('domain-help',()=>openUrl('https://dashboard.ngrok.com/domains'))">{{t('getDomain')}}</NButton></span></template>
      <NInput v-model:value="form.domain" :disabled="disabled" :input-props="{'aria-label':t('domain')}" placeholder="your-domain.ngrok-free.dev"/>
    </NFormItem>
    <NFormItem v-if="form.httpsProvider==='pinggy' && form.pinggyMode==='named' || form.httpsProvider==='localxpose'" :label="t('domain')" path="domain" :rule="domainRule()" required>
      <NInput v-model:value="form.domain" :disabled="disabled" :input-props="{'aria-label':t('domain')}" placeholder="mcp.example.com"/>
    </NFormItem>
    <slot name="domain"/>

  </template>
</template>
<style scoped>.connection-option{display:inline-flex;align-items:center;gap:6px;vertical-align:top}.recommend-tag{flex-shrink:0}.recommend-help{display:block;max-width:320px;line-height:1.6}.listener-options{margin:0 0 24px}.credential-heading{display:flex;width:100%;align-items:center;justify-content:space-between;gap:16px}</style>
