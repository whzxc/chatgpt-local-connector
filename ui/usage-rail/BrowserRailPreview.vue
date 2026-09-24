<script setup lang="ts">
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue';
import { useEventListener, useLocalStorage, useWindowSize } from '@vueuse/core';
import m from '../../shared/usage-panel.json';
import { NDropdown } from 'naive-ui';
import { t } from '../i18n';
import { panelPreferences as preferences, readPanelPreferences, savePanelPreferences } from './preferences';
import { requestedAgentSettings, requestedProvider } from '../subscriptions/navigation';
import { subscriptionSnapshotKey } from '../subscriptions/useSubscriptions';
import { clamp, type Point } from './geometry';
import type { Dock, PanelGeometry, PanelState } from './layout';
import UsageRail from './UsageRail.vue';

// The browser supplies a viewport and pointer input. UsageRail owns
// every painted shape, ring, detail card, animation and pagination control.
const snapshot = inject(subscriptionSnapshotKey)!;
const count = computed(() => snapshot.value?.settings.enabled
  ? snapshot.value.providers.filter(p => p.eligible && p.selected).length : 0);
const { width, height } = useWindowSize();
const savedPosition = useLocalStorage('clc-dev-usage-rail-position', { x: window.innerWidth - 96, y: 96 });
const position = ref({ ...savedPosition.value });
const dock = ref<Dock>('floating');
const expanded = ref(true);
const surface = ref<HTMLElement>();
let geometry: PanelGeometry | undefined;
function acceptGeometry(value: PanelGeometry) { geometry = value; }
const providerId = ref<string | null>(null);
const pressed = ref(false);
const layout = computed(() => {
  const p = preferences.value;
  const scale = p.size === 'small' ? m.smallScale : p.size === 'large' ? m.largeScale : 1;
  const horizontal = dock.value === 'top' || dock.value === 'bottom';
  const floating = dock.value === 'floating';
  const percentages = !horizontal || p.horizontalPercentages;
  const item = (horizontal ? percentages ? 38 : 36 : m.groupHeight) * scale;
  let padding = 54 * scale
    - (floating ? 32 * scale : 0);
  let gap = m.groupGap * (p.spacing === 'compact' ? .6 : p.spacing === 'roomy' ? 1.4 : 1) * scale;
  const n = Math.max(1, count.value), available = Math.max(1, horizontal ? width.value : height.value);
  // Match the native layout: compress gaps, then padding, then page.
  if (2 * padding + item * n + gap * (n - 1) > available) {
    if (n > 1) gap = Math.min(gap, Math.max(0, (available - 2 * padding - item * n) / (n - 1)));
    padding = Math.min(padding, Math.max((floating ? 12 : 36) * scale, (available - item * n - gap * (n - 1)) / 2));
  }
  const footer = 2 * padding + item * n + gap * (n - 1) > available ? 24 * scale : 0;
  const visibleCount = footer ? Math.max(1, Math.floor((available - 2 * padding - footer) / (item + gap))) : n;
  const length = 2 * padding + item * visibleCount + gap * (visibleCount - 1) + footer;
  const thickness = (horizontal && percentages ? 78 : m.railWidth) * scale;
  const railWidth = horizontal ? length : thickness, railHeight = horizontal ? thickness : length;
  const maxX = Math.max(0, width.value - railWidth), maxY = Math.max(0, height.value - railHeight);
  const x = dock.value === 'left' ? 0 : dock.value === 'right' ? maxX : clamp(position.value.x, 0, maxX);
  const y = dock.value === 'top' ? 0 : dock.value === 'bottom' ? maxY : clamp(position.value.y, 0, maxY);
  return {
    width: width.value, height: height.value, rail: { x, y, width: railWidth, height: railHeight },
    edge: floating ? (x + railWidth / 2 > width.value / 2 ? 'right' as const : 'left' as const) : dock.value as Exclude<Dock, 'floating'>,
    metrics: { visibleCount, footer, scale, length, thickness, padding, pitch: item + gap,
      item, percentages, round: true, horizontal },
  };
});
const state = computed<PanelState>(() => ({
  expanded: dock.value === 'floating' || pressed.value || expanded.value, slot: null, providerId: providerId.value, generation: 1,
  display: 'browser', dock: dock.value, pressed: pressed.value,
  layout: count.value ? layout.value : null, preferences: preferences.value,
}));
watch([width, height, () => layout.value.rail.height], () => {
  if (dock.value !== 'floating') return;
  const { x, y } = layout.value.rail;
  if (position.value.x !== x || position.value.y !== y) position.value = { x, y };
});
let stopped = false;
async function readPreferences() {
  try {
    if (!stopped) await readPanelPreferences();
  } catch { /* The preview remains usable while the native panel is unavailable. */ }
}
onMounted(readPreferences);
useEventListener(window, 'focus', readPreferences);

function hit(points: Point[], x: number, y: number) {
  let inside = false;
  for (let i = 0, j = points.length - 1; i < points.length; j = i++) {
    const a = points[i]!, b = points[j]!;
    if ((a[1] > y) !== (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]) inside = !inside;
  }
  return inside;
}
let leaveTimer: ReturnType<typeof setTimeout> | undefined;
function cancelLeave() { clearTimeout(leaveTimer); leaveTimer = undefined; }
function leave() {
  if (drag || leaveTimer || menu.value) return;
  leaveTimer = setTimeout(() => {
    leaveTimer = undefined;
    if (drag) return;
    providerId.value = null;
    expanded.value = false;
  }, m.leaveMs);
}
let cursor: Pick<PointerEvent, 'clientX' | 'clientY'> | undefined;
function hover(event: Pick<PointerEvent, 'clientX' | 'clientY'>) {
  cursor = event;
  const g = geometry;
  if (!g || drag || menu.value) return;
  const x = event.clientX, y = event.clientY;
  const ring = g.rings.find(r => Math.hypot(x - r.x, y - r.y) <= r.radius);
  // Observe a wider wake zone without blocking clicks on unpainted pixels.
  const { rail, metrics } = layout.value;
  const wakeLength = m.collapsedLength * metrics.scale / 2, wakeWidth = m.wakeWidth * metrics.scale;
  const near = dock.value === 'left' ? x <= wakeWidth
    : dock.value === 'right' ? x >= width.value - wakeWidth
    : dock.value === 'top' ? y <= wakeWidth
    : dock.value === 'bottom' ? y >= height.value - wakeWidth : false;
  const wake = near && (metrics.horizontal
    ? Math.abs(x - rail.x - rail.width / 2) <= wakeLength
    : Math.abs(y - rail.y - rail.height / 2) <= wakeLength);
  if (ring || wake || hit(g.rail, x, y) || hit(g.detail, x, y) || hit(g.corridor, x, y)) {
    cancelLeave(); expanded.value = true;
    if (ring) providerId.value = ring.providerId;
  }
  else leave();
}
let drag: { id: number; startX: number; startY: number; grabX: number; grabY: number; moved: boolean } | undefined;
let suppressClick = false;
function down(event: PointerEvent) {
  const g = geometry;
  if (event.button !== 0 || !event.isPrimary || !g || !hit(g.rail, event.clientX, event.clientY)
    || hit(g.controls, event.clientX, event.clientY)) return;
  cancelLeave(); suppressClick = false; expanded.value = true; cursor = event;
  drag = { id: event.pointerId, startX: event.clientX, startY: event.clientY,
    grabX: (event.clientX - layout.value.rail.x) / layout.value.rail.width,
    grabY: (event.clientY - layout.value.rail.y) / layout.value.rail.height, moved: false };
}
useEventListener(window, 'pointermove', event => {
  cursor = event;
  if (!drag || event.pointerId !== drag.id) { hover(event); return; }
  const dx = event.clientX - drag.startX, dy = event.clientY - drag.startY;
  if (!drag.moved && Math.hypot(dx, dy) < m.dragThreshold) return;
  if (!drag.moved) surface.value?.setPointerCapture(event.pointerId);
  drag.moved = true; pressed.value = true; providerId.value = null;
  // Viewport edge zones only; native screen/window management stays native.
  const edges: { edge: Exclude<Dock, 'floating'>; distance: number }[] = [
    { edge: 'left', distance: event.clientX }, { edge: 'right', distance: width.value - event.clientX },
    { edge: 'top', distance: event.clientY }, { edge: 'bottom', distance: height.value - event.clientY },
  ];
  const nearest = edges.sort((a, b) => a.distance - b.distance)[0]!;
  dock.value = nearest.distance < (dock.value === nearest.edge ? m.undockDistance : m.dockDistance)
    ? nearest.edge : 'floating';
  position.value = {
    x: clamp(event.clientX - drag.grabX * layout.value.rail.width, 0, Math.max(0, width.value - layout.value.rail.width)),
    y: clamp(event.clientY - drag.grabY * layout.value.rail.height, 0, Math.max(0, height.value - layout.value.rail.height)),
  };
});
function release() {
  if (!drag) return;
  suppressClick = drag.moved;
  savedPosition.value = { x: layout.value.rail.x, y: layout.value.rail.y };
  const id = drag.id;
  drag = undefined; pressed.value = false;
  if (surface.value?.hasPointerCapture(id)) surface.value.releasePointerCapture(id);
  if (cursor) hover(cursor);
}
useEventListener(window, ['pointerup', 'pointercancel'], event => { if (event.pointerId === drag?.id) release(); });
useEventListener(window, 'blur', () => { release(); cancelLeave(); providerId.value = null; expanded.value = false; });
useEventListener(document, 'pointerleave', leave);
function click(event: MouseEvent) {
  if (suppressClick) { event.preventDefault(); event.stopPropagation(); suppressClick = false; }
}
function focus(event: FocusEvent) {
  const id = (event.target as HTMLElement).dataset.providerId;
  if (id) { cancelLeave(); expanded.value = true; providerId.value = id; }
}
function open(id: string) { if (!suppressClick) requestedProvider.value = id; }
const menu = ref(false), menuX = ref(0), menuY = ref(0);
const menuOptions = computed(() => [
  {key:'settings',label:t('settings')},
  {key:'reset',label:t('usageRestoreDefaults')},
  {key:'hide',label:t('usageHide')},
]);
const menuError = ref('');
function contextMenu(event:MouseEvent) {
  cancelLeave(); menuX.value=event.clientX;menuY.value=event.clientY;menu.value=true;
}
async function selectMenu(key:string) {
  menu.value=false; menuError.value='';
  try {
    if(key==='settings') requestedAgentSettings.value=true;
    else if(key==='reset') {
      await savePanelPreferences({resetDefaults:true});
      dock.value='right'; position.value={x:width.value,y:height.value/2-layout.value.rail.height/2};
      savedPosition.value={...position.value};providerId.value=null;
    } else if(key==='hide') await savePanelPreferences({visible:false});
  } catch(e) { menuError.value=String(e); }
}
onUnmounted(() => { stopped = true; cancelLeave(); });
</script>

<template>
  <div v-if="preferences.visible !== false" ref="surface" class="browser-rail-preview" @pointerdown="down" @lostpointercapture="release"
    @contextmenu.prevent="contextMenu" @click.capture="click" @focusin="focus" @focusout="leave" @keydown.esc="providerId = null">
    <NDropdown trigger="manual" :show="menu" :x="menuX" :y="menuY" :options="menuOptions" @select="selectMenu" @clickoutside="menu=false"/>
    <div v-if="menuError" role="alert" class="rail-menu-error">{{menuError}}</div>
    <UsageRail :state="state" @geometry="acceptGeometry" @open="open"/>
  </div>
</template>

<style scoped>
.rail-menu-error{position:fixed;bottom:16px;left:16px;color:var(--ink);background:var(--surface);padding:12px;pointer-events:auto}
.browser-rail-preview{position:fixed;inset:0;z-index:20;pointer-events:none}
</style>
