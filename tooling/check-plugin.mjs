import { readFile, stat, mkdtemp, rm, access } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { toolFingerprint } from './catalog.mjs';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const directory = path.resolve(process.argv[2] || 'dist/plugin-package/darwin-aarch64/plugins/clc');
const manifest = JSON.parse(await readFile(path.join(directory, '.codex-plugin/plugin.json'), 'utf8'));
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
if (manifest.name !== 'clc' || manifest.version !== pkg.version || manifest.mcpServers !== './.mcp.json' || manifest.apps) throw new Error('Plugin identity/version/standalone entry mismatch');
for (const file of ['LICENSE','skills/local-connector/SKILL.md','assets/icon.svg']) await access(path.join(directory, file));
if (await readFile(path.join(directory,'skills/local-connector/SKILL.md'),'utf8') !== await readFile(new URL('../plugins/clc/skills/local-connector/SKILL.md',import.meta.url),'utf8')) throw new Error('Bundled workflow skill differs from this source');
const config = JSON.parse(await readFile(path.join(directory, '.mcp.json'), 'utf8'));
const server = config.mcpServers.clc;
if (Object.keys(config.mcpServers).length !== 1 || server.cwd !== '.' || server.command !== (process.platform === 'win32' ? './bin/local-connector.exe' : './bin/local-connector') || server.args.join() !== 'mcp') throw new Error('Plugin must use its own relative executable');
const binary = path.resolve(directory, server.command);
const mode = (await stat(binary)).mode;
if (process.platform !== 'win32' && !(mode & 0o111)) throw new Error('Plugin binary is not executable');
if (execFileSync(binary, ['--version'], { encoding:'utf8' }).trim() !== pkg.version) throw new Error('Plugin binary version mismatch');
const state = await mkdtemp(path.join(tmpdir(), 'clc-plugin-check-'));
const client = new Client({ name:'clc-artifact-check', version:pkg.version });
try {
  await client.connect(new StdioClientTransport({ command:binary, args:['mcp'], cwd:directory, env:{ ...process.env, CLC_STATE_DIR:state, CODEX_HOME:path.join(state,"codex"), CLC_PLUGIN_DEV_HTML:'' }, stderr:'inherit' }));
  const tools = await client.listTools();
  for (const name of ['connector_overview','connector_task_usage','connector_usage_refresh','projects','agents','agent_create','agent_read','agent_wait']) {
    if (!tools.tools.some(tool => tool.name === name)) throw new Error(`Missing plugin tool: ${name}`);
  }
  const catalog = JSON.parse(await readFile(new URL('../native/src/catalog.json',import.meta.url),'utf8'));
  const names = new Set(catalog.tools.filter(t=>t.name!=='connector_verify').map(t=>t.name));
  if (toolFingerprint(catalog.tools.filter(t=>names.has(t.name))).toolSchemaDigest !== toolFingerprint(tools.tools.filter(t=>names.has(t.name))).toolSchemaDigest) throw new Error('Bundled native tool definitions differ from this source catalog');
  const resources = await client.listResources();
  if (resources.resources.length !== 1) throw new Error('Missing native UI resource');
  const resource = await client.readResource({ uri:resources.resources[0].uri });
  const html = resource.contents[0].text;
  if (typeof html !== 'string' || !html.includes('<div id="root">') || /<script[^>]+src=/.test(html)) throw new Error('Release UI must be embedded and self-contained');
  if (html !== await readFile(new URL('../dist/plugin/app.html',import.meta.url),'utf8')) throw new Error('Bundled UI differs from this source build; rebuild the native executable.');
  let overview;
  for (let attempt=0;attempt<20;attempt++) {
    overview = await client.callTool({ name:'connector_overview', arguments:{} });
    if (overview._meta?.usage?.state === 'ready') break;
    await new Promise(resolve=>setTimeout(resolve,100));
  }
  if (overview._meta?.usage?.state !== 'ready') throw new Error('Standalone Core did not serve its usage panel');
  console.log(`Standalone plugin verified: v${pkg.version}, ${tools.tools.length} tools, embedded UI, independent Core.`);
} finally {
  await client.close();
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    try { await access(path.join(state, 'web/native.json')); }
    catch { break; }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  try { await access(path.join(state, 'web/native.json')); throw new Error(`Core did not stop after its last client closed: ${state}`); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  await rm(state, { recursive:true, force:true });
}
