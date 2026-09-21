import { watch } from 'vue';
import { useStorage } from '@vueuse/core';
export const theme = useStorage('theme', 'system');
watch(theme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
