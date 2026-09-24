<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { NButton } from 'naive-ui';
import { X } from '@lucide/vue';
import { panelLayers, panelHandoff, panelAnchorAt } from '../composables/panels';
import { t } from '../i18n';
const props = withDefaults(defineProps<{ show: boolean; title: string; origin?: { x: number; y: number; size: number; height?: number }; busy?: boolean; continuation?: boolean; headerless?: boolean; unframed?: boolean; width?: number }>(), { width: 800 });
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
    if (motions.size) {
      resizing?.cancel(); resizing = undefined;
      height = node.offsetHeight;
      return;
    }
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
let replacement: HTMLElement | undefined;
let replacing = false;
let handoffBounds: DOMRect | undefined;
let anchorElement: HTMLElement | undefined;
function closed() {
  release(); emit('closed');
  void nextTick(() => {
    if (replacement?.isConnected) {
      const target = replacement;
      replacement = undefined;
      panelHandoff.trigger = target;
      target.focus({ preventScroll: true });
      target.click();
      void nextTick(() => { panelHandoff.from = undefined; panelHandoff.trigger = undefined; });
    } else if (!replacing && !props.continuation && trigger?.isConnected) trigger.focus({ preventScroll: true });
  });
}
const motions = new Map<Element, () => void>();
function cancelTransition(element: Element) { motions.get(element)?.(); }
onBeforeUnmount(() => { release(); motions.forEach(cancel => cancel()); });
function transition(element: Element, done: () => void, opening: boolean) {
  cancelTransition(element);
  const node = element.querySelector<HTMLElement>('.elastic-panel')!;
  if (!opening && props.continuation) {
    panelHandoff.from ??= node.getBoundingClientRect();
    done(); return;
  }
  if (!opening && !replacing) {
    const index = panelLayers.value.indexOf(layer!);
    const previous = index > 0 ? panelLayers.value[index - 1]?.querySelector<HTMLElement>('.elastic-panel') : undefined;
    if (previous) {
      const from = node.getBoundingClientRect();
      const to = previous.getBoundingClientRect();
      if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
        const rest = getComputedStyle(previous).transform;
        const motion = previous.animate([
          {transform:`translate(${from.x + from.width/2 - window.innerWidth/2}px,${from.y + from.height/2 - window.innerHeight/2}px) scale(${from.width/Math.max(1,to.width)},${from.height/Math.max(1,to.height)})`,opacity:1},
          {transform:rest,opacity:1},
        ], {duration:320,easing:'cubic-bezier(.22,1,.36,1)',fill:'both'});
        const contents = Array.from(previous.children, child => child.animate([{opacity:0},{opacity:1}],{duration:320,fill:'both'}));
        void motion.finished.then(() => {motion.cancel();contents.forEach(item=>item.cancel());},()=>{contents.forEach(item=>item.cancel());});
      }
      done(); return;
    }
  }
  if ((!opening && replacing) || window.matchMedia('(prefers-reduced-motion: reduce)').matches) { done(); return; }
  const { x, y, size, height: originHeight } = origin;
  const width = Math.max(1, node.offsetWidth), height = Math.max(1, node.offsetHeight);
  const source = opening && !handoffBounds ? anchorElement : undefined;
  const sourceStyle = source ? getComputedStyle(source) : undefined;
  const targetStyle = getComputedStyle(node);
  const scaleX = size / width, scaleY = originHeight / height;
  const sourceRadius = sourceStyle ? parseFloat(sourceStyle.borderTopLeftRadius) || 0 : 0;
  const collapsed = {
    transform: `translate(${x + size / 2 - window.innerWidth / 2}px,${y + originHeight / 2 - window.innerHeight / 2}px) scale(${size / width},${originHeight / height})`,
    opacity: source ? 1 : 0,
    borderRadius: source ? `${sourceRadius / scaleX}px / ${sourceRadius / scaleY}px` : targetStyle.borderRadius,
    backgroundColor: sourceStyle && sourceStyle.backgroundColor !== 'rgba(0, 0, 0, 0)' ? sourceStyle.backgroundColor : targetStyle.backgroundColor,
    boxShadow: source ? 'none' : targetStyle.boxShadow,
  };
  const expanded = { transform: `translate(${position.value.x}px,${position.value.y}px) scale(1,1)`, opacity: 1, borderRadius: targetStyle.borderRadius, backgroundColor: targetStyle.backgroundColor, boxShadow: targetStyle.boxShadow };
  const duration = opening ? (handoffBounds ? 320 : 480) : 220;
  node.style.willChange = 'transform, opacity';
  const frames = opening && handoffBounds
    ? [{ transform: `translate(${handoffBounds.x + handoffBounds.width / 2 - window.innerWidth / 2}px,${handoffBounds.y + handoffBounds.height / 2 - window.innerHeight / 2}px) scale(${handoffBounds.width / width},${handoffBounds.height / height})`, opacity: 1 }, expanded]
    : opening ? [collapsed, expanded] : [expanded, collapsed];
  const animation = node.animate(frames, {
    duration, easing: opening ? 'cubic-bezier(.22,1,.36,1)' : 'cubic-bezier(.4,0,.6,1)', fill: 'both',
  });
  let ghost: HTMLElement | undefined;
  let ghostAnimation: Animation | undefined;
  if (source) {
    ghost = source.cloneNode(true) as HTMLElement;
    const originals = [source, ...source.querySelectorAll('*')];
    const copies = [ghost, ...ghost.querySelectorAll('*')];
    originals.forEach((original, index) => {
      const copy = copies[index] as HTMLElement | SVGElement;
      const css = getComputedStyle(original);
      for (const property of css) copy.style.setProperty(property, css.getPropertyValue(property));
      copy.removeAttribute('id');
      copy.style.animation = 'none'; copy.style.transition = 'none';
    });
    ghost.inert = true;
    ghost.setAttribute('aria-hidden', 'true');
    Object.assign(ghost.style, {position:'fixed',left:`${x}px`,top:`${y}px`,right:'auto',bottom:'auto',width:`${size}px`,height:`${originHeight}px`,margin:'0',transform:'none',pointerEvents:'none',zIndex:'56'});
    element.appendChild(ghost);
    ghostAnimation = ghost.animate([
      {opacity:1,transform:'scale(1)'},
      {opacity:0,transform:'scale(1.3)'},
    ], {duration:200,easing:'ease-out',fill:'both'});
  }
  const contentAnimations = Array.from(node.children, child => child.animate(
    opening ? [{ opacity: 0 }, { opacity: 1 }] : [{ opacity: 1 }, { opacity: 0 }],
    { duration: opening && handoffBounds ? duration : opening ? 280 : 100, delay: opening && !handoffBounds ? 80 : 0, easing: 'cubic-bezier(.22,1,.36,1)', fill: 'both' },
  ));
  const cleanup = () => {
    motions.delete(element); animation.cancel(); node.style.willChange = '';
    contentAnimations.forEach(item => item.cancel());
    ghostAnimation?.cancel(); ghost?.remove();
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
  handoffBounds = panelHandoff.from ?? panelLayers.value.at(-1)?.querySelector<HTMLElement>('.elastic-panel')?.getBoundingClientRect();
  trigger = panelHandoff.trigger ?? (document.activeElement instanceof HTMLElement ? document.activeElement : undefined);
  panelHandoff.from = undefined; panelHandoff.trigger = undefined;
  replacing = false;
  const anchor = props.origin;
  const anchored = anchor && Number.isFinite(anchor.x) && Number.isFinite(anchor.y)
    && anchor.size > 0 && (anchor.height ?? anchor.size) > 0
    && anchor.x + anchor.size > 0 && anchor.y + (anchor.height ?? anchor.size) > 0
    && anchor.x < window.innerWidth && anchor.y < window.innerHeight;
  origin = anchored
    ? { ...anchor, height: anchor.height ?? anchor.size }
    : { x: (window.innerWidth - 1) / 2, y: (window.innerHeight - 1) / 2, size: 1, height: 1 };
  anchorElement = anchored ? Array.from(document.querySelectorAll<HTMLElement>('button, a, [role="button"], [data-panel-anchor]')).find(candidate => {
    if (element.contains(candidate)) return false;
    const rect = candidate.getBoundingClientRect();
    return Math.abs(rect.x - origin.x) < 2 && Math.abs(rect.y - origin.y) < 2
      && Math.abs(rect.width - origin.size) < 2 && Math.abs(rect.height - origin.height) < 2;
  }) : undefined;
  panelLayers.value = [...panelLayers.value, layer];
  transition(element, done, true);
  void nextTick(() => panel.value?.focus());
}
const leave = (element: Element, done: () => void) => transition(element, done, false);
const close = () => { if (props.show && topmost.value && !props.busy) emit('close'); };
// Only our backdrop can dismiss the panel. Browser annotation overlays and
// teleported popovers are outside the panel DOM, but are not backdrop clicks.
let backdropPressed = false;
function isBackdrop(event: MouseEvent) {
  if (event.target !== event.currentTarget) return false;
  const rect = panel.value?.getBoundingClientRect();
  return !!rect && (event.clientX < rect.left || event.clientX > rect.right
    || event.clientY < rect.top || event.clientY > rect.bottom);
}
function backdropDown(event: PointerEvent) { backdropPressed = event.button === 0 && isBackdrop(event); }
function cancelBackdrop() { backdropPressed = false; }
function backdropClick(event: MouseEvent) {
  const dismiss = backdropPressed && isBackdrop(event);
  backdropPressed = false;
  if (!dismiss || !props.show || !topmost.value || props.busy) return;
  const target = panelAnchorAt(event.clientX, event.clientY, trigger);
  if (target && panel.value) {
    panelHandoff.from = panel.value.getBoundingClientRect();
    replacement = target;
    replacing = true;
  }
  close();
}
</script>
<template>
  <Teleport defer to=".app-scene">
    <Transition :css="false" appear @enter="enter" @leave="leave" @after-leave="closed" @enter-cancelled="cancelTransition" @leave-cancelled="cancelTransition">
      <div v-if="show" class="panel-layer" :class="{ recessed: !topmost }" :style="{zIndex:55 + Math.max(0,panelLayers.indexOf(layer!)) * 2}" :inert="!topmost" @pointerdown="backdropDown" @pointercancel="cancelBackdrop" @click="backdropClick">
      <section ref="panel" class="elastic-panel" :class="{unframed}" :style="style" tabindex="-1" role="dialog" aria-modal="true" :aria-label="title" @keydown.esc.stop="close">
        <header v-if="!headerless" class="panel-header" :class="{ dragging }" @pointerdown="startDrag" @pointermove="moveDrag" @pointerup="endDrag" @pointercancel="endDrag" @lostpointercapture="endDrag"><div class="panel-title"><h1>{{title}}</h1><span v-if="$slots['title-actions']" class="panel-actions"><slot name="title-actions"/></span></div><div class="panel-actions"><slot name="actions"/><NButton quaternary circle :disabled="busy" :aria-label="t('close')" :title="t('close')" @click="close"><template #icon><X :size="22"/></template></NButton></div></header>
        <div class="panel-content" :class="{headerless, unframed, 'with-footer':!!$slots.footer}"><slot/></div>
        <footer v-if="$slots.footer" class="panel-footer"><slot name="footer"/></footer>
      </section>
      </div>
    </Transition>
  </Teleport>
</template>
<style scoped>
.panel-layer{position:fixed;inset:0;display:flex;align-items:center;justify-content:center;padding:20px;box-sizing:border-box}
.panel-layer.recessed{visibility:hidden;pointer-events:none}
.elastic-panel{position:relative;flex-shrink:0;width:min(var(--preferred-width),calc(100vw - 40px));max-height:calc(100dvh - 40px);box-sizing:border-box;z-index:55;background:var(--surface);border:1px solid var(--line);border-radius:28px;box-shadow:var(--shadow-panel);transition:width 320ms cubic-bezier(.22,1,.36,1);display:flex;flex-direction:column;overflow:hidden;outline:none;transform-origin:center;transform:translate(0,0) scale(1,1)}
.panel-header{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:28px 28px 12px;flex-shrink:0}.panel-header h1{font-size:22px;font-weight:600;margin:0}.panel-actions{display:flex;align-items:center;gap:8px}
.panel-title{display:flex;align-items:center;gap:10px}
.panel-header{cursor:grab;user-select:none;touch-action:none}.panel-header.dragging{cursor:grabbing}.panel-actions{cursor:default;touch-action:auto}
.panel-content{padding:0 28px 28px;overflow:auto;overscroll-behavior:contain;min-height:0;flex:0 1 auto}
.panel-content.headerless{padding-top:28px}
.elastic-panel.unframed{background:transparent;border:0;border-radius:22px;box-shadow:none}
.panel-content.unframed{padding:0;display:flex;overflow:hidden}
.panel-content.with-footer{padding-bottom:12px}
.panel-footer{display:flex;justify-content:flex-end;align-items:center;gap:8px;padding:12px 28px 28px;flex-shrink:0}.panel-footer :slotted(.action-spacer){flex:1}
@media(prefers-reduced-motion:reduce){.elastic-panel{transition:none}}
</style>
