import { computed, watch } from 'vue';
import { useStorage } from '@vueuse/core';
export const theme = useStorage('theme', 'system');
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });

export const themeColors = {
  forest: { label: 'themeForest', light: '#315f51', dark: '#91bea7' },
  ocean: { label: 'themeOcean', light: '#28659a', dark: '#90bce5' },
  violet: { label: 'themeViolet', light: '#7051a3', dark: '#bca4e1' },
  amber: { label: 'themeAmber', light: '#936017', dark: '#dfba78' },
  rose: { label: 'themeRose', light: '#a04766', dark: '#e0a0b6' },
} as const;
export const themeColor = useStorage<string>('theme-color', 'forest');
export const accent = computed(() => themeColors[themeColor.value as keyof typeof themeColors] || themeColors.forest);
function sceneMix(hex: string, base: number[], amount: number) {
  return base.map((value, index) => Math.round(value * (1 - amount) + parseInt(hex.slice(1 + index * 2, 3 + index * 2), 16) * amount));
}
export const sceneColors = computed(() => ({
  light: sceneMix(accent.value.light, [255, 255, 255], .13),
  dark: sceneMix(accent.value.dark, [23, 33, 30], .12),
}));
watch(accent, value => {
  const style = document.documentElement.style;
  style.setProperty('--green', `light-dark(${value.light},${value.dark})`);
  style.setProperty('--primary-color', 'var(--green)');
  style.setProperty('--accent-light', value.light);
  style.setProperty('--accent-dark', value.dark);
  style.setProperty('--accent-soft', `light-dark(color-mix(in srgb, ${value.light} 9%, #fff),color-mix(in srgb, ${value.dark} 12%, #202522))`);
  style.setProperty('--scene-color', `light-dark(rgb(${sceneColors.value.light.join(' ')}),rgb(${sceneColors.value.dark.join(' ')}))`);
}, { immediate: true });

export const translucent = useStorage('translucent', false);
watch(translucent, value => { document.documentElement.dataset.translucent = String(value); }, { immediate: true });
