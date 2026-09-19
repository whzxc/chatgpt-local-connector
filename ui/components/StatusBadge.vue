<script setup lang="ts">
import { computed } from "vue";
const props = defineProps<{ state?: string }>();
const labels: Record<string, string> = {
  running: "运行中",
  connected: "已连接",
  ready: "已就绪",
  stopped: "已停止",
  starting: "启动中",
  connecting: "连接中",
  error: "异常",
  failed: "失败",
  degraded: "连接待就绪",
  unknown: "待确认",
  disconnected: "未连接",
  unconfigured: "未配置",
  authenticated: "已登录",
  unauthenticated: "未登录",
  pending: "等待授权",
  ok: "通过",
  pass: "通过",
  passed: "通过",
  fail: "失败",
  warning: "需关注",
  warn: "需关注",
};
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
    labels[state || ""] || state || "读取中"
  }}</span>
</template>
