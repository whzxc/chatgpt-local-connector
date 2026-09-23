import { computed, ref } from 'vue';
import { api } from './useConnector';
import antigravityIcon from '../assets/brand-reserve/agents/antigravity/mono.svg?raw';
import codexIcon from '../assets/brand-reserve/agents/codex/mono.svg?raw';
import piIcon from '../assets/brand-reserve/agents/pi/mono.svg?raw';
import opencodeIcon from '../assets/brand-reserve/agents/opencode/mono.svg?raw';
import claudeIcon from '../assets/brand-reserve/agents/claude/color.svg?raw';
import cursorIcon from '../assets/brand-reserve/agents/cursor/mono.svg?raw';
import grokIcon from '../assets/brand-reserve/agents/grok/mono.svg?raw';
import copilotIcon from '../assets/brand-reserve/agents/copilot/color.svg?raw';
import kimiIcon from '../assets/brand-reserve/agents/kimi/mono.svg?raw';
import qwenIcon from '../assets/brand-reserve/agents/qwen/color.svg?raw';
import kiroIcon from '../assets/brand-reserve/agents/kiro/color.svg?raw';
import devinIcon from '../assets/brand-reserve/agents/devin/color.svg?raw';
import clineIcon from '../assets/brand-reserve/agents/cline/mono.svg?raw';
import junieIcon from '../assets/brand-reserve/agents/junie/color.svg?raw';
import hermesIcon from '../assets/brand-reserve/agents/hermes/mono.svg?raw';

export type Agent = { adapter?: { state: string; error?: string | null }; agent: string; installed?: boolean; available?: boolean; enabled?: boolean; status?: string; version?: string | null; displayName?: string };
const cacheKey = 'clc.agents';
function readCache(): Agent[] | undefined {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(cacheKey) || 'null');
    if (Array.isArray(value) && value.every(a => a && typeof a.agent === 'string'
      && typeof a.installed === 'boolean' && typeof a.available === 'boolean'
      && (a.enabled === undefined || typeof a.enabled === 'boolean')
      && (a.version == null || typeof a.version === 'string')
      && (a.displayName === undefined || typeof a.displayName === 'string'))) return value;
  } catch { /* Storage may be unavailable; discovery still works. */ }
}
const cached = readCache();
const agents=ref<Agent[]>(cached || []), loaded=ref(!!cached), loading=ref(false), saving=ref(''), error=ref('');
function persist() {
  try {
    // Persist display facts only, never paths, descriptors or live process readiness.
    localStorage.setItem(cacheKey, JSON.stringify(agents.value.map(({agent, installed, available, enabled, version, displayName}) =>
      ({agent, installed, available, enabled, version, displayName}))));
  } catch { /* A storage failure must not discard a successful discovery. */ }
}
export const icons: Record<string, string> = { antigravity:antigravityIcon, codex: codexIcon, pi: piIcon, opencode: opencodeIcon, claude: claudeIcon, cursor: cursorIcon, gemini: antigravityIcon, grok: grokIcon, copilot: copilotIcon, kimi: kimiIcon, qwen: qwenIcon, kiro: kiroIcon, devin: devinIcon, cline: clineIcon, junie: junieIcon, hermes: hermesIcon };
export const agentLinks: Record<string, string> = {
  codex: 'https://github.com/openai/codex',
  pi: 'https://pi.dev/',
  opencode: 'https://opencode.ai/',
  claude: 'https://github.com/anthropics/claude-code',
  cursor: 'https://cursor.com/docs/cli/acp',
  gemini: 'https://github.com/google-gemini/gemini-cli',
  grok: 'https://docs.x.ai/build/cli/reference',
  copilot: 'https://docs.github.com/en/copilot/reference/copilot-cli-reference/acp-server',
  kimi: 'https://www.kimi.com/code/',
  qwen: 'https://github.com/QwenLM/qwen-code',
  kiro: 'https://kiro.dev/',
  devin: 'https://docs.devin.ai/desktop/acp',
  cline: 'https://docs.cline.bot/',
  junie: 'https://junie.jetbrains.com/',
  hermes: 'https://hermes-agent.nousresearch.com/',
};
export const name = (agent: Agent) => ({codex:'Codex',claude:'Claude',cursor:'Cursor',gemini:'Antigravity',grok:'Grok',opencode:'OpenCode',copilot:'GitHub Copilot',kimi:'Kimi',qwen:'Qwen',kiro:'Kiro',devin:'Devin',cline:'Cline',junie:'Junie',hermes:'Hermes',pi:'Pi'}[agent.agent] || agent.displayName) || ({ codex: 'Codex', pi: 'Pi', opencode: 'OpenCode' }[agent.agent] || agent.agent);
const orderKey = 'clc.agent-order';
const order = ref<string[]>([]);
try {
  const stored: unknown = JSON.parse(localStorage.getItem(orderKey) || '[]');
  if (Array.isArray(stored) && stored.every(id => typeof id === 'string')) order.value = [...new Set(stored)];
} catch { /* Discovery supplies the default order. */ }
const orderedAgents = computed(() => [...agents.value].sort((a, b) => {
  const ai = order.value.indexOf(a.agent), bi = order.value.indexOf(b.agent);
  if (ai >= 0 || bi >= 0) return (ai < 0 ? Infinity : ai) - (bi < 0 ? Infinity : bi);
  return Number(b.agent === 'codex') - Number(a.agent === 'codex') || Number(!!b.installed) - Number(!!a.installed) || name(a).localeCompare(name(b));
}));
function moveAgent(from: string, to: string) {
  const ids = orderedAgents.value.map(a => a.agent);
  const start = ids.indexOf(from), end = ids.indexOf(to);
  if (start < 0 || end < 0 || start === end) return;
  ids.splice(start, 1); ids.splice(end, 0, from);
  order.value = ids;
  try { localStorage.setItem(orderKey, JSON.stringify(ids)); } catch { /* Keep the current session order. */ }
}
let pendingRefresh: Promise<void> | undefined;
let revision = 0;
let preparationPoll: ReturnType<typeof setTimeout> | undefined;
function refresh(manual = false): Promise<void> {
  if (saving.value) return Promise.resolve();
  if (pendingRefresh) return manual ? pendingRefresh.then(() => refresh(true)) : pendingRefresh;
  if (manual || !loaded.value) loading.value = true;
  const currentRevision = revision;
  pendingRefresh = (async () => {
    try {
      const result = await api<{ agents: Agent[] }>('agents', manual ? 'POST' : 'GET');
      if (currentRevision !== revision) return;
      agents.value = result.agents; loaded.value = true; error.value = '';
      persist();
      if (preparationPoll) clearTimeout(preparationPoll);
      if (agents.value.some(a => a.adapter?.state === 'preparing')) preparationPoll = setTimeout(() => void refresh(), 1500);
    } catch (e) { if (currentRevision === revision) error.value = e instanceof Error ? e.message : String(e); }
    finally { loading.value = false; pendingRefresh = undefined; }
  })();
  return pendingRefresh;
}
async function toggle(agent: Agent, enabled: boolean) {
  revision++;
  saving.value = agent.agent;
  try {
    const result = await api<{ enabled: boolean; preparing?: boolean }>('agents', 'PUT', { agent: agent.agent, enabled });
    agent.enabled = result.enabled;
    if (result.preparing) agent.adapter = { state: 'preparing' };
    persist();
    error.value = '';
  } catch (e) { error.value = e instanceof Error ? e.message : String(e); return; }
  finally { saving.value = ''; }
  await refresh(true);
}

export function useAgents() { return { agents, orderedAgents, moveAgent, loaded, loading, saving, error, refresh, toggle }; }
