<script lang="ts">
export const usageValueButtonTheme = {
  textColorText: 'inherit', textColorTextHover: 'inherit',
  textColorTextPressed: 'inherit', textColorTextFocus: 'inherit',
};
export const usagePopoverTheme = (rail: boolean, horizontalGap = 40) => ({
  borderRadius: '20px',
  padding: '18px',
  fontSize: '11.5px',
  ...(rail ? { color: '#000000', textColor: '#f5f5f7' } : {}),
  boxShadow: rail ? 'none' : '0 8px 24px rgba(0,0,0,.12)',
  space: '12px',
  spaceArrow: rail ? `${horizontalGap}px` : '28px',
});
</script>
<script setup lang="ts">
import { computed, watch } from 'vue';
import { usePreferredDark } from '@vueuse/core';
import { NConfigProvider, darkTheme, enUS, zhCN, dateEnUS, dateZhCN, type GlobalThemeOverrides } from 'naive-ui';
import { locale } from '../i18n';
import { theme, accent, sceneColors, translucent } from '../theme';
import { isDesktop } from '../platform';
const props=defineProps<{forceDark?:boolean}>();
const systemDark = usePreferredDark();
const dark = computed(() => props.forceDark || theme.value === 'dark' || (theme.value === 'system' && systemDark.value));
if (isDesktop && navigator.platform.toLowerCase().includes('win')) {
  document.documentElement.dataset.nativeWindowsFrame = 'true';
  watch([dark, sceneColors, translucent], async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_windows_appearance', { color: dark.value ? sceneColors.value.dark : sceneColors.value.light, dark: dark.value, translucent: translucent.value });
  }, { immediate: true });
}
function tint(hex: string, target: number, amount: number) {
  return '#' + [1, 3, 5].map(offset => Math.round(parseInt(hex.slice(offset, offset + 2), 16) * (1 - amount) + target * amount).toString(16).padStart(2, '0')).join('');
}
const overrides = computed<GlobalThemeOverrides>(() => ({
  common: { primaryColor: dark.value ? accent.value.dark : accent.value.light, primaryColorHover: tint(dark.value ? accent.value.dark : accent.value.light, 255, .15), primaryColorPressed: tint(dark.value ? accent.value.dark : accent.value.light, 0, .15), borderRadius: '8px', fontFamily: "Inter,-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC',sans-serif", fontSize: '13px', heightMedium: '36px' },
  Radio: { buttonBorderRadius: '9px', buttonHeightMedium: '30px', buttonColor: 'transparent', buttonColorActive: dark.value ? tint(accent.value.dark, 32, .78) : '#ffffff', buttonBorderColor: 'transparent', buttonBorderColorActive: 'transparent', buttonBorderColorHover: 'transparent', buttonTextColorActive: dark.value ? accent.value.dark : accent.value.light, buttonBoxShadow: 'none', buttonBoxShadowHover: 'none', buttonBoxShadowFocus: 'none' },
  InputOtp: { inputWidthMedium: '36px' },
  Tag: { heightTiny: '16px', fontSizeTiny: '10px' },
  Form: { labelFontSizeTop: '13px', labelPaddingVertical: '0 0 8px 0', feedbackFontSizeMedium: '12px' },
  Card: { borderRadius: '16px', paddingMedium: '24px' },
}));
</script>
<template><NConfigProvider :theme="dark ? darkTheme : null" :theme-overrides="overrides" :locale="locale==='zh-CN' ? zhCN : enUS" :date-locale="locale==='zh-CN' ? dateZhCN : dateEnUS"><slot/></NConfigProvider></template>
