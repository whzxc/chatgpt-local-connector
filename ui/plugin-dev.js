// Development-only resource reload over the existing MCP bridge. No network access.
(() => {
  const { uri, revision } = window.__CLC_DEV__;
  if (location.protocol === 'blob:') URL.revokeObjectURL(location.href);
  let pending;
  let sequence = 0;
  let stopped = false;
  let timer;
  let initialResult = window.__CLC_DEV_INITIAL_RESULT__;
  const schedule = () => { timer = setTimeout(poll, 1500); };
  function poll() {
    if (stopped) return;
    if (document.hidden) { schedule(); return; }
    pending = `clc-dev-${Date.now()}-${++sequence}`;
    window.parent.postMessage({ jsonrpc: '2.0', id: pending, method: 'resources/read', params: { uri } }, '*');
    timer = setTimeout(() => { pending = undefined; schedule(); }, 20000);
  }
  window.addEventListener('message', event => {
    if (event.source !== window.parent || event.data?.jsonrpc !== '2.0') return;
    if (event.data.method === 'ui/notifications/tool-result') initialResult = event.data.params;
    // The host does not replay the entrypoint result when this frame reloads.
    // Carry its actual result forward so scope/thread binding remains identical.
    if (event.data.result?.hostContext && initialResult) {
      setTimeout(() => window.dispatchEvent(new MessageEvent('message', {
        source: window.parent,
        data: { jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: initialResult },
      })), 0);
    }
    if (event.data.method === 'ui/resource-teardown') { stopped = true; clearTimeout(timer); return; }
    if (!pending || event.data.id !== pending) return;
    pending = undefined;
    clearTimeout(timer);
    const content = event.data.result?.contents?.find(item => item.uri === uri);
    if (content?._meta?.['clc/devRevision'] && content._meta['clc/devRevision'] !== revision && content.text) {
      stopped = true;
      const initial = JSON.stringify(initialResult ?? null).replace(/</g, '\\u003c');
      const html = content.text.replace('<head>', `<head><script>window.__CLC_DEV_INITIAL_RESULT__=${initial};<\/script>`);
      location.replace(URL.createObjectURL(new Blob([html], { type: 'text/html' })));
    } else schedule();
  });
  window.addEventListener('pagehide', () => { stopped = true; clearTimeout(timer); });
  schedule();
})();
