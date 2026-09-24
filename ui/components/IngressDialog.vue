<script setup lang="ts">
import { ref, reactive, computed, onUnmounted, watch } from 'vue';
import { NForm, NFormItem, NInput, NButton, NAlert, NText, type FormInst } from 'naive-ui';
import ElasticPanel from './ElasticPanel.vue';
import ToolPolicyField from './ToolPolicyField.vue';
import type { ToolPolicy } from '../toolPolicy';
import SourceIcon from './SourceIcon.vue';
import { required, domainRule, validDomain } from '../formRules';
import { t } from '../i18n';
import { controlSource, curatedSources, nextSourceName } from '../controlSources';
import { openUrl } from '../platform';
import SingleChoice from './SingleChoice.vue';
import { displayMessage } from '../messages';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { connectionForm } from '../composables/connectionForm';
import ConnectionFields from './ConnectionFields.vue';

const props = defineProps<{ origin: { x: number; y: number; size: number }; ingress?: Ingress; defaultChatGPT?: boolean }>();
const emit = defineEmits<{ close: []; saved: [ingress: Ingress] }>();
const panelOpen = ref(true);
const hasPicker = !props.ingress && !props.defaultChatGPT;
const formOpen = ref(!hasPicker);
const formOrigin = ref(props.origin);
function formClosed() {
  if (!hasPicker) { panelClosed(); return; }
  if (savedIngress) panelOpen.value = false;
  else selected.value = '';
}
let savedIngress: Ingress | undefined;
function panelClosed() { savedIngress ? emit('saved', savedIngress) : emit('close'); }
const { refresh, status } = useConnector();
const formRef = ref<FormInst>();
const presetNames = Object.fromEntries(curatedSources.map(p => [p.id, p.displayName]));
const entry = ref(props.ingress);
const toolPolicy = ref<ToolPolicy>(props.ingress?.toolPolicy ?? 'all');
const providerReady = computed(() => {
  const config=entry.value?.config;
  switch(form.httpsProvider) {
    case 'custom': return true;
    case 'cloudflare': return form.cloudflareMode==='quick' || !!(form.cloudflareToken.trim() || config?.hasCloudflareToken);
    case 'ngrok': return !!(form.ngrokAuthtoken.trim() || config?.hasNgrokAuthtoken);
    case 'pinggy': return form.pinggyMode==='quick' || !!(form.pinggyToken.trim() || config?.hasPinggyToken);
    case 'localxpose': return !!(form.localxposeAccessToken.trim() || config?.hasLocalxposeAccessToken);
  }
});
const canSave = computed(() => !!name.value.trim() && new TextEncoder().encode(name.value).length<=120
  && (selected.value!=='custom' || /^[a-zA-Z0-9_-]+$/.test(source.value))
  && (form.connectionMode==='tunnel'
    ? /^tunnel_[a-zA-Z0-9_-]+$/.test(form.tunnelId) && !!(form.apiKey.trim() || entry.value?.config.hasApiKey)
    : providerReady.value
      && (form.httpsProvider!=='custom' || validDomain(form.domain))
      && (form.httpsProvider!=='cloudflare' || form.cloudflareMode!=='named' || validDomain(form.domain))
      && (form.httpsProvider!=='pinggy' || form.pinggyMode==='quick' || validDomain(form.domain))
      && (form.httpsProvider!=='localxpose' || validDomain(form.domain))
      && (form.httpsProvider!=='ngrok' || form.ngrokMode==='quick' || validDomain(form.domain))));
function configPayload() {
  const config:Record<string,unknown>={...form};
  delete config.connectionMode;
  delete config.domain;
  const fixed = form.connectionMode==='https' && (form.httpsProvider==='custom' || form.httpsProvider==='cloudflare' && form.cloudflareMode==='named' || form.httpsProvider==='ngrok' && form.ngrokMode==='named' || form.httpsProvider==='pinggy' && form.pinggyMode==='named' || form.httpsProvider==='localxpose' && form.localxposeMode==='named');
  const origin = fixed && form.domain.trim() ? `https://${form.domain.trim()}` : '';
  config.httpsUrl = origin ? `${origin}/mcp` : '';
  config.ngrokEndpoint = form.httpsProvider==='ngrok' ? origin : '';
  if(!entry.value) { delete config.httpsPort; delete config.httpsHost; }
  for(const key of ['apiKey','cloudflareToken','ngrokAuthtoken','pinggyToken','localxposeAccessToken']) if(!config[key]) delete config[key];
  return config;
}
const selected = ref(props.ingress ? (controlSource(props.ingress.controlSource)?.curated ? props.ingress.controlSource : 'custom') : props.defaultChatGPT ? 'chatgpt' : '');
const source = ref(props.ingress?.controlSource || '');
const name = ref(props.ingress?.name || nextSourceName(selected.value || 'chatgpt', status.value?.ingresses.map(i=>i.name) || []));
const preset = computed(() => controlSource(selected.value));
const form = reactive(connectionForm(props.ingress?.config));
if(!form.domain && props.ingress?.url && (form.httpsProvider==='custom' || form.httpsProvider==='cloudflare' && form.cloudflareMode==='named')) form.domain=new URL(props.ingress.url).hostname;
if (props.ingress) form.connectionMode=props.ingress.transport==='openai-tunnel' ? 'tunnel' : 'https';
else if (preset.value) form.connectionMode=preset.value.recommendedTransport==='openai-tunnel' ? 'tunnel' : 'https';
const auth = ref(props.ingress?.auth === 'oauth' ? 'oauth' : props.ingress?.auth === 'bearer' ? 'bearer' : 'none');
const working = ref(false), error = ref('');
function generateToken() {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return Array.from(bytes, byte => byte.toString(16).padStart(2,'0')).join('');
}
watch([()=>form.connectionMode,()=>form.httpsProvider,()=>form.cloudflareMode,()=>form.ngrokMode,()=>form.pinggyMode,()=>form.localxposeMode],()=>{
  form.domain='';
});
onUnmounted(() => { form.apiKey=''; form.cloudflareToken=''; form.ngrokAuthtoken=''; form.pinggyToken=''; form.localxposeAccessToken=''; });
const model = computed(() => ({...form, name:name.value, source:source.value}));
function choose(value: string, event: MouseEvent) {
  const button = event.currentTarget as HTMLElement;
  const rect = (button.querySelector('.platform-icon') || button).getBoundingClientRect();
  formOrigin.value = { x: rect.x, y: rect.y, size: rect.width };
  Object.assign(form, connectionForm());
  toolPolicy.value = 'all'; error.value = '';
  selected.value = value; source.value = value === 'custom' ? '' : value;
  name.value = value==='custom' ? '' : nextSourceName(value, status.value?.ingresses.map(i=>i.name) || []);
  form.connectionMode = preset.value?.recommendedTransport==='openai-tunnel' ? 'tunnel' : 'https';
  form.cloudflareMode = form.connectionMode==='tunnel' ? 'quick' : 'named';
  auth.value = preset.value?.httpsAuth ?? 'bearer';
  formOpen.value = true;
}
async function action(fn: () => Promise<void>) {
  working.value = true; error.value = '';
  try { await fn(); } catch(e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { await refresh().catch(() => {}); working.value = false; }
}
async function save() {
  if (working.value || !canSave.value) return;
  try { await formRef.value?.validate(); } catch { return; }
  await action(async () => {
    const bearer=form.connectionMode==='https' && auth.value==='bearer';
    const metadata={toolPolicy:toolPolicy.value,name:name.value.trim(),controlSource:selected.value==='custom' ? source.value : selected.value,auth:form.connectionMode==='tunnel' ? 'openai' : auth.value,
      ...(bearer && entry.value?.auth!=='bearer' ? {bearerToken:generateToken()} : {})};
    if(entry.value) await api(`ingress/${entry.value.id}/stop`,'POST');
    const saved=await api<Ingress>(entry.value ? `ingress/${entry.value.id}` : 'ingress',entry.value ? 'PUT':'POST',{...metadata,transport:form.connectionMode==='tunnel' ? 'openai-tunnel':'https',config:configPayload()});
    entry.value=saved;
    form.apiKey='';form.cloudflareToken='';form.ngrokAuthtoken=''; form.pinggyToken=''; form.localxposeAccessToken='';
    await refresh().catch(() => {});
    savedIngress=saved; formOpen.value=false;
  });
}
async function remove() { await action(async () => { await api(`ingress/${entry.value!.id}`, 'DELETE'); formOpen.value=false; }); }
</script>
<template>
  <ElasticPanel :continuation="!!savedIngress" v-if="hasPicker" :show="panelOpen" :origin="origin" :width="600" :title="t('addControlSource')" @close="panelOpen=false" @closed="panelClosed">
    <div class="source-choices"><NButton v-for="option in [...curatedSources.map(p=>p.id),'custom']" :key="option" text :aria-label="presetNames[option] || t('customControlSource')" :title="presetNames[option] || t('customControlSource')" @click="choose(option,$event)"><span class="source-choice"><SourceIcon :platform="option" :add="option==='custom'"/><span>{{presetNames[option] || t('customControlSource')}}</span></span></NButton></div>
  </ElasticPanel>
  <ElasticPanel :continuation="!!savedIngress" :show="formOpen" :origin="formOrigin" :width="600" :title="(entry ? t('editControlSource') : t('addControlSource')) + (selected ? ' · ' + (presetNames[selected] || t('customControlSource')) : '')" :busy="working" @close="formOpen=false" @closed="formClosed">

    <NForm v-if="selected" ref="formRef" :model="model" :disabled="working" label-placement="top" @submit.prevent="save()">
      <NAlert :show-icon="false" v-if="form.connectionMode==='https' && auth==='bearer' && preset && !preset.supportedAuth.includes('bearer')" type="warning">{{t('presetAuthMismatch')}}</NAlert>
      <NFormItem :label="t('sourceName')" path="name" :rule="required()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}"><template #label><span class="token-heading"><span>{{t('sourceName')}} <NText type="error">*</NText></span><NButton v-if="preset?.docs[0]" text type="primary" size="tiny" @click="openUrl(preset.docs[0])">{{t('officialSetup')}}</NButton></span></template><NInput v-model:value="name" :input-props="{'aria-label':t('sourceName')}"/></NFormItem>
      <NFormItem v-if="selected==='custom'" :label="t('controlSourceId')" path="source" :rule="[{...required()}, {pattern:/^[a-zA-Z0-9_-]+$/,message:t('validClientId'),trigger:'input'}]"><NInput :input-props="{'aria-label':t('controlSourceId')}" v-model:value="source" placeholder="my-client"/></NFormItem>
      <ConnectionFields :recommend-tunnel="selected==='chatgpt'" :form="form" :config="entry?.config" :disabled="working" :allow-tunnel="preset?.recommendedTransport==='openai-tunnel' || selected==='custom'">
        <template #domain>
          <NFormItem v-if="form.httpsProvider==='custom' || form.httpsProvider==='cloudflare' && form.cloudflareMode==='named'" :label="t('domain')" path="domain" :rule="domainRule()" :show-require-mark="false" :label-props="{for:''}" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
            <template #label><span class="token-heading"><span>{{t('domain')}} <NText type="error">*</NText></span><NButton v-if="form.httpsProvider==='cloudflare'" text type="primary" :aria-label="t('getDomain')" size="tiny" @click="openUrl('https://dash.cloudflare.com/?to=/:account/tunnels')">{{t('getDomain')}}</NButton></span></template>
            <div class="url-field">
              <NInput v-model:value="form.domain" :input-props="{'aria-label':t('domain')}" placeholder="mcp.example.com"/>
            </div>
          </NFormItem>
        </template>
      </ConnectionFields>
      <template v-if="form.connectionMode==='https'">
        <NFormItem :label="t('authentication')" path="auth"><SingleChoice v-model:value="auth" :label="t('authentication')" :disabled="working" :options="[{label:t('authNone'),value:'none'},{label:'Bearer',value:'bearer'},{label:'OAuth',value:'oauth'}]"/></NFormItem>

      </template>

      <ToolPolicyField v-model:value="toolPolicy" :disabled="working"/>
      <NAlert :show-icon="false" v-if="error" type="error" role="alert">{{displayMessage(error)}}</NAlert>
    </NForm>
    <template v-if="selected" #footer>
      <NButton v-if="entry" type="error" secondary :disabled="working" @click="remove">{{t('remove')}}</NButton>
      <span class="action-spacer"/>
      <NButton :disabled="working" @click="formOpen=false">{{t('cancel')}}</NButton>
      <NButton type="primary" :disabled="working || !canSave" :loading="working" @click="save()">{{t('save')}}</NButton>
    </template>
  </ElasticPanel>
</template>
<style scoped>.url-field{width:100%;display:grid;gap:10px}.token-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;width:100%}.source-choices{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px;padding:12px 0}.source-choices .n-button{height:116px;white-space:normal}.source-choice{display:flex;flex-direction:column;align-items:center;gap:8px;font-size:13px}</style>
