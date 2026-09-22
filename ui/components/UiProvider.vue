<script setup lang="ts">
import { computed } from 'vue';
import { usePreferredDark } from '@vueuse/core';
import { NConfigProvider, darkTheme, enUS, zhCN, dateEnUS, dateZhCN, type GlobalThemeOverrides } from 'naive-ui';
import { locale } from '../i18n';
import { theme } from '../theme';
const systemDark = usePreferredDark();
const dark = computed(() => theme.value === 'dark' || (theme.value === 'system' && systemDark.value));
const overrides = computed<GlobalThemeOverrides>(() => ({
  common: { primaryColor: dark.value ? '#91bea7' : '#315f51', primaryColorHover: dark.value ? '#abd0bc' : '#437761', primaryColorPressed: dark.value ? '#7aaa91' : '#264c40', borderRadius: '8px', fontFamily: "Inter,-apple-system,BlinkMacSystemFont,'Segoe UI','PingFang SC',sans-serif", fontSize: '13px', heightMedium: '36px' },
  Radio: { buttonBorderRadius: '9px', buttonHeightMedium: '30px', buttonColor: 'transparent', buttonColorActive: dark.value ? '#40594c' : '#ffffff', buttonBorderColor: 'transparent', buttonBorderColorActive: 'transparent', buttonBorderColorHover: 'transparent', buttonTextColorActive: dark.value ? '#d1ebdc' : '#315f51', buttonBoxShadow: 'none', buttonBoxShadowHover: 'none', buttonBoxShadowFocus: 'none' },
  Tag: { heightTiny: '16px', fontSizeTiny: '10px' },
  Form: { labelFontSizeTop: '13px', labelPaddingVertical: '0 0 8px 0', feedbackFontSizeMedium: '12px' },
  Card: { borderRadius: '16px', paddingMedium: '24px' },
}));
</script>
<template><NConfigProvider :theme="dark ? darkTheme : null" :theme-overrides="overrides" :locale="locale==='zh-CN' ? zhCN : enUS" :date-locale="locale==='zh-CN' ? dateZhCN : dateEnUS"><slot/></NConfigProvider></template>
