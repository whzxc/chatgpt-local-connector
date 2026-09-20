<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from 'vue';
import { Download, X } from '@lucide/vue';
import { useAppUpdate } from '../composables/useAppUpdate';

const { update, checking, phase, active, available, progress, downloaded, error, message, dialogOpen, dismissUpdate, install, cancel, skip, openDownloads } = useAppUpdate();
const dialog = ref<HTMLDialogElement>();
watch([dialogOpen, dialog], () => {
  if (dialogOpen.value && !dialog.value?.open) dialog.value?.showModal();
  else if (!dialogOpen.value && dialog.value?.open) dialog.value.close();
}, { flush: 'post' });
onBeforeUnmount(() => dialog.value?.close());
</script>

<template>
  <Teleport to="body">
    <dialog ref="dialog" class="app-update-dialog" aria-labelledby="app-update-title" aria-describedby="app-update-description" @cancel.prevent="dismissUpdate" @close="dismissUpdate">
      <header class="update-heading">
        <div class="update-icon"><Download aria-hidden="true"/></div>
        <div><h2 id="app-update-title">{{available ? `发现新版本 ${update?.version}` : '应用更新'}}</h2><p id="app-update-description">安装后将重启应用，并恢复之前的连接状态。</p></div>
        <button class="ghost dismiss" aria-label="稍后提醒" :disabled="active" @click="dismissUpdate"><X aria-hidden="true"/></button>
      </header>
      <div class="update-body">
        <p v-if="available && update?.notes" class="update-notes">{{update.notes}}</p>
        <p v-else-if="available" class="hint">新版本已准备好，可以下载并安装。</p>
        <div v-if="active" role="status" aria-live="polite">
          <p>{{phase === 'installing' ? '正在安装，即将重启…' : progress === undefined ? `正在下载 · ${(downloaded / 1048576).toFixed(1)} MB` : `正在下载 · ${progress}%`}}</p>
          <progress v-if="phase==='downloading'" :value="progress" max="100" aria-label="更新下载进度"/>
        </div>
        <p v-if="error" class="error-detail" role="alert">{{error}}</p>
        <p v-if="message" class="hint" role="status">{{message}}</p>
      </div>
      <footer>
        <button v-if="available && !active" class="text-button" @click="skip">跳过此版本</button>
        <button v-if="error && !active" class="text-button" @click="openDownloads">手动下载 ↗</button>
        <div class="update-actions">
          <button v-if="!active" autofocus @click="dismissUpdate">稍后</button>
          <button v-if="phase==='downloading'" @click="cancel">取消下载</button>
          <button v-if="available" class="primary" :disabled="active || checking" @click="install">{{phase==='installing' ? '正在安装…' : phase==='downloading' ? '正在下载…' : error ? '重试安装' : '下载并安装'}}</button>
        </div>
      </footer>
    </dialog>
  </Teleport>
</template>

<style scoped>
.app-update-dialog { width: min(580px, calc(100vw - 40px)); max-height: calc(100dvh - 48px); padding: 0; border: 1px solid var(--line); border-radius: 18px; background: var(--surface); color: var(--ink); box-shadow: 0 24px 80px #0003; }
.app-update-dialog[open] { display: flex; flex-direction: column; }
.app-update-dialog::backdrop { background: #14251f66; backdrop-filter: blur(4px); }
.update-heading { display: flex; flex-shrink: 0; align-items: flex-start; gap: 14px; padding: 24px 24px 20px; }
.update-icon { display: grid; place-items: center; flex-shrink: 0; width: 42px; height: 42px; border-radius: 12px; background: var(--soft); color: var(--green); }
h2 { margin: 0 0 6px; font-size: 20px; }
.update-heading p { margin: 0; font-size: 12px; color: var(--muted); }
.dismiss { margin-left: auto; padding: 4px; flex-shrink: 0; }
.dismiss svg { width: 18px; height: 18px; }
.update-body { min-height: 0; overflow-y: auto; padding: 0 24px 20px; }
.update-notes { white-space: pre-wrap; overflow-wrap: anywhere; font-size: 13px; line-height: 1.8; margin: 0; }
progress { width: 100%; accent-color: var(--green); }
footer { display: flex; flex-shrink: 0; align-items: center; flex-wrap: wrap; gap: 12px; padding: 18px 24px; border-top: 1px solid var(--line); }
.update-actions { display: flex; gap: 10px; margin-left: auto; }
</style>
