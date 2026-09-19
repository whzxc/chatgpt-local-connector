<script setup lang="ts">
import { useAppUpdate } from '../composables/useAppUpdate';
const { update, checking, phase, active, available, progress, downloaded, error, message, autoCheck, supported, check, install, cancel, skip, setAutoCheck, openDownloads } = useAppUpdate();
</script>
<template>
  <section class="settings-block app-update">
    <div class="card-head"><div><h2>应用更新</h2><p>更新完成后重启应用，并恢复更新前的连接状态。</p></div><button :disabled="!supported || checking || active" @click="check()">{{checking ? '正在检查…' : '检查更新'}}</button></div>
    <label v-if="supported" class="preference-row"><span><strong>自动检查更新</strong><small>有新版本时提醒，由你决定何时安装。</small></span><input type="checkbox" role="switch" :checked="autoCheck" @change="setAutoCheck(($event.target as HTMLInputElement).checked)" /></label>
    <p v-else class="hint">开发预览不检查或安装更新。</p>
    <template v-if="available">
      <p><strong>新版本 {{update?.version}}</strong></p>
      <pre v-if="update?.notes" class="update-notes">{{update.notes}}</pre>
      <div v-if="active" role="status" aria-live="polite">
        <p>{{phase === 'installing' ? '正在安装，即将重启…' : progress === undefined ? `正在下载 · ${(downloaded / 1048576).toFixed(1)} MB` : `正在下载 · ${progress}%`}}</p>
        <progress v-if="phase==='downloading'" :value="progress" max="100" aria-label="更新下载进度" />
      </div>
      <div class="actions"><button class="primary" :disabled="active || checking" @click="install">{{error ? '重试安装' : '下载并安装'}}</button><button v-if="phase==='downloading'" @click="cancel">取消下载</button><button v-if="!active" class="text-button" @click="skip">跳过此版本</button></div>
    </template>
    <p v-if="error" class="error-detail" role="alert">{{error}}</p>
    <p v-if="message" class="hint" role="status">{{message}}</p>
    <button class="text-button" @click="openDownloads">打开手动下载页 ↗</button>
  </section>
</template>
<style scoped>
.update-notes { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 220px; overflow-y: auto; font: inherit; font-size: 13px; line-height: 1.6; }
progress { width: 100%; accent-color: var(--accent, #315f51); }
</style>
