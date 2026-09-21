<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { NButton, NProgress, NAlert } from 'naive-ui';
import FormDialog from './FormDialog.vue';
import { useAppUpdate } from '../composables/useAppUpdate';

const { update, checking, phase, active, available, progress, downloaded, error, message, dialogOpen, dismissUpdate, install, cancel, skip, openDownloads } = useAppUpdate();
</script>

<template>
  <FormDialog :show="dialogOpen" :title="available ? t('versionValueIsAvailable',{version:update?.version}) : t('appUpdates')" :busy="active" @close="dismissUpdate">
    <p class="hint">{{t('installationRestartsTheAppAndRestoresThePrevious')}}</p>
    <p v-if="available && update?.notes" class="update-notes">{{update.notes}}</p>
    <p v-else-if="available" class="hint">{{t('aNewVersionIsReadyToDownloadAnd')}}</p>
    <div v-if="active" role="status" aria-live="polite"><p>{{phase==='installing' ? t('installingRestartingSoon') : progress===undefined ? t('downloadingValueMb',{size:(downloaded/1048576).toFixed(1)}) : t('downloadingValue',{percent:progress})}}</p><NProgress v-if="phase==='downloading'" type="line" :percentage="progress ?? 0" :processing="progress===undefined" :show-indicator="false" :aria-label="t('updateDownloadProgress')"/></div>
    <NAlert v-if="error" type="error">{{displayMessage(error)}}</NAlert><p v-if="message" role="status">{{displayMessage(message)}}</p>
    <template #footer><NButton v-if="available && !active" text @click="skip">{{t('skipThisVersion')}}</NButton><NButton v-if="error && !active" text @click="openDownloads">{{t('downloadManually')}}</NButton><span class="action-spacer"/><NButton v-if="!active" @click="dismissUpdate">{{t('later')}}</NButton><NButton v-if="phase==='downloading'" @click="cancel">{{t('cancelDownload')}}</NButton><NButton v-if="available" type="primary" :loading="active" :disabled="active || checking" @click="install">{{phase==='installing' ? t('installing') : phase==='downloading' ? t('downloading') : error ? t('retryInstallation') : t('downloadAndInstall')}}</NButton></template>
  </FormDialog>
</template>
<style scoped>.update-notes{white-space:pre-wrap;overflow-wrap:anywhere;font-size:13px;line-height:1.8}</style>
