import { readFile, writeFile, readdir, mkdir, copyFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import path from 'node:path';
const pkg = JSON.parse(await readFile('package.json', 'utf8'));
let version = pkg.version;
const repo = 'whzxc/chatgpt-local-connector';
const base = `https://github.com/${repo}/releases/download/v${version}`;
const config = JSON.parse(await readFile('desktop/tauri.conf.json', 'utf8'));
const [mode, ...args] = process.argv.slice(2);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const json = file => readFile(file, 'utf8').then(JSON.parse);
const writeJson = (file, value) => writeFile(file, JSON.stringify(value, null, 2) + '\n');
async function files(dir) {
  const out = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...await files(file)); else out.push(file);
  }
  return out;
}
function verifySignatures(artifacts) {
  execFileSync('cargo', ['run', '--quiet', '--locked', '--manifest-path', 'tooling/verifier/Cargo.toml', '--', 'desktop/tauri.conf.json', ...artifacts], { stdio: 'inherit' });
}
async function validateManifest(manifest, dir) {
  const keys = ['darwin-aarch64','windows-x86_64','windows-x86_64-nsis'];
  if (manifest.version !== version || Object.keys(manifest.platforms).sort().join() !== keys.sort().join()) throw new Error('Incomplete release platform manifest');
  for (const value of Object.values(manifest.platforms)) {
    if (!value.url.startsWith(base + '/')) throw new Error('Unexpected updater download origin/version');
    const name = value.url.slice(base.length + 1);
    if (name !== path.basename(name) || !/\.(gz|exe)$/.test(name)) throw new Error('Invalid updater asset name');
    if ((await readFile(path.join(dir,name+'.sig'),'utf8')).trim() !== value.signature) throw new Error('Manifest signature mismatch');
    await readFile(path.join(dir,name));
  }
}
async function check() {
  parts(version);
  if (config.version !== version) throw new Error('Tauri version mismatch');
  for (const file of ['native/Cargo.toml', 'desktop/Cargo.toml']) {
    if (!(await readFile(file, 'utf8')).includes(`version = "${version}"`)) throw new Error(`${file}: version mismatch`);
  }
  const lock = await json('package-lock.json');
  if (lock.version !== version || lock.packages[''].version !== version) throw new Error('package-lock version mismatch');
  for (const [name, entry] of Object.entries(lock.packages)) {
    if (entry.resolved && !entry.resolved.startsWith('https://registry.npmjs.org/')) throw new Error(`${name}: package-lock requires a non-public npm registry`);
  }
  for (const [file, name] of [['native/Cargo.lock','connector-core'],['desktop/Cargo.lock','connector-core'],['desktop/Cargo.lock','local-connector-desktop']]) {
    if (!(await readFile(file,'utf8')).includes(`name = "${name}"\nversion = "${version}"`)) throw new Error(`${file}: ${name} version mismatch`);
  }
  if (!config.plugins.updater.pubkey || config.plugins.updater.endpoints[0] !== `https://github.com/${repo}/releases/latest/download/latest.json`) throw new Error('Updater configuration is incomplete');
  if (process.env.GITHUB_REF_TYPE === 'tag' && process.env.GITHUB_REF_NAME !== `v${version}`) throw new Error('Tag/version mismatch');
  console.log(`Release configuration ready: v${version}, ${repo}`);
}
async function cask(dir) {
  const dmg = (await files(dir)).filter(f => f.endsWith('_aarch64.dmg'));
  if (dmg.length !== 1) throw new Error('Expected one Apple Silicon DMG');
  await writeFile(path.join(dir, 'local-connector.rb'), `cask "local-connector" do
  version "${version}"
  sha256 "${hash(await readFile(dmg[0]))}"

  url "https://github.com/${repo}/releases/download/v#{version}/Local.Connector_#{version}_aarch64.dmg"
  name "Local Connector"
  desc "Connect ChatGPT to local Codex Desktop"
  homepage "https://github.com/${repo}"
  depends_on arch: :arm64
  auto_updates true
  app "Local Connector.app"

  caveats <<~EOS
    This application is not Developer ID signed or notarized.
    See the repository installation guide if macOS blocks the first launch.
    Codex Desktop and a ChatGPT connection are required.
    Choose OpenAI Secure MCP Tunnel or HTTPS MCP in the application.
  EOS
end
`);
}
const releaseFiles = ['package.json','package-lock.json','desktop/tauri.conf.json','native/Cargo.toml','desktop/Cargo.toml','native/Cargo.lock','desktop/Cargo.lock','CHANGELOG.md'];
const git = (...args) => execFileSync('git', args, {encoding:'utf8'}).trim();
function parts(value) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(value)) throw new Error(`Invalid stable version: ${value}`);
  return value.split('.').map(BigInt);
}
function compare(a, b) {
  const x=parts(a), y=parts(b);
  for (let i=0;i<3;i++) if(x[i]!==y[i]) return x[i]>y[i]?1:-1;
  return 0;
}
function latestPublished() {
  const tags=execFileSync('gh',['api',`repos/${repo}/releases`,'--paginate','--jq','.[] | select(.draft == false and .prerelease == false) | .tag_name'],{encoding:'utf8'}).trim();
  return tags ? tags.split('\n').map(tag=>tag.replace(/^v/,'')).sort(compare).at(-1) : undefined;
}
function changesSince(previous) {
  const range=previous ? `refs/tags/v${previous}..HEAD` : 'HEAD';
  if(previous) {
    git('rev-parse','--verify',`refs/tags/v${previous}`);
    git('merge-base','--is-ancestor',`refs/tags/v${previous}`,'HEAD');
  }
  const messages=git('log','--no-merges','--format=%s',range);
  if(!messages) throw new Error('No changes since the published release');
  return messages.split('\n').map(subject=>`- ${subject}`).join('\n');
}
async function notes() {
  const changelog=await readFile('CHANGELOG.md','utf8');
  const entry=changelog.split(`## ${version}\n`)[1]?.split('\n## ')[0]?.trim();
  if(entry) return entry;
  const previous=latestPublished();
  return changesSince(previous);
}
async function sync() {
  config.version = version; await writeJson('desktop/tauri.conf.json', config);
  const lock = await json('package-lock.json'); lock.version = version; lock.packages[''].version = version; await writeJson('package-lock.json',lock);
  for (const file of ['native/Cargo.toml','desktop/Cargo.toml']) await writeFile(file,(await readFile(file,'utf8')).replace(/^version = "[^"]+"/m,`version = "${version}"`));
  for (const file of ['native/Cargo.lock','desktop/Cargo.lock']) await writeFile(file,(await readFile(file,'utf8')).replace(/(name = "(?:connector-core|local-connector-desktop)"\nversion = ")[^"]+/g,(_, prefix) => prefix + version));
  await check();
}
async function prepare() {
  if(args.some(arg=>arg!=='--dry-run')) throw new Error('prepare [--dry-run]');
  parts(version);
  const previous=latestPublished();
  const bump=previous && compare(version,previous)<=0;
  const target=bump ? [...parts(previous).slice(0,2),parts(previous)[2]+1n].join('.') : version;
  console.log(`Published: ${previous ?? 'none'}; current: ${version}; target: ${target}; release commits: ${bump?1:0}`);
  if(args.includes('--dry-run')) return;
  if(git('status','--porcelain')) throw new Error('Commit or set aside existing changes before release preparation');
  if(!bump) { await check(); return; }
  await check();
  git('var','GIT_AUTHOR_IDENT');
  git('var','GIT_COMMITTER_IDENT');
  const changes=changesSince(previous);
  const changelog=await readFile('CHANGELOG.md','utf8');
  version=target; pkg.version=target;
  await writeJson('package.json',pkg);
  const entry=`## ${version}\n\n${changes}\n\n`;
  if(!changelog.includes(`## ${version}\n`)) await writeFile('CHANGELOG.md',/^## /m.test(changelog)?changelog.replace(/(?=^## )/m,entry):changelog.trimEnd()+'\n\n'+entry);
  await sync();
  git('add','--',...releaseFiles);
  git('commit','-m',`chore(release): 更新版本至 ${version}`,'--',...releaseFiles);
}
if (mode === 'prepare') await prepare();
else if (mode === 'published-check') {
  await check(); const previous=latestPublished();
  if(previous && compare(version,previous)<=0) throw new Error(`Version ${version} must exceed published ${previous}; run release:prepare before rehearsal`);
}
else if (mode === 'check') await check();
else if (mode === 'sync') {
  await sync();
} else if (mode === 'stage') {
  await check();
  const [target, input, output] = args;
  if (!['darwin-aarch64','windows-x86_64'].includes(target) || !input || !output) throw new Error('stage <darwin-aarch64|windows-x86_64> <bundle-dir> <output-dir>');
  await mkdir(output,{recursive:true});
  if ((await readdir(output)).length) throw new Error('Stage directory must be empty; use a fresh directory');
  const candidates = (await files(input)).filter(f => target === 'darwin-aarch64' ? /(?:_aarch64\.dmg|\.app\.tar\.gz(?:\.sig)?)$/.test(f) : /\.exe(?:\.sig)?$/.test(f));
  for (const file of candidates) {
    let name = path.basename(file).replaceAll(' ','.');
    if (target === 'darwin-aarch64' && name.includes('.app.tar.gz')) name = name.replace('.app.tar.gz', `_${version}_aarch64.app.tar.gz`);
    await copyFile(file,path.join(output,name));
  }
  const staged = await files(output);
  const payloads = staged.filter(f => target === 'darwin-aarch64' ? f.endsWith('.app.tar.gz') : /\.exe$/.test(f));
  if (payloads.length !== 1) throw new Error('Missing/duplicate updater bundles');
  if (target === 'darwin-aarch64' && staged.filter(f=>f.endsWith('_aarch64.dmg')).length !== 1) throw new Error('Missing Apple Silicon DMG');
  verifySignatures(payloads);
  const platforms = {};
  for (const file of payloads) {
    const value = { signature:(await readFile(file+'.sig','utf8')).trim(), url:`${base}/${path.basename(file)}` };
    const keys = target === 'darwin-aarch64' ? ['darwin-aarch64'] : ['windows-x86_64','windows-x86_64-nsis'];
    for (const key of keys) platforms[key] = value;
  }
  await writeJson(path.join(output,`${target}.json`), { version, platforms });
  console.log(`Staged ${target} artifacts with verified signatures.`);
} else if (mode === 'finalize') {
  await check(); const [dir] = args;
  const fragments = await Promise.all(['darwin-aarch64','windows-x86_64'].map(t=>json(path.join(dir,t+'.json'))));
  if (fragments.some(f=>f.version!==version)) throw new Error('Mixed release versions');
  const releaseNotes = await notes();
  await writeJson(path.join(dir,'latest.json'),{version,notes:releaseNotes,pub_date:new Date().toISOString(),platforms:Object.assign({},...fragments.map(f=>f.platforms))});
  await validateManifest(await json(path.join(dir,'latest.json')), dir);
  await writeFile(path.join(dir,'release-notes.md'),releaseNotes+'\n');
  await cask(dir);
  const assets = (await files(dir)).filter(f=>/\.(dmg|exe|gz|sig)$/.test(f)||path.basename(f)==='local-connector.rb').sort();
  verifySignatures(assets.filter(f=>/\.(exe|gz)$/.test(f)));
  await writeFile(path.join(dir,'SHA256SUMS.txt'),(await Promise.all(assets.map(async f=>`${hash(await readFile(f))}  ${path.basename(f)}`))).join('\n')+'\n');
  console.log('Complete release metadata and Homebrew cask generated. Nothing uploaded.');
} else if (mode === 'cask') await cask(args[0]);
else if (mode === 'verify-published') {
  const dir = args[0]; const local = await json(path.join(dir,'latest.json'));
  await validateManifest(local, dir);
  const fetchBytes = async url => {
    for (let attempt = 0; attempt < 12; attempt++) {
      const response = await fetch(url, { signal: AbortSignal.timeout(120000) });
      if (response.ok) return Buffer.from(await response.arrayBuffer());
      if (![404, 502, 503, 504].includes(response.status) || attempt === 11) throw new Error(`${response.status}: ${url}`);
      await delay(5000);
    }
  };
  let remote;
  for (let attempt = 0; attempt < 12; attempt++) {
    remote = JSON.parse(await fetchBytes(`https://github.com/${repo}/releases/latest/download/latest.json`));
    if (JSON.stringify(remote) === JSON.stringify(local)) break;
    if (attempt === 11) throw new Error('Published latest.json mismatch');
    await delay(5000);
  }
  const assets=(await files(dir)).filter(f=>/\.(dmg|exe|gz|sig)$/.test(f)||f.endsWith('SHA256SUMS.txt')||f.endsWith('local-connector.rb'));
  for(const file of assets) if(hash(await fetchBytes(`${base}/${path.basename(file)}`))!==hash(await readFile(file))) throw new Error(`Published asset mismatch: ${path.basename(file)}`);
  console.log('Published manifest and all artifact bytes match the verified local release.');
} else throw new Error('Expected prepare, published-check, check, sync, stage, finalize, cask, or verify-published');
