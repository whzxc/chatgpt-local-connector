import { execFileSync } from 'node:child_process';
import { lstat, readdir, rm, stat, utimes } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Cargo has no per-project size limit. Prune whole output groups so its
// fingerprints never point at a partially deleted dependency.
const root = fileURLToPath(new URL('../', import.meta.url));
const limit = 10 * 1024 ** 3;
const keepArg = process.argv.find(arg => arg.startsWith('--keep='))?.slice('--keep='.length);
const keep = keepArg && path.normalize(keepArg);
const dryRun = process.argv.includes('--dry-run');
const targetDirs = ['desktop/target', 'native/target', 'tooling/verifier/target'];

async function size(directory) {
  if (process.platform !== 'win32') {
    return Number(execFileSync('du', ['-sk', directory], { encoding: 'utf8' }).trim().split(/\s+/)[0]) * 1024;
  }
  let bytes = 0;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const child = path.join(directory, entry.name);
    if (entry.isDirectory()) bytes += await size(child);
    else if (entry.isFile()) bytes += (await stat(child)).size;
  }
  return bytes;
}

async function removeGroup(group) {
  if (!group.relative.startsWith(`desktop${path.sep}target${path.sep}`)) {
    await rm(group.name, { recursive: true, force: true, maxRetries: 3 });
    return 0;
  }
  async function clearProfile(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const bundle = entries.find(entry => entry.name === 'bundle' && entry.isDirectory());
    if (!bundle) {
      await rm(directory, { recursive: true, force: true, maxRetries: 3 });
      return;
    }
    for (const entry of entries) {
      if (entry !== bundle) await rm(path.join(directory, entry.name), { recursive: true, force: true, maxRetries: 3 });
    }
  }
  if (['debug', 'release'].includes(path.basename(group.name))) {
    await clearProfile(group.name);
  } else {
    for (const entry of await readdir(group.name, { withFileTypes: true })) {
      const child = path.join(group.name, entry.name);
      if (entry.isDirectory() && ['debug', 'release'].includes(entry.name)) await clearProfile(child);
      else await rm(child, { recursive: true, force: true, maxRetries: 3 });
    }
  }
  return (await lstat(group.name).catch(() => null))?.isDirectory() ? size(group.name) : 0;
}

if (!process.env.CI) {
  const groups = [];
  for (const relative of targetDirs) {
    const directory = path.join(root, relative);
    if (!(await lstat(directory).catch(() => null))?.isDirectory()) continue;
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const name = path.join(directory, entry.name);
      groups.push({ name, relative: path.join(relative, entry.name), bytes: await size(name), modified: (await stat(name)).mtimeMs });
    }
  }
  if (keep && !dryRun) {
    const current = groups.find(group => keep === group.relative || keep.startsWith(group.relative + path.sep));
    if (current) await utimes(current.name, new Date(), new Date());
  }
  let total = groups.reduce((sum, group) => sum + group.bytes, 0);
  if (total > limit) {
    for (const group of groups.filter(group => !keep || !(keep === group.relative || keep.startsWith(group.relative + path.sep))).sort((a, b) => a.modified - b.modified)) {
      if (total <= limit) break;
      console.log(`${dryRun ? 'Would remove' : 'Removing'} ${group.relative} (${(group.bytes / 1024 ** 3).toFixed(2)} GiB)`);
      const retained = dryRun ? 0 : await removeGroup(group);
      total -= group.bytes - retained;
    }
  }
  console.log(`Rust build cache: ${(total / 1024 ** 3).toFixed(2)} GiB (limit ${(limit / 1024 ** 3).toFixed(0)} GiB${dryRun ? ', dry run' : ''})`);
  if (total > limit && !dryRun) console.warn('The current build output alone exceeds the cache limit; it will be eligible for pruning on the next command.');
}
