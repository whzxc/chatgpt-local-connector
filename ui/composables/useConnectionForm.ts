import { reactive } from 'vue';
import { api, useConnector } from './useConnector';
export function useConnectionForm() {
  const { status } = useConnector();
  const config = status.value!.config;
  const form = reactive({
    connectionMode: config.connectionMode || 'tunnel',
    httpsRequireAuth: config.httpsRequireAuth ?? false,
    tunnelId: config.tunnelId, apiKey: '',
    httpsUrl: config.httpsUrl || '', httpsHost: config.httpsHost || '127.0.0.1',
    httpsPort: config.httpsPort || 8787, httpsApiKey: '',
  });
  async function save() {
    const { tunnelBinary, codexBinary, autoStart, proxyMode, proxyUrl } = status.value!.config;
    await api('config', 'PUT', { ...form, tunnelBinary, codexBinary, autoStart, proxyMode, proxyUrl,
      tunnelId: form.tunnelId.trim(), apiKey: form.apiKey.trim(),
      httpsUrl: form.httpsUrl.trim(), httpsHost: form.httpsHost.trim(), httpsApiKey: form.httpsApiKey.trim(),
    });
    form.apiKey = ''; form.httpsApiKey = '';
  }
  return { form, save };
}
export type ConnectionForm = ReturnType<typeof useConnectionForm>['form'];
