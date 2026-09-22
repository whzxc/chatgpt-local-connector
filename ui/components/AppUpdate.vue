<script setup lang="ts">
import { displayMessage } from '../messages';
import { NButton, NSwitch } from 'naive-ui';
import { t } from '../i18n';
import { useConnector } from '../composables/useConnector';
import { openUrl } from '../platform';
import SettingsGroup from './SettingsGroup.vue';
import SettingsRow from './SettingsRow.vue';
import { useAppUpdate } from '../composables/useAppUpdate';
const { update, checking, active, visible, autoDownload, setAutoDownload, error, message, autoCheck, supported, dialogOpen, showUpdate, check, setAutoCheck, openDownloads } = useAppUpdate();
const { status, notify } = useConnector();
const openRepository = () => openUrl('https://github.com/whzxc/chatgpt-local-connector').catch(cause => notify(String(cause), true));
</script>
<template>
  <SettingsGroup :title="t('aboutApp')">
    <SettingsRow title="Local Connector" :description="t('currentVersion')">
      <span>{{ status?.version ?? '—' }}</span>
    </SettingsRow>
    <SettingsRow :title="t('checkForANewVersion')" :description="supported ? t('theAppRestartsAfterUpdatingAndRestoresThe') : t('checkForAndInstallUpdatesInTheDesktop')">
      <NButton :disabled="!supported || checking || active" @click="check()">{{checking ? t('checkingLabel') : t('checkForUpdates')}}</NButton>
    </SettingsRow>
    <SettingsRow v-if="supported" :title="t('automaticallyCheckForUpdates')" :description="t('notifyMeOfNewVersionsIChooseWhen')" control-id="settings-auto-update">
      <NSwitch id="settings-auto-update" :aria-label="t('automaticallyCheckForUpdates')"  :value="autoCheck" @update:value="setAutoCheck" />
    </SettingsRow>
    <SettingsRow :title="t('automaticallyDownloadUpdates')" :description="t('automaticallyDownloadUpdatesHelp')" control-id="settings-auto-download">
      <NSwitch id="settings-auto-download" :aria-label="t('automaticallyDownloadUpdates')" :value="autoDownload" @update:value="setAutoDownload" />
    </SettingsRow>
    <SettingsRow v-if="visible" :title="t('newVersionValue', { version: update?.version })">
      <NButton @click="showUpdate">{{ t('viewUpdate') }}</NButton>
    </SettingsRow>
    <div v-if="!dialogOpen && (error || message)" class="update-details">
      <p v-if="error" class="error-detail" role="alert">{{displayMessage(error)}}</p>
      <p v-if="message" class="hint" role="status">{{displayMessage(message)}}</p>
    </div>
    <SettingsRow :title="t('manualDownload')">
      <NButton text type="primary" @click="openDownloads">{{ t('openDownloads') }}</NButton>
    </SettingsRow>
    <SettingsRow :title="t('openSourceRepository')" :description="t('mitLicense')">
      <NButton text type="primary" @click="openRepository">GitHub ↗</NButton>
    </SettingsRow>
  </SettingsGroup>
</template>
<style scoped>
.update-details { margin: 0 16px; padding: 16px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }
.update-details p { font-size: 12px; }
</style>
