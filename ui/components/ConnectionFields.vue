<script setup lang="ts">
import { computed, onUnmounted } from 'vue';
import { NFormItem, NInput, NButton, NTooltip, NTag, NText } from 'naive-ui';
import { t } from '../i18n';
import SingleChoice from './SingleChoice.vue';
import { providerIcons } from '../assets/brand-reserve/providers';
import { required, domainRule } from '../formRules';
import { useConnector, type Config } from '../composables/useConnector';
import type { ConnectionForm } from '../composables/connectionForm';
import { providers, needsCredential, credentialSaved, clearCredentials } from '../ingressConfig';
import { openUrl } from '../platform';
const props = defineProps<{ form: ConnectionForm; config?: Config; disabled?: boolean; allowTunnel: boolean; recommendTunnel?: boolean }>();
const { run } = useConnector();
const https = computed(() => props.form.connectionMode === 'https');
const provider = computed(() => providers[props.form.httpsProvider]);
const modeField = computed(() => provider.value.modes.length > 1 ? provider.value.mode : null);
const credential = computed(() => {
  const field = provider.value.credential;
  return field ? { field, saved: credentialSaved(props.config, field), label: provider.value.credentialLabel, help: provider.value.help } : null;
});
const providerOptions = computed(() => Object.entries(providers).map(([value, provider]) => ({
  value, label: value === 'custom' ? t('customDomain') : provider.label,
  icon: providerIcons[value as keyof typeof providerIcons],
})));
const modeOptions = computed(() => provider.value.modes.map(value => ({ value, label: t(value === 'named' ? 'fixedDomain' : 'temporaryDomain') })));
const needsToken = computed(() => needsCredential(props.form));
onUnmounted(() => clearCredentials(props.form));
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
    <NFormItem :label="t('provider') + (allowTunnel ? '' : ' - HTTPS MCP')" path="httpsProvider"><SingleChoice v-model:value="form.httpsProvider" :label="t('provider')" :disabled="disabled" :options="providerOptions"/></NFormItem>
    <NFormItem v-if="modeField" :label="t('domainMode')" :path="modeField"><SingleChoice v-model:value="form[modeField]" :label="t('domainMode')" :disabled="disabled" :options="modeOptions"/></NFormItem>
    <NFormItem v-if="needsToken && credential" :path="credential.field" :rule="credential.saved ? undefined : required()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}"><template #label><span class="credential-heading"><span>{{credential.label}} <NText type="error">*</NText></span><NButton text type="primary" size="tiny" :aria-label="t('getToken')" @click="run('provider-help',()=>openUrl(provider.help))">{{t('getToken')}}</NButton></span></template><NInput v-model:value="form[credential.field]" type="password" show-password-on="click" :disabled="disabled" :placeholder="credential.saved ? t('savedLeaveBlankToKeep') : t('getToken')" :input-props="{autocomplete:'new-password','aria-label':credential.label}"/></NFormItem>
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
