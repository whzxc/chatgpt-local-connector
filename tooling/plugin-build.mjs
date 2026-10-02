import { execFileSync } from 'node:child_process';
import { mkdir, writeFile, readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { buildNative } from './build-native.mjs';

await import('./build-plugin.mjs');
const args = process.argv.slice(2);
const target = args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined;
const release = !args.includes('--debug');
if (!['darwin','win32'].includes(process.platform) || !['arm64','x64'].includes(process.arch) || (process.platform === 'darwin' && process.arch !== 'arm64') || (process.platform === 'win32' && process.arch !== 'x64')) throw new Error('Plugin packages require Apple Silicon macOS or Windows x64.');
if (target && target !== (process.platform === 'darwin' ? 'aarch64-apple-darwin' : 'x86_64-pc-windows-msvc')) throw new Error('Build and verify this plugin on its native target platform.');
const binary = args.includes('--binary') ? path.resolve(args[args.indexOf('--binary') + 1]) : await buildNative({ release, target });
// Git Bash can put GNU tar ahead of Windows' ZIP-capable bsdtar.
const tar = process.platform === 'win32' ? path.join(process.env.SystemRoot, 'System32', 'tar.exe') : 'tar';
const platform = `${process.platform === 'darwin' ? 'darwin' : 'windows'}-${process.arch === 'arm64' ? 'aarch64' : 'x86_64'}`;
const root = fileURLToPath(new URL(`../dist/plugin-package/${platform}/`, import.meta.url));
await rm(root,{recursive:true,force:true});
const plugin = path.join(root, 'plugins/clc');
execFileSync(binary, ['export', plugin], { stdio: 'inherit' });
await mkdir(path.join(root, '.agents/plugins'), { recursive: true });
await writeFile(path.join(root, '.agents/plugins/marketplace.json'), JSON.stringify({
  name: 'local-connector', interface: { displayName: 'Local Connector' },
  plugins: [{ name: 'clc', source: { source: 'local', path: './plugins/clc' }, policy: { installation: 'AVAILABLE', authentication: 'ON_INSTALL' }, category: 'Productivity' }],
}, null, 2) + '\n');
execFileSync(process.execPath, [fileURLToPath(new URL('check-plugin.mjs', import.meta.url)), plugin], { stdio: 'inherit' });
const pkg = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
const archive = path.resolve(root, `../CLC.Plugin_${pkg.version}_${platform}.zip`);
await rm(archive, { force:true });
if (process.platform === 'darwin') execFileSync('/usr/bin/zip', ['-q','-r',archive,'.'], { cwd:root, stdio:'inherit' });
else execFileSync(tar, ['-c','--format','zip','-f',archive,'-C',root,'.'], { stdio:'inherit' });
const unpacked = await mkdtemp(path.join(tmpdir(), 'clc-plugin-package-'));
try {
  execFileSync(tar, ['-xf',archive,'-C',unpacked], {stdio:'inherit'});
  execFileSync(process.execPath, [fileURLToPath(new URL('check-plugin.mjs',import.meta.url)),path.join(unpacked,'plugins/clc')], {stdio:'inherit'});
} finally { await rm(unpacked,{recursive:true,force:true}); }
await writeFile(archive+'.sha256',createHash('sha256').update(await readFile(archive)).digest('hex')+'  '+path.basename(archive)+'\n');
console.log(`Standalone plugin marketplace: ${root}\nArchive: ${archive}`);
