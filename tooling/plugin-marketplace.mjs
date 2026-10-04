import { execFileSync } from 'node:child_process';
import { chmod, copyFile, cp, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import path from 'node:path';

const version = JSON.parse(await readFile('package.json', 'utf8')).version;
const json = file => readFile(file, 'utf8').then(JSON.parse);
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const writeJson = (file, value) => writeFile(file, JSON.stringify(value, null, 2) + '\n');
const git = (directory, ...args) => execFileSync('git', ['-C', directory, ...args], { encoding: 'utf8' }).trim();
const platforms = ['darwin-aarch64', 'windows-x86_64'];
const binaryName = platform => `local-connector${platform.startsWith('windows') ? '.exe' : ''}`;

async function inventory(directory, prefix = '') {
  const result = [];
  for (const entry of await readdir(path.join(directory, prefix), { withFileTypes: true })) {
    const relative = path.posix.join(prefix, entry.name);
    if (entry.isDirectory()) result.push(...await inventory(directory, relative));
    else if (entry.isFile()) result.push(relative);
    else throw new Error(`Unsupported package entry: ${relative}`);
  }
  return result.sort();
}

async function assemble(artifacts, output) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Marketplace releases require a stable version');
  await mkdir(output, { recursive: true });
  if ((await readdir(output)).length) throw new Error('Marketplace output must be empty');
  const temporary = await mkdtemp(path.join(tmpdir(), 'clc-marketplace-'));
  try {
    const roots = [];
    const hashes = {};
    const payloads = platforms.map(platform => path.join(artifacts, `CLC.Core_${version}_${platform}.bin`));
    execFileSync('cargo', ['run', '--quiet', '--locked', '--manifest-path', 'tooling/verifier/Cargo.toml', '--', 'desktop/tauri.conf.json', ...payloads], { stdio: 'inherit' });
    for (const [index, platform] of platforms.entries()) {
      const root = path.join(temporary, platform);
      await mkdir(root);
      const archive = path.join(artifacts, `CLC.Plugin_${version}_${platform}.zip`);
      // unzip works on macOS and the Linux release runner and preserves executable modes.
      execFileSync('unzip', ['-q', archive, '-d', root], { stdio: 'inherit' });
      const plugin = path.join(root, 'plugins/clc');
      const manifest = await json(path.join(plugin, '.codex-plugin/plugin.json'));
      if (manifest.name !== 'clc' || manifest.version !== version || manifest.mcpServers !== './.mcp.json') throw new Error('Plugin identity/version mismatch');
      const binary = await readFile(path.join(plugin, 'bin', binaryName(platform)));
      if (!binary.equals(await readFile(payloads[index]))) throw new Error(`${platform}: ZIP binary differs from signed Core`);
      hashes[platform] = digest(binary);
      const mcp = await json(path.join(plugin, '.mcp.json'));
      const server = mcp.mcpServers.clc;
      if (Object.keys(mcp.mcpServers).length !== 1 || server.command !== `./bin/${binaryName(platform)}` || server.cwd !== '.' || JSON.stringify(server.args) !== '["mcp"]' || server.env) throw new Error('Unexpected MCP launch configuration');
      // Windows CreateProcess resolves the extensionless command to the sibling .exe.
      server.command = './bin/local-connector';
      await writeJson(path.join(plugin, '.mcp.json'), mcp);
      roots.push(root);
    }
    const common = root => inventory(root).then(files => files.filter(file => !file.startsWith('plugins/clc/bin/')));
    const files = await common(roots[0]);
    if (JSON.stringify(files) !== JSON.stringify(await common(roots[1]))) throw new Error('Platform package layouts differ');
    for (const file of files) {
      if (!(await readFile(path.join(roots[0], file))).equals(await readFile(path.join(roots[1], file)))) throw new Error(`Platform metadata differs: ${file}`);
    }
    const market = await json(path.join(roots[0], '.agents/plugins/marketplace.json'));
    if (market.name !== 'local-connector' || market.plugins.length !== 1 || market.plugins[0].name !== 'clc' || market.plugins[0].source.path !== './plugins/clc') throw new Error('Marketplace identity mismatch');
    await cp(roots[0], output, { recursive: true });
    await copyFile(path.join(roots[1], 'plugins/clc/bin/local-connector.exe'), path.join(output, 'plugins/clc/bin/local-connector.exe'));
    await chmod(path.join(output, 'plugins/clc/bin/local-connector'), 0o755);
    await copyFile('LICENSE', path.join(output, 'LICENSE'));
    await writeJson(path.join(output, 'release.json'), { version, sourceCommit: git('.', 'rev-parse', 'HEAD'), binaries: hashes });
    await writeFile(path.join(output, '.gitattributes'), '* -text\nplugins/clc/bin/* binary\n');
    await writeFile(path.join(output, 'README.md'), `# Local Connector plugin marketplace\n\nInstallable native plugin for Apple Silicon macOS and Windows x64. No Node/npm runtime is required.\n\n\`\`\`sh\ncodex plugin marketplace add whzxc/clc-plugins --ref stable\ncodex plugin add clc@local-connector\n\`\`\`\n\nUpdate with \`codex plugin marketplace upgrade local-connector\`, then reload the host. Existing sessions keep their running version until reloaded.\n\n[Installation and migration](https://github.com/whzxc/chatgpt-local-connector/blob/main/docs/plugin.md) · [Source and releases](https://github.com/whzxc/chatgpt-local-connector)\n\nThe stable branch contains signed release payloads assembled by the source repository's release workflow. Version tags preserve each published snapshot. Do not edit generated files here. MIT licensed.\n`);
    console.log(`Git marketplace assembled: ${output} (v${version}, both platforms)`);
  } finally { await rm(temporary, { recursive: true, force: true }); }
}

async function publish(source, checkout) {
  const release = await json(path.join(source, 'release.json'));
  if (release.version !== version || release.sourceCommit !== git('.', 'rev-parse', 'HEAD')) throw new Error('Marketplace source commit/version mismatch');
  if (git(checkout, 'branch', '--show-current') !== 'stable' || git(checkout, 'status', '--porcelain')) throw new Error('Publication needs a clean stable checkout');
  for (const platform of platforms) {
    if (digest(await readFile(path.join(source, 'plugins/clc/bin', binaryName(platform)))) !== release.binaries[platform]) throw new Error('Marketplace binary hash mismatch');
  }
  let previous;
  try { previous = await json(path.join(checkout, 'release.json')); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (previous) {
    const compare = version.localeCompare(previous.version, 'en', { numeric: true });
    if (compare < 0) throw new Error('Refusing to downgrade stable');
    if (compare === 0 && JSON.stringify(previous) !== JSON.stringify(release)) throw new Error('Published versions are immutable');
  }
  // This is a dedicated distribution checkout: replace its tracked payload only.
  for (const entry of await readdir(checkout)) if (entry !== '.git') await rm(path.join(checkout, entry), { recursive: true, force: true });
  await cp(source, checkout, { recursive: true });
  git(checkout, 'add', '--all');
  git(checkout, 'update-index', '--chmod=+x', 'plugins/clc/bin/local-connector');
  if (git(checkout, 'diff', '--cached', '--name-only')) {
    if (previous?.version === version) throw new Error('Published versions are immutable');
    git(checkout, '-c', 'user.name=CLC Release', '-c', 'user.email=release@users.noreply.github.com', 'commit', '-m', `chore(release): 发布插件 ${version}`);
  }
  const tag = `v${version}`;
  const existing = git(checkout, 'tag', '--list', tag);
  if (existing && git(checkout, 'rev-parse', tag) !== git(checkout, 'rev-parse', 'HEAD')) throw new Error('Version tag already points to another snapshot');
  if (!existing) git(checkout, 'tag', tag);
  git(checkout, 'push', '--atomic', 'origin', 'HEAD:refs/heads/stable', `refs/tags/${tag}`);
  const remote = git(checkout, 'ls-remote', 'origin', 'refs/heads/stable', `refs/tags/${tag}`);
  if (remote.split('\n').length !== 2 || remote.split('\n').some(line => !line.startsWith(git(checkout, 'rev-parse', 'HEAD') + '\t'))) throw new Error('Published marketplace refs did not match');
  console.log(`Published marketplace ${tag}: ${git(checkout, 'rev-parse', 'HEAD')}`);
}

const [mode, first, second] = process.argv.slice(2);
if (!first || !second) throw new Error('Usage: plugin-marketplace.mjs assemble <release-artifacts> <empty-output> | publish <assembled-marketplace> <stable-checkout>');
if (mode === 'assemble') await assemble(path.resolve(first), path.resolve(second));
else if (mode === 'publish') await publish(path.resolve(first), path.resolve(second));
else throw new Error(`Unknown command: ${mode}`);
