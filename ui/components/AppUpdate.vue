<script setup lang="ts">
import { displayMessage } from '../messages';
import { NButton, NSwitch } from 'naive-ui';
import { t } from '../i18n';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import { useAppUpdate } from '../composables/useAppUpdate';
const { update, checking, active, available, error, message, autoCheck, supported, dialogOpen, showUpdate, check, setAutoCheck, openDownloads } = useAppUpdate();
</script>
<template>
  <SettingsGroup :title="t('appUpdates')">
    <SettingsRow :title="t('checkForANewVersion')" :description="supported ? t('theAppRestartsAfterUpdatingAndRestoresThe') : t('checkForAndInstallUpdatesInTheDesktop')">
      <NButton :disabled="!supported || checking || active" @click="check()">{{checking ? t('checkingLabel') : t('checkForUpdates')}}</NButton>
    </SettingsRow>
    <SettingsRow v-if="supported" :title="t('automaticallyCheckForUpdates')" :description="t('notifyMeOfNewVersionsIChooseWhen')" control-id="settings-auto-update">
      <NSwitch id="settings-auto-update" :aria-label="t('automaticallyCheckForUpdates')"  :value="autoCheck" @update:value="setAutoCheck" />
    </SettingsRow>
    <SettingsRow v-if="available" :title="t('newVersionValue', { version: update?.version })">
      <NButton @click="showUpdate">{{ t('viewUpdate') }}</NButton>
    </SettingsRow>
    <div v-if="!dialogOpen && (error || message)" class="update-details">
      <p v-if="error" class="error-detail" role="alert">{{displayMessage(error)}}</p>
      <p v-if="message" class="hint" role="status">{{displayMessage(message)}}</p>
    </div>
    <SettingsRow :title="t('manualDownload')">
      <NButton text type="primary" @click="openDownloads">{{ t('openDownloads') }}</NButton>
    </SettingsRow>
  </SettingsGroup>
</template>
<style scoped>
.update-details { margin: 0 16px; padding: 16px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }
.update-details p { font-size: 12px; }
</style>
