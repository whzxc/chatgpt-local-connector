<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { NAlert, NButton, NCard, NEmpty, NFormItem, NInputOtp, NSpin, NTooltip } from 'naive-ui';
import { CircleHelp } from '@lucide/vue';
import { api } from '../composables/useConnector';
import { t } from '../i18n';
const props = defineProps<{ ingressId: string; running: boolean; disabled?: boolean }>();
const emit = defineEmits<{ busy: [value: boolean] }>();
interface Pending { id: string; clientName: string; redirectUri: string; comparison: string; status: 'pending' | 'waiting' | 'expired'; }
interface Grant { id: string; clientName: string; }
const grants = ref<Grant[]>([]), pending = ref<Pending[]>([]), codes = ref<Record<string, string[]>>({});
const codeErrors = ref<Record<string, string>>({});
const error = ref(''), busy = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined, disposed = false, revision = 0;
async function refresh(afterAction = false) {
  if (busy.value && !afterAction) return;
  if (!props.running) { grants.value = []; pending.value = []; codes.value = {}; codeErrors.value = {}; error.value = ''; return; }
  const version = revision;
  try {
    const result = await api<{ grants: Grant[]; pending: Pending[] }>(`ingress/${props.ingressId}/oauth`);
    if (disposed || version !== revision || !props.running) return;
    grants.value = result.grants; pending.value = result.pending; error.value = '';
    codes.value = Object.fromEntries(result.pending.map(p => [p.id, codes.value[p.id] || []]));
    codeErrors.value = Object.fromEntries(result.pending.map(p => [p.id, codeErrors.value[p.id] || '']));
  } catch (e) { if (!disposed && version === revision) error.value = e instanceof Error ? e.message : String(e); }
}
async function poll() {
  await refresh();
  if (!disposed) timer = setTimeout(poll, 3000);
}
async function action(route: string, body: object) {
  if (busy.value || props.disabled || !props.running) return;
  revision++; busy.value = true; emit('busy', true); error.value = '';
  try {
    await api(`ingress/${props.ingressId}/oauth/${route}`, 'POST', body);
    await refresh(true);
    return true;
  } catch (e) { if (!disposed) error.value = e instanceof Error ? e.message : String(e); return false; }
  finally { busy.value = false; emit('busy', false); }
}
function sourceDomain(request: Pending) {
  try { return new URL(request.redirectUri).hostname; } catch { return ''; }
}
function updateCode(id: string, value: string[]) {
  codes.value[id] = value.map(char => char.toUpperCase());
  codeErrors.value[id] = '';
}
async function approve(request: Pending, value: string[]) {
  if (busy.value || props.disabled || !props.running) return;
  const comparison = value.join('').toUpperCase();
  if (comparison !== request.comparison) {
    codeErrors.value[request.id] = t('oauthCodeMismatch');
    return;
  }
  request.status = 'waiting';
  if (!await action('decision', { id: request.id, allow: true, comparison })) request.status = 'pending';
}
function deny(request: Pending) {
  void action('decision', { id: request.id, allow: false, comparison: request.comparison });
}
onMounted(() => { void poll(); });
onUnmounted(() => { disposed = true; clearTimeout(timer); emit('busy', false); });
</script>
<template>
  <section class="oauth-grants">
    <h3 class="oauth-heading"><span>OAuth</span><NTooltip><template #trigger><NButton text class="oauth-help-button" :aria-label="t('oauthConnectHint')"><template #icon><CircleHelp :size="12"/></template></NButton></template><span class="oauth-help">{{t('oauthConnectHint')}}</span></NTooltip></h3>
    <NCard v-for="request in pending" :key="request.id" class="oauth-request" size="small">
      <template #header>
        <div class="oauth-request-title"><span>{{request.clientName}}</span><small>{{sourceDomain(request)}}</small></div>
      </template>
      <template #header-extra><NButton size="small" type="error" :disabled="busy || disabled || !running" @click="deny(request)">{{t('oauthDeny')}}</NButton></template>
      <div class="oauth-code-entry">
      <NSpin v-if="request.status === 'waiting'" size="small"/>
      <NAlert v-else-if="request.status === 'expired'" type="error" :show-icon="false">{{t('oauthWaitExpired')}}</NAlert>
      <NFormItem v-else :show-label="false" :show-feedback="!!codeErrors[request.id]" :feedback="codeErrors[request.id]" :validation-status="codeErrors[request.id] ? 'error' : undefined">
        <NInputOtp :value="codes[request.id]" :length="8" :gap="8" :disabled="busy || disabled || !running" :allow-input="char => /^[a-z0-9]$/i.test(char)" :aria-label="t('oauthComparison')" @update:value="updateCode(request.id, $event)" @finish="approve(request, $event)"/>
      </NFormItem>
      <p class="oauth-code-caption">{{request.status === 'waiting' ? t('oauthWaiting') : request.status === 'expired' ? '' : t('oauthEnterCode')}}</p>
      </div>
    </NCard>
    <div v-for="grant in grants" :key="grant.id" class="oauth-grant"><span>{{grant.clientName}}</span><NButton size="small" :disabled="busy || disabled" @click="action('revoke', { id: grant.id })">{{t('oauthRevoke')}}</NButton></div>
    <NEmpty v-if="!grants.length && !pending.length && !error" :description="t('oauthNoGrants')"/>
    <NAlert v-if="error" :show-icon="false" type="error">{{error}}</NAlert>
  </section>
</template>
<style scoped>
.oauth-grants{width:100%;margin-bottom:16px}.oauth-grants h3{font-size:13px;font-weight:500}.oauth-heading{display:flex;align-items:center;gap:6px}.oauth-help-button{width:16px;height:16px}.oauth-help{display:block;max-width:320px}.oauth-grant{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:8px 0}.oauth-grant span{min-width:0;overflow-wrap:anywhere}.oauth-grant small{display:block;color:var(--muted)}
.oauth-code-entry{display:flex;flex-direction:column;align-items:center;padding:14px 0 18px;overflow-x:auto}.oauth-code-caption{margin:12px 0 0;color:var(--muted);font-size:14px;text-align:center}.oauth-request{margin-bottom:12px}.oauth-request-title{display:flex;align-items:baseline;flex-wrap:wrap;gap:4px 8px;padding-right:12px;font-size:14px;overflow-wrap:anywhere}.oauth-request-title small{font-size:12px;font-weight:400;color:var(--muted)}
</style>
