import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { homedir } from 'node:os';
const root = new URL('../', import.meta.url);
const args = [fileURLToPath(new URL('node_modules/@tauri-apps/cli/tauri.js', root)), 'build'];
if (process.platform === 'win32') args.push('--config', 'tauri.windows.conf.json');
if (process.env.TAURI_SIGNING_PRIVATE_KEY) args.push('--config', JSON.stringify({ bundle: { createUpdaterArtifacts: true } }));
if (process.argv.includes('--release') && !process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('发布构建需要项目专用 TAURI_SIGNING_PRIVATE_KEY');
args.push(...process.argv.slice(2).filter(a => a !== '--release'));
// Keep platform-signing credentials out of ad-hoc builds.
const env = { ...process.env };
for (const key of Object.keys(env)) if (/^(APPLE_|CSC_|WIN_CSC_)/.test(key)) delete env[key];
// Avoid embedding private source and Cargo registry paths in release binaries.
const flags = env.CARGO_ENCODED_RUSTFLAGS?.split('\x1f') ?? env.RUSTFLAGS?.trim().split(/\s+/).filter(Boolean) ?? [];
flags.push(`--remap-path-prefix=${homedir()}=/build-user`, `--remap-path-prefix=${fileURLToPath(root)}=/workspace/`);
env.CARGO_ENCODED_RUSTFLAGS = flags.join('\x1f');
delete env.RUSTFLAGS;
execFileSync(process.execPath, args, { cwd: fileURLToPath(new URL('desktop/', root)), env, stdio: 'inherit' });
