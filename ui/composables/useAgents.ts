import { ref } from 'vue';
import { api } from './useConnector';
import codexIcon from '../assets/brand-reserve/agents/codex/color.svg?raw';
import piIcon from '../assets/brand-reserve/agents/pi/mono.svg?raw';
import opencodeIcon from '../assets/brand-reserve/agents/opencode/mono.svg?raw';
import claudeIcon from '../assets/brand-reserve/agents/claude/color.svg?raw';
import cursorIcon from '../assets/brand-reserve/agents/cursor/mono.svg?raw';
import geminiIcon from '../assets/brand-reserve/agents/gemini/color.svg?raw';
import grokIcon from '../assets/brand-reserve/agents/grok/mono.svg?raw';
import copilotIcon from '../assets/brand-reserve/agents/copilot/color.svg?raw';
import kimiIcon from '../assets/brand-reserve/agents/kimi/color.svg?raw';
import qwenIcon from '../assets/brand-reserve/agents/qwen/color.svg?raw';
import kiroIcon from '../assets/brand-reserve/agents/kiro/color.svg?raw';
import devinIcon from '../assets/brand-reserve/agents/devin/color.svg?raw';
import clineIcon from '../assets/brand-reserve/agents/cline/mono.svg?raw';
import junieIcon from '../assets/brand-reserve/agents/junie/color.svg?raw';
import hermesIcon from '../assets/brand-reserve/agents/hermes/mono.svg?raw';

export type Agent = { agent: string; installed?: boolean; available?: boolean; enabled?: boolean; status?: string; version?: string | null; displayName?: string };
const agents=ref<Agent[]>([]), loaded=ref(false), loading=ref(false), saving=ref(''), error=ref('');
export const icons: Record<string, string> = { codex: codexIcon, pi: piIcon, opencode: opencodeIcon, claude: claudeIcon, cursor: cursorIcon, gemini: geminiIcon, grok: grokIcon, copilot: copilotIcon, kimi: kimiIcon, qwen: qwenIcon, kiro: kiroIcon, devin: devinIcon, cline: clineIcon, junie: junieIcon, hermes: hermesIcon };
export const name = (agent: Agent) => agent.displayName || ({ codex: 'Codex', pi: 'Pi', opencode: 'OpenCode' }[agent.agent] || agent.agent);
let pendingRefresh: Promise<void> | undefined;
let revision = 0;
function refresh(manual = false): Promise<void> {
  if (saving.value) return Promise.resolve();
  if (manual) loading.value = true;
  if (pendingRefresh) return pendingRefresh;
  const currentRevision = revision;
  pendingRefresh = (async () => {
    try {
      const result = await api<{ agents: Agent[] }>('agents');
      if (currentRevision !== revision) return;
      agents.value = result.agents; loaded.value = true; error.value = '';
    } catch (e) { if (currentRevision === revision) error.value = e instanceof Error ? e.message : String(e); }
    finally { loading.value = false; pendingRefresh = undefined; }
  })();
  return pendingRefresh;
}
async function toggle(agent: Agent, enabled: boolean) {
  revision++;
  saving.value = agent.agent;
  try {
    const result = await api<{ enabled: boolean }>('agents', 'PUT', { agent: agent.agent, enabled });
    agent.enabled = result.enabled;
    error.value = '';
  } catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { saving.value = ''; }
}

export function useAgents() { return { agents, loaded, loading, saving, error, refresh, toggle }; }
