import assert from 'node:assert/strict';
import { access, lstat, mkdir, mkdtemp, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  DEFAULT_GATE_NAMES,
  createDefaultGates,
  resolveGateTimings,
  runAggregateVerification,
} from '../scripts/aggregate-verifier.mjs';
import { buildSourceManifest, collectSourceFiles } from '../packages/source-integrity/src/index.js';

const wait = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

async function makeRoot() {
  const root = await mkdtemp(join(tmpdir(), 'smc-aggregate-verifier-'));
  await mkdir(join(root, '.tmp'));
  await writeFile(join(root, 'aggregate-fixture.txt'), 'aggregate verifier fixture\n');
  const files = await collectSourceFiles(root, { excludeGeneratedMetadata: true });
  const manifest = buildSourceManifest(files);
  await writeFile(join(root, 'SOURCE_MANIFEST.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  return root;
}

async function removeRoot(root) {
  await rm(root, { recursive: true, force: true });
}

function command(source) {
  return [process.execPath, '-e', source];
}

function fixtureGates({ failingIndex = -1, source = 'process.exit(0);', timeoutMs = 5_000 } = {}) {
  return DEFAULT_GATE_NAMES.map((name, index) => ({
    name,
    command: failingIndex === index ? command('process.exit(17);') : command(source),
    timeoutMs,
    termGraceMs: 100,
    killGraceMs: 500,
  }));
}

async function waitForPath(path, timeoutMs = 2_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await access(path);
      return;
    } catch {
      await wait(20);
    }
  }
  throw new Error(`timed out waiting for ${path}`);
}

function assertProcessAbsent(pid) {
  assert.throws(() => process.kill(Number(pid), 0), (error) => error?.code === 'ESRCH');
}

test('aggregate verifier publishes a complete Track B gate summary', async () => {
  const root = await makeRoot();
  try {
    const result = await runAggregateVerification({ root, gates: fixtureGates() });
    assert.equal(result.status, 'PASS');
    assert.equal(result.exitCode, 0);
    assert.equal(result.summaryPublished, true);
    assert.equal(result.summaryPublicationAttempted, true);
    assert.equal(result.summary.schemaVersion, 2);
    assert.equal(result.summary.gateCount, 10);
    assert.deepEqual(result.summary.gates.map((gate) => gate.status), Array(10).fill('PASS'));
    const manifest = JSON.parse(await readFile(join(root, 'SOURCE_MANIFEST.json'), 'utf8'));
    assert.deepEqual(result.summary.sourceIdentity, {
      manifestPath: 'SOURCE_MANIFEST.json',
      status: 'PASS',
      declared: { sourceTreeHash: manifest.sourceTreeHash, fileCount: manifest.fileCount },
      observed: { sourceTreeHash: manifest.sourceTreeHash, fileCount: manifest.fileCount },
      verifiedAt: result.summary.sourceIdentity.verifiedAt,
      mismatches: [],
      error: null,
    });
    assert.match(result.summary.sourceIdentity.verifiedAt, /^\d{4}-\d{2}-\d{2}T/);
    assert.equal(result.summary.gates.find((gate) => gate.gate === 'source-manifest').status, 'PASS');
    assert.equal(result.summary.lock.released, true);
    assert.equal(result.summary.aggregateCleanup.survivorsNotProven, 0);
    for (const gate of result.summary.gates) {
      assert.equal(gate.logCreated, true);
      assert.equal(gate.cleanup.processGroupChecked, true);
      assert.equal(gate.cleanup.groupAbsent, true);
      assert.equal(gate.cleanup.survivorStatus, 'ABSENT');
      assert.equal(typeof gate.exitCode, 'number');
      assert.equal(gate.signal, null);
      assert.ok(gate.startedAt);
      assert.ok(gate.endedAt);
      assert.ok(gate.durationMs >= 0);
      assert.ok(gate.logPath.endsWith(`${gate.gate}.log`));
      assert.equal(gate.maxLogBytes, 10 * 1024 * 1024);
      assert.equal(gate.logIdentity.valid, true);
      assert.equal(gate.logIdentity.opened.dev, gate.logIdentity.observed.dev);
      assert.equal(gate.logIdentity.opened.ino, gate.logIdentity.observed.ino);
      assert.equal(gate.logIdentity.observed.size, gate.logBytes);
    }
    const summaryText = await readFile(result.summaryPath, 'utf8');
    assert.ok(summaryText.endsWith('\n'));
    assert.deepEqual(JSON.parse(summaryText).sourceIdentity, result.summary.sourceIdentity);
    const entries = await readdir(join(root, '.tmp', 'verify'));
    assert.equal(entries.some((entry) => entry.includes('.summary.json.') && entry.endsWith('.tmp')), false);
    await assert.rejects(() => lstat(join(root, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(root);
  }
});

test('final source identity mismatch downgrades an otherwise-PASS aggregate and is published', async () => {
  const root = await makeRoot();
  try {
    await writeFile(join(root, 'aggregate-fixture.txt'), 'tampered after manifest generation\n');
    const result = await runAggregateVerification({ root, gates: fixtureGates() });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    assert.equal(result.summaryPublished, true);
    assert.deepEqual(result.summary.gates.map((gate) => gate.status), Array(10).fill('PASS'));
    assert.equal(result.summary.sourceIdentity.status, 'FAIL');
    assert.equal(result.summary.error.code, 'E_VERIFY_SOURCE_IDENTITY');
    assert.ok(result.summary.sourceIdentity.verifiedAt);
    assert.ok(result.summary.sourceIdentity.mismatches.some((mismatch) => mismatch.field === 'sourceTreeHash'));
    const persisted = JSON.parse(await readFile(result.summaryPath, 'utf8'));
    assert.deepEqual(persisted.sourceIdentity, result.summary.sourceIdentity);
    assert.deepEqual(persisted.error, result.summary.error);
  } finally {
    await removeRoot(root);
  }
});

test('missing, malformed, and symlinked production manifests fail closed with a typed identity error', async () => {
  const cases = [
    {
      name: 'missing',
      mutate: async (root) => rm(join(root, 'SOURCE_MANIFEST.json')),
    },
    {
      name: 'malformed',
      mutate: async (root) => writeFile(join(root, 'SOURCE_MANIFEST.json'), '{ malformed json\n'),
    },
    {
      name: 'symlinked',
      mutate: async (root) => {
        const target = join(root, '.tmp', 'manifest-target.json');
        await writeFile(target, '{}');
        await rm(join(root, 'SOURCE_MANIFEST.json'));
        await symlink(target, join(root, 'SOURCE_MANIFEST.json'));
      },
    },
  ];
  for (const testCase of cases) {
    const root = await makeRoot();
    try {
      await testCase.mutate(root);
      const result = await runAggregateVerification({ root, gates: fixtureGates() });
      assert.equal(result.status, 'FAIL', testCase.name);
      assert.equal(result.exitCode, 1, testCase.name);
      assert.equal(result.summaryPublished, true, testCase.name);
      assert.equal(result.summary.sourceIdentity.status, 'FAIL', testCase.name);
      assert.equal(result.summary.error.code, 'E_VERIFY_SOURCE_IDENTITY', testCase.name);
      assert.equal(result.summary.sourceIdentity.manifestPath, 'SOURCE_MANIFEST.json', testCase.name);
      assert.ok(result.summary.sourceIdentity.verifiedAt, testCase.name);
      const persisted = JSON.parse(await readFile(result.summaryPath, 'utf8'));
      assert.deepEqual(persisted.sourceIdentity, result.summary.sourceIdentity, testCase.name);
      assert.deepEqual(persisted.error, result.summary.error, testCase.name);
    } finally {
      await removeRoot(root);
    }
  }
});

test('fail-fast records FAIL and leaves every unstarted gate as NOT_RUN', async () => {
  const root = await makeRoot();
  try {
    const result = await runAggregateVerification({ root, gates: fixtureGates({ failingIndex: 1 }) });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    assert.deepEqual(result.summary.gates.map((gate) => gate.status), [
      'PASS', 'FAIL', ...Array(8).fill('NOT_RUN'),
    ]);
    const failed = result.summary.gates[1];
    assert.equal(failed.exitCode, 17);
    assert.equal(failed.signal, null);
    assert.equal(failed.cleanup.processGroupChecked, true);
    assert.equal(failed.cleanup.groupAbsent, true);
    assert.equal(result.summary.sourceIdentity.status, 'PENDING');
    assert.equal(result.summary.sourceIdentity.verifiedAt, null);
    for (const gate of result.summary.gates.slice(2)) {
      assert.equal(gate.startedAt, null);
      assert.equal(gate.endedAt, null);
      assert.equal(gate.logCreated, false);
      assert.equal(gate.cleanup.survivorStatus, 'NOT_APPLICABLE');
      assert.equal(gate.timeoutMs, 5_000);
    }
    assert.equal(result.summary.lock.released, true);
  } finally {
    await removeRoot(root);
  }
});

test('timeout terminates a grandchild process group and proves absence immediately and after five seconds', async () => {
  const root = await makeRoot();
  const pidFile = join(root, '.tmp', 'grandchild.pid');
  const source = [
    "const fs = require('node:fs');",
    "const { spawn } = require('node:child_process');",
    "const child = spawn(process.execPath, ['-e', \"process.on('SIGTERM', () => {}); setInterval(() => {}, 1000)\"], { stdio: 'ignore' });",
    'fs.writeFileSync(process.env.SMC_GRANDCHILD_PID_FILE, String(child.pid));',
    "process.on('SIGTERM', () => {});",
    'setInterval(() => {}, 1000);',
  ].join('');
  try {
    const result = await runAggregateVerification({
      root,
      env: { ...process.env, SMC_GRANDCHILD_PID_FILE: pidFile },
      gates: fixtureGates({ source, timeoutMs: 600 }).slice(0, 1),
    });
    assert.equal(result.status, 'TIMEOUT');
    assert.equal(result.exitCode, 124);
    const gate = result.summary.gates[0];
    assert.equal(gate.status, 'TIMEOUT');
    assert.equal(gate.timedOut, true);
    assert.equal(gate.cleanup.processGroupChecked, true);
    assert.equal(gate.cleanup.term.sent, true);
    assert.ok(gate.cleanup.kill?.sent === true || gate.cleanup.afterTerm?.absent === true);
    assert.equal(gate.cleanup.groupAbsent, true);
    assert.equal(gate.cleanup.survivorStatus, 'ABSENT');
    await waitForPath(pidFile);
    const grandchildPid = await readFile(pidFile, 'utf8');
    assertProcessAbsent(grandchildPid);
    await wait(5_000);
    assertProcessAbsent(grandchildPid);
    await assert.rejects(() => lstat(join(root, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(root);
  }
});

test('abort signal classifies the running gate as CANCELLED and cleans its owned group', async () => {
  const root = await makeRoot();
  const controller = new AbortController();
  const source = 'setInterval(() => {}, 1000);';
  const gates = fixtureGates({ source, timeoutMs: 5_000 }).slice(0, 2);
  try {
    const verification = runAggregateVerification({ root, signal: controller.signal, gates });
    setTimeout(() => controller.abort(new Error('test cancellation')), 250).unref();
    const result = await verification;
    assert.equal(result.status, 'CANCELLED');
    assert.equal(result.exitCode, 130);
    assert.equal(result.summary.gates[0].status, 'CANCELLED');
    assert.equal(result.summary.gates[0].cancellationRequested, true);
    assert.equal(result.summary.gates[0].cleanup.groupAbsent, true);
    assert.equal(result.summary.gates[1].status, 'NOT_RUN');
    assert.equal(result.summary.lock.released, true);
  } finally {
    await removeRoot(root);
  }
});

test('concurrent aggregate verification is rejected without stealing the lock', async () => {
  const root = await makeRoot();
  const controller = new AbortController();
  const gates = fixtureGates({ source: 'setInterval(() => {}, 1000);', timeoutMs: 5_000 }).slice(0, 1);
  const lockPath = join(root, '.tmp', 'verify.lock');
  try {
    const firstVerification = runAggregateVerification({ root, signal: controller.signal, gates });
    await waitForPath(lockPath);
    await assert.rejects(
      () => runAggregateVerification({ root, gates }),
      (error) => error?.code === 'E_VERIFY_LOCKED',
    );
    assert.ok((await readFile(join(lockPath, 'owner'), 'utf8')).trim());
    controller.abort(new Error('release test lock'));
    const result = await firstVerification;
    assert.equal(result.status, 'CANCELLED');
    await assert.rejects(() => lstat(lockPath), (error) => error?.code === 'ENOENT');
  } finally {
    controller.abort(new Error('cleanup'));
    await removeRoot(root);
  }
});

test('symlinked verify workspace is refused before lock acquisition', async () => {
  const root = await makeRoot();
  const target = await mkdtemp(join(tmpdir(), 'smc-aggregate-symlink-target-'));
  try {
    await symlink(target, join(root, '.tmp', 'verify'));
    await assert.rejects(
      () => runAggregateVerification({ root, gates: fixtureGates().slice(0, 1) }),
      (error) => error?.code === 'E_VERIFY_SYMLINK',
    );
    await assert.rejects(() => lstat(join(root, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(root);
    await removeRoot(target);
  }
});

test('bounded timeout and grace overrides are validated before execution', async () => {
  const root = await makeRoot();
  try {
    const result = await runAggregateVerification({
      root,
      env: {
        ...process.env,
        SMC_VERIFY_GATE_TIMEOUT_MS: '100',
        SMC_VERIFY_TERM_GRACE_MS: '50',
        SMC_VERIFY_KILL_GRACE_MS: '50',
        SMC_VERIFY_MAX_LOG_BYTES: '2048',
      },
      gates: fixtureGates({ timeoutMs: 5_000 }).slice(0, 1),
    });
    assert.equal(result.status, 'PASS');
    assert.equal(result.summary.gates[0].timeoutMs, 100);
    assert.equal(result.summary.gates[0].termGraceMs, 50);
    assert.equal(result.summary.gates[0].killGraceMs, 50);
    assert.equal(result.summary.gates[0].maxLogBytes, 2048);
  } finally {
    await removeRoot(root);
  }

  const invalidRoot = await makeRoot();
  try {
    await assert.rejects(
      () => runAggregateVerification({
        root: invalidRoot,
        env: { ...process.env, SMC_VERIFY_GATE_TIMEOUT_MS: '99' },
        gates: fixtureGates().slice(0, 1),
      }),
      (error) => error?.code === 'E_VERIFY_CONFIG',
    );
    await assert.rejects(() => lstat(join(invalidRoot, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(invalidRoot);
  }

  const invalidMaxRoot = await makeRoot();
  try {
    await assert.rejects(
      () => runAggregateVerification({
        root: invalidMaxRoot,
        env: { ...process.env, SMC_VERIFY_MAX_LOG_BYTES: '1023' },
        gates: fixtureGates().slice(0, 1),
      }),
      (error) => error?.code === 'E_VERIFY_CONFIG',
    );
    await assert.rejects(() => lstat(join(invalidMaxRoot, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(invalidMaxRoot);
  }
});

test('Track B aggregate gates use explicit bounded defaults and global overrides', () => {
  const testsGate = createDefaultGates('/tmp/smc-aggregate-tests-budget').find((gate) => gate.name === 'tests');
  assert.ok(testsGate);
  assert.equal(testsGate.timeoutMs, 900_000);

  const trackbGate = createDefaultGates('/tmp/smc-aggregate-trackb-budget').find((gate) => gate.name === 'trackb');
  assert.ok(trackbGate);
  assert.equal(trackbGate.timeoutMs, 120_000);
  assert.equal(resolveGateTimings(trackbGate, { SMC_VERIFY_GATE_TIMEOUT_MS: '500000' }).timeoutMs, 500_000);
  assert.equal(resolveGateTimings(trackbGate, {}).timeoutMs, 120_000);
});

test('aggregate gates run the explicitly selected package manager with unchanged script arguments', () => {
  const outputDir = '/tmp/smc-package-manager-gates';
  const npmGates = createDefaultGates(outputDir, {});
  const pnpmGates = createDefaultGates(outputDir, { SMC_PACKAGE_MANAGER: 'pnpm' });
  assert.deepEqual(pnpmGates.map((gate) => gate.command), npmGates.map((gate) => ['pnpm', ...gate.command.slice(1)]));
  assert.deepEqual(pnpmGates.find((gate) => gate.name === 'tests')?.command, ['pnpm', 'test']);
  assert.deepEqual(pnpmGates.find((gate) => gate.name === 'studio-contract')?.command, [
    'pnpm', 'run', 'smoke:studio:contract', '--',
    '/tmp/smc-package-manager-gates/studio-project',
    '/tmp/smc-package-manager-gates/studio-contract',
  ]);
  assert.throws(
    () => createDefaultGates(outputDir, { SMC_PACKAGE_MANAGER: 'yarn' }),
    { code: 'E_VERIFY_CONFIG' },
  );
});

test('gate log deletion is a FAIL even when the command exits zero', async () => {
  const root = await makeRoot();
  const source = [
    "const fs = require('node:fs');",
    'fs.unlinkSync(process.env.SMC_VERIFY_GATE_LOG_PATH);',
    'process.exit(0);',
  ].join('');
  try {
    const result = await runAggregateVerification({
      root,
      gates: [{
        name: 'delete-log',
        command: command(source),
        timeoutMs: 5_000,
        termGraceMs: 100,
        killGraceMs: 500,
      }],
    });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    const gate = result.summary.gates[0];
    assert.equal(gate.status, 'FAIL');
    assert.equal(gate.exitCode, 0);
    assert.equal(gate.logIdentity.valid, false);
    assert.equal(gate.logIdentity.observed, null);
    assert.match(gate.error, /log-integrity: log path unavailable after close/);
    assert.equal(gate.cleanup.groupAbsent, true);
    assert.equal(result.summary.lock.released, true);
    await assert.rejects(() => lstat(join(root, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(root);
  }
});

test('gate log replacement is detected with lstat identity checks', async () => {
  const root = await makeRoot();
  const replacementPath = join(root, '.tmp', 'replacement.log');
  await writeFile(replacementPath, 'unsafe replacement\n', { mode: 0o600 });
  const source = [
    "const fs = require('node:fs');",
    'const path = process.env.SMC_VERIFY_GATE_LOG_PATH;',
    'fs.unlinkSync(path);',
    'fs.symlinkSync(process.env.SMC_REPLACEMENT_LOG_PATH, path);',
    'process.exit(0);',
  ].join('');
  try {
    const result = await runAggregateVerification({
      root,
      env: { ...process.env, SMC_REPLACEMENT_LOG_PATH: replacementPath },
      gates: [{
        name: 'replace-log',
        command: command(source),
        timeoutMs: 5_000,
        termGraceMs: 100,
        killGraceMs: 500,
      }],
    });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    const gate = result.summary.gates[0];
    assert.equal(gate.status, 'FAIL');
    assert.equal(gate.exitCode, 0);
    assert.equal(gate.logIdentity.valid, false);
    assert.equal(gate.logIdentity.observed.mode & 0o170000, 0o120000);
    assert.match(gate.logIdentity.error, /regular non-symlink/);
    assert.match(gate.error, /log-integrity/);
    assert.equal(gate.cleanup.groupAbsent, true);
  } finally {
    await removeRoot(root);
  }
});

test('combined stdout/stderr log limit fails and kills a child plus grandchild', async () => {
  const root = await makeRoot();
  const pidFile = join(root, '.tmp', 'overflow-grandchild.pid');
  const source = [
    "const fs = require('node:fs');",
    "const { spawn } = require('node:child_process');",
    "const grandchild = spawn(process.execPath, ['-e', \"process.on('SIGTERM', () => {}); setInterval(() => {}, 1000)\"], { stdio: 'ignore' });",
    'fs.writeFileSync(process.env.SMC_GRANDCHILD_PID_FILE, String(grandchild.pid));',
    "process.on('SIGTERM', () => {});",
    "process.stderr.write('y'.repeat(8192));",
    'setInterval(() => {}, 1000);',
  ].join('');
  try {
    const result = await runAggregateVerification({
      root,
      env: { ...process.env, SMC_GRANDCHILD_PID_FILE: pidFile },
      gates: [{
        name: 'log-overflow',
        command: command(source),
        timeoutMs: 5_000,
        termGraceMs: 100,
        killGraceMs: 500,
        maxLogBytes: 1_024,
      }],
    });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    const gate = result.summary.gates[0];
    assert.equal(gate.status, 'FAIL');
    assert.equal(gate.resourceLimitExceeded, true);
    assert.equal(gate.logTruncated, true);
    assert.ok(gate.observedOutputBytes > gate.maxLogBytes);
    assert.ok(gate.logBytes <= gate.maxLogBytes);
    assert.equal(gate.cleanup.processGroupChecked, true);
    assert.equal(gate.cleanup.groupAbsent, true);
    assert.equal(gate.cleanup.survivorStatus, 'ABSENT');
    const logText = await readFile(join(root, '.tmp', 'verify', 'log-overflow.log'), 'utf8');
    assert.ok(logText.includes('[aggregate verifier log truncated after 1024 bytes]'));
    await waitForPath(pidFile);
    const grandchildPid = await readFile(pidFile, 'utf8');
    assertProcessAbsent(grandchildPid);
    await wait(250);
    assertProcessAbsent(grandchildPid);
  } finally {
    await removeRoot(root);
  }
});

test('final summary writer failure returns FAIL without claiming a durable final summary', async () => {
  const root = await makeRoot();
  const calls = [];
  const summaryWriter = async (path, snapshot) => {
    calls.push({ path, status: snapshot.status });
    if (snapshot.status !== 'RUNNING') throw new Error('injected final summary writer failure');
    await writeFile(path, `${JSON.stringify(snapshot)}\n`, { mode: 0o600 });
  };
  try {
    const result = await runAggregateVerification({
      root,
      gates: fixtureGates({ timeoutMs: 5_000 }).slice(0, 1),
      summaryWriter,
    });
    assert.equal(result.status, 'FAIL');
    assert.equal(result.exitCode, 1);
    assert.equal(result.summaryPublished, false);
    assert.equal(result.summaryPublicationAttempted, true);
    assert.equal(result.summaryPath, null);
    assert.equal(result.summaryAttemptedPath, join(root, '.tmp', 'verify', 'summary.json'));
    assert.equal(result.error.code, 'E_VERIFY_SUMMARY_PUBLISH');
    assert.match(result.error.message, /final summary publication failed/);
    assert.equal(result.summary.error.code, 'E_VERIFY_SUMMARY_PUBLISH');
    assert.ok(calls.some(({ status }) => status === 'RUNNING'));
    assert.ok(calls.some(({ status }) => status === 'PASS'));
    const persisted = JSON.parse(await readFile(result.summaryAttemptedPath, 'utf8'));
    assert.equal(persisted.status, 'RUNNING');
    assert.equal(result.summary.lock.released, true);
    await assert.rejects(() => lstat(join(root, '.tmp', 'verify.lock')), (error) => error?.code === 'ENOENT');
  } finally {
    await removeRoot(root);
  }
});
