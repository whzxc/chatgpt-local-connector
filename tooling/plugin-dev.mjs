import { createServer } from 'node:net';
import { spawn, execFileSync } from 'node:child_process';
import { mkdir, writeFile, access } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { buildNative } from './build-native.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const state = path.resolve(process.env.CLC_STATE_DIR || path.join(homedir(), '.local/state/chatgpt-local-connector-dev'));
const marketplace = path.resolve(process.env.CLC_PLUGIN_DEV_DIR || path.join(homedir(), '.codex/local-connector-dev'));
const origin = 'http://127.0.0.1:5187';
const env = { ...process.env, CLC_STATE_DIR:state, CLC_PLUGIN_DEV_URL:origin };
await new Promise((resolve,reject)=>{const probe=createServer();probe.once('error',()=>reject(new Error('端口 5187 已被使用；请先停止现有开发服务器。')));probe.listen(5187,'127.0.0.1',()=>probe.close(resolve));});
await import('./build-plugin.mjs');
const binary = await buildNative();
const plugin = path.join(marketplace, 'plugins/clc');
execFileSync(process.execPath, [path.join(root,'tooling/export-plugin.mjs'),'--binary',binary,'--output',plugin,'--state-dir',state,'--dev-url',origin], { cwd:root, stdio:'inherit' });
await mkdir(path.join(marketplace, '.agents/plugins'), { recursive:true });
await writeFile(path.join(marketplace,'.agents/plugins/marketplace.json'), JSON.stringify({ name:'clc-dev', interface:{displayName:'Local Connector Dev'}, plugins:[{name:'clc',source:{source:'local',path:'./plugins/clc'},policy:{installation:'AVAILABLE',authentication:'ON_INSTALL'},category:'Productivity'}] },null,2)+'\n');
const client = new Client({name:'clc-dev-owner',version:'1'});
let vite;
let stopping = false;
async function stop() {
  if (stopping) return;
  stopping = true;
  vite?.kill();
  await client.close();
}
process.on('SIGINT', () => { void stop(); });
process.on('SIGTERM', () => { void stop(); });
try {
  await client.connect(new StdioClientTransport({command:binary,args:['mcp'],env,stderr:'inherit'}));
  vite = spawn(process.execPath, [path.join(root,'node_modules/vite/bin/vite.js'),'--config',path.join(root,'vite.config.ts')], {cwd:root,env,stdio:'inherit'});
  const exited = new Promise(resolve => { vite.on('exit',code=>resolve(code)); vite.on('error',error=>{console.error(error.message);resolve(1);}); });
  let ready = false;
  for (let attempt=0;attempt<50;attempt++) {
    try { ready = (await fetch(`${origin}/plugin.html`,{signal:AbortSignal.timeout(200)})).ok; } catch {}
    if (ready || vite.exitCode !== null || stopping) break;
    await new Promise(resolve=>setTimeout(resolve,100));
  }
  if (!ready) throw new Error('Plugin Vite failed to start on port 5187.');
  let codex = 'codex';
  if (process.platform === 'darwin') {
    const bundled = '/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex';
    try { await access(bundled); codex = bundled; } catch {}
  }
  execFileSync(codex,['plugin','marketplace','add',marketplace],{stdio:'inherit'});
  execFileSync(codex,['plugin','add','clc@clc-dev'],{stdio:'inherit'});
  console.log(`Plugin dev ready. Open Local Connector from the clc-dev marketplace in ChatGPT Desktop.\nUI: ${origin}/plugin.html\nState: ${state}\nReact/CSS use HMR. Rust, manifest and skills require restarting this command and reloading the host plugin. The Core remains alive while another local entrypoint uses it.`);
  const code = await exited;
  if (!stopping && code) process.exitCode = code;
} finally {
  await stop();
}
