import { shallowRef } from 'vue';

// Keep nested panels above their parent and block the workspace until closing finishes.
export const panelLayers = shallowRef<HTMLElement[]>([]);

// A panel switch carries its current bounds into the next panel's entrance.
export const panelHandoff: { from?: DOMRect; trigger?: HTMLElement } = {};

export function panelAnchorAt(x: number, y: number, exclude?: HTMLElement) {
  return Array.from(document.querySelectorAll<HTMLElement>('[data-panel-anchor]')).find(node => {
    if (node === exclude || node.closest('.panel-layer') || node.matches(':disabled')) return false;
    const rect = node.getBoundingClientRect(), css = getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && css.visibility !== 'hidden' && Number(css.opacity) > 0
      && x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  });
}
