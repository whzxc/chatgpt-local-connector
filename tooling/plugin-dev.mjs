import { spawn, execFileSync } from 'node:child_process';
import { mkdir, writeFile, access } from 'node:fs/promises';
import { watch } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { buildNative } from './build-native.mjs';
import { devStateDir } from './dev-state.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const state = process.env.CLC_STATE_DIR ? devStateDir : `${devStateDir}-dev`;
const marketplace = path.resolve(process.env.CLC_PLUGIN_DEV_DIR || path.join(homedir(), '.codex/local-connector-dev'));
if (process.argv.length > 2) throw new Error('Usage: npm run plugin:dev');
const html = path.join(root, 'dist/plugin/app.html');
const env = { ...process.env, CLC_STATE_DIR: state, CLC_PLUGIN_DEV_HTML: html };
await import('./build-plugin.mjs');
const binary = await buildNative();
const plugin = path.join(marketplace, 'plugins/clc');
execFileSync(process.execPath, [path.join(root,'tooling/export-plugin.mjs'),'--binary',binary,'--output',plugin,'--state-dir',state,'--dev-html',html], { cwd:root, stdio:'inherit' });
await mkdir(path.join(marketplace, '.agents/plugins'), { recursive:true });
await writeFile(path.join(marketplace,'.agents/plugins/marketplace.json'), JSON.stringify({ name:'clc-dev', interface:{displayName:'Local Connector Dev'}, plugins:[{name:'clc',source:{source:'local',path:'./plugins/clc'},policy:{installation:'AVAILABLE',authentication:'ON_INSTALL'},category:'Productivity'}] },null,2)+'\n');
const client = new Client({name:'clc-dev-owner',version:'1'});
let builder;
let watcher;
let debounce;
let dirty = false;
let stopping = false;
let finish;
const stopped = new Promise(resolve => { finish = resolve; });
async function stop() {
  if (stopping) return;
  stopping = true;
  clearTimeout(debounce);
  watcher?.close();
  builder?.kill();
  await client.close();
  finish();
}
function rebuild() {
  if (stopping) return;
  if (builder) { dirty = true; return; }
  dirty = false;
  builder = spawn(process.execPath, [path.join(root,'tooling/build-plugin.mjs')], {cwd:root,stdio:'inherit'});
  builder.on('error', error => console.error(error.message));
  builder.on('close', code => {
    builder = undefined;
    if (code) console.error('Panel build failed; the last working panel remains available. Save a correction to retry.');
    else console.log('Panel rebuilt. Open development panels reload automatically.');
    if (dirty) rebuild();
  });
}
process.on('SIGINT', () => { void stop(); });
process.on('SIGTERM', () => { void stop(); });
try {
  await client.connect(new StdioClientTransport({command:binary,args:['mcp'],env,stderr:'inherit'}));
  let codex = 'codex';
  if (process.platform === 'darwin') {
    const bundled = '/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex';
    try { await access(bundled); codex = bundled; } catch {}
  }
  execFileSync(codex,['plugin','marketplace','add',marketplace],{stdio:'inherit'});
  execFileSync(codex,['plugin','add','clc@clc-dev'],{stdio:'inherit'});
  watcher = watch(path.join(root,'ui'), {recursive:true}, (_event, filename) => {
    if (!filename || !/\.(tsx?|css|js|json|html)$/.test(filename)) return;
    clearTimeout(debounce);
    debounce = setTimeout(rebuild, 200);
  });
  console.log(`Plugin dev ready. Open Local Connector from the clc-dev marketplace in ChatGPT Desktop.\nState: ${state}\nSaving UI source rebuilds the embedded resource and reloads open panels through MCP. Panel state resets. No HTTP server or certificate is needed. Rust, manifest and skill changes require restarting this command and reloading the host plugin.`);
  await stopped;
} finally {
  await stop();
}
