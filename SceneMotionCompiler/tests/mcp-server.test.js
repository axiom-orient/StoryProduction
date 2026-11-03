import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseMcpLaunchConfig } from '../apps/mcp-server/src/launch-config.js';

test('MCP launch config accepts only repository and project roots', () => {
  const scope = parseMcpLaunchConfig(['--root', '/repo', '--project', '/work/project'], { cwd: '/launch' });
  assert.equal(scope.repoRoot, '/repo');
  assert.equal(scope.projectRoot, '/work/project');
  assert.throws(() => parseMcpLaunchConfig(['--root', '/repo', '--project', '/work/project', '--provider-command', 'provider'], { cwd: '/launch' }), /Unknown option/);
});

test('MCP startup diagnostics use stderr and leave protocol stdout empty', async () => {
  const child = spawn(process.execPath, ['apps/mcp-server/src/cli.js', '--project', join(tmpdir(), 'smc-mcp-missing-input')], { cwd: process.cwd(), stdio: ['ignore', 'pipe', 'pipe'] });
  const result = await collectChild(child);
  assert.equal(result.code, 1);
  assert.equal(result.stdout, '');
  assert.match(result.stderr, /^\[smc-mcp\] MCP_ARGUMENT_REQUIRED:/);
});

test('stdio MCP exposes seven non-approval production and Track B tools', { timeout: 30_000 }, async (t) => {
  const projectRoot = await mkdtemp(join(tmpdir(), 'smc-mcp-project-'));
  const child = spawn(process.execPath, ['apps/mcp-server/src/cli.js', '--root', process.cwd(), '--project', projectRoot], { cwd: process.cwd(), stdio: ['pipe', 'pipe', 'pipe'] });
  t.after(async () => {
    if (child.exitCode === null) {
      child.kill('SIGTERM');
      await new Promise((resolve) => child.once('exit', resolve));
    }
    await rm(projectRoot, { recursive: true, force: true });
  });
  const client = createLineClient(child);
  const discovered = await client.request('server/discover', { _meta: modernRequestMeta() });
  assert.deepEqual(discovered.supportedVersions, ['2026-07-28']);
  assert.equal(discovered.resultType, 'complete');
  assert.equal(discovered._meta['io.modelcontextprotocol/serverInfo'].name, 'scene-motion-compiler');
  const listed = await client.request('tools/list', { _meta: modernRequestMeta() });
  assert.equal(listed.resultType, 'complete');
  assert.deepEqual(listed.tools.map((tool) => tool.name).sort(), [
    'production_prepare', 'compile_scene_html', 'scene_project_narrative_bundle', 'trackb_evidence_measure',
    'trackb_release_prepare', 'trackb_release_promote', 'trackb_release_status'
  ].sort());
});

test('stdio MCP rejects the legacy initialize handshake before accepting modern claimed requests', { timeout: 30_000 }, async (t) => {
  const projectRoot = await mkdtemp(join(tmpdir(), 'smc-mcp-modern-only-'));
  const child = spawn(process.execPath, ['apps/mcp-server/src/cli.js', '--root', process.cwd(), '--project', projectRoot], { cwd: process.cwd(), stdio: ['pipe', 'pipe', 'pipe'] });
  t.after(async () => {
    if (child.exitCode === null) {
      child.kill('SIGTERM');
      await new Promise((resolve) => child.once('exit', resolve));
    }
    await rm(projectRoot, { recursive: true, force: true });
  });
  const client = createLineClient(child);
  await assert.rejects(
    () => client.request('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'legacy-client', version: '1' } }),
    /Unsupported protocol version|modern-only|2025-era/
  );
  const discovered = await client.request('server/discover', { _meta: modernRequestMeta() });
  assert.deepEqual(discovered.supportedVersions, ['2026-07-28']);
  assert.equal(discovered.resultType, 'complete');
});

function modernRequestMeta() {
  return {
    'io.modelcontextprotocol/protocolVersion': '2026-07-28',
    'io.modelcontextprotocol/clientCapabilities': {}
  };
}

function createLineClient(child) {
  let nextId = 1; let buffer = ''; let stderr = ''; const pending = new Map();
  child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => { stderr += chunk; });
  child.stdout.on('data', (chunk) => {
    buffer += chunk;
    while (buffer.includes('\n')) {
      const index = buffer.indexOf('\n'); const line = buffer.slice(0, index); buffer = buffer.slice(index + 1); if (!line) continue;
      let message; try { message = JSON.parse(line); } catch (error) { for (const waiter of pending.values()) waiter.reject(error); pending.clear(); continue; }
      const waiter = pending.get(message.id); if (!waiter) continue; pending.delete(message.id); message.error ? waiter.reject(new Error(message.error.message)) : waiter.resolve(message.result);
    }
  });
  child.on('exit', (code, signal) => { for (const waiter of pending.values()) waiter.reject(new Error(`MCP exited: ${code}/${signal}; ${stderr}`)); pending.clear(); });
  return {
    request(method, params) { const id = nextId++; return new Promise((resolve, reject) => { const timer = setTimeout(() => { pending.delete(id); reject(new Error(`MCP timeout: ${method}; ${stderr}`)); }, 10_000); pending.set(id, { resolve: (value) => { clearTimeout(timer); resolve(value); }, reject: (error) => { clearTimeout(timer); reject(error); } }); child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`); }); },
  };
}
function collectChild(child) { return new Promise((resolve) => { let stdout = ''; let stderr = ''; child.stdout.setEncoding('utf8'); child.stderr.setEncoding('utf8'); child.stdout.on('data', (chunk) => { stdout += chunk; }); child.stderr.on('data', (chunk) => { stderr += chunk; }); child.once('exit', (code, signal) => resolve({ code, signal, stdout, stderr })); }); }
