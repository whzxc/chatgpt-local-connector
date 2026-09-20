import { computed, ref } from 'vue';
import { isDesktop, openUrl } from '../platform';
const releaseUrl = 'https://github.com/whzxc/chatgpt-local-connector/releases/latest';
type Update = { available: boolean; version?: string; notes?: string; date?: string; restarting?: boolean };
const update = ref<Update>();
const checking = ref(false);
const phase = ref<'idle' | 'downloading' | 'installing'>('idle');
const downloaded = ref(0);
const total = ref<number>();
const error = ref('');
const message = ref('');
const dialogOpen = ref(false);
let dismissedVersion = '';
const autoCheck = ref(localStorage.getItem('update-auto-check') !== 'off');
const skipped = ref(localStorage.getItem('update-skipped-version') || '');
const supported = isDesktop;
const active = computed(() => phase.value !== 'idle');
const available = computed(() => update.value?.available && update.value.version !== skipped.value);
const progress = computed(() => total.value ? Math.min(100, Math.round(downloaded.value / total.value * 100)) : undefined);
async function check(manual = true) {
  if (!supported || checking.value || active.value) return;
  checking.value = true;
  if (manual) { error.value = ''; message.value = ''; skipped.value = ''; localStorage.removeItem('update-skipped-version'); }
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    update.value = await invoke<Update>('check_update');
    error.value = ''; message.value = '';
    localStorage.setItem('update-last-check', String(Date.now()));
    if (available.value && (manual || update.value.version !== dismissedVersion)) showUpdate();
    if (manual && !update.value.available) message.value = '已是最新版本。';
  } catch (cause) {
    if (manual) error.value = `检查更新失败：${String(cause)}。可重试或前往下载页。`;
  } finally { checking.value = false; }
}
async function install() {
  if (!update.value?.version || active.value) return;
  error.value = ''; message.value = ''; downloaded.value = 0; total.value = undefined;
  phase.value = 'downloading';
  let unlisten: (() => void) | undefined;
  let restarting = false;
  try {
    const [{ invoke }, { listen }] = await Promise.all([import('@tauri-apps/api/core'), import('@tauri-apps/api/event')]);
    unlisten = await listen<{ downloaded?: number; total?: number; phase: 'downloading' | 'installing' }>('update-progress', event => {
      phase.value = event.payload.phase;
      if (event.payload.downloaded !== undefined) downloaded.value = event.payload.downloaded;
      if (event.payload.total !== undefined) total.value = event.payload.total;
    });
    const result = await invoke<Update>('install_update', { version: update.value.version });
    if (result.restarting) { restarting = true; phase.value = 'installing'; return; }
    if (!result.available) { update.value = result; message.value = '已是最新版本。'; }
  } catch (cause) {
    if (String(cause) === 'UPDATE_CANCELLED') message.value = '已取消下载，当前连接未改变。';
    else error.value = `更新失败：${String(cause)}`;
  } finally { unlisten?.(); if (!restarting) phase.value = 'idle'; }
}
async function cancel() {
  try { const { invoke } = await import('@tauri-apps/api/core'); await invoke('cancel_update'); } catch (cause) { error.value = String(cause); }
}
function skip() {
  if (active.value) return;
  skipped.value = update.value?.version || '';
  localStorage.setItem('update-skipped-version', skipped.value);
  dismissUpdate();
}
function showUpdate() { dialogOpen.value = true; }
function dismissUpdate() {
  if (active.value) return;
  dismissedVersion = update.value?.version || '';
  dialogOpen.value = false;
}
function setAutoCheck(value: boolean) {
  autoCheck.value = value;
  localStorage.setItem('update-auto-check', value ? 'on' : 'off');
}
export function startUpdateChecks() {
  if (!supported) return () => {};
  let lastAttempt = 0;
  const tick = () => {
    if (document.hidden || !autoCheck.value) return;
    const previous = Math.max(lastAttempt, Number(localStorage.getItem('update-last-check') || 0));
    if (Date.now() - previous < 6 * 60 * 60 * 1000) return;
    lastAttempt = Date.now(); void check(false);
  };
  const first = setTimeout(tick, 5000);
  const timer = setInterval(tick, 60_000);
  return () => { clearTimeout(first); clearInterval(timer); };
}
async function openDownloads() {
  try { await openUrl(releaseUrl); } catch (cause) { error.value = `无法打开下载页：${String(cause)}`; }
}
export const useAppUpdate = () => ({ update, checking, phase, active, available, progress, downloaded, error, message, autoCheck, supported, dialogOpen, showUpdate, dismissUpdate, check, install, cancel, skip, setAutoCheck, openDownloads });
