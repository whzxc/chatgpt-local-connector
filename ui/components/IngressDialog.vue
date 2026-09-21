<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, watch } from 'vue';
import { NForm, NFormItem, NInput, NButton, NAlert, type FormInst } from 'naive-ui';
import FormDialog from './FormDialog.vue';
import SourceIcon from './SourceIcon.vue';
import { required, validMcpUrl } from '../formRules';
import { t } from '../i18n';
import SingleChoice from './SingleChoice.vue';
import { displayMessage } from '../messages';
import { api, useConnector, type Ingress } from '../composables/useConnector';
import { connectionForm } from '../composables/connectionForm';
import ConnectionFields from './ConnectionFields.vue';
import CopyField from './CopyField.vue';
import { useClipboard } from '@vueuse/core';

const props = defineProps<{ ingress?: Ingress; defaultChatGPT?: boolean }>();
const emit = defineEmits<{ close: [] }>();
const { refresh, status } = useConnector();
const formRef = ref<FormInst>();
const presetNames: Record<string,string> = {chatgpt:'ChatGPT',notion:'Notion',slack:'Slack'};
const entry = ref(props.ingress);
const draft = ref<Ingress>();
const acquiring = ref(false);
const liveEntry = computed(() => draft.value || status.value?.ingresses.find(i=>i.id===entry.value?.id) || entry.value);
const urlReady = computed(() => validMcpUrl(form.httpsUrl));
const providerReady = computed(() => form.httpsProvider==='custom' || form.httpsProvider==='cloudflare' && form.cloudflareMode==='quick' || !!(form.httpsProvider==='ngrok' ? form.ngrokAuthtoken.trim() || entry.value?.config.hasNgrokAuthtoken : form.cloudflareToken.trim() || entry.value?.config.hasCloudflareToken));
const canSave = computed(() => !!name.value.trim() && name.value.length<=128
  && (selected.value!=='custom' || /^[a-zA-Z0-9_-]+$/.test(source.value) && !presetNames[source.value.toLowerCase()])
  && (form.connectionMode==='tunnel'
    ? /^tunnel_[a-zA-Z0-9_-]+$/.test(form.tunnelId) && !!(form.apiKey.trim() || entry.value?.config.hasApiKey)
    : providerReady.value && urlReady.value && (auth.value==='none' || !!token.value || entry.value?.auth==='bearer')));
function configPayload() {
  const config:Record<string,unknown>={...form};
  delete config.connectionMode;
  if(!entry.value) { delete config.httpsPort; delete config.httpsHost; }
  for(const key of ['apiKey','cloudflareToken','ngrokAuthtoken']) if(!config[key]) delete config[key];
  return config;
}
async function discardDraft() {
  const id=draft.value?.id; draft.value=undefined;
  if(id) await api(`ingress-drafts/${id}`,'DELETE').catch(()=>{});
}
async function obtainUrl() {
  if(working.value || !providerReady.value) return;
  await action(async()=>{
    form.httpsUrl=''; acquiring.value=true;
    if(entry.value) {
      if(entry.value.running) await api(`ingress/${entry.value.id}/stop`,'POST');
      entry.value=await api<Ingress>(`ingress/${entry.value.id}`,'PUT',{config:configPayload()});
      await api(`ingress/${entry.value.id}/start`,'POST');
    } else {
      await discardDraft();
      draft.value=await api<Ingress>('ingress-drafts','POST',{config:configPayload()});
    }
    await pollUrl();
  });
}
let polling=false;
async function pollUrl() {
  if(polling || disposed || !(draft.value || entry.value)) return;
  polling=true;
  const id=draft.value?.id || entry.value!.id, pending=!!draft.value;
  try {
    const next=await api<Ingress>(pending ? `ingress-drafts/${id}` : `ingress/${id}`);
    if(disposed || (pending ? draft.value?.id : entry.value?.id)!==id) return;
    if(pending) draft.value=next; else entry.value=next;
    if(form.connectionMode==='https' && form.httpsProvider!=='custom' && next.url) { form.httpsUrl=next.url; acquiring.value=false; }
  } catch(e) { if(!disposed) error.value=String(e); }
  finally {polling=false;}
}
const selected = ref(props.ingress ? (['chatgpt','notion','slack'].includes(props.ingress.controlSource) ? props.ingress.controlSource : 'custom') : props.defaultChatGPT ? 'chatgpt' : '');
const source = ref(props.ingress?.controlSource || '');
const name = ref(props.ingress?.name || presetNames[selected.value] || 'ChatGPT');
const form = reactive(connectionForm(props.ingress?.config));
if(props.ingress?.url) form.httpsUrl=props.ingress.url;
if (selected.value && selected.value!=='chatgpt') form.connectionMode='https';
const auth = ref(props.ingress?.auth === 'bearer' ? 'bearer' : 'none');
const token = ref(''), working = ref(false), error = ref('');
const readingKey = ref(false), deliveredToken = ref('');
const { copy:copyToken, copied:tokenCopied } = useClipboard({copiedDuring:1500});
function generateToken() {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  token.value = Array.from(bytes, byte => byte.toString(16).padStart(2,'0')).join('');
}
watch([auth, urlReady, () => form.connectionMode], () => {
  if (auth.value==='bearer' && urlReady.value && form.connectionMode==='https' && entry.value?.auth!=='bearer' && !token.value) generateToken();
}, {immediate:true});
let disposed = false;
let pollTimer: ReturnType<typeof setInterval>;
watch([()=>form.connectionMode,()=>form.httpsProvider,()=>form.cloudflareMode,()=>form.cloudflareToken,()=>form.ngrokAuthtoken],()=>{
  if(working.value) return;
  form.httpsUrl=''; token.value=''; acquiring.value=false; void discardDraft();
});
onUnmounted(() => { disposed=true; clearInterval(pollTimer); void discardDraft(); form.apiKey=''; token.value=''; deliveredToken.value=''; });
onMounted(async () => {
  pollTimer=setInterval(()=>{if(draft.value || acquiring.value) void pollUrl();},2000);
  if (!entry.value?.config.hasApiKey || selected.value!=='chatgpt') return;
  readingKey.value=true;
  try {
    const credentials = await api<{apiKey:string}>(`ingress/${entry.value.id}/credentials`, 'POST');
    if (!disposed) form.apiKey=credentials.apiKey || '';
  } catch { if (!disposed) error.value=t('credentialReadFailed'); }
  finally { if(!disposed) readingKey.value=false; }
});
const model = computed(() => ({...form, name:name.value, source:source.value, token:token.value}));
function choose(value: string) {
  selected.value = value; source.value = value === 'custom' ? '' : value;
  name.value = ({ chatgpt:'ChatGPT', notion:'Notion', slack:'Slack', custom:'' })[value] || '';
  form.connectionMode = value === 'chatgpt' ? 'tunnel' : 'https';
  form.cloudflareMode = value === 'chatgpt' ? 'quick' : 'named';
  auth.value = value === 'chatgpt' ? 'none' : 'bearer';
}
async function action(fn: () => Promise<void>) {
  working.value = true; error.value = '';
  try { await fn(); } catch(e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { await refresh().catch(() => {}); working.value = false; }
}
async function save() {
  if (working.value || readingKey.value || !canSave.value) return;
  try { await formRef.value?.validate(); } catch { return; }
  await action(async () => {
    const bearer=form.connectionMode==='https' && auth.value==='bearer';
    const metadata={name:name.value.trim(),controlSource:selected.value==='custom' ? source.value : selected.value,auth:form.connectionMode==='tunnel' ? 'openai' : auth.value,...(bearer && token.value ? {bearerToken:token.value} : {})};
    let saved:Ingress;
    if(draft.value) {
      saved=await api<Ingress>(`ingress-drafts/${draft.value.id}/commit`,'POST',{...metadata,url:form.httpsUrl});
      draft.value=undefined;
    } else {
      const restart=!!entry.value;
      if(entry.value?.running) await api(`ingress/${entry.value.id}/stop`,'POST');
      saved=await api<Ingress>(entry.value ? `ingress/${entry.value.id}` : 'ingress',entry.value ? 'PUT':'POST',{...metadata,transport:form.connectionMode==='tunnel' ? 'openai-tunnel':'https',config:configPayload()});
      if(restart) await api(`ingress/${saved.id}/start`,'POST');
    }
    entry.value=saved;
    if(bearer && token.value) deliveredToken.value=token.value;
    form.apiKey='';form.cloudflareToken='';form.ngrokAuthtoken='';token.value='';
    if(!deliveredToken.value) emit('close');
  });
}
async function stop() { await action(async () => { await api(`ingress/${entry.value!.id}/stop`, 'POST'); entry.value = await api<Ingress>(`ingress/${entry.value!.id}`); }); }
async function remove() { await action(async () => { await api(`ingress/${entry.value!.id}`, 'DELETE'); emit('close'); }); }
</script>
<template>
  <FormDialog :show="true" :title="(entry ? t('editControlSource') : t('addControlSource')) + (selected ? ' · ' + (presetNames[selected] || t('customControlSource')) : '')" :busy="working" @close="emit('close')">
    <div v-if="deliveredToken" class="token-field"><NInput :value="deliveredToken" readonly type="password" show-password-on="click" :input-props="{'aria-label':'Bearer token'}"/><NButton @click="copyToken(deliveredToken)">{{tokenCopied ? t('copied') : t('copy')}}</NButton><p class="token-help">{{t('savedBearerHelp')}}</p><NAlert v-if="error" type="error">{{displayMessage(error)}}</NAlert></div>
    <div v-else-if="!selected" class="source-choices"><NButton v-for="option in ['chatgpt','notion','slack','custom']" :key="option" text :aria-label="presetNames[option] || t('customControlSource')" :title="presetNames[option] || t('customControlSource')" @click="choose(option)"><SourceIcon :platform="option" :add="option==='custom'"/></NButton></div>
    <NForm v-else ref="formRef" :model="model" :disabled="working || readingKey" label-placement="top" @submit.prevent="save()">
      <NFormItem :label="t('sourceName')" path="name" :rule="required()"><NInput v-model:value="name" :input-props="{'aria-label':t('sourceName')}"/></NFormItem>
      <NFormItem v-if="selected==='custom'" :label="t('controlSourceId')" path="source" :rule="[{...required()}, {pattern:/^[a-zA-Z0-9_-]+$/,message:t('validClientId'),trigger:'input'}, {validator:() => !presetNames[source.trim().toLowerCase()],message:t('choosePresetSource'),trigger:'input'}]"><NInput :input-props="{'aria-label':t('controlSourceId')}" v-model:value="source" placeholder="my-client"/></NFormItem>
      <ConnectionFields :form="form" :config="entry?.config" :disabled="working || readingKey" :allow-tunnel="selected==='chatgpt'">
        <template #mcp-url>
          <NFormItem label="MCP URL" path="httpsUrl" :validation-status="form.httpsUrl && !urlReady ? 'error' : undefined" :feedback="form.httpsUrl && !urlReady ? t('validHttpsUrl') : undefined">
            <NInput v-if="form.httpsProvider==='custom'" v-model:value="form.httpsUrl" :input-props="{'aria-label':'MCP URL'}" placeholder="https://connector.example.com/mcp"/>
            <div v-else class="url-field">
              <SingleChoice v-if="(liveEntry?.discoveredUrls?.length || 0)>1" v-model:value="form.httpsUrl" label="MCP URL" :disabled="working" :options="(liveEntry?.discoveredUrls || []).map(value=>({value,label:value}))"/>
              <CopyField v-else-if="urlReady" :value="form.httpsUrl" label="MCP URL"/>
              <NButton v-else :disabled="!providerReady || working" :loading="working || liveEntry?.state==='starting'" @click="obtainUrl">{{t('obtainMcpUrl')}}</NButton>
              <NAlert v-if="liveEntry?.routeNotice" type="info">{{liveEntry.routeNotice}}</NAlert>
              <CopyField v-if="draft && form.cloudflareMode==='named' && !urlReady" :value="`http://127.0.0.1:${draft.config.httpsPort}`" :label="t('httpReverseProxyTarget')"/>
              <NAlert v-if="liveEntry?.error" type="error">{{displayMessage(liveEntry.error)}}</NAlert>
            </div>
          </NFormItem>
        </template>
      </ConnectionFields>
      <template v-if="form.connectionMode==='https' && urlReady">
        <NFormItem :label="t('authentication')" path="auth"><SingleChoice v-model:value="auth" :label="t('authentication')" :disabled="working || readingKey" :options="[{label:t('authNone'),value:'none'},{label:'Bearer',value:'bearer'}]"/></NFormItem>
        <NFormItem v-if="auth==='bearer'" path="token" :show-require-mark="false" :rule="entry?.auth==='bearer' ? undefined : required()" :label-style="{width:'100%',display:'grid',gridTemplateColumns:'minmax(0,1fr)'}">
          <template #label><span class="token-heading"><span>Bearer token<span v-if="entry?.auth!=='bearer'"> *</span></span><NButton text type="primary" size="tiny" :aria-label="t('generateNewToken')" :disabled="working || readingKey" @click="generateToken">{{t('generateNewToken')}}</NButton></span></template>
          <div class="token-field"><NInput :value="token" readonly type="password" show-password-on="click" :input-props="{autocomplete:'off','aria-label':'Bearer token'}" :placeholder="t('existingBearerPreserved')"/><NButton :disabled="!token || working" @click="copyToken(token)">{{tokenCopied ? t('copied') : t('copy')}}</NButton><p class="token-help">{{token ? t('generatedBearerHelp') : t('existingBearerHelp')}}</p></div>
        </NFormItem>
      </template>

      <NAlert v-if="error" type="error" role="alert">{{displayMessage(error)}}</NAlert>
    </NForm>
    <template v-if="selected" #footer>
      <NButton v-if="deliveredToken" type="primary" :disabled="working" @click="emit('close')">{{t('close')}}</NButton>
      <template v-else>
      <NButton v-if="entry" type="error" secondary :disabled="working || readingKey" @click="remove">{{t('remove')}}</NButton>
      <span class="action-spacer"/>
      <NButton v-if="entry" :disabled="working || readingKey || !entry.running" @click="stop">{{t('disconnect')}}</NButton>
      <NButton type="primary" :disabled="readingKey || working || !canSave" :loading="working" @click="save()">{{entry ? t('saveAndReconnect') : t('save')}}</NButton>
      </template>
    </template>
  </FormDialog>
</template>
<style scoped>.url-field{width:100%;display:grid;gap:10px}.token-heading{display:flex;align-items:center;justify-content:space-between;gap:16px;width:100%}.token-field{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:8px;width:100%}.token-help{grid-column:1/-1;margin:0;color:var(--muted);font-size:12px;line-height:1.5}.source-choices{display:flex;justify-content:space-evenly;gap:16px;padding:20px 0}.source-choices .n-button{width:72px;height:72px;padding:14px}</style>
