import { t } from '../i18n';
import { computed, ref } from 'vue';
import { isDesktop, openUrl } from '../platform';
const releaseUrl = 'https://github.com/whzxc/chatgpt-local-connector/releases/latest';
type Update = { available: boolean; version?: string; notes?: string; date?: string; restarting?: boolean; ready?: boolean };
const update = ref<Update>();
const checking = ref(false);
const phase = ref<'idle' | 'downloading' | 'installing'>('idle');
const downloaded = ref(0);
const total = ref<number>();
const error = ref('');
const message = ref('');
const dialogOpen = ref(false);
const announcement = ref<Update>();
const autoDownload = ref(localStorage.getItem('update-auto-download') !== 'off');
let dismissedVersion = '';
let directRequested = false;
const autoCheck = ref(localStorage.getItem('update-auto-check') !== 'off');
const skipped = ref(localStorage.getItem('update-skipped-version') || '');
const supported = isDesktop;
const active = computed(() => phase.value !== 'idle');
const available = computed(() => update.value?.available && update.value.version !== skipped.value);
const visible = computed(() => available.value && (!autoDownload.value || update.value?.ready));
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
    if (available.value && autoDownload.value && !update.value.ready) await transfer(false);
    if (visible.value && (manual || update.value?.version !== dismissedVersion)) showUpdate();
    if (manual && !update.value.available) message.value = t('youAreUpToDate');
  } catch (cause) {
    if (manual) error.value = t('updateCheckFailedValueRetryOrVisitThe', { error: String(cause) });
  } finally { checking.value = false; }
}
async function transfer(installing: boolean, direct = false) {
  if (!update.value?.version || active.value) return;
  error.value = ''; message.value = ''; downloaded.value = 0; total.value = undefined;
  phase.value = installing && update.value.ready ? 'installing' : 'downloading';
  let unlisten: (() => void) | undefined;
  let restarting = false;
  try {
    const [{ invoke }, { listen }] = await Promise.all([import('@tauri-apps/api/core'), import('@tauri-apps/api/event')]);
    unlisten = await listen<{ downloaded?: number; total?: number; phase: 'downloading' | 'installing' }>('update-progress', event => {
      phase.value = event.payload.phase;
      if (event.payload.downloaded !== undefined) downloaded.value = event.payload.downloaded;
      if (event.payload.total !== undefined) total.value = event.payload.total;
    });
    if (installing) {
      if (direct) localStorage.setItem('update-announcement', JSON.stringify(update.value));
      else localStorage.removeItem('update-announcement');
    }
    const result = await invoke<Update>(installing ? 'install_update' : 'download_update', { version: update.value.version });
    if (result.restarting) { restarting = true; phase.value = 'installing'; return; }
    update.value = result;
    if (installing) localStorage.removeItem('update-announcement');
    if (!result.available) { update.value = result; message.value = t('youAreUpToDate'); }
  } catch (cause) {
    if (installing) {
      localStorage.removeItem('update-announcement');
      if (update.value) update.value.ready = false;
    }
    if (direct) showUpdate();
    if (String(cause) === 'UPDATE_CANCELLED') message.value = t('downloadCancelledTheCurrentConnectionIsUnchanged');
    else error.value = t('updateFailedValue', { error: String(cause) });
  } finally { unlisten?.(); if (!restarting) phase.value = 'idle'; }
}
const install = () => transfer(true, directRequested);
const installDirect = () => { directRequested = true; return transfer(true, true); };
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
  directRequested = false;
}
function setAutoCheck(value: boolean) {
  autoCheck.value = value;
  localStorage.setItem('update-auto-check', value ? 'on' : 'off');
}
function setAutoDownload(value: boolean) {
  autoDownload.value = value;
  localStorage.setItem('update-auto-download', value ? 'on' : 'off');
  if (value && !active.value) { dialogOpen.value = false; void check(false); }
}
async function restoreAnnouncement() {
  try {
    const saved = localStorage.getItem('update-announcement');
    if (!saved) return;
    const pending = JSON.parse(saved) as Update;
    const { getVersion } = await import('@tauri-apps/api/app');
    if (pending.version === await getVersion()) announcement.value = pending;
    localStorage.removeItem('update-announcement');
  } catch { localStorage.removeItem('update-announcement'); }
}
export function startUpdateChecks() {
  if (!supported) return () => {};
  void restoreAnnouncement();
  let lastAttempt = 0;
  const tick = () => {
    if (!autoCheck.value) return;
    const previous = Math.max(lastAttempt, Number(localStorage.getItem('update-last-check') || 0));
    if (Date.now() - previous < 60 * 60 * 1000) return;
    lastAttempt = Date.now(); void check(false);
  };
  const first = setTimeout(tick, 5000);
  const timer = setInterval(tick, 60_000);
  return () => { clearTimeout(first); clearInterval(timer); };
}
async function openDownloads() {
  try { await openUrl(releaseUrl); } catch (cause) { error.value = t('unableToOpenDownloadsValue', { error: String(cause) }); }
}
export const useAppUpdate = () => ({ update, visible, autoDownload, setAutoDownload, announcement, installDirect, checking, phase, active, available, progress, downloaded, error, message, autoCheck, supported, dialogOpen, showUpdate, dismissUpdate, check, install, cancel, skip, setAutoCheck, openDownloads });
