#!/usr/bin/env node
import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createStudioServer } from '../apps/studio-host/src/server.js';
import { STUDIO_COMMANDS } from '../packages/application/src/command-registry.js';

const projectRoot = await mkdtemp(join(tmpdir(), 'smc-trackb-studio-'));
const server = createStudioServer({ port: 0, projectRoot, repoRoot: process.cwd(), localOperatorId: 'operator.lifecycle-qa' });
await listen(server);
const origin = `http://127.0.0.1:${server.address().port}`;
try {
  const metadata = await fetch(`${origin}/api/metadata`);
  assert.equal(metadata.status, 200);
  const commands = Object.keys((await metadata.json()).commands ?? {}).sort();
  assert.deepEqual(commands, [...STUDIO_COMMANDS].sort());

  const forbidden = await fetch(`${origin}/api/execute`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ command: 'project.status', input: {} })
  });
  assert.equal(forbidden.status, 403);
  assert.equal((await forbidden.json()).error.code, 'studio-command-forbidden');
  console.log(JSON.stringify({ schemaVersion: 1, status: 'PASS', surface: 'trackb-studio', native: 'NOT_RUN_ENVIRONMENT' }, null, 2));
} finally {
  await close(server);
  await rm(projectRoot, { recursive: true, force: true });
}

function listen(server) {
  return new Promise((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveListen);
  });
}
function close(server) {
  return new Promise((resolveClose, reject) => server.close((error) => error ? reject(error) : resolveClose()));
}
