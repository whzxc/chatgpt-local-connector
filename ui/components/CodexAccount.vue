<script setup lang="ts">
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
      cause instanceof Error ? cause.message : "无法读取 Codex 登录状态";
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
        <h2>Codex 账号</h2>
        <p>在 Codex Desktop 中完成这台设备的授权。</p>
      </div>
      <StatusBadge :state="login.state" />
    </div>
    <p v-if="error" class="error-detail" role="alert">{{ error }}</p>
    <p class="hint">{{ login.message }}</p>
    <div class="actions">
      <button
        :disabled="!!busy || login.state === 'authenticated'"
        @click="authorize"
      >
        {{ busy === "codex-login" ? "正在发起授权…" : "登录 Codex" }}</button
      ><button :disabled="!!busy || checking" @click="refresh">
        {{ checking ? "检查中…" : "检查登录状态" }}
      </button>
    </div>
  </section>
</template>
