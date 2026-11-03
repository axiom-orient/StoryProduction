import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, readFile, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  MacosStudioLaunchError,
  macosOperatorId,
  startMacosStudio
} from '../apps/studio-host/src/macos-launcher.js';
import { validateStudioProjectRoot } from '../apps/studio-host/src/project-root.js';

async function readyProject() {
  const root = await mkdtemp(join(tmpdir(), 'smc-macos-launcher-'));
  await mkdir(join(root, 'handoff'));
  await writeFile(join(root, 'handoff/scene-motion-asset-handoff.v2.json'), '{}');
  return root;
}

test('macOS launcher project validation requires a real published handoff path', async () => {
  const root = await readyProject();
  const empty = await mkdtemp(join(tmpdir(), 'smc-macos-launcher-empty-'));
  const linked = `${root}-link`;
  try {
    assert.equal(await validateStudioProjectRoot(root), await realpath(root));
    await assert.rejects(
      validateStudioProjectRoot(empty),
      (error) => error instanceof MacosStudioLaunchError && error.code === 'project-not-ready'
    );
    await symlink(root, linked);
    await assert.rejects(
      validateStudioProjectRoot(linked),
      (error) => error instanceof MacosStudioLaunchError && error.code === 'project-unsafe'
    );
  } finally {
    await rm(linked, { force: true });
    await rm(root, { recursive: true, force: true });
    await rm(empty, { recursive: true, force: true });
  }
});

test('macOS launcher starts the valid project and stops cleanly', async () => {
  const ready = await readyProject();
  let launched;
  try {
    launched = await startMacosStudio({
      repoRoot: process.cwd(),
      projectRoot: ready,
      operatorId: 'operator.macos.test'
    });
    assert.equal(launched.status, 'RUNNING');
    assert.equal(launched.projectRoot, await realpath(ready));
    assert.match(launched.origin, /^http:\/\/127\.0\.0\.1:\d+$/);
    assert.equal(launched.server.listening, true);
  } finally {
    await launched?.stop();
    await rm(ready, { recursive: true, force: true });
  }
});

test('macOS launcher rejects an unready project before creating a server', async () => {
  const empty = await mkdtemp(join(tmpdir(), 'smc-macos-launcher-empty-'));
  try {
    await assert.rejects(
      startMacosStudio({ repoRoot: process.cwd(), projectRoot: empty, operatorId: 'operator.macos.test' }),
      (error) => error instanceof MacosStudioLaunchError && error.code === 'project-not-ready'
    );
  } finally {
    await rm(empty, { recursive: true, force: true });
  }
});

test('macOS launcher derives a bounded opaque operator id without exposing the username', () => {
  const id = macosOperatorId('사용자 name with spaces');
  assert.match(id, /^operator\.macos\.[a-f0-9]{16}$/);
  assert.equal(id.includes('사용자'), false);
});

test('macOS launcher child stops when its native parent exits', { timeout: 10_000 }, async () => {
  const ready = await readyProject();
  const logPath = join(ready, 'watchdog.log');
  const launcherScript = fileURLToPath(new URL('../scripts/launch-studio-macos.mjs', import.meta.url));
  const helperSource = String.raw`
    const { spawn } = require('node:child_process');
    const fs = require('node:fs');
    const [logPath, launcherScript, projectRoot] = process.argv.slice(1);
    const output = fs.createWriteStream(logPath, { flags: 'a' });
    const child = spawn(process.execPath, [launcherScript, '--project', projectRoot, '--parent-pid', String(process.pid)], {
      stdio: ['ignore', 'pipe', 'pipe']
    });
    let buffered = '';
    const consume = (chunk) => {
      const text = chunk.toString('utf8');
      output.write(text);
      buffered += text;
      if (buffered.includes('SMC_STUDIO_READY\t')) {
        process.stdout.write(String(child.pid));
        setTimeout(() => process.exit(0), 50);
      }
    };
    child.stdout.on('data', consume);
    child.stderr.on('data', consume);
    child.once('error', (error) => { output.write(String(error)); process.exit(3); });
    setTimeout(() => process.exit(2), 5000);
  `;
  let childPid;
  try {
    const helper = spawn(process.execPath, ['-e', helperSource, logPath, launcherScript, ready], {
      stdio: ['ignore', 'pipe', 'pipe']
    });
    const stdout = [];
    const stderr = [];
    helper.stdout.on('data', (chunk) => stdout.push(Buffer.from(chunk)));
    helper.stderr.on('data', (chunk) => stderr.push(Buffer.from(chunk)));
    const helperCode = await new Promise((resolveExit, reject) => {
      helper.once('error', reject);
      helper.once('exit', resolveExit);
    });
    assert.equal(helperCode, 0, Buffer.concat(stderr).toString('utf8'));
    childPid = Number(Buffer.concat(stdout).toString('utf8'));
    assert.equal(Number.isSafeInteger(childPid) && childPid > 1, true);
    assert.match(await readFile(logPath, 'utf8'), /SMC_STUDIO_READY\t/);
    await waitUntil(() => !isPidAlive(childPid), 4_000);
    assert.equal(isPidAlive(childPid), false);
  } finally {
    if (childPid && isPidAlive(childPid)) process.kill(childPid, 'SIGTERM');
    await rm(ready, { recursive: true, force: true });
  }
});

function isPidAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error?.code === 'ESRCH') return false;
    throw error;
  }
}

async function waitUntil(predicate, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await new Promise((resolveWait) => setTimeout(resolveWait, 50));
  }
}
