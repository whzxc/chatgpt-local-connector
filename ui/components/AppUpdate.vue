<script setup lang="ts">
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import { useAppUpdate } from '../composables/useAppUpdate';
const { update, checking, phase, active, available, progress, downloaded, error, message, autoCheck, supported, check, install, cancel, skip, setAutoCheck, openDownloads } = useAppUpdate();
</script>
<template>
  <SettingsGroup title="应用更新">
    <SettingsRow title="检查新版本" :description="supported ? '更新后重启应用，并恢复之前的连接状态。' : '请在桌面应用中检查和安装更新。'">
      <button :disabled="!supported || checking || active" @click="check()">{{checking ? '正在检查…' : '检查更新'}}</button>
    </SettingsRow>
    <SettingsRow v-if="supported" title="自动检查更新" description="有新版本时提醒，由你决定何时安装。" control-id="settings-auto-update">
      <input id="settings-auto-update" class="settings-switch" type="checkbox" role="switch" :checked="autoCheck" @change="setAutoCheck(($event.target as HTMLInputElement).checked)" />
    </SettingsRow>
    <div v-if="available || error || message" class="update-details">
      <template v-if="available">
        <p class="update-title">新版本 {{update?.version}}</p>
        <pre v-if="update?.notes" class="update-notes">{{update.notes}}</pre>
        <div v-if="active" role="status" aria-live="polite">
          <p>{{phase === 'installing' ? '正在安装，即将重启…' : progress === undefined ? `正在下载 · ${(downloaded / 1048576).toFixed(1)} MB` : `正在下载 · ${progress}%`}}</p>
          <progress v-if="phase==='downloading'" :value="progress" max="100" aria-label="更新下载进度" />
        </div>
        <div class="actions"><button class="primary" :disabled="active || checking" @click="install">{{error ? '重试安装' : '下载并安装'}}</button><button v-if="phase==='downloading'" @click="cancel">取消下载</button><button v-if="!active" class="text-button" @click="skip">跳过此版本</button></div>
      </template>
      <p v-if="error" class="error-detail" role="alert">{{error}}</p>
      <p v-if="message" class="hint" role="status">{{message}}</p>
    </div>
    <SettingsRow title="手动下载">
      <button class="text-button" @click="openDownloads">打开下载页 ↗</button>
    </SettingsRow>
  </SettingsGroup>
</template>
<style scoped>
.update-details { margin: 0 16px; padding: 16px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }
.update-details p { font-size: 12px; }
.update-title { font-weight: 600; margin-bottom: 8px; }
.update-notes { white-space: pre-wrap; overflow-wrap: anywhere; max-height: 220px; overflow-y: auto; font: inherit; font-size: 13px; line-height: 1.6; }
progress { width: 100%; accent-color: var(--accent, #315f51); }
</style>
