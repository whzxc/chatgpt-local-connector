<script setup lang="ts">
import { displayMessage } from '../messages';
import { t } from '../i18n';
import { computed, onMounted, ref, nextTick } from 'vue';
import { NButton, NSwitch } from 'naive-ui';
import { Bot, RefreshCw, X, Maximize2, Minimize2 } from '@lucide/vue';
import { useAgents, icons, name, type Agent } from '../composables/useAgents';
const { agents, loaded, loading, saving, error, refresh, toggle } = useAgents();
const props=defineProps<{origin:{x:number;y:number;size:number}}>();
const open=ref(true);
const wide=ref(false);
const panel=ref<HTMLElement>();
const originStyle=computed(()=>({'--origin-x':`${props.origin.x}px`,'--origin-y':`${props.origin.y}px`,'--origin-size':`${props.origin.size}px`}));
const emit = defineEmits<{ close: [] }>();
const initialAgent: Agent = { agent: 'codex', displayName: 'Codex' };
const sorted = computed(() => [...agents.value]
  .sort((a, b) => Number(b.agent === 'codex') - Number(a.agent === 'codex') || Number(!!b.installed) - Number(!!a.installed) || name(a).localeCompare(name(b))));
const visible = computed(() => !loaded.value ? [initialAgent] : sorted.value);
onMounted(async () => { void refresh(); await nextTick(); panel.value?.focus(); });
</script>
<template>
  <Teleport to=".app-scene">
    <Transition name="agents-panel" appear @after-leave="emit('close')">
      <section v-if="open" ref="panel" class="agents-panel" :class="{wide}" :style="originStyle" tabindex="-1" role="region" aria-label="Agents" @keydown.esc.stop="open=false">
        <header class="agents-header">
          <h1>Agents</h1>
          <div class="agents-actions">
            <NButton quaternary circle :disabled="loading || !!saving" :aria-label="t('refreshAgents')" :title="t('refreshAgents')" :aria-busy="loading" @click="refresh(true)"><template #icon><RefreshCw :size="20" :class="{spinning:loading}"/></template></NButton>
            <NButton quaternary circle :aria-label="t('close')" :title="t('close')" @click="open=false"><template #icon><X :size="22"/></template></NButton>
          </div>
        </header>
        <div class="agents-content">
    <div v-for="agent in visible" :key="agent.agent" class="agent-row">
      <component :is="agent.installed ? 'label' : 'div'" :for="agent.installed ? `agent-${agent.agent}` : undefined" class="agent-info">
        <span v-if="icons[agent.agent]" class="agent-icon" v-html="icons[agent.agent]" aria-hidden="true" />
        <Bot v-else class="agent-fallback" :size="22" aria-hidden="true" />
        <span class="agent-copy">
          <span class="agent-name">{{ name(agent) }}<span v-if="agent.agent === 'codex'" class="agent-default">{{ t('default') }}</span></span>
          <span v-if="agent.installed && agent.version" class="agent-version">{{ agent.version }}</span>
        </span>
      </component>
      <NSwitch v-if="agent.installed" :id="`agent-${agent.agent}`"
        :aria-label="t('allowConnectorToUseValue', { agent: name(agent) })" :value="agent.agent === 'codex' || agent.enabled"
        :disabled="agent.agent === 'codex' || !!saving || loading || typeof agent.enabled !== 'boolean'"
        :title="agent.agent === 'codex' ? t('codexIsAlwaysEnabled') : t('allowConnectorToAcceptRequestsForThisAgent')"
        @update:value="toggle(agent, $event)" />
    </div>
    <p v-if="error" class="agent-error" role="alert">{{displayMessage(error)}}</p>
        </div>
        <NButton class="agents-width-toggle" quaternary circle :aria-label="t(wide ? 'restorePanelWidth' : 'expandPanelWidth')" :title="t(wide ? 'restorePanelWidth' : 'expandPanelWidth')" :aria-pressed="wide" @click="wide=!wide"><template #icon><component :is="wide ? Minimize2 : Maximize2" :size="20"/></template></NButton>
      </section>
    </Transition>
  </Teleport>
</template>
<style scoped>
.agents-panel{--panel-width:min(800px,calc(100vw - 40px));--panel-left:clamp(20px,calc(var(--origin-x) + var(--origin-size) + 28px - var(--panel-width)),calc(100vw - var(--panel-width) - 20px));position:fixed;inset:20px auto 20px var(--panel-left);width:var(--panel-width);z-index:55;background:var(--surface);border:1px solid var(--line);border-radius:28px;box-shadow:0 12px 48px #132a2418;display:flex;flex-direction:column;overflow:hidden;outline:none;clip-path:inset(0 round 28px)}
.agents-header{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:24px 28px 12px;flex-shrink:0}
.agents-header h1{font-size:22px;font-weight:600;margin:0}
.agents-actions{display:flex;align-items:center;gap:8px}
.agents-content{padding:0 28px 24px;overflow:auto;overscroll-behavior:contain;min-height:0}
.agents-panel-enter-active,.agents-panel-leave-active{transition:clip-path 480ms cubic-bezier(.22,1,.36,1)}
.agents-panel-leave-active{transition-duration:220ms}
.agents-panel-enter-from,.agents-panel-leave-to{clip-path:inset(calc(var(--origin-y) - 20px) calc(var(--panel-width) - var(--origin-x) + var(--panel-left) - var(--origin-size)) calc(100dvh - var(--origin-y) - var(--origin-size) - 20px) calc(var(--origin-x) - var(--panel-left)) round 50px)}
.agents-panel-enter-active .agents-header,.agents-panel-enter-active .agents-content{animation:agents-reveal 380ms 100ms both}
.agents-panel-leave-active .agents-header,.agents-panel-leave-active .agents-content{opacity:0;transition:opacity 120ms}
@keyframes agents-reveal{from{opacity:0}to{opacity:1}}
.spinning{animation:agents-spin 1s linear infinite}
@keyframes agents-spin{to{transform:rotate(360deg)}}
@media(prefers-reduced-motion:reduce){.agents-panel-enter-active,.agents-panel-leave-active{transition:none}.agents-panel-enter-active .agents-header,.agents-panel-enter-active .agents-content,.spinning{animation:none}}

.agent-row { display: flex; align-items: center; gap: 16px; min-height: 66px; margin: 0; padding: 12px 0; }
.agent-row + .agent-row { border-top: 1px solid var(--line); }
.agent-info { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; margin: 0; }
.agent-icon { display: flex; width: 22px; height: 22px; flex-shrink: 0; }
.agent-icon :deep(svg) { width: 100%; height: 100%; }
.agent-fallback { flex-shrink: 0; }
.agent-copy { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.agent-name { font-size: 13px; font-weight: 500; line-height: 20px; }
.agent-default { color: var(--muted); font-weight: 400; }
.agent-version { font-size: 12px; line-height: 18px; color: var(--muted); overflow-wrap: anywhere; }

.agent-error { color: #a46651; font-size: 12px; padding: 0 0 10px; margin: 0; }
</style>

<style scoped>
.agents-panel.wide{--panel-width:calc(100vw - 40px);--panel-left:20px}
.agents-panel>.agents-width-toggle{position:absolute;right:16px;bottom:14px;display:none}
@media(min-width:840px){.agents-panel>.agents-width-toggle{display:inline-flex}.agents-content{padding-bottom:68px}.agents-panel::after{content:'';position:absolute;bottom:0;left:0;right:0;height:54px;background:var(--surface);pointer-events:none}.agents-panel>.agents-width-toggle{z-index:1}}
</style>
