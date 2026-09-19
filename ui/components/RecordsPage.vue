<script setup lang="ts">
import { computed, ref } from 'vue';
import { Search } from '@lucide/vue';
import { useConnector } from '../composables/useConnector';
const { status } = useConnector();
const filter = ref('全部');
const query = ref('');
// Preserve raw diagnostics in the service, but only show useful events here.
const internalMessage = /^(?:provided|supplied|run|invoking|invoke|initialized custom fxevent\.Logger|OnStart hook (?:executing|executed)|OnStop hook (?:executing|executed)|mcp channel route resolved|TunnelServiceClient created|control-plane route resolved|dispatcher channels registered|Skipping MCP probe for transport|admin ui enabled|tls trust summary|health server listening|harpoon enabled|starting control-plane poller|poller started|harpoon startup catalog digest|tunnel-client startup summary|Codex detected without Tunnel MCP plugin|tunnel metadata fetched|stdio MCP command started)$/i;
const messages: Record<string, string> = {
  '连接进程已启动，正在检查 Tunnel 和 MCP…': '正在开启连接',
  '🟢 tunnel-client started': '连接已就绪',
  'dispatcher forwarded command to MCP server': '收到请求，已转交处理',
  '连接已停止': '连接已关闭',
};
const rows = computed(() => (status.value?.logs || []).flatMap((line, index) => {
  let entry: Record<string, unknown> = {};
  try { const value = JSON.parse(line); if (value && typeof value === 'object' && !Array.isArray(value)) entry = value; } catch { /* Plain log lines remain readable. */ }
  const rawMessage = String(entry.msg ?? entry.message ?? line);
  const level = String(entry.level ?? '').toLowerCase();
  const error = /error|fatal|panic/.test(level) || /\b(error|failed|failure)\b|失败|异常/i.test(rawMessage);
  const warning = /warn/.test(level) || /\bwarn(ing)?\b|警告/i.test(rawMessage);
  // Never hide a warning or error just because its source is internal.
  if (!error && !warning && (/debug|trace/.test(level) || internalMessage.test(rawMessage) || /^(?:🩺 HEALTH URL:|🌐 WEB UI:)/.test(rawMessage))) return [];
  const message = messages[rawMessage] ?? rawMessage;
  const kind = error ? '错误' : warning ? '警告'
    : /request|command|请求/i.test(rawMessage) ? '请求'
    : /tunnel|connect|连接|通道/i.test(rawMessage) ? '连接' : '信息';
  const rawTime = entry.time ?? entry.timestamp;
  const date = rawTime === undefined ? undefined : new Date(rawTime as string | number);
  const validDate = date && !Number.isNaN(date.getTime());
  const time = validDate ? `${date.toLocaleTimeString('zh-CN', { hour12: false })}.${String(date.getMilliseconds()).padStart(3, '0')}` : '—';
  const labels: Record<string, string> = { error: '错误', reason: '原因', status: '状态', method: '方法', duration: '耗时' };
  const detail = Object.keys(labels).flatMap(key => entry[key] === undefined ? [] : [`${labels[key]}：${typeof entry[key] === 'object' ? JSON.stringify(entry[key]) : String(entry[key])}`]).join(' · ');
  return [{ index, time, timestamp: validDate ? date.toLocaleString() : '原始记录未提供时间', kind, message, detail }];
}));
const visibleRows = computed(() => rows.value.filter(row => (filter.value === '全部' || row.kind === filter.value) && `${row.message} ${row.detail}`.toLowerCase().includes(query.value.toLowerCase())));
</script>
<template>
  <section class="records-page" aria-label="连接记录">
    <div class="records-toolbar"><div class="log-filters" aria-label="日志类型"><button v-for="kind in ['全部','连接','请求','信息','警告','错误']" :key="kind" :aria-pressed="filter===kind" :class="{active:filter===kind}" @click="filter=kind">{{kind}}</button></div><label class="log-search"><Search aria-hidden="true"/><input v-model="query" aria-label="搜索日志" placeholder="搜索记录"/></label><span class="log-count">{{visibleRows.length}} 条</span></div>
    <div class="log-table" role="table" aria-label="连接日志内容"><div class="log-table-head" role="row"><span role="columnheader">时间</span><span role="columnheader">类型</span><span role="columnheader">内容</span></div><div v-for="row in visibleRows" :key="row.index" class="log-entry" role="row"><time role="cell" :title="row.timestamp">{{row.time}}</time><span role="cell" class="log-kind" :class="{'is-error':row.kind==='错误','is-warning':row.kind==='警告'}">{{row.kind}}</span><div role="cell" class="log-content"><span>{{row.message}}</span><span v-if="row.detail" class="log-detail"> · {{row.detail}}</span></div></div></div>
    <p v-if="!visibleRows.length" class="empty records-empty">{{rows.length ? '没有匹配的记录。' : '暂无连接记录。'}}</p>
  </section>
</template>
