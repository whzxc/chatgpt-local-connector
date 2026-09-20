import { reactive } from 'vue';
import { api, useConnector } from './useConnector';
export function useConnectionForm() {
  const { status } = useConnector();
  const config = status.value!.config;
  const form = reactive({
    connectionMode: config.connectionMode || 'tunnel',
    httpsProvider: config.httpsProvider || (config.httpsUrl ? 'custom' : 'cloudflare'),
    ngrokAuthtoken: '',
    cloudflareMode: config.cloudflareMode || 'quick', cloudflareToken: '',
    tunnelId: config.tunnelId, apiKey: '',
    httpsUrl: config.httpsUrl || '', httpsHost: config.httpsHost || '127.0.0.1',
    httpsPort: config.httpsPort || 8787,
  });
  async function save() {
    const { tunnelBinary, codexBinary, autoStart, proxyMode, proxyUrl } = status.value!.config;
    await api('config', 'PUT', { ...form, tunnelBinary, codexBinary, autoStart, proxyMode, proxyUrl,
      tunnelId: form.tunnelId.trim(), apiKey: form.apiKey.trim(),
      httpsUrl: form.httpsUrl.trim(), httpsHost: form.httpsHost.trim(), ngrokAuthtoken: form.ngrokAuthtoken.trim(), cloudflareToken: form.cloudflareToken.trim(),
    });
    form.apiKey = ''; form.ngrokAuthtoken = ''; form.cloudflareToken = '';
  }
  return { form, save };
}
export type ConnectionForm = ReturnType<typeof useConnectionForm>['form'];
