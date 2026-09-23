import { useStorage } from '@vueuse/core';
import { t } from '../i18n';
import { remaining, type QuotaWindow } from './types';

export const quotaDisplay = useStorage<'remaining' | 'used'>('quota-display', 'remaining');
export const resetDisplay = useStorage<'countdown' | 'time'>('quota-reset-display', 'countdown');
export const quotaLabel = () => t(quotaDisplay.value === 'used' ? 'usageUsed' : 'usageRemaining');
export function quotaValue(window?: QuotaWindow) {
  const value = remaining(window);
  return value === undefined ? undefined : quotaDisplay.value === 'used' ? 100 - value : value;
}
export function quotaPercent(window?: QuotaWindow) {
  const value = quotaValue(window);
  return value === undefined ? '—' : value > 0 && value < 1 ? '<1%' : `${Math.floor(value)}%`;
}

export const usagePeriods = useStorage<string[]>('usage-periods', ['today', 'yesterday', 'last7', 'last30']);
export const usagePeriodLabels = { today: 'usageToday', yesterday: 'usageYesterday', last7: 'usageLast7', last30: 'usageLast30' } as const;

export const maxVisibleAgents = useStorage<number>('agents-max-visible', 4);
