import { computed, watch } from 'vue';
import { fixedColors } from './colors';
import './tokens.css';
import { useStorage } from '@vueuse/core';
export const theme = useStorage('theme', 'system');
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });

export const themeColors = {
  forest: { label: 'themeForest', light: '#6f9d89', dark: '#afd1bf' },
  blue: { label: 'themeBlue', light: '#007aff', dark: '#0a84ff' },
  purple: { label: 'themePurple', light: '#943d96', dark: '#bf5af2' },
  pink: { label: 'themePink', light: '#f8509e', dark: '#ff375f' },
  red: { label: 'themeRed', light: '#e1393e', dark: '#ff453a' },
  orange: { label: 'themeOrange', light: '#f78218', dark: '#ff9f0a' },
  yellow: { label: 'themeYellow', light: '#ffc626', dark: '#ffd60a' },
  green: { label: 'themeGreen', light: '#63ba47', dark: '#32d74b' },
  gray: { label: 'themeGray', light: '#999998', dark: '#98989d' },
} as const;
export const themeColor = useStorage<string>('theme-color', 'forest');
export const accent = computed(() => themeColors[themeColor.value as keyof typeof themeColors] || themeColors.forest);
watch(themeColor, value => {
  if (!Object.hasOwn(themeColors, value)) themeColor.value = 'forest';
}, { immediate: true });
watch(accent, value => {
  const style = document.documentElement.style;
  style.setProperty('--accent', `light-dark(${value.light},${value.dark})`);
  style.setProperty('--primary-color', 'var(--accent)');
  style.setProperty('--accent-light', value.light);
  style.setProperty('--accent-dark', value.dark);
}, { immediate: true });

for (const [name, color] of Object.entries(fixedColors)) {
  document.documentElement.style.setProperty('--' + name.replace(/[A-Z]/g, letter => '-' + letter.toLowerCase()), color);
}
