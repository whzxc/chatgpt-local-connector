import { execFileSync } from 'node:child_process';
import { mkdir, copyFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { homedir } from 'node:os';

export async function buildNative({ release = false, target } = {}) {
  const root = fileURLToPath(new URL('../', import.meta.url));
  const args = ['build', '--locked', '--manifest-path', 'native/Cargo.toml', '--bin', 'local-connector'];
  if (release) args.push('--release');
  if (target) args.push('--target', target);
  const env = { ...process.env };
  if (release) {
    const flags = env.CARGO_ENCODED_RUSTFLAGS?.split('\x1f') ?? env.RUSTFLAGS?.trim().split(/\s+/).filter(Boolean) ?? [];
    flags.push(`--remap-path-prefix=${homedir()}=/build-user`, `--remap-path-prefix=${root}=/workspace/`);
    env.CARGO_ENCODED_RUSTFLAGS = flags.join('\x1f');
    delete env.RUSTFLAGS;
  }
  execFileSync('cargo', args, { cwd: root, env, stdio: 'inherit' });
  const name = process.platform === 'win32' ? 'local-connector.exe' : 'local-connector';
  const binary = path.join(root, 'native/target', ...(target ? [target] : []), release ? 'release' : 'debug', name);
  if (process.platform === 'darwin') execFileSync('codesign', ['--force', '--sign', '-', binary], { stdio: 'inherit' });
  const destination = path.join(root, 'dist/native', name);
  await mkdir(path.dirname(destination), { recursive: true });
  await copyFile(binary, destination);
  return binary;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await import('./build-plugin.mjs');
  const args = process.argv.slice(2);
  await buildNative({ release: args.includes('--release'), target: args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined });
}
