<script setup lang="ts">
import { onUnmounted, ref } from 'vue';
import logo from '../assets/local-connector-head.png';

const faces = ['(≧▽≦)', '(｡•̀ᴗ-)✧', 'ヽ(✿ﾟ▽ﾟ)ノ', '(๑˃̵ᴗ˂̵)و', '(ﾉ◕ヮ◕)ﾉ*:･ﾟ✧', '( •̀ ω •́ )✧', '(づ｡◕‿‿◕｡)づ'];
const face = ref('');
const reaction = ref(0);
const motion = ref<Record<string, string>>({});
let timer: ReturnType<typeof setTimeout> | undefined;

function bounce() {
  clearTimeout(timer);
  const choices = faces.filter(value => value !== face.value);
  face.value = choices[Math.floor(Math.random() * choices.length)]!;
  const direction = Math.random() < .5 ? -1 : 1;
  motion.value = {
    '--hop-x': `${direction * (8 + Math.random() * 8)}px`,
    '--hop-y': `${-(15 + Math.random() * 12)}px`,
    '--hop-tilt': `${direction * (10 + Math.random() * 9)}deg`,
  };
  reaction.value++;
  timer = setTimeout(() => { face.value = ''; }, 1250);
}
onUnmounted(() => clearTimeout(timer));
</script>

<template>
  <div class="mascot" :style="motion">
    <button class="mascot-button" type="button" aria-label="戳一下脑袋" @click="bounce">
      <img :key="reaction" :src="logo" alt="" draggable="false" :class="{ 'mascot-bouncing': face }" />
    </button>
    <span v-if="face" :key="reaction" class="mascot-face" aria-hidden="true">{{ face }}</span>
  </div>
</template>

<style scoped>
.mascot { position: relative; z-index: 1; isolation: isolate; width: 174px; height: 174px; }
.mascot::before { content: ''; position: absolute; inset: 0; z-index: -1; border-radius: 42px; background: linear-gradient(145deg, #d4e4dc, #e4eee4); box-shadow: 0 24px 50px #3f614b12; transform: rotate(-7deg); pointer-events: none; }
.mascot-button { display: block; width: 100%; height: 100%; padding: 0; border: 0; border-radius: 42px; background: transparent; cursor: pointer; touch-action: manipulation; -webkit-tap-highlight-color: transparent; }
.mascot-button:hover, .mascot-button:active { background: transparent; transform: none; }
.mascot-button img { display: block; width: 100%; height: 100%; padding: 10px; object-fit: contain; transform: rotate(-7deg); pointer-events: none; user-select: none; }
.mascot-bouncing { animation: mascot-hop 650ms ease-out both; }
.mascot-face { position: absolute; bottom: calc(100% + 12px); left: 50%; width: max-content; max-width: 270px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface); color: var(--green); font-size: 15px; font-weight: 600; line-height: 22px; white-space: nowrap; box-shadow: 0 4px 14px #233b3510; pointer-events: none; animation: mascot-face 1250ms ease-out both; }
@keyframes mascot-hop {
  0%, 100% { transform: translate(0, 0) rotate(-7deg) scale(1); }
  14% { transform: translateY(5px) rotate(-10deg) scale(1.06, .92); }
  36% { transform: translate(var(--hop-x), var(--hop-y)) rotate(var(--hop-tilt)) scale(.95, 1.06); }
  58% { transform: translate(calc(var(--hop-x) * -.35), 2px) rotate(-13deg) scale(1.04, .96); }
  77% { transform: translate(calc(var(--hop-x) * .2), -7px) rotate(-2deg); }
}
@keyframes mascot-face {
  0% { opacity: 0; transform: translate(-50%, 7px) scale(.9); }
  18%, 65% { opacity: 1; transform: translate(-50%, 0) scale(1); }
  100% { opacity: 0; transform: translate(-50%, -10px) scale(1); }
}
@media (max-width: 700px) {
  .mascot { width: 112px; height: 112px; }
  .mascot::before, .mascot-button { border-radius: 28px; }
  .mascot-button img { padding: 6px; }
  .mascot-face { font-size: 11px; padding: 4px 6px; }
}
@media (prefers-reduced-motion: reduce) {
  .mascot-face { transform: translateX(-50%); }
}
</style>
