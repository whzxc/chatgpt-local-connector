import type { Config } from './useConnector';
export type ConnectionForm = Pick<Config, 'connectionMode' | 'httpsProvider' | 'cloudflareMode' | 'httpsUrl' | 'httpsHost' | 'httpsPort' | 'tunnelId'> & { apiKey: string; cloudflareToken: string; ngrokAuthtoken: string };
export function connectionForm(config?: Config): ConnectionForm {
  return { connectionMode: config?.connectionMode || 'tunnel', httpsProvider: config?.httpsProvider || 'cloudflare', cloudflareMode: config?.cloudflareMode || 'quick', httpsUrl: config?.httpsUrl || '', httpsHost: config?.httpsHost || '127.0.0.1', httpsPort: config?.httpsPort || 8787, tunnelId: config?.tunnelId || '', apiKey: '', cloudflareToken: '', ngrokAuthtoken: '' };
}
