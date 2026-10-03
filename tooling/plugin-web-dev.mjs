import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createServer } from 'vite';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { buildNative } from './build-native.mjs';
import { devStateDir } from './dev-state.mjs';

if (process.argv.length > 2) throw new Error('Usage: npm run plugin:web');
const root = fileURLToPath(new URL('../', import.meta.url));
const state = process.env.CLC_STATE_DIR ? devStateDir : `${devStateDir}-dev`;
process.env.CLC_STATE_DIR = state;
await import('./build-plugin.mjs');
const binary = await buildNative();
const client = new Client({ name: 'clc-plugin-web', version: '1' });
const tools = new Set(['connector_overview', 'connector_task_usage', 'connector_usage_refresh', 'connector_plugin_check', 'connector_plugin_update']);
let server;
let stopping = false;
async function stop() {
  if (stopping) return;
  stopping = true;
  try { await server?.close(); } finally { await client.close(); }
}
process.on('SIGINT', () => { void stop(); });
process.on('SIGTERM', () => { void stop(); });
try {
  await client.connect(new StdioClientTransport({ command: binary, args: ['mcp'], env: { ...process.env }, stderr: 'inherit' }));
  server = await createServer({
    configFile: path.join(root, 'vite.config.ts'),
    plugins: [{
      name: 'connector-plugin-web-host',
      configureServer(vite) {
        vite.middlewares.use((request, response, next) => {
          if (request.url !== '/__plugin/tools') return next();
          response.setHeader('Cache-Control', 'no-store');
          response.setHeader('Content-Type', 'application/json');
          if (request.method !== 'POST' || request.headers.host !== '127.0.0.1:5187'
            || request.headers.origin !== 'http://127.0.0.1:5187'
            || request.headers['sec-fetch-site'] === 'cross-site'
            || request.headers['x-clc-request'] !== '1') {
            response.writeHead(403); response.end(JSON.stringify({ error: '仅允许本机调试页面访问。' })); return;
          }
          void (async () => {
            const chunks = [];
            let size = 0;
            for await (const chunk of request) {
              size += chunk.length;
              if (size > 65536) { response.writeHead(413); response.end(); return; }
              chunks.push(Buffer.from(chunk));
            }
            const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
            if (!tools.has(body.name)) {
              response.writeHead(400); response.end(JSON.stringify({ error: '仅支持插件面板工具。' })); return;
            }
            const result = await client.callTool({ name: body.name, arguments: body.arguments ?? {},
              ...(typeof body.threadId === 'string' && body.threadId ? { _meta: { threadId: body.threadId } } : {}) }, undefined, { timeout: 330000 });
            if (!response.destroyed) response.end(JSON.stringify(result));
          })().catch(error => {
            if (!response.destroyed) { response.writeHead(502); response.end(JSON.stringify({ error: error.message })); }
          });
        });
      },
    }],
  });
  await server.listen();
  console.log('Plugin Web ready:\nOverview: http://127.0.0.1:5187/plugin.html?scope=global\nTask: http://127.0.0.1:5187/plugin.html?scope=thread\nReact/CSS changes hot-reload. Rust changes require restarting this command.');
} catch (error) {
  await stop();
  throw error;
}
