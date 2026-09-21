import { computed, watch } from 'vue';
import { usePreferredLanguages, useStorage } from '@vueuse/core';
import { createI18n } from 'vue-i18n';
import en from './locales/en.json';
import zhCN from './locales/zh-CN.json';

export type MessageKey = keyof typeof en;
export type Locale = 'en' | 'zh-CN';
export type LanguagePreference = 'auto' | Locale;
export const languageOptions = [
  { value: 'auto', label: 'Auto' },
  { value: 'en', label: 'English' },
  { value: 'zh-CN', label: '简体中文' },
] as const;
const normalizePreference = (value: unknown): LanguagePreference =>
  value === 'en' || value === 'zh-CN' ? value : 'auto';
const preference = useStorage<LanguagePreference>('language', 'auto', undefined, {
  serializer: { read: normalizePreference, write: value => value },
  writeDefaults: false,
});
export const language = computed(() => normalizePreference(preference.value));
const systemLanguages = usePreferredLanguages();
export function resolveLocale(languages: readonly string[]): Locale {
  for (const language of languages) {
    const tag = language.toLowerCase().replaceAll('_', '-');
    if (tag === 'en' || tag.startsWith('en-')) return 'en';
    if (tag === 'zh' || /^zh-(cn|sg|hans)(-|$)/.test(tag)) return 'zh-CN';
  }
  return 'en';
}
export const i18n = createI18n({
  legacy: false,
  locale: 'en' as Locale,
  fallbackLocale: 'en',
  messages: { en, 'zh-CN': zhCN },
});
export const { t, locale } = i18n.global;
export function setLanguage(value: string) {
  preference.value = normalizePreference(value);
}
watch([language, systemLanguages], ([selection, languages]) => {
  locale.value = selection === 'auto' ? resolveLocale(languages) : selection;
}, { immediate: true });
watch(locale, value => {
  document.documentElement.lang = value;
  if ('__TAURI_INTERNALS__' in window) {
    void import('@tauri-apps/api/core').then(({ invoke }) => invoke('set_ui_locale', { locale: value })).catch(console.error);
  }
}, { immediate: true });
