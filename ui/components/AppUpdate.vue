<script setup lang="ts">
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import { useAppUpdate } from '../composables/useAppUpdate';
const { update, checking, active, available, error, message, autoCheck, supported, dialogOpen, showUpdate, check, setAutoCheck, openDownloads } = useAppUpdate();
</script>
<template>
  <SettingsGroup title="应用更新">
    <SettingsRow title="检查新版本" :description="supported ? '更新后重启应用，并恢复之前的连接状态。' : '请在桌面应用中检查和安装更新。'">
      <button :disabled="!supported || checking || active" @click="check()">{{checking ? '正在检查…' : '检查更新'}}</button>
    </SettingsRow>
    <SettingsRow v-if="supported" title="自动检查更新" description="有新版本时提醒，由你决定何时安装。" control-id="settings-auto-update">
      <input id="settings-auto-update" class="settings-switch" type="checkbox" role="switch" :checked="autoCheck" @change="setAutoCheck(($event.target as HTMLInputElement).checked)" />
    </SettingsRow>
    <SettingsRow v-if="available" :title="`新版本 ${update?.version}`">
      <button @click="showUpdate">查看更新</button>
    </SettingsRow>
    <div v-if="!dialogOpen && (error || message)" class="update-details">
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
</style>
