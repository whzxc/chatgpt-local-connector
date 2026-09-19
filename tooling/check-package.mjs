import { readFile } from 'node:fs/promises';
import { toolFingerprint } from './catalog.mjs';
import { readdir, stat } from 'node:fs/promises';
import path from 'node:path';
const root = process.argv[2] || 'desktop/target/release/bundle/macos/Local Connector.app';
const catalog = JSON.parse(await readFile(new URL('../native/src/catalog.json', import.meta.url), 'utf8'));
toolFingerprint(catalog.tools);
let bytes = 0;
async function inspect(dir) {
  for (const item of await readdir(dir, { withFileTypes: true })) {
    const file = path.join(dir, item.name);
    if (['node_modules', 'runtime', 'node', 'node.exe', 'npm', 'npx', 'desktop-launch.mjs'].includes(item.name)) throw new Error(`产物仍包含旧运行环境：${file}`);
    if (item.isDirectory()) await inspect(file);
    else bytes += (await stat(file)).size;
  }
}
await inspect(root);
console.log(`原生发行产物检查通过：${(bytes / 1024 / 1024).toFixed(2)} MiB，无 Node/npm 运行环境。`);
