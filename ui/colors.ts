// Fixed semantic colors: independent of the selected decorative accent.
export const fixedColors = {
  black: '#000000', white: '#ffffff',
  railInk: '#f5f5f7', railMuted: '#8b8b90', railTrack: '#29292c',
  success: '#00a66c', warning: '#b97800', danger: '#d9363e',
  usageUnknown: '#67676b', usageCaution: '#ffc226',
  usageWarning: '#ff4f42', usageExhausted: '#d91721',
} as const;
export const usageColors = {
  unknown: fixedColors.usageUnknown, good: 'var(--accent)',
  caution: fixedColors.usageCaution, warning: fixedColors.usageWarning,
  exhausted: fixedColors.usageExhausted,
};
