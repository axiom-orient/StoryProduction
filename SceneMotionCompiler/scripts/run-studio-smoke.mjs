#!/usr/bin/env node
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { createStudioServer } from '../apps/studio-host/src/server.js';
import { STUDIO_COMMANDS } from '../packages/application/src/command-registry.js';

const positional = process.argv.slice(2).filter((value) => value !== '--contract-only');
const projectRoot = resolve(positional[0] ?? '.tmp/studio-project');
const outputRoot = resolve(positional[1] ?? '.tmp/studio-contract');
await rm(outputRoot, { recursive: true, force: true });
await mkdir(outputRoot, { recursive: true });

const server = createStudioServer({ port: 0, repoRoot: process.cwd(), projectRoot, localOperatorId: 'operator.studio-smoke' });
await listen(server);
const origin = `http://127.0.0.1:${server.address().port}`;
try {
  const dashboard = await fetch(`${origin}/`);
  const html = await dashboard.text();
  if (dashboard.status !== 200 || (html.match(/<h1\b/g) ?? []).length !== 1
    || html.includes('Track A') || html.includes('__TAURI__') || html.includes('renderVideo')) {
    throw new Error('Studio dashboard is not Track B-only');
  }
  const metadata = await fetch(`${origin}/api/metadata`);
  const body = await metadata.json();
  const commands = Object.keys(body.commands ?? {}).sort();
  if (metadata.status !== 200 || JSON.stringify(commands) !== JSON.stringify([...STUDIO_COMMANDS].sort())) {
    throw new Error(`Studio metadata boundary failed: ${commands.join(', ')}`);
  }
  const forbidden = await fetch(`${origin}/api/execute`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ command: 'project.compile', input: {} })
  });
  const forbiddenBody = await forbidden.json();
  if (forbidden.status !== 403 || forbiddenBody.error?.code !== 'studio-command-forbidden') {
    throw new Error('Studio retired command boundary failed');
  }
  await writeFile(join(outputRoot, 'studio-smoke.json'), `${JSON.stringify({ status: 'PASS', mode: 'HTTP_CONTRACT', origin }, null, 2)}\n`);
  console.log(JSON.stringify({ status: 'PASS', mode: 'HTTP_CONTRACT', outputRoot }, null, 2));
} finally {
  await close(server);
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
