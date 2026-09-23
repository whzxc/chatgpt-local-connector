<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { NButton, NProgress, NAlert } from 'naive-ui';
import ElasticPanel from './ElasticPanel.vue';
import { useAppUpdate } from '../composables/useAppUpdate';

const { update, announcement, checking, phase, active, available, progress, downloaded, error, message, dialogOpen, dismissUpdate, install, cancel, skip, openDownloads } = useAppUpdate();
</script>

<template>
  <ElasticPanel :show="dialogOpen" :width="600" :title="available ? t('versionValueIsAvailable',{version:update?.version}) : t('appUpdates')" :busy="active" @close="dismissUpdate">
    <p v-if="checking" role="status">{{t('checkingLabel')}}</p>
    <p v-if="available" class="hint">{{t('installationRestartsTheAppAndRestoresThePrevious')}}</p>
    <p v-if="available && update?.notes" class="update-notes">{{update.notes}}</p>
    <p v-else-if="available" class="hint">{{t('aNewVersionIsReadyToDownloadAnd')}}</p>
    <div v-if="active" role="status" aria-live="polite"><p>{{phase==='installing' ? t('installingRestartingSoon') : progress===undefined ? t('downloadingValueMb',{size:(downloaded/1048576).toFixed(1)}) : t('downloadingValue',{percent:progress})}}</p><NProgress v-if="phase==='downloading'" type="line" :percentage="progress ?? 0" :processing="progress===undefined" :show-indicator="false" :aria-label="t('updateDownloadProgress')"/></div>
    <NAlert :show-icon="false" v-if="error" type="error">{{displayMessage(error)}}</NAlert><p v-if="message" role="status">{{displayMessage(message)}}</p>
    <template #footer><NButton v-if="available && !active" text @click="skip">{{t('skipThisVersion')}}</NButton><NButton v-if="error && !active" text @click="openDownloads">{{t('downloadManually')}}</NButton><span class="action-spacer"/><NButton v-if="!active" @click="dismissUpdate">{{t('later')}}</NButton><NButton v-if="phase==='downloading'" @click="cancel">{{t('cancelDownload')}}</NButton><NButton v-if="available" type="primary" :loading="active" :disabled="active || checking" @click="install()">{{phase==='installing' ? t('installing') : phase==='downloading' ? t('downloading') : error ? t('retryInstallation') : update?.ready ? t('installUpdate') : t('downloadAndInstall')}}</NButton></template>
  </ElasticPanel>
  <ElasticPanel :show="!!announcement" :width="600" :title="t('updatedToVersion', { version: announcement?.version })" @close="announcement=undefined">
    <p class="update-notes">{{announcement?.notes || t('updateCompleted')}}</p>
    <template #footer><NButton type="primary" @click="announcement=undefined">{{t('gotIt')}}</NButton></template>
  </ElasticPanel>
</template>
<style scoped>.update-notes{white-space:pre-wrap;overflow-wrap:anywhere;font-size:13px;line-height:1.8}</style>
