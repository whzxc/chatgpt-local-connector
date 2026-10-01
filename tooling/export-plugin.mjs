import { cp, mkdir, readFile, writeFile, access } from 'node:fs/promises';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
const value = key => { const i = args.indexOf(key); return i < 0 ? undefined : args[i + 1]; };
const raw = value('--app-id');
const output = value('--output');
if (!raw || !output || args.some((a, i) => i % 2 === 0 && !['--app-id', '--output'].includes(a))) {
  throw new Error('Usage: node tooling/export-plugin.mjs --app-id <existing App ID or plugin detail URL> --output <new directory ending in clc>');
}
const id = raw.replace(/^https:\/\/chatgpt\.com\/plugins\//, '').replace(/^plugin_/, '');
if (!/^(asdk_app_|connector_|templated_apps_)[A-Za-z0-9_-]+$/.test(id)) throw new Error('Unsupported App ID');
const target = resolve(output);
if (!/[\\/]clc$/.test(target)) throw new Error('Output directory must be named clc');
try { await access(target); throw new Error('Output exists; choose an empty parent directory to preserve the installed plugin'); }
catch (e) { if (e.code !== 'ENOENT') throw e; }
await mkdir(target, { recursive: true });
await cp(new URL('../plugins/clc/', import.meta.url), target, { recursive: true });
const manifestPath = `${target}/.codex-plugin/plugin.json`;
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
manifest.apps = './.app.json';
await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
await writeFile(`${target}/.app.json`, JSON.stringify({ apps: { clc: { id } } }, null, 2) + '\n');
console.log(`Exported CLC plugin to ${target}. Install it through a local marketplace in ChatGPT desktop. The existing App retains its authentication and tool policy.`);
