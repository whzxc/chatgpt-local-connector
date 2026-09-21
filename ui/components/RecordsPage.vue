<script setup lang="ts">
import { displayMessage } from '../messages';
import { t, locale, type MessageKey } from '../i18n';
import { computed, ref } from 'vue';
import { Search } from '@lucide/vue';
import { useConnector } from '../composables/useConnector';
const { status } = useConnector();
const filter = ref<MessageKey>('all');
const query = ref('');
// Preserve raw diagnostics in the service, but only show useful events here.
const internalMessage = /^(?:provided|supplied|run|invoking|invoke|initialized custom fxevent\.Logger|OnStart hook (?:executing|executed)|OnStop hook (?:executing|executed)|mcp channel route resolved|TunnelServiceClient created|control-plane route resolved|dispatcher channels registered|Skipping MCP probe for transport|admin ui enabled|tls trust summary|health server listening|harpoon enabled|starting control-plane poller|poller started|harpoon startup catalog digest|tunnel-client startup summary|Codex detected without Tunnel MCP plugin|tunnel metadata fetched|stdio MCP command started)$/i;
const messages = computed<Record<string, string>>(() => ({
  '连接进程已启动，正在检查 Tunnel 和 MCP…': t('startingConnection'),
  '🟢 tunnel-client started': t('connectionReady'),
  'dispatcher forwarded command to MCP server': t('requestReceivedAndForwarded'),
  '连接已停止': t('connectionClosed'),
}));
const rows = computed(() => (status.value?.logs || []).flatMap((line, index) => {
  let entry: Record<string, unknown> = {};
  try { const value = JSON.parse(line); if (value && typeof value === 'object' && !Array.isArray(value)) entry = value; } catch { /* Plain log lines remain readable. */ }
  const rawMessage = String(entry.msg ?? entry.message ?? line);
  const level = String(entry.level ?? '').toLowerCase();
  const error = /error|fatal|panic/.test(level) || /\b(error|failed|failure)\b|失败|异常/i.test(rawMessage);
  const warning = /warn/.test(level) || /\bwarn(ing)?\b|警告/i.test(rawMessage);
  // Never hide a warning or error just because its source is internal.
  if (!error && !warning && (/debug|trace/.test(level) || internalMessage.test(rawMessage) || /^(?:🩺 HEALTH URL:|🌐 WEB UI:)/.test(rawMessage))) return [];
  const message = messages.value[rawMessage] ?? displayMessage(rawMessage);
  const kind: MessageKey = error ? 'error' : warning ? 'warning'
    : /request|command|请求/i.test(rawMessage) ? 'request'
    : /tunnel|connect|连接|通道/i.test(rawMessage) ? 'connection' : 'info';
  const rawTime = entry.time ?? entry.timestamp;
  const date = rawTime === undefined ? undefined : new Date(rawTime as string | number);
  const validDate = date && !Number.isNaN(date.getTime());
  const time = validDate ? `${date.toLocaleTimeString(locale.value, { hour12: false })}.${String(date.getMilliseconds()).padStart(3, '0')}` : '—';
  const labels: Record<string, string> = { error: t('error'), reason: t('reason'), status: t('status'), method: t('method'), duration: t('duration') };
  const detail = Object.keys(labels).flatMap(key => entry[key] === undefined ? [] : [`${labels[key]}: ${typeof entry[key] === 'object' ? JSON.stringify(entry[key]) : String(entry[key])}`]).join(' · ');
  return [{ index, time, timestamp: validDate ? date.toLocaleString(locale.value) : t('noTimestampInTheOriginalRecord'), kind, message, detail }];
}).reverse());
const visibleRows = computed(() => rows.value.filter(row => (filter.value === 'all' || row.kind === filter.value) && `${row.message} ${row.detail}`.toLowerCase().includes(query.value.toLowerCase())));
</script>
<template>
  <section class="records-page" :aria-label="t('connectionRecords')">
    <div class="records-toolbar"><div class="log-filters" :aria-label="t('logType')"><button v-for="kind in (['all','connection','request','info','warning','error'] as const)" :key="kind" :aria-pressed="filter===kind" :class="{active:filter===kind}" @click="filter=kind">{{t(kind)}}</button></div><label class="log-search"><Search aria-hidden="true"/><input v-model="query" :aria-label="t('searchLogs')" :placeholder="t('searchRecords')"/></label><span class="log-count">{{t('recordsCount', { count: visibleRows.length })}}</span></div>
    <div class="log-table" role="table" :aria-label="t('connectionLogContents')"><div class="log-table-head" role="row"><span role="columnheader">{{ t('time') }}</span><span role="columnheader">{{ t('type') }}</span><span role="columnheader">{{ t('content') }}</span></div><div v-for="row in visibleRows" :key="row.index" class="log-entry" role="row"><time role="cell" :title="row.timestamp">{{row.time}}</time><span role="cell" class="log-kind" :class="{'is-error':row.kind==='error','is-warning':row.kind==='warning'}">{{t(row.kind)}}</span><div role="cell" class="log-content"><span>{{row.message}}</span><span v-if="row.detail" class="log-detail"> · {{row.detail}}</span></div></div></div>
    <p v-if="!visibleRows.length" class="empty records-empty">{{rows.length ? t('noMatchingRecords') : t('noConnectionRecordsYet')}}</p>
  </section>
</template>
