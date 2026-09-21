<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { onMounted, ref } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { api, useConnector, type Login } from "../composables/useConnector";
import StatusBadge from "./StatusBadge.vue";
const { busy, run } = useConnector();
const login = ref<Login>({ state: "unknown" });
const error = ref("");
const checking = ref(false);
async function refresh() {
  if (checking.value) return;
  checking.value = true;
  try {
    login.value = await api<Login>("codex/login");
    error.value = "";
  } catch (cause) {
    error.value =
      cause instanceof Error ? cause.message : t('unableToReadCodexSignInStatus');
  } finally {
    checking.value = false;
  }
}
function authorize() {
  return run("codex-login", async () => {
    login.value = await api<Login>("codex/login", "POST");
    if (!login.value.message) await refresh();
  });
}
onMounted(() => {
  void refresh();
});
useIntervalFn(() => {
  if (!document.hidden) void refresh();
}, 10_000);
</script>
<template>
  <section class="card">
    <div class="card-head">
      <div>
        <h2>{{ t('codexAccount') }}</h2>
        <p>{{ t('authorizeThisDeviceInCodexDesktop') }}</p>
      </div>
      <StatusBadge :state="login.state" />
    </div>
    <p v-if="error" class="error-detail" role="alert">{{displayMessage(error)}}</p>
    <p class="hint">{{displayMessage(login.message)}}</p>
    <div class="actions">
      <button
        :disabled="!!busy || login.state === 'authenticated'"
        @click="authorize"
      >
        {{ busy === "codex-login" ? t('startingAuthorization') : t('signInToCodex') }}</button
      ><button :disabled="!!busy || checking" @click="refresh">
        {{ checking ? t('checkingLabel') : t('checkSignInStatus') }}
      </button>
    </div>
  </section>
</template>
