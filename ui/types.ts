export interface CoreSnapshot {
  chatgpt?: { code: string; verifiedAt: string | null };
  desktop?: { supported: boolean; state: string; message: string };
  eventStorageError?: string | null;
  version: string;
  pid: number;
  package: { sha: string; version: string };
  backendSession: string;
  activeTurns: number;
  activeWrites: number;
  pendingInteractions: number;
  liveProcesses: number;
  uncertain: boolean;
  draining: boolean;
  appServer: {
    state: string;
    observedAt: string | null;
    evidence: string;
    stale: boolean;
    error?: { code: string; stage: string };
  };
  account: { state: string; observedAt: string | null };
  transport?: { state: string; error?: string };
  logs?: string[];
  lastInbound: string | null;
  toolCount: number;
  registered: boolean;
  schemaDiscovered: string;
  operationVerified: string;
}
