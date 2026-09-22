import type { FormItemRule } from 'naive-ui';
import { t } from './i18n';
export const required = (): FormItemRule => ({ required: true, trigger: ['input','blur'], validator: (_rule, value) => typeof value === 'string' && value.trim() ? true : new Error(t('fieldRequired')) });
export function validMcpUrl(value:string):boolean {
  try { const url=new URL(value); return url.protocol==='https:' && !!url.hostname && !url.username && !url.password && url.pathname==='/mcp' && !url.search && !url.hash; } catch { return false; }
}
export const httpsUrl = (optional = false): FormItemRule => ({ trigger: ['input','blur'], validator: (_rule, value) => (!value && optional) || validMcpUrl(value) ? true : new Error(t('validHttpsUrl')) });

export function validNgrokEndpoint(value: string): boolean {
  const address=value.trim();
  if(!address || /[\s*]/.test(address)) return false;
  try {
    const url=new URL(address.includes('://') ? address : `https://${address}`);
    return url.protocol==='https:' && !!url.hostname && !url.username && !url.password && !url.port && url.pathname==='/' && !url.search && !url.hash;
  } catch { return false; }
}
