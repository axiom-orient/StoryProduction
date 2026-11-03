import { createHash } from 'node:crypto';
import { execFile } from 'node:child_process';
import { EventEmitter } from 'node:events';
import { chmod, lstat, mkdir, mkdtemp, readFile, readdir, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { existsSync, renameSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { PassThrough } from 'node:stream';
import { promisify } from 'node:util';
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES,
  openMeasurementSession,
  openTrackBCustodyProjectSession,
  verifyMeasurementArtifactResult,
  verifyMeasurementPublishResult,
} from '../packages/store/src/index.js';
import {
  TRACKB_CUSTODY_BINARY_NAME,
  TRACKB_MEASUREMENT_ARTIFACT_KINDS,
  TrackBCustodyError,
} from '../packages/store/src/trackb-custody-protocol.js';
import {
  defaultTrackBCustodyTargetTriple,
  resolveTrackBCustodyBinary,
  verifyTrackBCustodyReceipt,
} from '../packages/store/src/trackb-custody-binary.js';
import { TrackBCustodySession } from '../packages/store/src/trackb-custody-session.js';
import { hashTrackBDeliveryPointer, hashTrackBDeliverySet } from '../packages/contracts/src/trackb-contract.js';

const execFileAsync = promisify(execFile);

async function tempRoot(prefix) {
  return mkdtemp(join(await realpath(tmpdir()), prefix));
}

function receiptFor(bytes, targetTriple, extra = {}) {
  return {
    schemaVersion: 1,
    targetTriple,
    binaryName: TRACKB_CUSTODY_BINARY_NAME,
    binarySha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
    binaryBytes: bytes.length,
    toolVersion: 'test',
    ...extra,
  };
}

async function writeBundle(root, { targetTriple = 'x86_64-test-target', bytes = Buffer.from('#!/bin/sh\nexit 0\n'), receiptExtra = {} } = {}) {
  const binary = join(root, TRACKB_CUSTODY_BINARY_NAME);
  const receipt = join(root, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`);
  await writeFile(binary, bytes, { mode: 0o755 });
  await writeFile(receipt, `${JSON.stringify(receiptFor(bytes, targetTriple, receiptExtra))}\n`);
  return { binary, receipt, binaryPath: binary, receiptPath: receipt, targetTriple };
}

function fakeHelper(responder) {
  const child = new EventEmitter();
  child.stdout = new PassThrough();
  child.stderr = new PassThrough();
  child.exitCode = null;
  child.signalCode = null;
  let exited = false;
  child.stdin = {
    write(line, callback) {
      callback?.();
      const request = JSON.parse(line);
      queueMicrotask(() => {
        if (exited) return;
        child.stdout.write(`${JSON.stringify({ id: request.id, ok: true, result: responder(request) })}\n`);
        if (request.op === 'close') queueMicrotask(() => child.kill());
      });
      return true;
    },
    destroy() {},
  };
  child.kill = () => {
    if (exited) return;
    exited = true;
    child.exitCode = 0;
    child.emit('exit', 0, null);
    child.stdout.end();
    child.stderr.end();
  };
  return child;
}

function deliveryFixture(narrativeId) {
  const deliverySet = {
    schemaVersion: 2,
    deliverySetId: 'delivery.integration.v1',
    narrativeId,
    candidateHash: `sha256:${'a'.repeat(64)}`,
    authorityHash: `sha256:${'b'.repeat(64)}`,
    meaningQualificationHash: `sha256:${'2'.repeat(64)}`,
    baseDeliverySetHash: 'NONE',
    approvalHash: `sha256:${'c'.repeat(64)}`,
    artifacts: {
      projectionReceiptHash: `sha256:${'d'.repeat(64)}`,
      sceneGraphHash: `sha256:${'e'.repeat(64)}`,
      assetHandoffHash: `sha256:${'f'.repeat(64)}`,
      assetHandoffSceneGraphHash: `sha256:${'e'.repeat(64)}`,
      htmlHash: `sha256:${'a'.repeat(64)}`,
      provenanceHash: `sha256:${'b'.repeat(64)}`,
    },
    evidenceCellHashes: [`sha256:${'1'.repeat(64)}`],
    revision: 1,
    state: 'PROMOTED',
    deliverySetHash: '',
  };
  deliverySet.deliverySetHash = hashTrackBDeliverySet(deliverySet);
  const deliverySetSegment = deliverySet.deliverySetHash.slice('sha256:'.length);
  const deliverySetRelative = `delivery-sets/${deliverySetSegment}/trackb-delivery-set-v2.json`;
  const deliverySetPath = `scenes/trackb/${narrativeId}/${deliverySetRelative}`;
  const pointer = {
    schemaVersion: 2,
    pointerKey: `pointer.${narrativeId}`,
    narrativeId,
    revision: 1,
    state: 'PROMOTED',
    deliverySetHash: deliverySet.deliverySetHash,
    candidateHash: deliverySet.candidateHash,
    approvalHash: deliverySet.approvalHash,
    meaningQualificationHash: deliverySet.meaningQualificationHash,
    baseDeliverySetHash: deliverySet.baseDeliverySetHash,
    deliverySetPath,
    pointerHash: '',
  };
  pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
  return { deliverySet, pointer, deliverySetRelative, deliverySetPath };
}

test('custody resolver refuses missing bundle without PATH or source-tree fallback', async () => {
  const root = await tempRoot('smc-trackb-custody-resolver-');
  await assert.rejects(
    resolveTrackBCustodyBinary({ runtimeRoot: root, targetTriple: 'x86_64-test-target' }),
    (error) => error instanceof TrackBCustodyError && error.code === 'trackb-storage-integrity' && error.retryable === false,
  );
});

test('custody receipt has an exact shape and rejects hash, target, mode, final and ancestor symlink failures', async () => {
  const root = await tempRoot('smc-trackb-custody-receipt-');
  const bundle = await writeBundle(root);
  const valid = await resolveTrackBCustodyBinary(bundle);
  assert.equal(valid.binaryBytes, (await lstat(bundle.binary)).size);

  const unknownReceipt = receiptFor(await readFile(bundle.binary), bundle.targetTriple, { builtFromSourceHash: `sha256:${'0'.repeat(64)}` });
  await writeFile(bundle.receipt, JSON.stringify(unknownReceipt));
  await assert.rejects(resolveTrackBCustodyBinary(bundle), (error) => error.code === 'trackb-storage-integrity');

  const missingTool = receiptFor(await readFile(bundle.binary), bundle.targetTriple);
  delete missingTool.toolVersion;
  await writeFile(bundle.receipt, JSON.stringify(missingTool));
  await assert.rejects(resolveTrackBCustodyBinary(bundle), (error) => error.code === 'trackb-storage-integrity');

  await writeFile(bundle.receipt, JSON.stringify(receiptFor(await readFile(bundle.binary), bundle.targetTriple)));
  const binaryLink = join(root, 'custody-link');
  await symlink(bundle.binary, binaryLink);
  await assert.rejects(resolveTrackBCustodyBinary({ binaryPath: binaryLink, receiptPath: bundle.receipt, targetTriple: bundle.targetTriple }), (error) => error.code === 'trackb-storage-integrity');

  const ancestorLink = join(root, 'ancestor-link');
  await symlink(root, ancestorLink);
  await assert.rejects(resolveTrackBCustodyBinary({ binaryPath: join(ancestorLink, TRACKB_CUSTODY_BINARY_NAME), receiptPath: join(ancestorLink, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`), targetTriple: bundle.targetTriple }), (error) => error.code === 'trackb-storage-integrity');

  await chmod(bundle.binary, 0o644);
  await assert.rejects(resolveTrackBCustodyBinary(bundle), (error) => error.code === 'trackb-storage-integrity');
  await chmod(bundle.binary, 0o755);
  const actual = await readFile(bundle.binary);
  await writeFile(bundle.receipt, JSON.stringify(receiptFor(actual, 'aarch64-test-target')));
  await assert.rejects(resolveTrackBCustodyBinary(bundle), (error) => error.code === 'trackb-storage-integrity');
});

test('bundled target triple mapping is explicit and bounded', () => {
  assert.equal(defaultTrackBCustodyTargetTriple({ platform: 'darwin', arch: 'arm64' }), 'aarch64-apple-darwin');
  assert.equal(defaultTrackBCustodyTargetTriple({ platform: 'linux', arch: 'x64' }), 'x86_64-unknown-linux-gnu');
  assert.throws(() => defaultTrackBCustodyTargetTriple({ platform: 'win32', arch: 'x64' }), /unsupported custody target/);
});

test('session purpose is fixed and project ingress requires the seven-artifact order', async () => {
  const bytes = Buffer.from('project-artifact');
  const hash = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
  const projectChild = fakeHelper((request) => {
    if (request.op === 'readProjectArtifact') {
      const result = {
        kind: request.kind,
        path: request.path,
        exists: true,
        bytesBase64: bytes.toString('base64url'),
        byteLength: bytes.length,
        sha256: hash,
      };
      return result;
    }
    if (request.op === 'initialize') {
      return {
        protocolVersion: 1,
        narrativeId: request.narrativeId,
        mode: request.mode,
        namespacePresent: false,
      };
    }
    return { closed: true };
  });
  const project = new TrackBCustodySession({
    child: projectChild,
    projectRoot: '/private/tmp/project',
    resolved: {},
    purpose: 'project-ingress',
  });
  await assert.rejects(
    project.readProjectArtifact({ kind: 'projectionReceipt', path: 'artifact.bin' }),
    (error) => error.code === 'trackb-input-invalid',
  );
  for (const kind of ['narrativeAuthority', 'projectionReceipt', 'assetHandoff', 'html', 'provenance', 'evidenceReceipt']) {
    await project.readProjectArtifact({ kind, path: 'artifact.bin' });
  }
  await project.readProjectArtifact({ kind: 'sceneGraph', path: 'artifact.bin', projectionReceiptHash: hash });
  await project.initializeNarrative({ narrativeId: 'narrative.project-ingress', operationId: 'op.project-ingress', readOnly: true });
  await project.close();

  const narrative = new TrackBCustodySession({
    child: fakeHelper((request) => (request.op === 'close' ? { closed: true } : {})),
    projectRoot: '/private/tmp/project',
    resolved: {},
    purpose: 'narrative',
  });
  await assert.rejects(
    narrative.readProjectArtifact({ kind: 'narrativeAuthority', path: 'artifact.bin' }),
    (error) => error.code === 'trackb-input-invalid',
  );
  await assert.rejects(
    narrative.writeImmutable(`candidates/${'a'.repeat(64)}/trackb-release-candidate-v3.json`, bytes),
    (error) => error.code === 'trackb-input-invalid',
  );
  await narrative.close();
});

test('malformed ok:true helper responses fail storage integrity at every response boundary', async () => {
  const bytes = Buffer.from('payload');
  const validSnapshot = (path) => ({
    path,
    exists: true,
    snapshotToken: Buffer.from('opaque-snapshot').toString('base64url'),
    bytesBase64: bytes.toString('base64url'),
    byteLength: bytes.length,
    sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
    device: 1,
    inode: 2,
  });
  const validResult = (request) => {
    if (request.op === 'initialize') return {
      protocolVersion: 1,
      narrativeId: request.narrativeId,
      mode: request.mode,
      namespacePresent: true,
    };
    if (request.op === 'acquire') return {
      protocolVersion: 1,
      narrativeId: request.narrativeId,
      operationId: request.operationId,
      lockHeld: true,
    };
    if (request.op === 'snapshot' || request.op === 'read') return validSnapshot(request.path);
    if (request.op === 'writeImmutable') return {
      path: request.path,
      reused: false,
      snapshot: { ...validSnapshot(undefined), path: undefined },
    };
    if (request.op === 'publishPointer') return {
      path: request.path,
      reused: false,
      snapshot: {
        ...validSnapshot(undefined),
        bytesBase64: request.pointerBytesBase64,
        byteLength: Buffer.from(request.pointerBytesBase64, 'base64url').length,
        sha256: `sha256:${createHash('sha256').update(Buffer.from(request.pointerBytesBase64, 'base64url')).digest('hex')}`,
      },
    };
    return { closed: true };
  };
  const malformedResult = (request) => {
    if (request.op === 'initialize') return { ...validResult(request), unexpected: true };
    if (request.op === 'acquire') return { ...validResult(request), lockHeld: false };
    if (request.op === 'snapshot' || request.op === 'read') return { path: request.path, exists: false, snapshotToken: 'not-none' };
    if (request.op === 'writeImmutable') return { path: request.path, reused: 'false', snapshot: validSnapshot(undefined) };
    if (request.op === 'publishPointer') return { path: 'wrong-pointer.json', reused: false, snapshot: validSnapshot(undefined) };
    if (request.op === 'close') return { closed: 'yes' };
    return validResult(request);
  };
  const newSession = (operation) => new TrackBCustodySession({
    child: fakeHelper((request) => request.op === operation ? malformedResult(request) : validResult(request)),
    projectRoot: '/private/tmp/project',
    resolved: {},
    purpose: 'narrative',
  });

  const initialize = newSession('initialize');
  await assert.rejects(
    initialize.initializeNarrative({ narrativeId: 'narrative.malformed-init', operationId: 'op.malformed-init' }),
    (error) => error.code === 'trackb-storage-integrity',
  );
  await initialize.close();

  const acquire = newSession('acquire');
  await assert.rejects(
    acquire.initializeNarrative({ narrativeId: 'narrative.malformed-acquire', operationId: 'op.malformed-acquire' }),
    (error) => error.code === 'trackb-storage-integrity',
  );
  await acquire.close();

  const snapshot = newSession('snapshot');
  await snapshot.initializeNarrative({ narrativeId: 'narrative.malformed-snapshot', operationId: 'op.malformed-snapshot' });
  await assert.rejects(snapshot.snapshot('delivery-pointer-v2.json'), (error) => error.code === 'trackb-storage-integrity');
  await snapshot.close();

  const read = newSession('read');
  await read.initializeNarrative({ narrativeId: 'narrative.malformed-read', operationId: 'op.malformed-read' });
  await assert.rejects(read.read('delivery-pointer-v2.json'), (error) => error.code === 'trackb-storage-integrity');
  await read.close();

  const write = newSession('writeImmutable');
  await write.initializeNarrative({ narrativeId: 'narrative.malformed-write', operationId: 'op.malformed-write' });
  await assert.rejects(
    write.writeImmutable(`candidates/${'a'.repeat(64)}/trackb-release-candidate-v4.json`, bytes),
    (error) => error.code === 'trackb-storage-integrity',
  );
  await write.close();

  const publish = newSession('publishPointer');
  await publish.initializeNarrative({ narrativeId: 'narrative.malformed-publish', operationId: 'op.malformed-publish' });
  await assert.rejects(
    publish.publishPointer({
      expectedBaseDeliverySetHash: 'NONE',
      expectedPointerSnapshotToken: 'NONE',
      pointerBytes: bytes,
      deliverySetPath: `delivery-sets/${'a'.repeat(64)}/trackb-delivery-set-v2.json`,
      deliverySetBytes: bytes,
      deliverySetSnapshotToken: 'opaque',
    }),
    (error) => error.code === 'trackb-storage-integrity',
  );
  await publish.close();

  const close = newSession('close');
  await assert.rejects(close.close(), (error) => error.code === 'trackb-storage-integrity');
});

test('project root intermediate ancestor symlink fails closed without touching its target', async () => {
  const workspace = await tempRoot('smc-trackb-custody-project-root-link-');
  const target = await tempRoot('smc-trackb-custody-project-root-target-');
  const targetProject = join(target, 'project');
  const linkedParent = join(workspace, 'linked-parent');
  await mkdir(targetProject);
  await symlink(target, linkedParent);
  try {
    await assert.rejects(
      TrackBCustodySession.open({ projectRoot: join(linkedParent, 'project') }),
      (error) => error instanceof TrackBCustodyError
        && error.code === 'trackb-storage-integrity'
        && /project root ancestor/.test(error.message),
    );
    assert.deepEqual(await readdir(targetProject), []);
    assert.equal(existsSync(join(targetProject, 'scenes')), false);
  } finally {
    await rm(workspace, { recursive: true, force: true });
    await rm(target, { recursive: true, force: true });
  }
});

test('project session reads bounded artifacts before narrative initialization and verifies raw bindings', async () => {
  const root = await tempRoot('smc-trackb-custody-project-session-');
  const files = new Map([
    ['authority/narrative-authority.json', Buffer.from('authority')],
    ['projection/narrative-scene-projection.v3.json', Buffer.from('projection')],
    ['handoff/scene-motion-asset-handoff.v2.json', Buffer.from('asset-handoff')],
    ['out/scene.html', Buffer.from('html')],
    ['out/scene.provenance.json', Buffer.from('provenance')],
    ['out/scene.evidence-receipt.json', Buffer.from('evidence-receipt')],
    ['projection/generations/generation.001/scene-graph.json', Buffer.from('scene-graph')],
  ]);
  let session = null;
  try {
    if (process.platform === 'darwin') {
      await assert.rejects(
        openTrackBCustodyProjectSession({ projectRoot: root }),
        (error) => error.code === 'trackb-storage-integrity'
          && /protected signed install unavailable/.test(error.message),
      );
      return;
    }
    for (const [path, bytes] of files) {
      const absolute = join(root, path);
      await mkdir(join(absolute, '..'), { recursive: true });
      await writeFile(absolute, bytes);
    }
    session = await openTrackBCustodyProjectSession({ projectRoot: root });
    await assert.rejects(
      session.readProjectArtifact({
        kind: 'sceneGraph',
        path: 'projection/generations/generation.001/scene-graph.json',
        projectionReceiptHash: `sha256:${'a'.repeat(64)}`,
      }),
      (error) => error.code === 'trackb-input-invalid',
    );
    const authority = await session.readProjectArtifact({
      kind: 'narrativeAuthority',
      path: 'authority/narrative-authority.json',
    });
    assert.equal(authority.bytesBase64, files.get('authority/narrative-authority.json').toString('base64url'));
    const projection = await session.readProjectArtifact({
      kind: 'projectionReceipt',
      path: 'projection/narrative-scene-projection.v3.json',
    });
    const projectionHash = `sha256:${createHash('sha256').update(files.get('projection/narrative-scene-projection.v3.json')).digest('hex')}`;
    assert.deepEqual(projection, {
      kind: 'projectionReceipt',
      path: 'projection/narrative-scene-projection.v3.json',
      exists: true,
      bytesBase64: files.get('projection/narrative-scene-projection.v3.json').toString('base64url'),
      byteLength: files.get('projection/narrative-scene-projection.v3.json').length,
      sha256: projectionHash,
    });
    await assert.rejects(
      session.readProjectArtifact({
        kind: 'sceneGraph',
        path: 'projection/generations/generation.001/scene-graph.json',
        projectionReceiptHash: `sha256:${'b'.repeat(64)}`,
      }),
      (error) => error.code === 'trackb-input-invalid',
    );
    for (const [kind, path] of [
      ['assetHandoff', 'handoff/scene-motion-asset-handoff.v2.json'],
      ['html', 'out/scene.html'],
      ['provenance', 'out/scene.provenance.json'],
      ['evidenceReceipt', 'out/scene.evidence-receipt.json'],
    ]) {
      const result = await session.readProjectArtifact({ kind, path });
      assert.equal(result.bytesBase64, files.get(path).toString('base64url'));
      await assert.rejects(session.readProjectArtifact({ kind, path }), (error) => error.code === 'trackb-input-invalid');
    }
    const sceneGraph = await session.readProjectArtifact({
      kind: 'sceneGraph',
      path: 'projection/generations/generation.001/scene-graph.json',
      projectionReceiptHash: projectionHash,
    });
    assert.equal(sceneGraph.sha256, `sha256:${createHash('sha256').update(files.get('projection/generations/generation.001/scene-graph.json')).digest('hex')}`);
    await assert.rejects(
      session.readProjectArtifact({ kind: 'unknown', path: 'out/scene.html' }),
      (error) => error.code === 'trackb-input-invalid',
    );
    await session.initializeNarrative({ narrativeId: 'narrative.project-session', readOnly: true });
    await assert.rejects(
      session.readProjectArtifact({ kind: 'narrativeAuthority', path: 'authority/narrative-authority.json' }),
      (error) => error.code === 'trackb-input-invalid',
    );
  } finally {
    if (session) await session.close().catch(() => {});
    await rm(root, { recursive: true, force: true });
  }
});

test('measurement session reads custody bytes in fixed order and publishes bounded receipt bytes', async () => {
  const artifacts = {
    narrativeAuthority: Buffer.from('{"authority":true}'),
    html: Buffer.from('<!doctype html><html></html>'),
    provenance: Buffer.from('{"schemaVersion":2}'),
  };
  const calls = [];
  const snapshot = (bytes) => ({
    exists: true,
    snapshotToken: Buffer.from('measurement-snapshot').toString('base64url'),
    bytesBase64: bytes.toString('base64url'),
    byteLength: bytes.length,
    sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
    device: 1,
    inode: 2,
  });
  const child = fakeHelper((request) => {
    calls.push(request);
    if (request.op === 'readMeasurementArtifact') {
      const bytes = artifacts[request.kind];
      return {
        kind: request.kind,
        path: request.path,
        exists: true,
        bytesBase64: bytes.toString('base64url'),
        byteLength: bytes.length,
        sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
      };
    }
    if (request.op === 'publishMeasurementReceipt') {
      const bytes = Buffer.from(request.bytesBase64, 'base64url');
      return { path: request.path, reused: false, snapshot: snapshot(bytes) };
    }
    return { closed: true };
  });
  const session = new TrackBCustodySession({
    child,
    projectRoot: '/private/tmp/measurement-project',
    resolved: {},
    purpose: 'measurement',
  });

  await assert.rejects(
    session.readMeasurementArtifact({ kind: 'html', path: 'scene.html' }),
    (error) => error.code === 'trackb-input-invalid',
  );
  for (const [kind, path] of [
    ['narrativeAuthority', 'authority.json'],
    ['html', 'scene.html'],
    ['provenance', 'scene.provenance.json'],
  ]) {
    const result = await session.readMeasurementArtifact({ kind, path });
    assert.equal(result.kind, kind);
    assert.equal(result.path, path);
    assert.equal(result.exists, true);
  }
  await assert.rejects(
    session.readMeasurementArtifact({ kind: 'provenance', path: 'scene.provenance.json' }),
    (error) => error.code === 'trackb-input-invalid',
  );
  const receipt = Buffer.from('{"receipt":true}');
  const published = await session.publishMeasurementReceipt({
    path: 'out/scene.evidence-receipt.json',
    bytes: receipt,
  });
  assert.equal(published.reused, false);
  assert.equal(published.snapshot.bytesBase64, receipt.toString('base64url'));
  await session.close();

  assert.deepEqual(
    calls.filter((request) => request.op === 'readMeasurementArtifact').map((request) => request.kind),
    [...TRACKB_MEASUREMENT_ARTIFACT_KINDS],
  );
  assert.deepEqual(
    calls.filter((request) => request.op === 'publishMeasurementReceipt').map((request) => request.path),
    ['out/scene.evidence-receipt.json'],
  );
});

test('measurement custody validators reject malformed and kind-specific boundary responses', () => {
  const bytes = Buffer.from('x');
  const base = (kind, path) => ({
    kind,
    path,
    exists: true,
    bytesBase64: bytes.toString('base64url'),
    byteLength: bytes.length,
    sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
  });
  const bounds = [
    ['narrativeAuthority', 'authority.json', TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES],
    ['html', 'scene.html', TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES],
    ['provenance', 'scene.provenance.json', TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES],
  ];
  for (const [kind, path, maximum] of bounds) {
    const oversized = { ...base(kind, path), byteLength: maximum + 1 };
    assert.throws(
      () => verifyMeasurementArtifactResult(oversized, { kind, path }),
      (error) => error.code === 'trackb-storage-integrity',
    );
    assert.throws(
      () => verifyMeasurementArtifactResult({ ...base(kind, path), extra: true }, { kind, path }),
      (error) => error.code === 'trackb-storage-integrity',
    );
  }
  assert.throws(
    () => verifyMeasurementPublishResult({ path: 'out/receipt.json', reused: false, snapshot: {} }, { path: 'out/receipt.json', bytes: 'not-bytes' }),
    (error) => error.code === 'trackb-storage-integrity',
  );
});

test('measurement opener fails closed on Darwin without a pathname fallback', async () => {
  if (process.platform !== 'darwin') return;
  const root = await tempRoot('smc-trackb-custody-measurement-darwin-');
  try {
    await assert.rejects(
      openMeasurementSession({ projectRoot: root }),
      (error) => error instanceof TrackBCustodyError
        && error.code === 'trackb-storage-integrity'
        && /protected signed install unavailable/.test(error.message),
    );
    assert.deepEqual(await readdir(root), []);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('readOnly session communicates a no-lock mode and leaves an empty project untouched', async () => {
  const root = await tempRoot('smc-trackb-custody-read-only-empty-');
  try {
    if (process.platform === 'darwin') {
      await assert.rejects(
        TrackBCustodySession.open({
          projectRoot: root,
          narrativeId: 'narrative.read-only-empty',
          operationId: 'op.read-only-empty',
          readOnly: true,
        }),
        (error) => error.code === 'trackb-storage-integrity'
          && /protected signed install unavailable/.test(error.message),
      );
      assert.deepEqual(await readdir(root), []);
      return;
    }
    const session = await TrackBCustodySession.open({
      projectRoot: root,
      narrativeId: 'narrative.read-only-empty',
      operationId: 'op.read-only-empty',
      readOnly: true,
    });
    const absent = await session.snapshot('delivery-pointer-v2.json');
    assert.equal(absent.exists, false);
    await session.close();
    assert.equal(existsSync(join(root, 'scenes')), false);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('verified helper spawn failure is typed storage integrity without fallback', async () => {
  const root = await tempRoot('smc-trackb-custody-spawn-');
  const projectRoot = await tempRoot('smc-trackb-custody-spawn-project-');
  const bundle = await writeBundle(root, { bytes: Buffer.from('#!/bin/sh\nexit 23\n') });
  await assert.rejects(
    TrackBCustodySession.open({
      projectRoot,
      binaryPath: bundle.binary,
      receiptPath: bundle.receipt,
      targetTriple: bundle.targetTriple,
      narrativeId: 'narrative.spawn-failure',
      operationId: 'op.spawn-failure',
    }),
    (error) => error.code === 'trackb-storage-integrity' && error.retryable === false,
  );
});

test('verified inode is executed after pathname replacement, or the spawn fails closed', async () => {
  const root = await tempRoot('smc-trackb-custody-replacement-');
  const projectRoot = await tempRoot('smc-trackb-custody-replacement-project-');
  const marker = join(root, 'marker');
  const bundle = await writeBundle(root, {
    bytes: Buffer.from(`#!/bin/sh\nprintf original > ${marker}\nexit 0\n`),
  });
  const replacement = `${bundle.binary}.verified-original`;
  let replaced = false;
  const options = {
    projectRoot,
    binaryPath: bundle.binary,
    receiptPath: bundle.receipt,
    targetTriple: bundle.targetTriple,
    narrativeId: 'narrative.replacement',
    operationId: 'op.replacement',
    get env() {
      if (!replaced) {
        replaced = true;
        renameSync(bundle.binary, replacement);
        writeFileSync(bundle.binary, `#!/bin/sh\nprintf replacement > ${marker}\nexit 0\n`, { mode: 0o755 });
      }
      return process.env;
    },
  };
  try {
    const session = await TrackBCustodySession.open(options);
    await session.close();
    assert.equal(await readFile(marker, 'utf8'), 'original');
  } catch (error) {
    assert.equal(error.code, 'trackb-storage-integrity');
    assert.equal(existsSync(marker), false, 'replacement must never execute');
  }
});

test('runtime packaging requires --out, rejects source output, and verifies an external package', async () => {
  const missing = await tempRoot('smc-trackb-custody-package-missing-');
  const output = await tempRoot('smc-trackb-custody-package-output-');
  await assert.rejects(
    execFileAsync(process.execPath, ['scripts/package-trackb-runtime.mjs', '--runtime-root', join(missing, 'missing-runtime'), '--target-triple', 'x86_64-test-target', '--out', output], { cwd: process.cwd(), maxBuffer: 1024 * 1024 }),
    (error) => error.code === 1 && /unavailable|storage-integrity/.test(error.stderr ?? ''),
  );
  await assert.rejects(
    execFileAsync(process.execPath, ['scripts/package-trackb-runtime.mjs', '--runtime-root', join(missing, 'missing-runtime'), '--target-triple', 'x86_64-test-target'], { cwd: process.cwd(), maxBuffer: 1024 * 1024 }),
    (error) => error.code === 1 && /requires --out/.test(error.stderr ?? ''),
  );
  const bundleRoot = await tempRoot('smc-trackb-custody-package-source-');
  const bundle = await writeBundle(bundleRoot);
  await assert.rejects(
    execFileAsync(process.execPath, ['scripts/package-trackb-runtime.mjs', '--runtime-root', bundleRoot, '--target-triple', bundle.targetTriple, '--out', join(process.cwd(), 'runtime', 'native')], { cwd: process.cwd(), maxBuffer: 1024 * 1024 }),
    (error) => error.code === 1 && /outside the source runtime\/native tree/.test(error.stderr ?? ''),
  );
  const packageRoot = await tempRoot('smc-trackb-custody-package-final-');
  const packageResult = await execFileAsync(process.execPath, ['scripts/package-trackb-runtime.mjs', '--runtime-root', bundleRoot, '--target-triple', bundle.targetTriple, '--out', packageRoot], { cwd: process.cwd(), maxBuffer: 1024 * 1024 });
  assert.match(packageResult.stdout, /"sourceTreeFallback": false/);
  const packagedRoot = join(packageRoot, bundle.targetTriple);
  const packaged = await verifyTrackBCustodyReceipt({ binaryPath: join(packagedRoot, TRACKB_CUSTODY_BINARY_NAME), receiptPath: join(packagedRoot, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`), targetTriple: bundle.targetTriple });
  assert.equal(packaged.binaryBytes, bundle ? (await lstat(bundle.binary)).size : 0);
});

test('runtime packaging rejects a symlinked output ancestor before writing into source runtime', async () => {
  const bundleRoot = await tempRoot('smc-trackb-custody-package-symlink-source-');
  const bundle = await writeBundle(bundleRoot);
  const workspace = await tempRoot('smc-trackb-custody-package-symlink-output-');
  const sourceNativeRoot = join(process.cwd(), 'runtime', 'native');
  const outputAlias = join(workspace, 'runtime-native-alias');
  const blockedName = `blocked-${process.pid}-${Date.now()}`;
  const outputRoot = join(outputAlias, blockedName);
  const sourceTreeTarget = join(sourceNativeRoot, blockedName);
  await symlink(sourceNativeRoot, outputAlias);
  try {
    assert.equal(existsSync(sourceTreeTarget), false);
    await assert.rejects(
      execFileAsync(process.execPath, [
        'scripts/package-trackb-runtime.mjs',
        '--runtime-root', bundleRoot,
        '--target-triple', bundle.targetTriple,
        '--out', outputRoot,
      ], { cwd: process.cwd(), maxBuffer: 1024 * 1024 }),
      (error) => error.code === 1 && /symlink/.test(error.stderr ?? ''),
    );
    assert.equal(existsSync(sourceTreeTarget), false, 'symlinked output ancestor must not create source-tree output');
  } finally {
    await rm(workspace, { recursive: true, force: true });
    await rm(bundleRoot, { recursive: true, force: true });
  }
});

test('Node session uses the committed bundled helper and strict pointer protocol', async () => {
  const targetTriple = defaultTrackBCustodyTargetTriple();
  const bundledBinary = join(process.cwd(), 'runtime', 'native', targetTriple, TRACKB_CUSTODY_BINARY_NAME);
  const bundledReceipt = join(process.cwd(), 'runtime', 'native', targetTriple, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`);
  assert.equal(existsSync(bundledBinary), true, 'run trackb:custody:build before real-helper integration tests');
  assert.equal(existsSync(bundledReceipt), true);
  const resolved = await resolveTrackBCustodyBinary({ targetTriple });
  assert.equal(resolved.binaryPath, bundledBinary);
  const root = await tempRoot('smc-trackb-custody-integration-');
  const narrativeId = 'narrative.integration';
  const fixture = deliveryFixture(narrativeId);
  if (process.platform === 'darwin') {
    await assert.rejects(
      TrackBCustodySession.open({ projectRoot: root, narrativeId, operationId: 'op.integration' }),
      (error) => error.code === 'trackb-storage-integrity'
        && error.retryable === false
        && /protected signed install unavailable/.test(error.message),
    );
    assert.deepEqual(await readdir(root), [], 'Darwin fail-closed must not create project scenes');
    return;
  }
  const session = await TrackBCustodySession.open({ projectRoot: root, narrativeId, operationId: 'op.integration' });
  const absent = await session.snapshot('delivery-pointer-v2.json');
  assert.equal(absent.exists, false);
  const deliveryBytes = Buffer.from(JSON.stringify(fixture.deliverySet));
  const writtenDelivery = await session.writeImmutable(fixture.deliverySetRelative, deliveryBytes);
  assert.equal(writtenDelivery.reused, false);
  const deliverySnapshot = await session.snapshot(fixture.deliverySetRelative);
  const published = await session.publishPointer({
    expectedBaseDeliverySetHash: 'NONE',
    expectedPointerSnapshotToken: absent.snapshotToken,
    pointerBytes: Buffer.from(JSON.stringify(fixture.pointer)),
    deliverySetPath: fixture.deliverySetPath,
    deliverySetBytes: deliveryBytes,
    deliverySetSnapshotToken: deliverySnapshot.snapshotToken,
  });
  assert.equal(published.reused, false);
  await assert.rejects(session.publishPointer({
    expectedBaseDeliverySetHash: 'NONE',
    expectedPointerSnapshotToken: absent.snapshotToken,
    pointerBytes: Buffer.from(JSON.stringify(fixture.pointer)),
    deliverySetPath: fixture.deliverySetPath,
    deliverySetBytes: deliveryBytes,
    deliverySetSnapshotToken: deliverySnapshot.snapshotToken,
  }), (error) => error.code === 'trackb-cas-conflict');
  await session.close();
});
