<script setup lang="ts">
import { t } from '../i18n';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import logo from '../assets/local-connector-head.png';
import happy from '../assets/mascot/happy.png';
import surprised from '../assets/mascot/surprised.png';
import pout from '../assets/mascot/pout.png';
import closed from '../assets/mascot/closed.png';
import cross from '../assets/mascot/cross.png';
import squeezed from '../assets/mascot/squeezed.png';
import dizzy from '../assets/mascot/dizzy.png';

const props = defineProps<{ state: 'connected' | 'offline' | 'connecting' | 'stopping' | 'degraded' | 'error' | 'unavailable' | 'working' }>();
const stateExpressions = { connected: logo, offline: closed, connecting: surprised, stopping: squeezed,
  degraded: pout, error: cross, unavailable: dizzy, working: happy };
const expressions = [happy, surprised, pout, closed, cross, squeezed, dizzy];
const interactionExpression = ref<string>();
const stateExpression = computed(() => stateExpressions[props.state]);
const expression = computed(() => interactionExpression.value
  || (readyExpressions.value.includes(stateExpression.value) ? stateExpression.value : logo));
const sprites = [logo, ...expressions];
const spriteElements = ref<HTMLImageElement[]>([]);
const readyExpressions = ref<string[]>([]);
const head = ref<HTMLElement>();
let clickAnimation: Animation | undefined;
let lastEffect = -1;
let disposed = false;
let timer: ReturnType<typeof setTimeout> | undefined;

watch(() => props.state, () => {
  clearTimeout(timer);
  interactionExpression.value = undefined;
});

onMounted(() => {
  // Decode the actual resident nodes; never switch to a sprite that is not ready.
  spriteElements.value.forEach(image => {
    void image.decode().then(() => {
      if (!disposed) readyExpressions.value.push(image.getAttribute('src')!);
    }).catch(() => { /* Keep the current expression if this asset cannot decode. */ });
  });
});

function changeExpression() {
  clearTimeout(timer);
  const choices = readyExpressions.value.filter(src => src !== stateExpression.value && src !== expression.value);
  if (!choices.length) return;
  interactionExpression.value = choices[Math.floor(Math.random() * choices.length)]!;
}

function restoreExpressionLater() {
  clearTimeout(timer);
  timer = setTimeout(() => { interactionExpression.value = undefined; }, 3000 + Math.random() * 5000);
}

const position = ref({ x: 0, y: 0 });
const dragging = ref(false);
const headStyle = computed(() => ({
  transform: `translate(${position.value.x}px, ${position.value.y}px) rotate(${position.value.x * .08}deg)`,
}));
const reach = 140;
let frame = 0;
let suppressClick = false;
let drag: { pointerId: number; x: number; y: number; offsetX: number; offsetY: number; moved: boolean } | undefined;

function returnHome() {
  cancelAnimationFrame(frame);
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    position.value = { x: 0, y: 0 };
    return;
  }
  const origin = { ...position.value };
  const started = performance.now();
  function step(now: number) {
    const seconds = (now - started) / 1000;
    // A taut rubber band: fast recoil and several progressively smaller swings.
    const decay = Math.exp(-4 * seconds);
    const spring = decay * (Math.cos(22 * seconds) + 4 / 22 * Math.sin(22 * seconds));
    position.value = { x: origin.x * spring, y: origin.y * spring };
    if (seconds < 1.8) frame = requestAnimationFrame(step);
    else { position.value = { x: 0, y: 0 }; frame = 0; }
  }
  frame = requestAnimationFrame(step);
}

function startDrag(event: PointerEvent) {
  if (!event.isPrimary || event.button !== 0 || drag) return;
  cancelAnimationFrame(frame);
  clearTimeout(timer);
  clickAnimation?.cancel();
  suppressClick = false;
  // Invert the resistance curve so catching the returning head never jumps.
  const scale = 1 / (1 - Math.min(Math.hypot(position.value.x, position.value.y) / reach, .999));
  drag = { pointerId: event.pointerId, x: event.clientX, y: event.clientY,
    offsetX: position.value.x * scale, offsetY: position.value.y * scale, moved: false };
  dragging.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}

function moveDrag(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  const dx = event.clientX - drag.x, dy = event.clientY - drag.y;
  if (!drag.moved && Math.hypot(dx, dy) < 4) return;
  if (!drag.moved) changeExpression();
  drag.moved = true;
  const x = drag.offsetX + dx, y = drag.offsetY + dy;
  // Equal mouse travel produces progressively less movement in every direction.
  const resistance = 1 + Math.hypot(x, y) / reach;
  position.value = { x: x / resistance, y: y / resistance };
}

function endDrag(event: PointerEvent) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  suppressClick = drag.moved || event.type !== 'pointerup';
  drag = undefined;
  dragging.value = false;
  const button = event.currentTarget as HTMLElement;
  if (button.hasPointerCapture(event.pointerId)) button.releasePointerCapture(event.pointerId);
  returnHome();
  restoreExpressionLater();
}

function reactToClick(event: MouseEvent) {
  if (suppressClick && event.detail !== 0) { suppressClick = false; return; }
  if (drag) return;
  changeExpression();
  clickAnimation?.cancel();
  if (!window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    const direction = Math.random() < .5 ? -1 : 1;
    const x = direction * (8 + Math.random() * 8);
    const y = -(15 + Math.random() * 12);
    const tilt = direction * (10 + Math.random() * 9);
    const rest = 'rotate(-7deg)';
    const effects = [
      { name: 'bounce', duration: 650, frames: [
        { offset: 0, transform: rest },
        { offset: .14, transform: 'translateY(5px) rotate(-10deg) scale(1.06, .92)' },
        { offset: .36, transform: `translate(${x}px, ${y}px) rotate(${tilt}deg) scale(.95, 1.06)` },
        { offset: .58, transform: `translate(${-x * .35}px, 2px) rotate(-13deg) scale(1.04, .96)` },
        { offset: .77, transform: `translate(${x * .2}px, -7px) rotate(-2deg)` },
        { offset: 1, transform: rest },
      ] },
      { name: 'spin', duration: 850, frames: [
        { offset: 0, transform: rest },
        { offset: .16, transform: `rotate(${-7 - direction * 18}deg) scale(.94)` },
        { offset: .76, transform: `rotate(${-7 + direction * 375}deg) scale(1.05)` },
        { offset: 1, transform: `rotate(${-7 + direction * 360}deg)` },
      ] },
      { name: 'dodge', duration: 720, frames: [
        { offset: 0, transform: rest },
        { offset: .22, transform: `translate(${direction * 42}px, -12px) rotate(${direction * 20 - 7}deg) scale(.9)` },
        { offset: .46, transform: `translate(${direction * 38}px, -9px) rotate(${direction * 16 - 7}deg) scale(.92)` },
        { offset: .76, transform: `translate(${-direction * 9}px, 2px) rotate(${-direction * 8 - 7}deg)` },
        { offset: 1, transform: rest },
      ] },
      { name: 'flip', duration: 800, frames: [
        { offset: 0, transform: 'perspective(500px) rotateY(0deg) rotate(-7deg)' },
        { offset: .16, transform: `perspective(500px) rotateY(${-direction * 20}deg) rotate(-7deg)` },
        { offset: .8, transform: `perspective(500px) rotateY(${direction * 380}deg) rotate(-7deg)` },
        { offset: 1, transform: `perspective(500px) rotateY(${direction * 360}deg) rotate(-7deg)` },
      ] },
      { name: 'shake', duration: 600, frames: [
        { offset: 0, transform: rest },
        { offset: .18, transform: `translateX(${-direction * 10}px) rotate(${-7 - direction * 18}deg)` },
        { offset: .36, transform: `translateX(${direction * 9}px) rotate(${-7 + direction * 16}deg)` },
        { offset: .54, transform: `translateX(${-direction * 6}px) rotate(${-7 - direction * 11}deg)` },
        { offset: .73, transform: `translateX(${direction * 3}px) rotate(${-7 + direction * 6}deg)` },
        { offset: 1, transform: rest },
      ] },
      { name: 'jelly', duration: 750, frames: [
        { offset: 0, transform: rest },
        { offset: .2, transform: 'translateY(7px) rotate(-7deg) scale(1.22, .78)' },
        { offset: .4, transform: 'translateY(-7px) rotate(-7deg) scale(.84, 1.17)' },
        { offset: .6, transform: 'translateY(3px) rotate(-7deg) scale(1.1, .91)' },
        { offset: .8, transform: 'rotate(-7deg) scale(.96, 1.04)' },
        { offset: 1, transform: rest },
      ] },
    ];
    const choices = effects.map((_, index) => index).filter(index => index !== lastEffect);
    lastEffect = choices[Math.floor(Math.random() * choices.length)]!;
    const effect = effects[lastEffect]!;
    clickAnimation = head.value?.animate(effect.frames, { duration: effect.duration, easing: 'ease-out' });
    if (clickAnimation) clickAnimation.id = `mascot-${effect.name}`;
  }
  restoreExpressionLater();
}
onUnmounted(() => { disposed = true; clearTimeout(timer); cancelAnimationFrame(frame); clickAnimation?.cancel(); });
</script>

<template>
  <div class="mascot">
    <button class="mascot-button" :class="{ dragging }" :style="headStyle" type="button" :aria-label="t('pokeTheMascot')"
      @pointerdown="startDrag" @pointermove="moveDrag" @pointerup="endDrag"
      @pointercancel="endDrag" @lostpointercapture="endDrag" @click="reactToClick">
      <span ref="head" class="mascot-head" aria-hidden="true">
        <img v-for="src in sprites" :key="src" ref="spriteElements" :src="src" alt="" draggable="false"
          :style="{ visibility: src === expression ? 'visible' : 'hidden' }" />
      </span>
    </button>
  </div>
</template>

<style scoped>
.mascot { position: relative; z-index: 1; isolation: isolate; width: 174px; height: 174px; }
.mascot::before { content: ''; position: absolute; inset: 0; z-index: -1; border-radius: 42px; background: var(--accent-soft); box-shadow: 0 24px 50px color-mix(in srgb,var(--black) 7.06%,transparent); transform: rotate(-7deg); pointer-events: none; }
.mascot-button { display: block; width: 100%; height: 100%; padding: 0; border: 0; border-radius: 42px; background: transparent; cursor: grab; transition: none; touch-action: none; user-select: none; -webkit-tap-highlight-color: transparent; }
.mascot-button:hover, .mascot-button:active { background: transparent; }
.mascot-button.dragging { cursor: grabbing; }
.mascot-head { display: block; position: relative; width: 100%; height: 100%; transform: rotate(-7deg); pointer-events: none; }
.mascot-button img { position: absolute; inset: 0; display: block; width: 100%; height: 100%; padding: 10px; object-fit: contain; pointer-events: none; user-select: none; }
@media (max-width: 700px) {
  .mascot { width: 112px; height: 112px; }
  .mascot::before, .mascot-button { border-radius: 28px; }
  .mascot-button img { padding: 6px; }
}
</style>
