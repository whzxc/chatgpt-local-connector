<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { onClickOutside } from '@vueuse/core';
import { NButton } from 'naive-ui';
import { X } from '@lucide/vue';
import { panelLayers } from '../composables/panels';
import { t } from '../i18n';
const props = withDefaults(defineProps<{ show: boolean; title: string; origin?: { x: number; y: number; size: number; height?: number }; busy?: boolean; width?: number }>(), { width: 800 });
const emit = defineEmits<{ close: []; closed: [] }>();
const panel = ref<HTMLElement>();
const position = ref({ x: 0, y: 0 });
const dragging = ref(false);
let drag: { pointerId: number; x: number; y: number; left: number; top: number } | undefined;
function moveTo(x: number, y: number) {
  if (!panel.value) return;
  const maxX = Math.max(0, (window.innerWidth - panel.value.offsetWidth) / 2 - 20);
  const maxY = Math.max(0, (window.innerHeight - panel.value.offsetHeight) / 2 - 20);
  position.value = { x: Math.max(-maxX, Math.min(maxX, x)), y: Math.max(-maxY, Math.min(maxY, y)) };
}
function startDrag(event: PointerEvent) {
  if (event.button !== 0 || !event.isPrimary || !topmost.value || motions.size || (event.target as Element).closest('.panel-actions')) return;
  const header = event.currentTarget as HTMLElement;
  header.setPointerCapture(event.pointerId);
  drag = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, left: position.value.x, top: position.value.y };
  dragging.value = true;
  event.preventDefault();
}
function moveDrag(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  moveTo(drag.left + event.clientX - drag.x, drag.top + event.clientY - drag.y);
}
function endDrag(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  drag = undefined;
  dragging.value = false;
  const header = event.currentTarget as HTMLElement;
  if (header.hasPointerCapture(event.pointerId)) header.releasePointerCapture(event.pointerId);
}
watch(panel, (node, _, onCleanup) => {
  if (!node) return;
  let height = node.offsetHeight;
  let resizing: Animation | undefined;
  const observer = new ResizeObserver(() => {
    moveTo(position.value.x, position.value.y);
    if (resizing) return;
    const nextHeight = node.offsetHeight;
    const previousHeight = height;
    height = nextHeight;
    if (!props.show || Math.abs(nextHeight - previousHeight) < 1 || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    resizing = node.animate([{ height: `${previousHeight}px` }, { height: `${nextHeight}px` }], {
      duration: 320, easing: 'cubic-bezier(.22,1,.36,1)',
    });
    const animation = resizing;
    void animation.finished.then(() => { if (resizing === animation) resizing = undefined; }, () => {});
  });
  observer.observe(node);
  const constrainPosition = () => moveTo(position.value.x, position.value.y);
  window.addEventListener('resize', constrainPosition);
  onCleanup(() => { observer.disconnect(); resizing?.cancel(); window.removeEventListener('resize', constrainPosition); drag = undefined; dragging.value = false; });
}, { flush: 'post' });
const style = computed(() => ({ '--preferred-width': `${props.width}px`, transform: `translate(${position.value.x}px,${position.value.y}px) scale(1,1)` }));
let layer: HTMLElement | undefined;
let trigger: HTMLElement | undefined;
let origin = { x: 0, y: 0, size: 1, height: 1 };
const topmost = computed(() => panelLayers.value.at(-1) === layer);
function release() { panelLayers.value = panelLayers.value.filter(item => item !== layer); }
function closed() { release(); emit('closed'); void nextTick(() => { if (trigger?.isConnected) trigger.focus({ preventScroll: true }); }); }
const motions = new Map<Element, () => void>();
function cancelTransition(element: Element) { motions.get(element)?.(); }
onBeforeUnmount(() => { release(); motions.forEach(cancel => cancel()); });
function transition(element: Element, done: () => void, opening: boolean) {
  cancelTransition(element);
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) { done(); return; }
  const node = element.querySelector<HTMLElement>('.elastic-panel')!;
  const { x, y, size, height: originHeight } = origin;
  const width = Math.max(1, node.offsetWidth), height = Math.max(1, node.offsetHeight);
  const collapsed = {
    transform: `translate(${x + size / 2 - window.innerWidth / 2}px,${y + originHeight / 2 - window.innerHeight / 2}px) scale(${size / width},${originHeight / height})`,
    opacity: 0,
  };
  const expanded = { transform: `translate(${position.value.x}px,${position.value.y}px) scale(1,1)`, opacity: 1 };
  const duration = opening ? 480 : 220;
  node.style.willChange = 'transform, opacity';
  const animation = node.animate(opening ? [collapsed, expanded] : [expanded, collapsed], {
    duration, easing: opening ? 'cubic-bezier(.22,1,.36,1)' : 'cubic-bezier(.4,0,.6,1)', fill: 'both',
  });
  const contentAnimations = Array.from(node.children, child => child.animate(
    opening ? [{ opacity: 0 }, { opacity: 1 }] : [{ opacity: 1 }, { opacity: 0 }],
    { duration: opening ? 280 : 100, delay: opening ? 80 : 0, fill: 'both' },
  ));
  const cleanup = () => {
    motions.delete(element); animation.cancel(); node.style.willChange = '';
    contentAnimations.forEach(item => item.cancel());
  };
  motions.set(element, cleanup);
  void animation.finished.then(() => {
    if (motions.get(element) !== cleanup) return;
    cleanup(); done();
  }, () => {});
}
function enter(element: Element, done: () => void) {
  position.value = { x: 0, y: 0 };
  layer = element as HTMLElement;
  trigger = document.activeElement instanceof HTMLElement ? document.activeElement : undefined;
  const rect = trigger && trigger !== document.body ? trigger.getBoundingClientRect() : undefined;
  origin = props.origin ? { ...props.origin, height: props.origin.height ?? props.origin.size } : rect && rect.width && rect.height
    ? { x: rect.x, y: rect.y, size: rect.width, height: rect.height }
    : { x: window.innerWidth / 2, y: window.innerHeight / 2, size: 1, height: 1 };
  panelLayers.value = [...panelLayers.value, layer];
  transition(element, done, true);
  void nextTick(() => panel.value?.focus());
}
const leave = (element: Element, done: () => void) => transition(element, done, false);
const close = () => { if (props.show && topmost.value && !props.busy) emit('close'); };
onClickOutside(panel, close, { ignore: ['.v-binder-follower-content'] });
</script>
<template>
  <Teleport to=".app-scene">
    <Transition :css="false" appear @enter="enter" @leave="leave" @after-leave="closed" @enter-cancelled="cancelTransition" @leave-cancelled="cancelTransition">
      <div v-if="show" class="panel-layer" :class="{ recessed: !topmost }" :style="{zIndex:55 + Math.max(0,panelLayers.indexOf(layer!)) * 2}" :inert="!topmost">
      <section ref="panel" class="elastic-panel" :style="style" tabindex="-1" role="dialog" aria-modal="true" :aria-label="title" @keydown.esc.stop="close">
        <header class="panel-header" :class="{ dragging }" @pointerdown="startDrag" @pointermove="moveDrag" @pointerup="endDrag" @pointercancel="endDrag" @lostpointercapture="endDrag"><h1>{{title}}</h1><div class="panel-actions"><slot name="actions"/><NButton quaternary circle :disabled="busy" :aria-label="t('close')" :title="t('close')" @click="close"><template #icon><X :size="22"/></template></NButton></div></header>
        <div class="panel-content"><slot/></div>
        <footer v-if="$slots.footer" class="panel-footer"><slot name="footer"/></footer>
      </section>
      </div>
    </Transition>
  </Teleport>
</template>
<style scoped>
.panel-layer{position:fixed;inset:0;display:flex;align-items:center;justify-content:center;padding:20px;box-sizing:border-box}
.panel-layer.recessed .elastic-panel{box-shadow:0 4px 12px light-dark(#132a2414,#00000024),0 12px 32px light-dark(#132a241c,#00000038)}
.elastic-panel{position:relative;flex-shrink:0;width:min(var(--preferred-width),calc(100vw - 40px));max-height:calc(100dvh - 40px);box-sizing:border-box;z-index:55;background:var(--surface);border:1px solid var(--line);border-radius:28px;box-shadow:0 2px 6px light-dark(#132a2414,#00000033),0 12px 28px light-dark(#132a2426,#0000004d),0 32px 72px -12px light-dark(#132a2438,#00000080);transition:width 320ms cubic-bezier(.22,1,.36,1);display:flex;flex-direction:column;overflow:hidden;outline:none;transform-origin:center;transform:translate(0,0) scale(1,1)}
.panel-header{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:24px 28px 12px;flex-shrink:0}.panel-header h1{font-size:22px;font-weight:600;margin:0}.panel-actions{display:flex;align-items:center;gap:8px}
.panel-header{cursor:grab;user-select:none;touch-action:none}.panel-header.dragging{cursor:grabbing}.panel-actions{cursor:default;touch-action:auto}
.panel-content{padding:0 28px 12px;overflow:auto;overscroll-behavior:contain;min-height:0;flex:0 1 auto}
.panel-footer{display:flex;justify-content:flex-end;align-items:center;gap:8px;padding:12px 28px 20px;flex-shrink:0}.panel-footer :slotted(.action-spacer){flex:1}
@media(prefers-reduced-motion:reduce){.elastic-panel{transition:none}}
</style>
