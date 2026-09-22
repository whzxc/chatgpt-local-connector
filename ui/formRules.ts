import type { FormItemRule } from 'naive-ui';
import { t } from './i18n';
export const required = (): FormItemRule => ({ required: true, trigger: ['input','blur'], validator: (_rule, value) => typeof value === 'string' && value.trim() ? true : new Error(t('fieldRequired')) });
export function validDomain(value: string): boolean {
  const domain=value.trim();
  return domain.length<=253 && domain.includes('.') && domain.split('.').every(label => /^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$/.test(label));
}
export const domainRule = (optional = false): FormItemRule => ({ trigger: ['input','blur'], validator: (_rule, value) => (optional && !value.trim()) || validDomain(value) ? true : new Error(t('validDomain')) });
