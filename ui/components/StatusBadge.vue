<script setup lang="ts">
import { t } from '../i18n';
import { computed } from "vue";
const props = defineProps<{ state?: string }>();
const labels = computed<Record<string, string>>(() => ({
  running: t('running'),
  connected: t('connected'),
  ready: t('ready'),
  stopped: t('stopped'),
  starting: t('starting'),
  connecting: t('connectingLabel'),
  error: t('errorLabel'),
  failed: t('failed'),
  degraded: t('connectionNotReady'),
  unknown: t('unconfirmed'),
  disconnected: t('disconnected'),
  unconfigured: t('notConfigured'),
  authenticated: t('signedIn'),
  unauthenticated: t('signedOut'),
  pending: t('awaitingAuthorization'),
  ok: t('passed'),
  pass: t('passed'),
  passed: t('passed'),
  fail: t('failed'),
  warning: t('needsAttention'),
  warn: t('needsAttention'),
}));
const tone = computed(() =>
  [
    "running",
    "connected",
    "ready",
    "authenticated",
    "ok",
    "pass",
    "passed",
  ].includes(props.state || "")
    ? "good"
    : ["error", "failed", "fail"].includes(props.state || "")
      ? "bad"
      : ["starting", "connecting", "pending", "warn", "warning"].includes(
            props.state || "",
          )
        ? "wait"
        : "",
);
</script>
<template>
  <span class="badge" :class="tone">{{
    labels[state || ""] || state || t('loading')
  }}</span>
</template>
