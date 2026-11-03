import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, symlink, writeFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { measureTrackBHtml } from '../packages/browser-renderer/src/index.js';
import {
  installTrustedNavigationGuard,
  validateNavigationGuardBindingEvent
} from '../packages/browser-renderer/src/render.js';
import { TRACK_B_HANDLER_IMPLEMENTATIONS } from '../packages/application/src/trackb-commands.js';
import {
  hashTrackBAuthority,
  hashTrackBRawBytes,
  hashTrackBMeasuredEvidenceReceipt,
  validateTrackBMeasuredEvidenceReceipt
} from '../packages/contracts/src/trackb-contract.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import { canonicalHash, canonicalizeJson } from '../packages/core/src/canonical.js';

const ZERO_HASH = `sha256:${'0'.repeat(64)}`;

function createSemanticManifest() {
  const manifest = {
    schemaVersion: 1,
    sourceSemanticManifestHash: `sha256:${'1'.repeat(64)}`,
    entries: [
      {
        semanticId: 'semantic.action.001',
        shotId: 'shot.001',
        sourceSceneId: 'scene.001',
        kind: 'action',
        sourceText: '첫 장면',
        sourceLocator: { sceneId: 'scene.001', field: 'action', occurrenceIndex: 0 }
      },
      {
        semanticId: 'semantic.dialogue.001',
        shotId: 'shot.001',
        sourceSceneId: 'scene.001',
        kind: 'dialogue',
        sourceText: '같은 대사\n두 번째 줄',
        sourceLocator: { sceneId: 'scene.001', field: 'dialogue', occurrenceIndex: 0 }
      },
      {
        semanticId: 'semantic.dialogue.002',
        shotId: 'shot.002',
        sourceSceneId: 'scene.002',
        kind: 'dialogue',
        sourceText: '같은 대사\n두 번째 줄',
        sourceLocator: { sceneId: 'scene.002', field: 'dialogue', occurrenceIndex: 0 }
      }
    ]
  };
  manifest.entries = manifest.entries.map((entry) => ({ ...entry, entryHash: canonicalHash(entry) }));
  manifest.semanticManifestHash = canonicalHash(manifest);
  return manifest;
}

function buildHtml(manifest, {
  duplicateAnchor = false,
  missingShot = false,
  duplicateShot = false,
  externalRequest = false,
  runtimeException = false,
  consoleError = false,
  delayed = false,
  forgedNavigationGuard = false,
  shotIds = ['shot.001', 'shot.002']
} = {}) {
  const payloadBytes = Buffer.from(canonicalizeJson(manifest), 'utf8');
  const payload = payloadBytes.toString('base64url');
  const payloadHash = hashTrackBRawBytes(payloadBytes);
  const marker = `<div id="trackb-semantic-manifest" hidden aria-hidden="true" encoding="base64url-rfc4648" semanticManifestHash="${manifest.semanticManifestHash}" payloadHash="${payloadHash}">${payload}</div>`;
  const markerText = duplicateAnchor ? `${marker}${marker}` : marker;
  const shotOne = shotIds.includes('shot.001') ? '<div class="shot active" data-shot-id="shot.001">첫 번째</div>' : '';
  const shotTwo = !missingShot && shotIds.includes('shot.002') ? '<div class="shot" data-shot-id="shot.002">두 번째</div>' : '';
  const shotDuplicate = duplicateShot ? '<div class="shot" data-shot-id="shot.001">중복</div>' : '';
  const network = externalRequest ? '<img src="https://example.invalid/network.png" alt="blocked">' : '';
  const exception = runtimeException ? '<script>throw new Error("measured runtime failure")</script>' : '';
  const consoleFailure = consoleError ? '<script>console.error("measured console failure")</script>' : '';
  const delay = delayed ? '<script>while (true) {}</script>' : '';
  const forgedNavigation = forgedNavigationGuard ? '<script>(()=>{Object.defineProperty(globalThis,"__sfNavigationGuard",{value:{ready:true,error:null,attempts:[]},configurable:false,writable:false,enumerable:false});globalThis.open=Window.prototype.open;window.open("https://example.invalid/open");location.href="https://example.invalid/navigate";})();</script>' : '';
  return `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;background:#111;color:#fff}#stage{position:relative;width:640px;height:360px}.shot{position:absolute;inset:0;display:none;background:#234;padding:20px;font:24px sans-serif}.shot.active{display:block}</style></head><body><div id="stage">${shotOne}${shotTwo}${shotDuplicate}</div>${markerText}${network}${exception}${consoleFailure}${delay}${forgedNavigation}<script>(()=>{const shots=[...document.querySelectorAll('[data-shot-id]')];shots.forEach((shot)=>shot.addEventListener('click',()=>shots.forEach((candidate)=>candidate.classList.toggle('active',candidate===shot))));})();</script></body></html>`;
}

function runnerRequest(html, manifest, overrides = {}) {
  return {
    htmlBytes: Buffer.from(html, 'utf8'),
    narrativeId: 'narrative.measure-test',
    authorityHash: `sha256:${'2'.repeat(64)}`,
    assetManifestHash: `sha256:${'3'.repeat(64)}`,
    semanticManifestHash: manifest.semanticManifestHash,
    shotIds: ['shot.001', 'shot.002'],
    receiptId: 'receipt.measure-test.v2',
    timeoutMs: 30_000,
    ...overrides
  };
}

async function expectRunnerError(promise, code = 'trackb-evidence-incomplete') {
  await assert.rejects(promise, (error) => {
    assert.equal(error?.name, 'TrackBMeasuredEvidenceError');
    assert.equal(error.code, code);
    return true;
  });
}

test('Track B measured runner captures deterministic authority-order shots and canonical receipt', async () => {
  const manifest = createSemanticManifest();
  const html = buildHtml(manifest);
  const request = runnerRequest(html, manifest);
  const first = await measureTrackBHtml(request);
  const second = await measureTrackBHtml(request);
  assert.deepEqual(first.shotIds, ['shot.001', 'shot.002']);
  assert.equal(first.shotCount, 2);
  assert.equal(first.receiptBytes.toString('utf8'), canonicalizeJson(first.receipt));
  assert.equal(first.receiptHash, hashTrackBMeasuredEvidenceReceipt(first.receipt));
  assert.equal(validateTrackBMeasuredEvidenceReceipt(first.receipt, { registry: await loadTrackBContractRegistry() }).valid, true);
  assert.deepEqual(first.receipt.shots.map((shot) => shot.shotId), ['shot.001', 'shot.002']);
  assert.ok(first.receipt.shots.every((shot) => shot.screenshotHash.startsWith('sha256:')));
  assert.ok(first.receipt.shots.every((shot) => shot.runtime.networkRequestCount === 0 && shot.runtime.externalRequestCount === 0 && shot.runtime.navigationAttemptCount === 0 && shot.runtime.exceptionCount === 0));
  assert.ok(first.receipt.shots.every((shot) => shot.visual.stageGeometry.width > 0 && shot.visual.shotGeometry.width > 0));
  assert.equal(first.receipt.htmlHash, hashTrackBRawBytes(request.htmlBytes));
  assert.deepEqual(first.receipt, second.receipt);
  assert.deepEqual(first.receiptBytes, second.receiptBytes);
});

test('Track B measured runner rejects anchor, shot, network, runtime, and timeout failures', async () => {
  const manifest = createSemanticManifest();
  const cases = [
    ['duplicate anchor', buildHtml(manifest, { duplicateAnchor: true }), 'trackb-evidence-incomplete'],
    ['missing shot', buildHtml(manifest, { missingShot: true }), 'trackb-evidence-incomplete'],
    ['duplicate shot', buildHtml(manifest, { duplicateShot: true }), 'trackb-evidence-incomplete'],
    ['network attempt', buildHtml(manifest, { externalRequest: true }), 'trackb-evidence-incomplete'],
    ['runtime exception', buildHtml(manifest, { runtimeException: true }), 'trackb-evidence-incomplete'],
    ['console error', buildHtml(manifest, { consoleError: true }), 'trackb-evidence-incomplete']
  ];
  for (const [, html, code] of cases) {
    await expectRunnerError(measureTrackBHtml(runnerRequest(html, manifest)), code);
  }
  await expectRunnerError(measureTrackBHtml(runnerRequest(buildHtml(manifest, { delayed: true }), manifest, { timeoutMs: 1_000 })), 'trackb-evidence-timeout');
});

test('Track B measured runner rejects page-owned forged guard and navigation with no receipt', async () => {
  const manifest = createSemanticManifest();
  const root = await mkdtemp(join(tmpdir(), 'smc-forged-guard-'));
  const receiptPath = join(root, 'receipt.json');
  try {
    await assert.rejects(
      measureTrackBHtml(runnerRequest(buildHtml(manifest, { forgedNavigationGuard: true }), manifest)),
      (error) => error?.name === 'TrackBMeasuredEvidenceError' && error.code === 'trackb-evidence-incomplete'
    );
    await assert.rejects(readFile(receiptPath), (error) => error?.code === 'ENOENT');
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Track B navigation binding attestation rejects missing and wrong nonce/ready messages', () => {
  const bindingName = '__smcNavigationGuard_test';
  const nonce = 'nonce-test';
  const event = (payload) => ({ name: bindingName, executionContextId: 7, payload: JSON.stringify(payload) });
  assert.throws(
    () => validateNavigationGuardBindingEvent(event({ nonce: 'wrong', type: 'ready', url: 'about:blank' }), { bindingName, nonce }),
    /nonce mismatch/
  );
  assert.throws(
    () => validateNavigationGuardBindingEvent({ name: bindingName, executionContextId: 7 }, { bindingName, nonce }),
    /payload is missing/
  );
  assert.throws(
    () => validateNavigationGuardBindingEvent(event({ nonce, type: 'ready' }), { bindingName, nonce }),
    /ready attestation is invalid/
  );
});

test('Track B trusted guard fails closed when the ready binding attestation is missing', async () => {
  const listeners = new Map();
  const client = {
    on(name, listener) {
      listeners.set(name, listener);
      return () => listeners.delete(name);
    },
    async send(method) {
      if (method === 'Runtime.addBinding') return {};
      if (method === 'Page.addScriptToEvaluateOnNewDocument') return { identifier: 'test-script' };
      throw new Error(`unexpected fake CDP method: ${method}`);
    }
  };
  const guard = await installTrustedNavigationGuard(client, { knownExecutionContextIds: [11], timeoutMs: 10 });
  await assert.rejects(
    guard.waitForReady({ expectedUrl: 'about:blank', timeoutMs: 10 }),
    /ready attestation was not received/
  );
  guard.dispose();
  assert.equal(listeners.size, 0);
});

test('Track B measured receipt self-hash and tamper checks fail closed', async () => {
  const manifest = createSemanticManifest();
  const html = buildHtml(manifest);
  const result = await measureTrackBHtml(runnerRequest(html, manifest));
  const tampered = structuredClone(result.receipt);
  tampered.shots[0].screenshotHash = ZERO_HASH;
  assert.equal(validateTrackBMeasuredEvidenceReceipt(tampered, { registry: await loadTrackBContractRegistry() }).valid, false);
  assert.notEqual(hashTrackBMeasuredEvidenceReceipt(tampered), tampered.receiptHash);
});

test('Track B measurement command delegates unsafe-path handling to custody and preserves custody failure', async () => {
  const root = await mkdtemp(join(tmpdir(), 'smc-trackb-measure-command-'));
  try {
    const custodyCalls = [];
    const trackBRuntimeDependencies = {
      openMeasurementSession: async () => ({
        async readMeasurementArtifact({ kind, path }) {
          custodyCalls.push(`read:${kind}:${path}`);
          throw new Error('descriptor traversal rejected the symlink entry');
        },
        async publishMeasurementReceipt() {
          throw new Error('publish must not be reached after custody read failure');
        },
        async close() {
          custodyCalls.push('close');
          return { closed: true };
        },
      }),
    };
    await assert.rejects(
      TRACK_B_HANDLER_IMPLEMENTATIONS.measureEvidence(
        { projectRoot: root, trackBRuntimeDependencies },
        { narrativeAuthorityPath: '../authority.json', htmlPath: 'scene.html', provenancePath: 'scene.provenance.json', outputPath: 'receipt.json' }
      ),
      (error) => error.code === 'trackb-input-invalid'
    );
    const target = join(root, 'outside.json');
    await writeFile(target, '{}');
    await symlink(target, join(root, 'authority-link.json'));
    await assert.rejects(
      TRACK_B_HANDLER_IMPLEMENTATIONS.measureEvidence(
        { projectRoot: root, trackBRuntimeDependencies },
        { narrativeAuthorityPath: 'authority-link.json', htmlPath: 'scene.html', provenancePath: 'scene.provenance.json', outputPath: 'receipt.json' }
      ),
      (error) => error.code === 'trackb-storage-integrity'
    );
    assert.deepEqual(custodyCalls, ['read:narrativeAuthority:authority-link.json', 'close']);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Track B measurement command publishes, reuses, and conflicts through custody', async () => {
  const root = await mkdtemp(join(tmpdir(), 'smc-trackb-measure-command-positive-'));
  try {
    const sourceManifest = createSemanticManifest();
    const manifest = structuredClone(sourceManifest);
    manifest.entries = manifest.entries.filter((entry) => entry.shotId === 'shot.001');
    delete manifest.semanticManifestHash;
    manifest.semanticManifestHash = canonicalHash(manifest);
    const htmlBytes = Buffer.from(buildHtml(manifest, { shotIds: ['shot.001'] }), 'utf8');
    const authority = JSON.parse(await readFile('contracts/examples/trackb/trackb-narrative-authority.example.json', 'utf8'));
    authority.sourceRefs.semanticManifestHash = manifest.semanticManifestHash;
    authority.authorityHash = hashTrackBAuthority(authority);
    const assetManifestHash = canonicalHash({ schemaVersion: 1, assetManifestId: 'manifest.measure-command', assets: [] });
    const provenance = {
      schemaVersion: 2,
      narrativeId: authority.narrativeId,
      projectionReceiptHash: authority.sourceRefs.projectionReceiptHash,
      sceneGraphHash: authority.sourceRefs.sceneGraphHash,
      sourceSemanticManifestHash: authority.sourceRefs.sourceSemanticManifestHash,
      semanticManifestHash: manifest.semanticManifestHash,
      htmlHash: hashTrackBRawBytes(htmlBytes),
      embedding: {
        anchor: 'trackb-semantic-manifest',
        encoding: 'base64url-rfc4648',
        payloadHash: hashTrackBRawBytes(Buffer.from(canonicalizeJson(manifest), 'utf8'))
      },
      handoffHash: `sha256:${'6'.repeat(64)}`,
      assetManifestHash
    };
    provenance.provenanceHash = canonicalHash(provenance);
    const input = {
      narrativeAuthorityPath: 'authority.json',
      htmlPath: 'scene.html',
      provenancePath: 'scene.provenance.json',
      outputPath: 'out/scene.evidence-receipt.json'
    };
    const artifacts = {
      narrativeAuthority: Buffer.from(canonicalizeJson(authority), 'utf8'),
      html: htmlBytes,
      provenance: Buffer.from(canonicalizeJson(provenance), 'utf8'),
    };
    const custodyCalls = [];
    let publishedBytes = null;
    let currentReceiptId = 'receipt.measure-command.v1';
    const measuredResult = () => {
      const receipt = {
        schemaVersion: 2,
        receiptId: currentReceiptId,
        narrativeId: authority.narrativeId,
        authorityHash: authority.authorityHash,
        htmlHash: hashTrackBRawBytes(htmlBytes),
        assetManifestHash: provenance.assetManifestHash,
        semanticManifestHash: provenance.semanticManifestHash,
        rendererLockHash: `sha256:${'4'.repeat(64)}`,
        navigationGuard: {
          mechanism: 'cdp-binding-nonce',
          guardSourceHash: 'sha256:922cd3832fe5988739a4be38f2d4c15bde7a672fa92b7301ee212548a3bd1099'
        },
        shotIds: ['shot.001'],
        shots: [{
          shotId: 'shot.001',
          ordinal: 0,
          screenshotHash: `sha256:${'5'.repeat(64)}`,
          runtime: {
            networkRequestCount: 0,
            externalRequestCount: 0,
            navigationAttemptCount: 0,
            exceptionCount: 0,
            semanticManifestVerified: true,
            pass: true,
          },
          visual: {
            activeShotCount: 1,
            visibleShotCount: 1,
            activeShotId: 'shot.001',
            visibleShotId: 'shot.001',
            stageGeometry: { x: 0, y: 0, width: 1280, height: 720 },
            shotGeometry: { x: 0, y: 0, width: 1280, height: 720 },
            pass: true,
          },
        }],
      };
      receipt.receiptHash = hashTrackBMeasuredEvidenceReceipt(receipt);
      const receiptBytes = Buffer.from(canonicalizeJson(receipt), 'utf8');
      return {
        receipt,
        receiptBytes,
        receiptHash: receipt.receiptHash,
        htmlHash: receipt.htmlHash,
        rendererLockHash: receipt.rendererLockHash,
        shotIds: [...receipt.shotIds],
        shotCount: receipt.shots.length,
      };
    };
    const trackBRuntimeDependencies = {
      openMeasurementSession: async () => ({
        async readMeasurementArtifact({ kind, path }) {
          custodyCalls.push(`read:${kind}`);
          const bytes = artifacts[kind];
          return {
            kind,
            path,
            exists: true,
            bytesBase64: bytes.toString('base64url'),
            byteLength: bytes.length,
            sha256: hashTrackBRawBytes(bytes),
          };
        },
        async publishMeasurementReceipt(path, bytes) {
          custodyCalls.push('publish');
          const payload = Buffer.from(bytes);
          if (publishedBytes && !publishedBytes.equals(payload)) throw new Error('different measured evidence receipt');
          const reused = Boolean(publishedBytes);
          publishedBytes = publishedBytes ?? payload;
          return {
            path,
            reused,
            snapshot: {
              exists: true,
              snapshotToken: 'b2s',
              bytesBase64: publishedBytes.toString('base64url'),
              byteLength: publishedBytes.length,
              sha256: hashTrackBRawBytes(publishedBytes),
              device: 1,
              inode: 2,
            },
          };
        },
        async close() {
          custodyCalls.push('close');
          return { closed: true };
        },
      }),
      measureTrackBHtml: async () => measuredResult(),
    };
    const first = await TRACK_B_HANDLER_IMPLEMENTATIONS.measureEvidence({ projectRoot: root, trackBRuntimeDependencies }, input);
    assert.equal(first.receiptPath, input.outputPath);
    assert.equal(first.shotCount, 1);
    const reused = await TRACK_B_HANDLER_IMPLEMENTATIONS.measureEvidence({ projectRoot: root, trackBRuntimeDependencies }, input);
    assert.deepEqual(reused, first);
    currentReceiptId = 'receipt.measure-command.changed';
    await assert.rejects(
      TRACK_B_HANDLER_IMPLEMENTATIONS.measureEvidence({ projectRoot: root, trackBRuntimeDependencies }, input),
      (error) => error.code === 'trackb-storage-integrity' && /different measured evidence receipt/.test(error.message)
    );
    assert.deepEqual(custodyCalls, [
      'read:narrativeAuthority', 'read:html', 'read:provenance', 'publish', 'close',
      'read:narrativeAuthority', 'read:html', 'read:provenance', 'publish', 'close',
      'read:narrativeAuthority', 'read:html', 'read:provenance', 'publish', 'close',
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
