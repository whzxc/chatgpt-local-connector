import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { request as httpRequest } from 'node:http';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import path from 'node:path';

const root = fileURLToPath(new URL('.', import.meta.url));
const stateRoot = process.env.CLC_STATE_DIR || path.join(process.platform === 'win32' ? process.env.LOCALAPPDATA || path.join(homedir(), 'AppData/Local') : path.join(homedir(), '.local/state'), 'chatgpt-local-connector');
type Endpoint = { port: number; token: string };
export default defineConfig({
  root: fileURLToPath(new URL('./ui', import.meta.url)),
  plugins: [vue(), {
    name: 'connector-native-preview',
    configureServer(server) {
      let child: ReturnType<typeof spawn> | undefined;
      let preview: Promise<Endpoint> | undefined;
      function localPreview() {
        return preview ??= new Promise<Endpoint>((resolve, reject) => {
          child = spawn('cargo', ['run', '--quiet', '--manifest-path', 'native/Cargo.toml', '--bin', 'preview'], { cwd: root, stdio: ['ignore', 'pipe', 'inherit'] });
          child.on('error', reject);
          child.on('exit', () => { reject(new Error('原生预览进程已退出')); preview = undefined; });
          createInterface({ input: child.stdout! }).once('line', line => {
            try { resolve(JSON.parse(line)); } catch { reject(new Error('无效的原生预览响应')); }
          });
        });
      }
      function ownerIdentity() {
        try { return readFileSync(path.join(stateRoot, 'web/native.json'), 'utf8'); }
        catch { return ''; }
      }
      async function endpoint(): Promise<Endpoint> {
        try {
          const info = JSON.parse(readFileSync(path.join(stateRoot, 'web/native.json'), 'utf8'));
          if (Number.isInteger(info.port) && info.port > 0 && info.port <= 65535 && typeof info.token === 'string') {
            const response = await fetch(`http://127.0.0.1:${info.port}/healthz`, { headers: { Authorization: `Bearer ${info.token}` }, signal: AbortSignal.timeout(500) });
            if (response.ok && (await response.json()).instance === info.instance) return info;
          }
        } catch { /* No running native app: use a read-only native preview. */ }
        return localPreview();
      }
      const nativeRoot = path.join(root, 'native');
      server.watcher.add([path.join(nativeRoot, 'src'), path.join(nativeRoot, 'Cargo.toml')]);
      server.watcher.on('change', file => {
        if (file.startsWith(path.join(nativeRoot, 'src') + path.sep) || file === path.join(nativeRoot, 'Cargo.toml')) {
          child?.kill('SIGINT'); child = undefined; preview = undefined;
        }
      });
      server.httpServer?.once('close', () => child?.kill('SIGINT'));
      server.middlewares.use((request, response, next) => {
        if (!request.url?.startsWith('/api/')) return next();
        response.setHeader('Cache-Control', 'no-store');
        if (request.method !== 'GET' || request.headers.host !== '127.0.0.1:5173'
          || (request.headers.origin && request.headers.origin !== 'http://127.0.0.1:5173')
          || request.headers['sec-fetch-site'] === 'cross-site'
          || (request.url === '/api/config/credentials' && request.headers['x-clc-request'] !== '1')) {
          response.writeHead(403, { 'Content-Type': 'application/json' });
          response.end(JSON.stringify({ error: '开发预览只读，仅允许本机页面访问。' })); return;
        }
        void endpoint().then(info => {
          if (response.destroyed) return;
          const upstream = httpRequest({ hostname: '127.0.0.1', port: info.port, path: request.url, headers: { Authorization: `Bearer ${info.token}` } }, incoming => {
            response.writeHead(incoming.statusCode || 502, incoming.headers); incoming.pipe(response);
          });
          upstream.on('error', error => { if (!response.headersSent) response.writeHead(503, { 'Content-Type': 'application/json' }); response.end(JSON.stringify({ error: error.message })); });
          // Reconnect EventSource when the native app starts, exits, or is replaced.
          // Otherwise an already-open preview stream can stay attached to its old owner.
          const owner = ownerIdentity();
          const watchOwner = request.url === '/api/events' ? setInterval(() => {
            if (ownerIdentity() !== owner) { upstream.destroy(); response.end(); }
          }, 1000) : undefined;
          response.on('close', () => { clearInterval(watchOwner); upstream.destroy(); }); upstream.end();
        }).catch(error => { response.writeHead(503, { 'Content-Type': 'application/json' }); response.end(JSON.stringify({ error: error.message })); });
      });
    },
  }],
  build: { outDir: '../dist/ui', emptyOutDir: true },
  server: { host: '127.0.0.1', port: 5173, strictPort: true },
});
