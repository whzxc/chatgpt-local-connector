import type { Config } from './useConnector';
export type ConnectionForm = Pick<Config, 'connectionMode' | 'httpsProvider' | 'cloudflareMode' | 'httpsHost' | 'httpsPort' | 'tunnelId' | 'ngrokMode'> & { domain: string; apiKey: string; cloudflareToken: string; ngrokAuthtoken: string };
export function connectionForm(config?: Config): ConnectionForm {
  const address = config?.httpsProvider==='ngrok' ? config.ngrokEndpoint : config?.httpsUrl;
  const domain = address ? new URL(address.includes('://') ? address : `https://${address}`).hostname : '';
  return { domain, connectionMode: config?.connectionMode || 'tunnel', httpsProvider: config?.httpsProvider || 'ngrok', cloudflareMode: config?.cloudflareMode || 'quick', httpsHost: config?.httpsHost || '127.0.0.1', httpsPort: config?.httpsPort || 8787, tunnelId: config?.tunnelId || '', apiKey: '', cloudflareToken: '', ngrokAuthtoken: '', ngrokMode: config?.ngrokMode || 'named' };
}
