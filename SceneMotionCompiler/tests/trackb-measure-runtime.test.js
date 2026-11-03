import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { measureTrackBEvidence } from '../packages/application/src/trackb-measure-runtime.js';
import {
  hashTrackBMeasuredEvidenceReceipt,
  hashTrackBRawBytes,
} from '../packages/contracts/src/trackb-contract.js';
import { canonicalHash, canonicalizeJson } from '../packages/core/src/canonical.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';

const AUTHORITY_PATH = 'contracts/examples/trackb/trackb-narrative-authority.example.json';

async function fixture() {
  const authority = JSON.parse(await readFile(AUTHORITY_PATH, 'utf8'));
  const htmlBytes = Buffer.from('<!doctype html><html><body>custody</body></html>');
  const provenance = {
    schemaVersion: 2,
    narrativeId: authority.narrativeId,
    projectionReceiptHash: authority.sourceRefs.projectionReceiptHash,
    sceneGraphHash: authority.sourceRefs.sceneGraphHash,
    sourceSemanticManifestHash: authority.sourceRefs.sourceSemanticManifestHash,
    semanticManifestHash: authority.sourceRefs.semanticManifestHash,
    htmlHash: hashTrackBRawBytes(htmlBytes),
    embedding: {
      anchor: 'trackb-semantic-manifest',
      encoding: 'base64url-rfc4648',
      payloadHash: authority.sourceRefs.semanticManifestHash,
    },
    handoffHash: `sha256:${'6'.repeat(64)}`,
    assetManifestHash: `sha256:${'3'.repeat(64)}`,
  };
  provenance.provenanceHash = canonicalHash(provenance);
  const shotId = authority.shots[0].shotId;
  const receipt = {
    schemaVersion: 2,
    receiptId: 'receipt.measure-runtime.v2',
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
    shotIds: [shotId],
    shots: [{
      shotId,
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
        activeShotId: shotId,
        visibleShotId: shotId,
        stageGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        shotGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        pass: true,
      },
    }],
  };
  receipt.receiptHash = hashTrackBMeasuredEvidenceReceipt(receipt);
  const receiptBytes = Buffer.from(canonicalizeJson(receipt), 'utf8');
  const artifacts = {
    narrativeAuthority: Buffer.from(canonicalizeJson(authority), 'utf8'),
    html: htmlBytes,
    provenance: Buffer.from(canonicalizeJson(provenance), 'utf8'),
  };
  return {
    authority,
    provenance,
    htmlBytes,
    artifacts,
    receipt,
    receiptBytes,
    registry: await loadTrackBContractRegistry(),
    input: {
      narrativeAuthorityPath: 'authority.json',
      htmlPath: 'scene.html',
      provenancePath: 'scene.provenance.json',
      outputPath: 'out/scene.evidence-receipt.json',
    },
  };
}

function createCustody({ artifacts, receiptBytes, closeError = null, readError = null, publishError = null, reused = false }) {
  const calls = [];
  const snapshot = {
    exists: true,
    snapshotToken: Buffer.from('receipt-snapshot').toString('base64url'),
    bytesBase64: receiptBytes.toString('base64url'),
    byteLength: receiptBytes.length,
    sha256: hashTrackBRawBytes(receiptBytes),
    device: 1,
    inode: 2,
  };
  return {
    calls,
    async readMeasurementArtifact({ kind, path }) {
      calls.push(`read:${kind}`);
      if (readError) throw readError;
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
      calls.push('publish');
      if (publishError) throw publishError;
      assert.deepEqual(Buffer.from(bytes), receiptBytes);
      return { path, reused, snapshot };
    },
    async close() {
      calls.push('close');
      if (closeError) throw closeError;
      return { closed: true };
    },
  };
}

function measuredResult(fixtureValue) {
  return {
    receipt: fixtureValue.receipt,
    receiptBytes: fixtureValue.receiptBytes,
    receiptHash: fixtureValue.receipt.receiptHash,
    htmlHash: fixtureValue.receipt.htmlHash,
    rendererLockHash: fixtureValue.receipt.rendererLockHash,
    shotIds: [...fixtureValue.receipt.shotIds],
    shotCount: fixtureValue.receipt.shots.length,
  };
}

function dependenciesFor(fixtureValue, custody, { renderer = null } = {}) {
  const rendererCalls = [];
  return {
    rendererCalls,
    dependencies: {
      registry: fixtureValue.registry,
      openMeasurementSession: async ({ projectRoot }) => {
        assert.equal(projectRoot, '/absolute/project');
        return custody;
      },
      measureTrackBHtml: async (request) => {
        rendererCalls.push(request);
        return renderer ? renderer(request) : measuredResult(fixtureValue);
      },
    },
  };
}

test('measurement runtime uses custody bytes, preserves read order, publishes, and closes', async () => {
  const fixtureValue = await fixture();
  const custody = createCustody(fixtureValue);
  const { dependencies, rendererCalls } = dependenciesFor(fixtureValue, custody);
  const result = await measureTrackBEvidence({ projectRoot: '/absolute/project', input: fixtureValue.input }, dependencies);

  assert.equal(result.receiptPath, fixtureValue.input.outputPath);
  assert.equal(result.receiptHash, fixtureValue.receipt.receiptHash);
  assert.equal(result.shotCount, 1);
  assert.deepEqual(custody.calls, ['read:narrativeAuthority', 'read:html', 'read:provenance', 'publish', 'close']);
  assert.equal(rendererCalls.length, 1);
  assert.deepEqual(rendererCalls[0].htmlBytes, fixtureValue.htmlBytes);
  assert.equal(rendererCalls[0].narrativeId, fixtureValue.authority.narrativeId);
  assert.deepEqual(rendererCalls[0].shotIds, fixtureValue.authority.shots.map((shot) => shot.shotId));
});

test('measurement runtime preserves a primary custody/renderer error when close also fails', async () => {
  const fixtureValue = await fixture();
  const primary = new Error('renderer primary failure');
  const custody = createCustody({
    ...fixtureValue,
    readError: primary,
    closeError: new Error('custody close failure'),
  });
  const { dependencies } = dependenciesFor(fixtureValue, custody);

  await assert.rejects(
    measureTrackBEvidence({ projectRoot: '/absolute/project', input: fixtureValue.input }, dependencies),
    (error) => error.code === 'trackb-storage-integrity'
      && error.details?.custodyClose?.code === 'trackb-storage-integrity'
      && /custody close failure/.test(error.details.custodyClose.message),
  );
  assert.deepEqual(custody.calls, ['read:narrativeAuthority', 'close']);
});

test('measurement runtime reports close failure after an otherwise successful proof', async () => {
  const fixtureValue = await fixture();
  const custody = createCustody({ ...fixtureValue, closeError: new Error('close after success') });
  const { dependencies } = dependenciesFor(fixtureValue, custody);

  await assert.rejects(
    measureTrackBEvidence({ projectRoot: '/absolute/project', input: fixtureValue.input }, dependencies),
    (error) => error.code === 'trackb-storage-integrity'
      && /close after success/.test(error.details?.closeMessage ?? ''),
  );
  assert.equal(custody.calls.at(-1), 'close');
});
