// Development host only. The iframe runs the unchanged production panel and bridge.
const panel = document.getElementById('panel') as HTMLIFrameElement;
function route() {
  const params = new URLSearchParams(location.search);
  const threadId = params.get('threadId') || '';
  return { scope: threadId || params.get('scope') === 'thread' ? 'thread' : 'global', threadId };
}
const systemTheme = matchMedia('(prefers-color-scheme: dark)');
function hostContext() {
  const params = new URLSearchParams(location.search);
  const requested = params.get('theme');
  return { theme: requested === 'light' || requested === 'dark' ? requested : systemTheme.matches ? 'dark' : 'light',
    locale: params.get('locale') || navigator.language };
}
function send(message: unknown) {
  panel.contentWindow?.postMessage(message, location.origin);
}
async function callTool(name: string, args: Record<string, unknown> = {}) {
  const response = await fetch('/__plugin/tools', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'X-CLC-Request': '1' },
    body: JSON.stringify({ name, arguments: args, threadId: route().threadId }),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error || `HTTP ${response.status}`);
  return result;
}
window.addEventListener('message', async event => {
  if (event.source !== panel.contentWindow || event.origin !== location.origin || event.data?.jsonrpc !== '2.0') return;
  const { id, method, params: arguments_ } = event.data;
  try {
    if (method === 'ui/initialize') {
      send({ jsonrpc: '2.0', id, result: { protocolVersion: '2026-01-26', hostCapabilities: { experimental: { usageNavigation: true } }, hostContext: hostContext() } });
    } else if (method === 'ui/notifications/initialized') {
      const { scope, threadId } = route();
      send({ jsonrpc: '2.0', method: 'clc/notifications/route-changed', params: route() });
      const result = await callTool(scope === 'global' ? 'connector_overview' : 'connector_task_usage',
        threadId ? { threadId } : {});
      send({ jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: result });
    } else if (method === 'clc/notifications/navigate') {
      if (!['global', 'thread'].includes(arguments_?.scope) || typeof arguments_.threadId !== 'string') return;
      const url = new URL(location.href);
      url.searchParams.set('scope', arguments_.scope);
      if (arguments_.scope === 'thread' && arguments_.threadId) url.searchParams.set('threadId', arguments_.threadId);
      else url.searchParams.delete('threadId');
      if (url.href !== location.href) history.pushState(null, '', url);
    } else if (method === 'tools/call') {
      send({ jsonrpc: '2.0', id, result: await callTool(arguments_.name, arguments_.arguments) });
    } else if (id !== undefined) {
      send({ jsonrpc: '2.0', id, error: { code: -32601, message: `Unsupported preview method: ${method}` } });
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (id !== undefined) send({ jsonrpc: '2.0', id, error: { code: -32000, message } });
    else send({ jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: { _meta: { usage: {
      schemaVersion: 1, scope: route().scope, state: 'unavailable', connectorVersion: null, message,
    } } } });
  }
});
window.addEventListener('popstate', () => {
  send({ jsonrpc: '2.0', method: 'clc/notifications/route-changed', params: route() });
});
systemTheme.addEventListener('change', () => {
  send({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: hostContext() });
});
panel.src = '/plugin-panel.html';
