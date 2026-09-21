import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { after, before, test } from 'node:test';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { AjvJsonSchemaValidator } from '@modelcontextprotocol/sdk/validation/ajv';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { toolFingerprint } from '../tooling/catalog.mjs';
const catalog = JSON.parse(await readFile(new URL('../native/src/catalog.json', import.meta.url), 'utf8'));
const nativeDomains = catalog.domains;
const version = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8')).version;

let directory: string;
let client: Client;
let inventory: Awaited<ReturnType<Client['listTools']>>;
before(async () => {
  directory = await mkdtemp(path.join(tmpdir(), 'chatgpt-local-connector-test-'));
  const fixture = path.join(directory, 'app-server.mjs');
  await writeFile(fixture, `
import { createInterface } from 'node:readline';
const counts = new Map();
const emit = value => process.stdout.write(JSON.stringify(value) + '\\n');
for await (const line of createInterface({ input: process.stdin })) {
  const { id, method, params } = JSON.parse(line);
  if (id === undefined) continue;
  const calls = (counts.get(method) || 0) + 1; counts.set(method, calls);
  const reply = () => {
    if (params?.fail) return emit({ id, error: { code: -32042, message: 'native fixture rejection', data: { retryable: false, detail: params.fail } } });
    if (method === 'initialize') return emit({ id, result: { userAgent: 'fixture' } });
    if (method === 'fixture/counts') return emit({ id, result: Object.fromEntries(counts) });
    if (method === 'fixture/null') return emit({ id, result: null });
    if (method === 'fixture/array') return emit({ id, result: [null, 1, { native: true }] });
    if (method === 'model/list' || method === 'thread/list') return emit({ id, result: { data: [], nextCursor: null } });
    if (method === 'process/spawn') {
      emit({ method: 'process/outputDelta', params: { processHandle: params.processHandle, stream: 'stdout', deltaBase64: 'b2s=' } });
      emit({ method: 'process/exited', params: { processHandle: params.processHandle, exitCode: 0 } });
    }
    if (method === 'fs/watch') emit({ method: 'fs/changed', params: { watchId: params.watchId, paths: [params.path] } });
    emit({ id, result: { method, params, calls } });
  };
  if (params?.delay) setTimeout(reply, params.delay); else reply();
}
`);
  // The existing fake upstream exercises the compiled Rust MCP core, isolated from Desktop.
  await mkdir(path.join(directory, 'src'));
  const corePath = fileURLToPath(new URL('../native', import.meta.url)).replaceAll('\\', '/');
  await writeFile(path.join(directory, 'Cargo.toml'), `[package]
name="contract-fixture"
version="0.1.0"
edition="2021"
[dependencies]
connector-core={path=${JSON.stringify(corePath)},features=["test-fixture"]}
serde_json="1"
tokio={version="1",features=["full"]}
`);
  await writeFile(path.join(directory, 'src/main.rs'), `
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[tokio::main] async fn main() {
 let service=connector_core::service::Service::fixture(std::env::var("CLC_FIXTURE_BINARY").unwrap().into()).unwrap();
 let mut lines=BufReader::new(tokio::io::stdin()).lines();
 let mut out=tokio::io::stdout();
 while let Some(line)=lines.next_line().await.unwrap() {
  let request=serde_json::from_str(&line).unwrap();
  let response=connector_core::transport::mcp(&service,request).await.unwrap();
  if !response.is_null() {out.write_all(format!("{response}\\n").as_bytes()).await.unwrap();out.flush().await.unwrap();}
 }
 service.control.close().await;
}`);
  const target = fileURLToPath(new URL('../native/target/contracts', import.meta.url));
  execFileSync('cargo', ['build', '--quiet', '--manifest-path', path.join(directory, 'Cargo.toml'), '--target-dir', target], { stdio: 'inherit' });
  client = new Client({ name: 'chatgpt-local-connector-test', version: '1' });
  await client.connect(new StdioClientTransport({ command: path.join(target, 'debug', 'contract-fixture' + (process.platform === 'win32' ? '.exe' : '')), env: { ...process.env, CLC_STATE_DIR: directory, CLC_FIXTURE_SCRIPT: fixture, CLC_FIXTURE_BINARY: process.execPath } as Record<string, string>, stderr: 'pipe' }));
  inventory = await client.listTools();
});
after(async () => { await client?.close(); if (directory) await rm(directory, { recursive: true, force: true }); });

async function call(name: string, args: Record<string, unknown> = {}) {
  const response = await client.callTool({ name, arguments: args });
  return { response, result: (response.structuredContent as { result: any } | undefined)?.result };
}
async function completed(name: string, args: Record<string, unknown>) {
  let { result } = await call(name, args);
  const deadline = Date.now() + 5000;
  while (['pending', 'reserved', 'submitting'].includes(result.state)) {
    assert.ok(Date.now() < deadline, 'receipt completed before deadline');
    await new Promise(resolve => setTimeout(resolve, 20));
    result = (await call('codex_request', { requestId: args.requestId })).result;
  }
  return result;
}

test('MCP lists all existing native and generic agent tools and seven discoverable native domains with valid object schemas', async () => {
  const existing = ['connector_verify', 'projects', 'overview', 'tree', 'search', 'read', 'git', 'codex_tasks', 'codex_read', 'codex_wait', 'codex_create', 'codex_send', 'codex_interrupt', 'codex_capabilities', 'codex_request', 'codex_items', 'codex_schema', 'codex_query', 'codex_call', 'codex_pending', 'codex_respond', 'codex_events', 'control_output'];
  assert.deepEqual(inventory.tools.map(tool => tool.name).sort(), [...existing, 'agents', 'agent_capabilities', 'agent_tasks', 'agent_create', 'agent_read', 'agent_wait', 'agent_send', 'agent_interrupt', 'agent_events', 'agent_pending', 'agent_respond', 'agent_request', 'fs', 'command', 'process', 'mcp', 'file_search', 'codex_thread', 'codex_account'].sort());
  for (const tool of inventory.tools) {
    assert.equal(tool.inputSchema.type, 'object');
    new AjvJsonSchemaValidator().getValidator(tool.inputSchema);
  }
  const agents = (await call('agents')).result.agents;
  assert.deepEqual(agents.map((agent: { agent: string }) => agent.agent).sort(), ['claude', 'cline', 'codex', 'copilot', 'cursor', 'devin', 'gemini', 'grok', 'hermes', 'junie', 'kimi', 'kiro', 'opencode', 'pi', 'qwen']);
  for (const agent of agents.filter((agent: { protocol: string }) => agent.protocol === 'acp-v1')) {
    assert.equal(agent.descriptor.protocol, 'acp-v1');
    assert.ok(['native', 'adapter'].includes(agent.integration));
  }
  const command = inventory.tools.find(tool => tool.name === 'command')!;
  assert.match(command.description!, /sandbox\/permission/);
  assert.match(inventory.tools.find(tool => tool.name === 'process')!.description!, /非 Codex sandbox/);
  assert.ok(command.inputSchema.required?.includes('requestId'));
  assert.equal(inventory.tools.find(tool => tool.name === 'codex_account')!.annotations?.readOnlyHint, true);
  assert.equal((await call('fs', { action: 'writeFile', params: {} })).response.isError, true);
  assert.notEqual((await call('fs', { action: 'readFile', params: {} })).response.isError, true);
  assert.equal((await call('fs', { action: 'exec', params: {} })).response.isError, true);
});

test('every domain action maps to the exact native RPC without adding, dropping or normalizing params', async () => {
  const expected: Record<string, Record<string, string>> = {
    fs: Object.fromEntries(['readFile', 'writeFile', 'createDirectory', 'getMetadata', 'readDirectory', 'remove', 'copy', 'watch', 'unwatch'].map(action => [action, `fs/${action}`])),
    command: { exec: 'command/exec', write: 'command/exec/write', terminate: 'command/exec/terminate', resize: 'command/exec/resize' },
    process: { spawn: 'process/spawn', writeStdin: 'process/writeStdin', kill: 'process/kill', resizePty: 'process/resizePty' },
    mcp: { list: 'mcpServerStatus/list', 'resource/read': 'mcpServer/resource/read', 'tool/call': 'mcpServer/tool/call', 'oauth/login': 'mcpServer/oauth/login', 'event/stream/start': 'mcpServer/event/stream/start', 'event/stream/stop': 'mcpServer/event/stream/stop' },
    codex_thread: { search: 'thread/search', fork: 'thread/fork', 'name/set': 'thread/name/set', archive: 'thread/archive', unarchive: 'thread/unarchive', compact: 'thread/compact/start', rollback: 'thread/rollback', revert: 'thread/revert', 'backgroundTerminals/list': 'thread/backgroundTerminals/list', 'backgroundTerminals/clean': 'thread/backgroundTerminals/clean', 'backgroundTerminals/terminate': 'thread/backgroundTerminals/terminate', timeline: 'thread/timeline/list' },
    codex_account: { read: 'account/read', 'usage/read': 'account/usage/read', 'rateLimits/read': 'account/rateLimits/read', 'workspaceMessages/read': 'account/workspaceMessages/read' },
  };
  for (const [domain, actions] of Object.entries(expected)) {
    assert.deepEqual(Object.keys(nativeDomains[domain as keyof typeof nativeDomains].actions).sort(), Object.keys(actions).sort());
    for (const [action, method] of Object.entries(actions)) {
      const params = { nativeFutureField: { falseValue: false, zero: 0, empty: '', nullable: null, array: [1, null] } };
      const value = await completed(domain, { action, params, requestId: randomUUID() });
      const result = value.state ? value.result : value;
      assert.equal(result.method, method);
      assert.deepEqual(result.params, params);
    }
  }
});

test('typical fs, PTY, downstream MCP and file search calls preserve native fields and events without creating threads', async () => {
  const cases: Array<[string, string, Record<string, unknown>]> = [
    ['fs', 'writeFile', { path: '/tmp/native.txt', dataBase64: 'aGk=' }],
    ['fs', 'watch', { path: '/tmp/native.txt', watchId: 'watch-test' }],
    ['command', 'exec', { command: ['cat'], cwd: '/tmp', env: { LANG: null, EXAMPLE: 'yes' }, timeoutMs: 0, tty: true, size: { rows: 24, cols: 80 }, processId: 'command-test', streamStdin: true, streamStdoutStderr: true, sandboxPolicy: { type: 'readOnly' }, disableOutputCap: true }],
    ['process', 'spawn', { command: ['cat'], cwd: '/tmp', processHandle: 'process-test', timeoutMs: null, outputBytesCap: null, tty: true }],
    ['mcp', 'tool/call', { server: 'local', threadId: 'existing-thread', tool: 'inspect', arguments: { input: [1, 2] }, _meta: { progressToken: 7 } }],
  ];
  for (const [name, action, params] of cases) assert.deepEqual((await completed(name, { action, params, requestId: randomUUID() })).result.params, params);
  const params = { query: 'ctrlmcp', roots: ['/tmp'], cancellationToken: null };
  assert.deepEqual((await call('file_search', params)).result.params, params);
  assert.equal((await call('codex_account', { action: 'workspaceMessages/read' })).result.params, null);
  const events = (await call('codex_events')).result.events;
  assert.ok(events.some((event: any) => event.method === 'fs/changed' && event.params.watchId === 'watch-test'));
  assert.ok(events.some((event: any) => event.method === 'process/exited' && event.params.processHandle === 'process-test'));
  const counts = (await completed('codex_call', { method: 'fixture/counts', requestId: randomUUID() })).result;
  assert.equal(counts['thread/start'], undefined);
  assert.equal(counts['turn/start'], undefined);
});

test('mutation receipts remain idempotent across domain and escape hatch, including pending and conflicting retries', async () => {
  const requestId = randomUUID();
  const params = { path: '/tmp/retry', dataBase64: '', delay: 200 };
  const first = await call('fs', { action: 'writeFile', params, requestId });
  assert.equal(first.result.state, 'pending');
  const original = await completed('fs', { action: 'writeFile', params, requestId });
  const replay = await completed('codex_call', { method: 'fs/writeFile', params, requestId });
  assert.equal(replay.replayed, true);
  assert.deepEqual(replay.result, original.result);
  const conflict = await call('fs', { action: 'writeFile', params: { ...params, path: '/tmp/other' }, requestId });
  assert.equal(conflict.response.isError, true);
  assert.equal(conflict.result.error.code, 'REQUEST_ID_CONFLICT');
});

test('native errors retain code/message/data for both direct reads and mutation receipts; raw null/array results survive', async () => {
  const rpcError = { code: -32042, message: 'native fixture rejection', data: { retryable: false, detail: 'bad argument' } };
  const read = await call('fs', { action: 'readFile', params: { fail: 'bad argument' } });
  assert.equal(read.response.isError, true);
  assert.deepEqual(read.result.error.rpcError, rpcError);
  const write = await completed('process', { action: 'spawn', params: { fail: 'bad argument' }, requestId: randomUUID() });
  assert.equal(write.state, 'rejected');
  assert.deepEqual(write.result.error.rpcError, rpcError);
  assert.equal((await completed('codex_call', { method: 'fixture/null', params: null, requestId: randomUUID() })).result, null);
  assert.deepEqual((await completed('codex_call', { method: 'fixture/array', requestId: randomUUID() })).result, [null, 1, { native: true }]);
  assert.equal((await call('codex_tasks')).result.tasks.length, 0);
  assert.equal((await call('codex_capabilities')).result.version, version);
});

test('deployment fingerprints ignore tool/property ordering and runtime changes but detect every published definition change', async () => {
  const before = toolFingerprint(inventory.tools);
  const reorder = (value: any): any => Array.isArray(value) ? value.map(reorder) : value && typeof value === 'object'
    ? Object.fromEntries(Object.entries(value).reverse().map(([key, item]) => [key, reorder(item)])) : value;
  assert.deepEqual(toolFingerprint(reorder([...inventory.tools].reverse())), before);
  await call('codex_capabilities');
  assert.deepEqual(toolFingerprint((await client.listTools()).tools), before);
  for (const change of [
    (tools: any[]) => { tools[0].inputSchema.properties.added = { type: 'string' }; },
    (tools: any[]) => { tools[0].description += ' changed'; },
    (tools: any[]) => { tools[0].annotations.readOnlyHint = false; },
    (tools: any[]) => { tools[0].outputSchema.properties.result = { type: 'string' }; },
    (tools: any[]) => { tools.pop(); },
  ]) {
    const tools = structuredClone(inventory.tools); change(tools);
    assert.notEqual(toolFingerprint(tools).toolSchemaDigest, before.toolSchemaDigest);
  }
  assert.throws(() => toolFingerprint([inventory.tools[0], inventory.tools[0]]), /Duplicate/);
});

test('running owner retains startup package identity when the package manifest is replaced', async () => {
  const first = (await call('codex_capabilities')).result;
  await writeFile(path.join(directory, 'package.json'), JSON.stringify({ version: '999.0.0' }));
  const second = (await call('codex_capabilities')).result;
  assert.equal(first.version, version);
  assert.equal(second.version, first.version);
  assert.equal(second.backendSession, first.backendSession);
  assert.ok(first.backendSession);
});
