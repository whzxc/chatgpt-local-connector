import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

// Development owns the window only; operations are forwarded to the running build.
const root = new URL('../', import.meta.url);
if (process.argv.includes('--release')) throw new Error('desktop:dev 仅运行开发模式；发行构建请用 desktop:build。');
const env = { ...process.env };
if (spawnSync('cargo', ['--version'], { env, stdio: 'ignore' }).status !== 0) {
  const cargo = spawnSync('rustup', ['which', 'cargo'], { env, encoding: 'utf8' });
  if (cargo.status !== 0) throw new Error('请先安装 Rust stable 工具链，并确保 cargo 或 rustup 在 PATH 中。');
  env.PATH = path.dirname(cargo.stdout.trim()) + path.delimiter + (env.PATH || '');
}
const prune = spawnSync(process.execPath, [fileURLToPath(new URL('tooling/prune-build-cache.mjs', root))],
  { cwd: fileURLToPath(root), env, stdio: 'inherit' });
if (prune.status !== 0) throw new Error('无法检查 Rust 构建缓存');
const args = [fileURLToPath(new URL('node_modules/@tauri-apps/cli/tauri.js', root)), 'dev'];
if (process.platform === 'win32') args.push('--config', 'tauri.windows.conf.json');
args.push('--config', 'tauri.dev.conf.json', ...process.argv.slice(2));
args.push('--config', JSON.stringify({ build: { beforeDevCommand: {
  script: 'npm run dev:ui', cwd: fileURLToPath(root),
} } }));
const child = spawn(process.execPath, args, {
  cwd: fileURLToPath(new URL('desktop/', root)), stdio: 'inherit', env,
});
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', code => { process.exitCode = code ?? 1; });
