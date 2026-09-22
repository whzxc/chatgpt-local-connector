import { shallowRef } from 'vue';

// Keep nested panels above their parent and block the workspace until closing finishes.
export const panelLayers = shallowRef<HTMLElement[]>([]);
