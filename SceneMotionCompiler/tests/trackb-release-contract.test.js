import { singleSceneProjectionFixture } from './support/narrative-fixture.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, cp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  TRACK_B_HASH_DOMAINS,
  TRACK_B_RELEASE_COMMANDS,
  TRACK_B_APPROVAL_MAX_FUTURE_MS,
  TRACK_B_STUDIO_MAX_BODY_BYTES,
  assertTrackBStatusResult,
  assertTrackBDeliverySet,
  deriveTrackBMeaningQualification,
  deriveCandidateVerdict,
  hashTrackBAuthority,
  hashTrackBApprovalReceipt,
  hashTrackBDeliveryPointer,
  hashTrackBDeliverySet,
  hashTrackBEvidenceCell,
  hashTrackBMeasuredEvidenceReceipt,
  hashTrackBRawBytes,
  hashTrackBReleaseCandidate,
  isTrackBBaseHash,
  parseTrackBDateTime,
  isTrackBSafeRelativePath,
  validateTrackBApplicationEnvelope,
  validateTrackBApprovalReceipt,
  validateTrackBArtifactBindings,
  validateTrackBCommandResult,
  validateTrackBCommandInput,
  validateTrackBDeliveryPointer,
  validateTrackBDeliverySet,
  validateTrackBNarrativeAuthority,
  validateTrackBMeasuredEvidenceReceipt,
  validateTrackBReleaseCandidate,
  validateTrackBStatusResult,
  
} from '../packages/contracts/src/trackb-contract.js';
import {
  TRACK_B_STUDIO_ROUTE,
  validateTrackBStudioRequest
} from '../packages/application/src/trackb-release-contract.js';
import { CLI_COMMAND_REGISTRY, MCP_COMMAND_REGISTRY } from '../packages/application/src/command-registry.js';
import { loadTrackBArtifactContractRegistries } from '../packages/contracts/src/trackb-artifact-schemas.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import { SCHEMAS } from '../packages/core/src/schema-registry.js';
import { canonicalHash, canonicalizeJson } from '../packages/core/src/canonical.js';
import { composeSceneHtml } from '../packages/composer/src/scene-html.js';

const EXAMPLE_DIR = 'contracts/examples/trackb';
const EXAMPLES = Object.freeze({
  trackBNarrativeAuthority: 'trackb-narrative-authority.example.json',
  trackBReleaseCandidate: 'trackb-release-candidate.example.json',
  trackBMeasuredEvidenceReceipt: 'trackb-measured-evidence-receipt.example.json',
  trackBReleaseApprovalReceipt: 'trackb-release-approval-receipt.example.json',
  trackBDeliverySet: 'trackb-delivery-set.example.json',
  trackBDeliveryPointer: 'trackb-delivery-pointer.example.json'
});
const NEGATIVE_FIXTURES = JSON.parse(await readFile(join(EXAMPLE_DIR, 'negative-fixtures.json'), 'utf8'));
const EXECUTED_NEGATIVE_FIXTURES = new Set();

function markNegativeFixtures(...names) {
  for (const name of names) EXECUTED_NEGATIVE_FIXTURES.add(name);
}

async function loadExamples() {
  const values = {};
  for (const [key, file] of Object.entries(EXAMPLES)) values[key] = JSON.parse(await readFile(join(EXAMPLE_DIR, file), 'utf8'));
  return values;
}

async function registry() {
  return loadTrackBContractRegistry('contracts/schemas/trackb');
}

async function buildArtifactBindingVector(values, artifactRegistries) {
  const { sceneGraph, projection: projectionReceipt } = singleSceneProjectionFixture(values.trackBNarrativeAuthority.narrativeId);
  const sceneGraphHash = canonicalHash(sceneGraph);
  const projectionReceiptBytes = Buffer.from(`${JSON.stringify(projectionReceipt)}\n`, 'utf8');
  const handoff = JSON.parse(await readFile('contracts/examples/scene-motion-asset-handoff-v2.example.json', 'utf8'));
  handoff.sceneGraphHash = sceneGraphHash;
  handoff.projectionReceiptSha256 = hashTrackBRawBytes(projectionReceiptBytes).slice('sha256:'.length);
  handoff.sourceSemanticManifestHash = sceneGraph.semanticManifest.sourceSemanticManifestHash;
  handoff.semanticManifestHash = sceneGraph.semanticManifest.semanticManifestHash;
  const handoffBytes = Buffer.from(`${JSON.stringify(handoff)}\n`, 'utf8');
  const htmlBytes = Buffer.from(composeSceneHtml({ sceneGraph }), 'utf8');
  const htmlHash = hashTrackBRawBytes(htmlBytes);
  const authority = structuredClone(values.trackBNarrativeAuthority);
  authority.sourceRefs.projectionReceiptHash = hashTrackBRawBytes(projectionReceiptBytes);
  authority.sourceRefs.sceneGraphHash = sceneGraphHash;
  authority.sourceRefs.nvcReleaseBundleHash = projectionReceipt.bundleSha256;
  authority.sourceRefs.narrativePackageHash = `sha256:${projectionReceipt.packageSha256}`;
  authority.sourceRefs.narrativeSceneMapHash = `sha256:${projectionReceipt.sceneMapSha256}`;
  authority.sourceRefs.sourceSemanticManifestHash = sceneGraph.semanticManifest.sourceSemanticManifestHash;
  authority.sourceRefs.semanticManifestHash = sceneGraph.semanticManifest.semanticManifestHash;
  authority.meaningItems = sceneGraph.semanticManifest.entries.map((entry, index) => ({
    meaningId: `meaning.${entry.kind}.${String(index + 1).padStart(3, '0')}`,
    semanticRef: { semanticId: entry.semanticId, entryHash: entry.entryHash },
    shotId: entry.shotId,
    kind: entry.kind,
    sourceText: entry.sourceText
  }));
  authority.authorityHash = hashTrackBAuthority(authority);
  const assetManifest = { schemaVersion: 1, assetManifestId: 'manifest.contract', assets: [] };
  const provenance = {
    schemaVersion: 2,
    narrativeId: authority.narrativeId,
    projectionReceiptHash: authority.sourceRefs.projectionReceiptHash,
    sceneGraphHash,
    sourceSemanticManifestHash: sceneGraph.semanticManifest.sourceSemanticManifestHash,
    semanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
    htmlHash,
    embedding: {
      anchor: 'trackb-semantic-manifest',
      encoding: 'base64url-rfc4648',
      payloadHash: hashTrackBRawBytes(Buffer.from(canonicalizeJson(sceneGraph.semanticManifest), 'utf8'))
    },
    handoffHash: hashTrackBRawBytes(handoffBytes),
    assetManifestHash: canonicalHash(assetManifest)
  };
  provenance.provenanceHash = canonicalHash(provenance);
  const assetManifestHash = provenance.assetManifestHash;
  const evidenceReceipt = {
    schemaVersion: 2,
    receiptId: 'receipt.contract.v2',
    narrativeId: authority.narrativeId,
    authorityHash: authority.authorityHash,
    htmlHash,
    assetManifestHash,
    semanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
    rendererLockHash: `sha256:${'4'.repeat(64)}`,
    navigationGuard: {
      mechanism: 'cdp-binding-nonce',
      guardSourceHash: 'sha256:922cd3832fe5988739a4be38f2d4c15bde7a672fa92b7301ee212548a3bd1099'
    },
    shotIds: authority.shots.map((shot) => shot.shotId),
    shots: authority.shots.map((shot, ordinal) => ({
      shotId: shot.shotId,
      ordinal,
      screenshotHash: `sha256:${String(5 + ordinal).repeat(64)}`,
      runtime: {
        networkRequestCount: 0,
        externalRequestCount: 0,
        navigationAttemptCount: 0,
        exceptionCount: 0,
        semanticManifestVerified: true,
        pass: true
      },
      visual: {
        activeShotCount: 1,
        visibleShotCount: 1,
        activeShotId: shot.shotId,
        visibleShotId: shot.shotId,
        stageGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        shotGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        pass: true
      }
    }))
  };
  evidenceReceipt.receiptHash = hashTrackBMeasuredEvidenceReceipt(evidenceReceipt);
  const evidenceReceiptBytes = Buffer.from(canonicalizeJson(evidenceReceipt), 'utf8');
  const candidate = structuredClone(values.trackBReleaseCandidate);
  candidate.authorityHash = authority.authorityHash;
  candidate.evidenceReceiptHash = evidenceReceipt.receiptHash;
  const qualification = deriveTrackBMeaningQualification({
    authority,
    projectionReceipt,
    projectionReceiptBytes,
    sceneGraph,
    provenance,
    htmlBytes
  });
  assert.equal(qualification.valid, true, qualification.error);
  candidate.meaningQualificationHash = qualification.meaningQualificationHash;
  candidate.artifacts = {
    ...candidate.artifacts,
    projectionReceiptHash: authority.sourceRefs.projectionReceiptHash,
    sceneGraphHash,
    assetHandoffHash: hashTrackBRawBytes(handoffBytes),
    assetHandoffSceneGraphHash: sceneGraphHash,
    htmlHash,
    provenanceHash: provenance.provenanceHash
  };
  candidate.evidenceCells[0].meaningCoverage = authority.meaningItems.map((item) => ({
    meaningId: item.meaningId,
    entryHash: item.semanticRef.entryHash,
    status: 'SUPPORTED',
    representation: 'METADATA'
  }));
  candidate.evidenceCells[0].visualEvidence.htmlHash = htmlHash;
  candidate.evidenceCells[0].assetEvidence.assetManifestHash = provenance.assetManifestHash;
  candidate.evidenceCells[0].evidenceCellHash = hashTrackBEvidenceCell(candidate.evidenceCells[0]);
  candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
  return {
    candidate,
    artifactBindings: {
      registry: await registry(),
      authority,
      projectionReceipt,
      projectionReceiptBytes,
      sceneGraph,
      handoff,
      handoffBytes,
      htmlBytes,
      provenance,
      provenanceBytes: Buffer.from(`${JSON.stringify(provenance)}\n`, 'utf8'),
      evidenceReceipt,
      evidenceReceiptBytes,
      artifactRegistries
    }
  };
}

const NEGATIVE_REGISTRY_MUTATIONS = Object.freeze({
  path(value) { value.schemas[0].path = '../escape.json'; },
  id(value) { value.schemas[0].$id = 'https://scene-motion-compiler.local/contracts/trackb/other/v1'; },
  version(value) { value.schemas[0].version = 1; },
  logicalName(value) { value.schemas[0].logicalName = 'TrackBNarrativeAuthority v1'; },
  duplicate(value) { value.schemas[1].path = value.schemas[0].path; },
  extra(value) { value.schemas.push({ ...value.schemas[0], path: 'extra.schema.json', $id: 'https://scene-motion-compiler.local/contracts/trackb/extra/v1', logicalName: 'TrackBExtra v1' }); }
});

const NEGATIVE_TARGET_BY_MUTATION = Object.freeze({
  path: 'registry',
  id: 'registry',
  version: 'registry',
  logicalName: 'registry',
  duplicate: 'registry',
  extra: 'registry',
  'delete-required-authorityHash': 'trackBNarrativeAuthority',
  'receipt-v1-schema': 'trackBMeasuredEvidenceReceipt',
  'candidate-v3-schema': 'trackBReleaseCandidate',
  'add-extra-property': 'trackBReleaseCandidate',
  'add-snake-case-field': 'trackBReleaseCandidate',
  'change-authorityHash': 'trackBNarrativeAuthority',
  'change-candidateHash': 'trackBReleaseCandidate',
  'change-htmlHash': 'trackBReleaseCandidate',
  'duplicate-shotId': 'trackBReleaseCandidate',
  'unknown-meaningId': 'trackBReleaseCandidate',
  'promote-unsupported-meaning': 'trackBReleaseCandidate',
  'change-subject-candidateHash': 'trackBReleaseApprovalReceipt',
  'invalid-decision': 'trackBReleaseApprovalReceipt',
  'approval-with-wrong-decision': 'trackBStatusResult',
  'status-narrative-mismatch': 'trackBStatusResult',
  'delivery-path-parent': 'trackBDeliveryPointer',
  'delivery-path-current': 'trackBDeliveryPointer',
  'delivery-path-empty-segment': 'trackBDeliveryPointer',
  'delivery-path-backslash': 'trackBDeliveryPointer',
  'ordinal-as-string': 'trackBNarrativeAuthority',
  'runtime-pass-as-string': 'trackBReleaseCandidate',
  'delete-runtime-pass': 'trackBReleaseCandidate',
  'blank-story-text': 'trackBNarrativeAuthority',
  'invalid-date': 'trackBReleaseApprovalReceipt',
  'more-than-five-minutes-future': 'trackBReleaseApprovalReceipt'
});

async function executeRegistryNegativeFixture(item) {
  const root = await mkdtemp(join(tmpdir(), `smc-trackb-negative-${item.mutation}-`));
  try {
    await cp('contracts/schemas/trackb', root, { recursive: true });
    const registryPath = join(root, 'registry.json');
    const value = JSON.parse(await readFile(registryPath, 'utf8'));
    const mutate = NEGATIVE_REGISTRY_MUTATIONS[item.mutation];
    assert.equal(typeof mutate, 'function', `unknown registry mutation ${item.mutation}`);
    mutate(value);
    await writeFile(registryPath, `${JSON.stringify(value)}\n`);
    try {
      await loadTrackBContractRegistry(root);
      return { valid: true, errors: [] };
    } catch (error) {
      return { valid: false, errors: [{ code: error.code ?? 'trackb-registry-invalid', message: error.message }] };
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

async function executeSemanticNegativeFixture(item, values, loaded, artifactRegistries) {
  if (item.mutation === 'change-htmlHash') {
    const vector = await buildArtifactBindingVector(values, artifactRegistries);
    vector.candidate.artifacts.htmlHash = `sha256:${'0'.repeat(64)}`;
    vector.candidate.candidateHash = hashTrackBReleaseCandidate(vector.candidate);
    return validateTrackBReleaseCandidate(vector.candidate, {
      registry: loaded,
      authority: vector.artifactBindings.authority,
      artifactBindings: vector.artifactBindings
    });
  }
  const candidate = structuredClone(values.trackBReleaseCandidate);
  if (item.mutation === 'change-candidateHash') {
    candidate.candidateHash = `sha256:${'0'.repeat(64)}`;
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'duplicate-shotId') {
    candidate.evidenceCells.push(structuredClone(candidate.evidenceCells[0]));
    candidate.evidenceCells[1].evidenceCellHash = hashTrackBEvidenceCell(candidate.evidenceCells[1]);
    candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
    return validateTrackBReleaseCandidate(candidate, { registry: loaded, authority: values.trackBNarrativeAuthority });
  }
  if (item.mutation === 'unknown-meaningId') {
    candidate.evidenceCells[0].meaningCoverage[0].meaningId = 'meaning.unknown';
    candidate.evidenceCells[0].evidenceCellHash = hashTrackBEvidenceCell(candidate.evidenceCells[0]);
    candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
    return validateTrackBReleaseCandidate(candidate, { registry: loaded, authority: values.trackBNarrativeAuthority });
  }
  if (item.mutation === 'promote-unsupported-meaning') {
    candidate.candidateVerdict = 'PROMOTE';
    candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
    return validateTrackBReleaseCandidate(candidate, { registry: loaded, authority: values.trackBNarrativeAuthority });
  }
  if (item.mutation === 'change-authorityHash') {
    const authority = structuredClone(values.trackBNarrativeAuthority);
    authority.authorityHash = `sha256:${'0'.repeat(64)}`;
    return validateTrackBNarrativeAuthority(authority, { registry: loaded });
  }
  if (item.mutation === 'change-subject-candidateHash') {
    const approval = structuredClone(values.trackBReleaseApprovalReceipt);
    approval.subject.candidateHash = `sha256:${'0'.repeat(64)}`;
    approval.approvalHash = hashTrackBApprovalReceipt(approval);
    return validateTrackBApprovalReceipt(approval, {
      registry: loaded,
      candidateHash: values.trackBReleaseCandidate.candidateHash,
      baseDeliverySetHash: 'NONE',
      now: new Date('2026-08-28T00:00:00Z')
    });
  }
  if (item.target === 'trackBDeliveryPointer') {
    const pointer = structuredClone(values.trackBDeliveryPointer);
    pointer.deliverySetPath = {
      'delivery-path-parent': '../secret/trackb-delivery-set.json',
      'delivery-path-current': './trackb-delivery-set.json',
      'delivery-path-empty-segment': 'a//b',
      'delivery-path-backslash': 'a\\b'
    }[item.mutation];
    assert.equal(typeof pointer.deliverySetPath, 'string', `unknown pointer path mutation ${item.mutation}`);
    pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
    return validateTrackBDeliveryPointer(pointer, { registry: loaded });
  }
  throw new Error(`unknown semantic mutation ${item.mutation}`);
}

async function executeNegativeFixture(item, values, loaded, artifactRegistries) {
  assert.equal(item.target, NEGATIVE_TARGET_BY_MUTATION[item.mutation], `${item.name} target does not match its executable mutation`);
  if (item.kind === 'registry') return executeRegistryNegativeFixture(item);
  if (item.kind === 'transport') {
    const hash = `sha256:${'a'.repeat(64)}`;
    const status = {
      narrativeId: 'narrative.demo',
      pointer: { state: 'PROMOTED', currentDeliverySetHash: hash, candidateHash: hash, approvalHash: hash, pointerRevision: 1, pointerHash: hash },
      candidateStatus: { state: 'APPROVED', candidateHash: hash, baseDeliverySetHash: 'NONE', candidateVerdict: 'PROMOTE', approvalHash: hash, decision: 'REVISE' }
    };
    if (item.mutation === 'status-narrative-mismatch') status.narrativeId = 'narrative.other';
    return validateTrackBCommandResult('trackb.release.status', status, { input: { narrativeId: 'narrative.demo', candidateHash: hash } });
  }
  if (item.kind === 'semantic') return executeSemanticNegativeFixture(item, values, loaded, artifactRegistries);

  if (item.mutation === 'delete-required-authorityHash') {
    const authority = structuredClone(values.trackBNarrativeAuthority);
    delete authority.sourceRefs.projectionReceiptHash;
    return validateTrackBNarrativeAuthority(authority, { registry: loaded });
  }
  if (item.mutation === 'receipt-v1-schema') {
    const receipt = structuredClone(values.trackBMeasuredEvidenceReceipt);
    receipt.schemaVersion = 1;
    delete receipt.navigationGuard;
    return validateTrackBMeasuredEvidenceReceipt(receipt, { registry: loaded });
  }
  if (item.mutation === 'candidate-v3-schema') {
    const candidate = structuredClone(values.trackBReleaseCandidate);
    candidate.schemaVersion = 3;
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'add-extra-property') {
    const candidate = structuredClone(values.trackBReleaseCandidate);
    candidate.unexpected = true;
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'add-snake-case-field') {
    const candidate = structuredClone(values.trackBReleaseCandidate);
    candidate.evidenceCells[0].meaning_coverage = [];
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'invalid-decision') {
    const approval = structuredClone(values.trackBReleaseApprovalReceipt);
    approval.decision = 'MAYBE';
    return validateTrackBApprovalReceipt(approval, { registry: loaded });
  }
  if (item.mutation === 'ordinal-as-string') {
    const authority = structuredClone(values.trackBNarrativeAuthority);
    authority.shots[0].ordinal = '0';
    return validateTrackBNarrativeAuthority(authority, { registry: loaded });
  }
  if (item.mutation === 'runtime-pass-as-string') {
    const candidate = structuredClone(values.trackBReleaseCandidate);
    candidate.evidenceCells[0].runtimeEvidence.pass = 'true';
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'delete-runtime-pass') {
    const candidate = structuredClone(values.trackBReleaseCandidate);
    delete candidate.evidenceCells[0].runtimeEvidence.pass;
    return validateTrackBReleaseCandidate(candidate, { registry: loaded });
  }
  if (item.mutation === 'blank-story-text') {
    const authority = structuredClone(values.trackBNarrativeAuthority);
    authority.story.theme = ' \n\t ';
    return validateTrackBNarrativeAuthority(authority, { registry: loaded });
  }
  if (item.mutation === 'invalid-date') {
    const approval = structuredClone(values.trackBReleaseApprovalReceipt);
    approval.decidedAt = '2026-02-30T00:00:00Z';
    approval.approvalHash = hashTrackBApprovalReceipt(approval);
    return validateTrackBApprovalReceipt(approval, { registry: loaded, now: new Date('2026-08-28T00:00:00Z') });
  }
  if (item.mutation === 'more-than-five-minutes-future') {
    const approval = structuredClone(values.trackBReleaseApprovalReceipt);
    approval.decidedAt = '2026-08-28T00:06:00Z';
    approval.approvalHash = hashTrackBApprovalReceipt(approval);
    return validateTrackBApprovalReceipt(approval, { registry: loaded, now: new Date('2026-08-28T00:00:00Z') });
  }
  throw new Error(`unknown negative mutation ${item.mutation}`);
}

test('Track B registry declares exactly six immutable schemas and core SCHEMAS IDs', async () => {
  const loaded = await registry();
  assert.equal(loaded.definitions.length, 6);
  const keys = loaded.definitions.map((item) => item.key);
  assert.deepEqual(keys, Object.keys(EXAMPLES));
  for (const definition of loaded.definitions) {
    assert.equal(loaded.definition(definition.file).id, definition.id);
    assert.equal(SCHEMAS[definition.key], definition.id);
    assert.equal(loaded.schema(definition.id).schema.$id, definition.id);
  }
});

test('explicit loader rejects registry path, id, version, logical-name, and extra-entry drift', async () => {
  const source = 'contracts/schemas/trackb';
  const mutations = {
    path: (value) => { value.schemas[0].path = '../escape.json'; },
    id: (value) => { value.schemas[0].$id = 'https://scene-motion-compiler.local/contracts/trackb/other/v1'; },
    version: (value) => { value.schemas[0].version = 1; },
    logicalName: (value) => { value.schemas[0].logicalName = 'TrackBNarrativeAuthority v1'; },
    duplicate: (value) => { value.schemas[1].path = value.schemas[0].path; },
    extra: (value) => { value.schemas.push({ ...value.schemas[0], path: 'extra.schema.json', $id: 'https://scene-motion-compiler.local/contracts/trackb/extra/v1', logicalName: 'TrackBExtra v1' }); }
  };
  const registryCases = NEGATIVE_FIXTURES.cases.filter((item) => item.kind === 'registry');
  assert.deepEqual(registryCases.map((item) => item.mutation).sort(), Object.keys(mutations).sort());
  for (const [label, mutate] of Object.entries(mutations)) {
    const root = await mkdtemp(join(tmpdir(), `smc-trackb-registry-${label}-`));
    try {
      await cp(source, root, { recursive: true });
      const registryPath = join(root, 'registry.json');
      const value = JSON.parse(await readFile(registryPath, 'utf8'));
      mutate(value);
      await writeFile(registryPath, `${JSON.stringify(value)}\n`);
      await assert.rejects(() => loadTrackBContractRegistry(root), /path|immutable|safe|unique|exactly six|invalid|version/i, label);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  }
  markNegativeFixtures(...registryCases.map((item) => item.name));
});

test('all six valid examples pass strict schema validation and self-hash vectors', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  for (const [key, value] of Object.entries(values)) assert.equal(loaded.validate(key, value).valid, true, key);
  assert.equal(hashTrackBAuthority(values.trackBNarrativeAuthority), values.trackBNarrativeAuthority.authorityHash);
  assert.equal(hashTrackBMeasuredEvidenceReceipt(values.trackBMeasuredEvidenceReceipt), values.trackBMeasuredEvidenceReceipt.receiptHash);
  assert.equal(hashTrackBEvidenceCell(values.trackBReleaseCandidate.evidenceCells[0]), values.trackBReleaseCandidate.evidenceCells[0].evidenceCellHash);
  assert.equal(hashTrackBReleaseCandidate(values.trackBReleaseCandidate), values.trackBReleaseCandidate.candidateHash);
  const legacyCandidateDomainHash = canonicalHash(Object.fromEntries(
    TRACK_B_HASH_DOMAINS.candidateHash.map((field) => [field, values.trackBReleaseCandidate[field]])
  ));
  assert.notEqual(hashTrackBReleaseCandidate(values.trackBReleaseCandidate), legacyCandidateDomainHash);
  assert.throws(
    () => hashTrackBReleaseCandidate({ ...values.trackBReleaseCandidate, schemaVersion: 3, candidateHash: legacyCandidateDomainHash }),
    (error) => error.code === 'trackb-input-invalid',
  );
  assert.equal(hashTrackBApprovalReceipt(values.trackBReleaseApprovalReceipt), values.trackBReleaseApprovalReceipt.approvalHash);
  assert.equal(hashTrackBDeliverySet(values.trackBDeliverySet), values.trackBDeliverySet.deliverySetHash);
  assert.equal(hashTrackBDeliveryPointer(values.trackBDeliveryPointer), values.trackBDeliveryPointer.pointerHash);
  assert.deepEqual(Object.keys(TRACK_B_HASH_DOMAINS), ['authorityHash', 'evidenceCellHash', 'receiptHash', 'meaningQualificationHash', 'candidateHash', 'candidateV4Hash', 'approvalHash', 'deliverySetHash', 'pointerHash']);
});

test('nested required, unknown, snake_case, wrong primitive, and no implicit default are rejected', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  const missing = structuredClone(values.trackBNarrativeAuthority);
  delete missing.sourceRefs.projectionReceiptHash;
  assert.equal(loaded.validate('trackBNarrativeAuthority', missing).valid, false);
  markNegativeFixtures('required-field-missing');
  const extra = structuredClone(values.trackBReleaseCandidate);
  extra.unexpected = true;
  assert.equal(loaded.validate('trackBReleaseCandidate', extra).valid, false);
  markNegativeFixtures('unknown-property');
  const snake = structuredClone(values.trackBReleaseCandidate);
  snake.evidenceCells[0].meaning_coverage = [];
  assert.equal(loaded.validate('trackBReleaseCandidate', snake).valid, false);
  markNegativeFixtures('snake-case');
  const wrongInteger = structuredClone(values.trackBNarrativeAuthority);
  wrongInteger.shots[0].ordinal = '0';
  assert.equal(loaded.validate('trackBNarrativeAuthority', wrongInteger).valid, false);
  markNegativeFixtures('integer-primitive');
  const wrongBoolean = structuredClone(values.trackBReleaseCandidate);
  wrongBoolean.evidenceCells[0].runtimeEvidence.pass = 'true';
  assert.equal(loaded.validate('trackBReleaseCandidate', wrongBoolean).valid, false);
  markNegativeFixtures('boolean-no-coercion');
  const missingNested = structuredClone(values.trackBReleaseCandidate);
  delete missingNested.evidenceCells[0].runtimeEvidence.pass;
  assert.equal(loaded.validate('trackBReleaseCandidate', missingNested).valid, false);
  markNegativeFixtures('default-forbidden');
  const whitespace = structuredClone(values.trackBNarrativeAuthority);
  whitespace.story.theme = ' \n\t ';
  assert.equal(loaded.validate('trackBNarrativeAuthority', whitespace).valid, false);
  markNegativeFixtures('whitespace-text');
  for (const definition of loaded.definitions) assert.doesNotThrow(() => loaded.schema(definition.key));
});

test('authority semantic validation enforces self-hash, unique shot ids, zero-based order, and meaning shot references', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  assert.equal(validateTrackBNarrativeAuthority(values.trackBNarrativeAuthority, { registry: loaded }).valid, true);
  const invalid = structuredClone(values.trackBNarrativeAuthority);
  invalid.shots[0].sceneId = 'scene.changed';
  assert.equal(validateTrackBNarrativeAuthority(invalid, { registry: loaded }).valid, false);
  markNegativeFixtures('authority-self-hash');
  const duplicate = structuredClone(values.trackBNarrativeAuthority);
  duplicate.shots.push({ ...duplicate.shots[0] });
  duplicate.shots[1].ordinal = 1;
  duplicate.authorityHash = hashTrackBAuthority(duplicate);
  assert.equal(validateTrackBNarrativeAuthority(duplicate, { registry: loaded }).valid, false);
});

test('current composer baseline remains ITERATE only because measured gate is ITERATE', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  assert.equal(values.trackBReleaseCandidate.candidateVerdict, 'ITERATE');
  assert.equal(values.trackBReleaseCandidate.evidenceCells[0].meaningCoverage.every((item) => item.status === 'SUPPORTED'), true);
  assert.equal(values.trackBReleaseCandidate.evidenceCells[0].meaningCoverage.every((item) => item.representation === 'METADATA'), true);
  assert.equal(deriveCandidateVerdict(values.trackBReleaseCandidate), 'ITERATE');
  assert.equal(validateTrackBReleaseCandidate(values.trackBReleaseCandidate, { registry: loaded, authority: values.trackBNarrativeAuthority }).valid, true);
  const fakePromote = structuredClone(values.trackBReleaseCandidate);
  fakePromote.candidateVerdict = 'PROMOTE';
  fakePromote.candidateHash = hashTrackBReleaseCandidate(fakePromote);
  assert.equal(validateTrackBReleaseCandidate(fakePromote, { registry: loaded, authority: values.trackBNarrativeAuthority }).valid, false);
  markNegativeFixtures('unsupported-meaning');
  const badHash = structuredClone(values.trackBReleaseCandidate);
  badHash.candidateId = 'candidate.changed';
  assert.equal(validateTrackBReleaseCandidate(badHash, { registry: loaded }).valid, false);
  markNegativeFixtures('candidate-self-hash');
});

test('candidate shot and meaning coverage are exact bijections', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  const duplicateShot = structuredClone(values.trackBReleaseCandidate);
  duplicateShot.evidenceCells.push(structuredClone(duplicateShot.evidenceCells[0]));
  duplicateShot.evidenceCells[1].evidenceCellHash = hashTrackBEvidenceCell(duplicateShot.evidenceCells[1]);
  duplicateShot.candidateHash = hashTrackBReleaseCandidate(duplicateShot);
  assert.match(validateTrackBReleaseCandidate(duplicateShot, { registry: loaded, authority: values.trackBNarrativeAuthority }).errors.map((item) => item.code).join(','), /trackb-evidence-incomplete/);
  markNegativeFixtures('shot-bijection');
  const unknownMeaning = structuredClone(values.trackBReleaseCandidate);
  unknownMeaning.evidenceCells[0].meaningCoverage[0].meaningId = 'meaning.unknown';
  unknownMeaning.evidenceCells[0].evidenceCellHash = hashTrackBEvidenceCell(unknownMeaning.evidenceCells[0]);
  unknownMeaning.candidateHash = hashTrackBReleaseCandidate(unknownMeaning);
  assert.equal(validateTrackBReleaseCandidate(unknownMeaning, { registry: loaded, authority: values.trackBNarrativeAuthority }).valid, false);
  markNegativeFixtures('meaning-bijection');
});

test('raw projection receipt hash is byte-bound and whitespace drift fails', () => {
  const canonical = Buffer.from('{"schemaVersion":1,"sceneGraphHash":"sha256:' + 'a'.repeat(64) + '"}\n', 'utf8');
  const drifted = Buffer.from('{ "schemaVersion": 1, "sceneGraphHash": "sha256:' + 'a'.repeat(64) + '" }\n', 'utf8');
  const expected = hashTrackBRawBytes(canonical);
  assert.equal(hashTrackBRawBytes(drifted) === expected, false);
  assert.match(expected, /^sha256:[a-f0-9]{64}$/);
});

test('artifact binding helper checks raw bytes, embedded identity, and provenance canonical hash', async () => {
  const values = await loadExamples();
  const artifactRegistries = await loadTrackBArtifactContractRegistries({ smcSchemaDir: 'contracts/schemas', sceneAssetCompilerSchemaDir: 'contracts/schemas/scene-asset-compiler' });
  const loaded = await registry();
  const vector = await buildArtifactBindingVector(values, artifactRegistries);
  const { candidate, artifactBindings } = vector;
  const good = validateTrackBArtifactBindings(candidate, artifactBindings);
  assert.equal(good.valid, true, JSON.stringify(good.errors));
  const bad = validateTrackBArtifactBindings(candidate, { ...artifactBindings, projectionReceiptBytes: Buffer.from(`${JSON.stringify(artifactBindings.projectionReceipt)}\n `, 'utf8') });
  assert.equal(bad.valid, false);
  const missingRegistry = validateTrackBArtifactBindings(candidate, { ...artifactBindings, registry: undefined });
  assert.equal(missingRegistry.valid, false);
  const malformedProjection = validateTrackBArtifactBindings(candidate, { ...artifactBindings, projectionReceipt: { ...artifactBindings.projectionReceipt, sceneGraphHash: 1 } });
  assert.equal(malformedProjection.valid, false);
  const malformedGraph = validateTrackBArtifactBindings(candidate, { ...artifactBindings, sceneGraph: { ...artifactBindings.sceneGraph, shots: [] } });
  assert.equal(malformedGraph.valid, false);
  const malformedHandoff = { ...artifactBindings.handoff };
  delete malformedHandoff.sceneGraphHash;
  const malformedHandoffResult = validateTrackBArtifactBindings(candidate, { ...artifactBindings, handoff: malformedHandoff });
  assert.equal(malformedHandoffResult.valid, false);
  const selfReference = validateTrackBArtifactBindings(candidate, { ...artifactBindings, projectionReceipt: { ...artifactBindings.projectionReceipt, projectionReceiptHash: candidate.artifacts.projectionReceiptHash } });
  assert.equal(selfReference.valid, false);
  const requiredEvidenceKeys = ['authority', 'projectionReceipt', 'projectionReceiptBytes', 'sceneGraph', 'handoff', 'handoffBytes', 'htmlBytes', 'provenance', 'evidenceReceipt', 'evidenceReceiptBytes'];
  for (const key of requiredEvidenceKeys) {
    const incomplete = { ...artifactBindings };
    delete incomplete[key];
    const result = validateTrackBArtifactBindings(candidate, incomplete);
    assert.equal(result.valid, false, `missing ${key} must fail closed`);
    assert.ok(result.errors.some((error) => error.code === 'trackb-artifact-binding-invalid'), `missing ${key} error code`);
  }
  for (const key of ['smcRegistry', 'sceneAssetCompilerRegistry']) {
    const incomplete = { ...artifactBindings, artifactRegistries: { ...artifactBindings.artifactRegistries } };
    delete incomplete.artifactRegistries[key];
    const result = validateTrackBArtifactBindings(candidate, incomplete);
    assert.equal(result.valid, false, `missing ${key} must fail closed`);
    assert.ok(result.errors.some((error) => error.code === 'trackb-artifact-binding-invalid'), `missing ${key} error code`);
  }
  assert.equal(validateTrackBReleaseCandidate(candidate, { registry: loaded, authority: artifactBindings.authority, artifactBindings }).valid, true);
  markNegativeFixtures('artifact-binding');
});

test('meaning qualification rejects duplicate anchor ids on any tag and executable marker attributes', async () => {
  const values = await loadExamples();
  const artifactRegistries = await loadTrackBArtifactContractRegistries({ smcSchemaDir: 'contracts/schemas', sceneAssetCompilerSchemaDir: 'contracts/schemas/scene-asset-compiler' });
  const { artifactBindings } = await buildArtifactBindingVector(values, artifactRegistries);
  const html = artifactBindings.htmlBytes.toString('utf8');
  for (const mutation of [
    html.replace('<!DOCTYPE html>', '<!DOCTYPE html><span id="trackb-semantic-manifest">spoof</span>'),
    html.replace('<!DOCTYPE html>', '<!DOCTYPE html><script id=trackb-semantic-manifest></script>'),
    html.replace('<!DOCTYPE html>', '<!DOCTYPE html><span id="trackb&#45;semantic-manifest">spoof</span>'),
    html.replace('<div id="trackb-semantic-manifest"', '<div onload="alert(1)" id="trackb-semantic-manifest"')
  ]) {
    const htmlBytes = Buffer.from(mutation, 'utf8');
    const provenance = { ...artifactBindings.provenance, htmlHash: hashTrackBRawBytes(htmlBytes) };
    delete provenance.provenanceHash;
    provenance.provenanceHash = canonicalHash(provenance);
    const qualification = deriveTrackBMeaningQualification({
      authority: artifactBindings.authority,
      projectionReceipt: artifactBindings.projectionReceipt,
      projectionReceiptBytes: artifactBindings.projectionReceiptBytes,
      sceneGraph: artifactBindings.sceneGraph,
      provenance,
      htmlBytes
    });
    assert.equal(qualification.valid, false);
    assert.match(qualification.error, /anchor id must occur exactly once|exact non-executable contract/);
  }
});

test('approval receipt subject and human Studio origin are strict', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  assert.equal(loaded.validate('trackBReleaseApprovalReceipt', values.trackBReleaseApprovalReceipt).valid, true);
  const mismatch = structuredClone(values.trackBReleaseApprovalReceipt);
  mismatch.subject.candidateHash = `sha256:${'0'.repeat(64)}`;
  assert.equal(validateTrackBCommandInput('trackb.release.approve', { narrativeId: 'narrative.demo', candidateHash: values.trackBReleaseCandidate.candidateHash, baseDeliverySetHash: 'NONE', decision: 'APPROVE', reason: 'reviewed', decidedAt: '2026-08-28T00:00:00Z' }).valid, true);
  assert.equal(loaded.validate('trackBReleaseApprovalReceipt', mismatch).valid, true);
  const semantic = validateTrackBApprovalReceipt(mismatch, { registry: loaded, candidateHash: values.trackBReleaseCandidate.candidateHash, baseDeliverySetHash: 'NONE', now: new Date('2026-08-28T00:00:00Z') });
  assert.equal(semantic.valid, false);
  markNegativeFixtures('approval-subject');
  const nonHuman = structuredClone(values.trackBReleaseApprovalReceipt);
  nonHuman.actor.actorKind = 'system';
  assert.equal(loaded.validate('trackBReleaseApprovalReceipt', nonHuman).valid, false);
  const invalidDecision = structuredClone(values.trackBReleaseApprovalReceipt);
  invalidDecision.decision = 'MAYBE';
  assert.equal(loaded.validate('trackBReleaseApprovalReceipt', invalidDecision).valid, false);
  markNegativeFixtures('approval-decision');
  const invalidCalendar = structuredClone(values.trackBReleaseApprovalReceipt);
  invalidCalendar.decidedAt = '2026-02-30T00:00:00Z';
  invalidCalendar.approvalHash = hashTrackBApprovalReceipt(invalidCalendar);
  assert.equal(validateTrackBApprovalReceipt(invalidCalendar, { registry: loaded, now: new Date('2026-08-28T00:00:00Z') }).valid, false);
  markNegativeFixtures('invalid-calendar');
  const future = structuredClone(values.trackBReleaseApprovalReceipt);
  future.decidedAt = '2026-08-28T00:06:00Z';
  future.approvalHash = hashTrackBApprovalReceipt(future);
  assert.equal(validateTrackBApprovalReceipt(future, { registry: loaded, now: new Date('2026-08-28T00:00:00Z') }).valid, false);
  markNegativeFixtures('future-date');
});

test('delivery set and pointer bindings require a promotable candidate and exact approved subject', async () => {
  const loaded = await registry();
  const values = await loadExamples();
  const candidate = structuredClone(values.trackBReleaseCandidate);
  candidate.evidenceCells[0].gateVerdict = 'PROMOTE';
  for (const item of candidate.evidenceCells[0].meaningCoverage) item.status = 'SUPPORTED';
  candidate.evidenceCells[0].evidenceCellHash = hashTrackBEvidenceCell(candidate.evidenceCells[0]);
  candidate.candidateVerdict = 'PROMOTE';
  candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
  const approval = structuredClone(values.trackBReleaseApprovalReceipt);
  approval.subject = {
    candidateHash: candidate.candidateHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash: candidate.baseDeliverySetHash
  };
  approval.decision = 'APPROVE';
  approval.approvalHash = hashTrackBApprovalReceipt(approval);
  const deliverySet = structuredClone(values.trackBDeliverySet);
  deliverySet.candidateHash = candidate.candidateHash;
  deliverySet.authorityHash = candidate.authorityHash;
  deliverySet.meaningQualificationHash = candidate.meaningQualificationHash;
  deliverySet.baseDeliverySetHash = candidate.baseDeliverySetHash;
  deliverySet.approvalHash = approval.approvalHash;
  deliverySet.artifacts = structuredClone(candidate.artifacts);
  deliverySet.evidenceCellHashes = candidate.evidenceCells.map((cell) => cell.evidenceCellHash);
  deliverySet.deliverySetHash = hashTrackBDeliverySet(deliverySet);
  assert.equal(validateTrackBDeliverySet(deliverySet, { registry: loaded, candidate, approval }).valid, true);
  assert.equal(assertTrackBDeliverySet(deliverySet, { registry: loaded, candidate, approval }), deliverySet);
  const artifactMismatch = structuredClone(deliverySet);
  artifactMismatch.artifacts.htmlHash = `sha256:${'0'.repeat(64)}`;
  artifactMismatch.deliverySetHash = hashTrackBDeliverySet(artifactMismatch);
  assert.equal(validateTrackBDeliverySet(artifactMismatch, { registry: loaded, candidate, approval }).valid, false);
  const notPromotable = structuredClone(candidate);
  notPromotable.candidateVerdict = 'ITERATE';
  notPromotable.candidateHash = hashTrackBReleaseCandidate(notPromotable);
  assert.equal(validateTrackBDeliverySet(deliverySet, { registry: loaded, candidate: notPromotable, approval }).valid, false);
  const reviseApproval = structuredClone(approval);
  reviseApproval.decision = 'REVISE';
  reviseApproval.approvalHash = hashTrackBApprovalReceipt(reviseApproval);
  assert.equal(validateTrackBDeliverySet(deliverySet, { registry: loaded, candidate, approval: reviseApproval }).valid, false);
  const staleApproval = structuredClone(approval);
  staleApproval.subject.candidateHash = `sha256:${'0'.repeat(64)}`;
  staleApproval.approvalHash = hashTrackBApprovalReceipt(staleApproval);
  assert.equal(validateTrackBDeliverySet(deliverySet, { registry: loaded, candidate, approval: staleApproval }).valid, false);
  const pointer = structuredClone(values.trackBDeliveryPointer);
  pointer.candidateHash = deliverySet.candidateHash;
  pointer.approvalHash = deliverySet.approvalHash;
  pointer.meaningQualificationHash = deliverySet.meaningQualificationHash;
  pointer.baseDeliverySetHash = deliverySet.baseDeliverySetHash;
  pointer.deliverySetHash = deliverySet.deliverySetHash;
  pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
  assert.equal(validateTrackBDeliveryPointer(pointer, { registry: loaded, deliverySet, candidate, approval }).valid, true);
  const pointerMismatch = structuredClone(pointer);
  pointerMismatch.approvalHash = `sha256:${'0'.repeat(64)}`;
  pointerMismatch.pointerHash = hashTrackBDeliveryPointer(pointerMismatch);
  assert.equal(validateTrackBDeliveryPointer(pointerMismatch, { registry: loaded, deliverySet, candidate, approval }).valid, false);
});

test('command input and canonical transport projections are exact with required narrativeId', () => {
  assert.deepEqual(TRACK_B_RELEASE_COMMANDS, ['trackb.evidence.measure', 'trackb.release.prepare', 'trackb.release.approve', 'trackb.release.promote', 'trackb.release.status']);
  assert.equal(CLI_COMMAND_REGISTRY.some(({ name }) => name === 'trackb-release-approve'), false);
  assert.equal(MCP_COMMAND_REGISTRY.some(({ name }) => name === 'trackb_release_approve'), false);
  assert.equal(CLI_COMMAND_REGISTRY.find(({ name }) => name === 'trackb-release-promote')?.command, 'trackb.release.promote');
  assert.equal(MCP_COMMAND_REGISTRY.find(({ name }) => name === 'trackb_release_promote')?.command, 'trackb.release.promote');
  const approve = { narrativeId: 'narrative.demo', candidateHash: `sha256:${'a'.repeat(64)}`, baseDeliverySetHash: 'NONE', decision: 'APPROVE', reason: 'human review', decidedAt: '2026-08-28T00:00:00Z' };
  const fixedNow = new Date('2026-08-28T00:00:00Z');
  assert.equal(validateTrackBCommandInput('trackb.release.approve', approve, { now: fixedNow }).valid, true);
  const missingNarrative = { ...approve };
  delete missingNarrative.narrativeId;
  assert.equal(validateTrackBCommandInput('trackb.release.approve', missingNarrative).valid, false);
  assert.equal(validateTrackBCommandInput('trackb.release.status', { narrativeId: 'narrative.demo' }).valid, true);
  assert.equal(validateTrackBCommandInput('trackb.release.status', { narrativeId: 'narrative.demo', candidateHash: `sha256:${'a'.repeat(64)}` }).valid, true);
  assert.equal(validateTrackBCommandInput('trackb.release.status', { narrativeId: 'narrative.demo', candidateHash: null }).valid, false);
  assert.equal(validateTrackBCommandInput('trackb.release.status', { narrativeId: 'narrative.demo', extra: true }).valid, false);
  assert.equal(validateTrackBCommandInput('trackb.release.status', {}).valid, false);
  const future = { ...approve, decidedAt: '2026-08-28T00:06:00Z' };
  assert.equal(validateTrackBCommandInput('trackb.release.approve', future, { now: fixedNow }).valid, false);
  const invalidCalendar = { ...approve, decidedAt: '2026-02-30T00:00:00Z' };
  assert.equal(validateTrackBCommandInput('trackb.release.approve', invalidCalendar, { now: fixedNow }).valid, false);
  const whitespaceReason = { ...approve, reason: ' \n\t ' };
  assert.equal(validateTrackBCommandInput('trackb.release.approve', whitespaceReason, { now: fixedNow }).valid, false);
  assert.equal(parseTrackBDateTime('2026-02-29T00:00:00Z'), null);
  assert.equal(parseTrackBDateTime('2024-02-29T00:00:00Z')?.epochMilliseconds, Date.parse('2024-02-29T00:00:00Z'));
  assert.equal(TRACK_B_APPROVAL_MAX_FUTURE_MS, 300000);
});

test('Studio framing enforces exact POST body, content type, origin, session, and 8 MiB cap', () => {
  const body = { command: 'trackb.release.approve', input: { narrativeId: 'narrative.demo', candidateHash: `sha256:${'a'.repeat(64)}`, baseDeliverySetHash: 'NONE', decision: 'APPROVE', reason: 'human review', decidedAt: '2026-08-28T00:00:00Z' } };
  const frame = { method: 'POST', path: '/api/execute', contentType: 'application/json', bodyBytes: 100, origin: 'http://127.0.0.1:4173', allowedOrigin: 'http://127.0.0.1:4173', sessionValid: true };
  assert.equal(validateTrackBStudioRequest(body, frame).valid, true);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, method: 'GET' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, path: '/api/review-session' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, contentType: 'text/plain' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, bodyBytes: TRACK_B_STUDIO_MAX_BODY_BYTES + 1 }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, bodyBytes: 1.5 }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, origin: 'http://localhost:4173' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, origin: '' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, allowedOrigin: 'https://example.com' }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, sessionValid: false }).valid, false);
  assert.equal(validateTrackBStudioRequest(body, { ...frame, sessionValid: 'true' }).valid, false);
  const missingFrame = { ...frame };
  delete missingFrame.method;
  assert.equal(validateTrackBStudioRequest(body, missingFrame).valid, false);
  const bodyOverride = structuredClone(body);
  bodyOverride.input.actorId = 'spoof';
  assert.equal(validateTrackBStudioRequest(bodyOverride, frame).valid, false);
  assert.equal(TRACK_B_STUDIO_ROUTE.path, '/api/execute');
  assert.equal(TRACK_B_STUDIO_ROUTE.sessionBootstrap.path, '/api/review-session');
});

test('status pointer and candidateStatus are strict discriminated variants without null synthesis', () => {
  const hash = `sha256:${'a'.repeat(64)}`;
  const qualificationHash = `sha256:${'b'.repeat(64)}`;
  const absent = { narrativeId: 'narrative.demo', pointer: { state: 'ABSENT' } };
  const withoutCandidate = { narrativeId: 'narrative.demo' };
  const withCandidate = { narrativeId: 'narrative.demo', candidateHash: hash };
  assert.equal(validateTrackBStatusResult(absent).valid, false);
  assert.equal(validateTrackBStatusResult(absent, { input: withoutCandidate }).valid, true);
  const promoted = { narrativeId: 'narrative.demo', pointer: { state: 'PROMOTED', currentDeliverySetHash: hash, candidateHash: hash, approvalHash: hash, meaningQualificationHash: qualificationHash, pointerRevision: 1, pointerHash: hash }, candidateStatus: { state: 'PROMOTED', candidateHash: hash, baseDeliverySetHash: 'NONE', meaningQualificationHash: qualificationHash, candidateVerdict: 'PROMOTE', deliverySetHash: hash, approvalHash: hash } };
  assert.equal(validateTrackBStatusResult(promoted, { input: withCandidate }).valid, true);
  const narrativeMismatch = { ...promoted, narrativeId: 'narrative.other' };
  const directMismatch = validateTrackBStatusResult(narrativeMismatch, { input: withCandidate });
  assert.equal(directMismatch.valid, false);
  assert.ok(directMismatch.errors.some((error) => error.code === 'trackb-narrative-mismatch'));
  const nonePointer = { narrativeId: 'narrative.demo', pointer: { state: 'ABSENT', currentDeliverySetHash: 'NONE' } };
  assert.equal(validateTrackBStatusResult(nonePointer, { input: withoutCandidate }).valid, false);
  const wrongDecision = { ...promoted, candidateStatus: { state: 'APPROVED', candidateHash: hash, baseDeliverySetHash: 'NONE', meaningQualificationHash: qualificationHash, candidateVerdict: 'PROMOTE', approvalHash: hash, decision: 'REVISE' } };
  assert.equal(validateTrackBStatusResult(wrongDecision, { input: withCandidate }).valid, false);
  const missingCandidateStatus = structuredClone(promoted);
  delete missingCandidateStatus.candidateStatus;
  assert.equal(validateTrackBStatusResult(missingCandidateStatus, { input: withCandidate }).valid, false);
  const forbiddenCandidateStatus = structuredClone(promoted);
  assert.equal(validateTrackBStatusResult(forbiddenCandidateStatus, { input: withoutCandidate }).valid, false);
  const mismatchedCandidateStatus = structuredClone(promoted);
  mismatchedCandidateStatus.candidateStatus.candidateHash = `sha256:${'b'.repeat(64)}`;
  assert.equal(validateTrackBStatusResult(mismatchedCandidateStatus, { input: withCandidate }).valid, false);
  markNegativeFixtures('approval-status');
  const wrongVerdict = { ...promoted, candidateStatus: { state: 'PROMOTED', candidateHash: hash, baseDeliverySetHash: 'NONE', meaningQualificationHash: qualificationHash, candidateVerdict: 'ITERATE', deliverySetHash: hash, approvalHash: hash } };
  assert.equal(validateTrackBStatusResult(wrongVerdict, { input: withCandidate }).valid, false);
  assert.throws(() => assertTrackBStatusResult(nonePointer, { input: withoutCandidate }), /required|unsupported|strict/i);
});

test('application envelope validator preserves flat result/error shape', () => {
  const ok = { schemaVersion: 1, ok: true, command: 'trackb.release.status', metadata: {}, result: { narrativeId: 'narrative.demo', pointer: { state: 'ABSENT' } } };
  const input = { narrativeId: 'narrative.demo' };
  assert.equal(validateTrackBApplicationEnvelope(ok, { command: 'trackb.release.status', input }).valid, true);
  assert.equal(validateTrackBApplicationEnvelope(ok, { command: 'trackb.release.status' }).valid, false);
  const nested = { ...ok, result: { result: ok.result } };
  assert.equal(validateTrackBApplicationEnvelope(nested, { command: 'trackb.release.status', input }).valid, false);
  const bad = { schemaVersion: 1, ok: false, command: 'trackb.release.status', metadata: {}, error: { code: 'trackb-input-invalid', message: 'bad', retryable: false } };
  assert.equal(validateTrackBApplicationEnvelope(bad, { command: 'trackb.release.status', input }).valid, true);
  assert.equal(validateTrackBApplicationEnvelope(bad, { command: 'trackb.release.status' }).valid, false);
  assert.equal(validateTrackBApplicationEnvelope(ok, { command: 'trackb.release.status', input }).valid, true);
  assert.equal(validateTrackBApplicationEnvelope(ok, { command: 'trackb.release.status' }).valid, false);
  assert.equal(validateTrackBStatusResult(ok.result, { input }).valid, true);
  assert.equal(validateTrackBStatusResult(ok.result).valid, false);
  const mismatchEnvelope = { ...ok, result: { ...ok.result, narrativeId: 'narrative.other' } };
  for (const result of [
    validateTrackBApplicationEnvelope(mismatchEnvelope, { command: 'trackb.release.status', input }),
    validateTrackBApplicationEnvelope(mismatchEnvelope, { command: 'trackb.release.status', input }),
    validateTrackBStatusResult(mismatchEnvelope.result, { input })
  ]) {
    assert.equal(result.valid, false);
    assert.ok(result.errors.some((error) => error.code === 'trackb-narrative-mismatch'));
  }
});

test('command result declarations reject nulls, nested result wrappers, and invalid status states', () => {
  const valid = { narrativeId: 'narrative.demo', pointer: { state: 'ABSENT' } };
  const input = { narrativeId: 'narrative.demo' };
  assert.equal(validateTrackBCommandResult('trackb.release.status', valid).valid, false);
  assert.equal(validateTrackBCommandResult('trackb.release.status', valid, { input }).valid, true);
  const mismatch = { ...valid, narrativeId: 'narrative.other' };
  const mismatchResult = validateTrackBCommandResult('trackb.release.status', mismatch, { input });
  assert.equal(mismatchResult.valid, false);
  assert.ok(mismatchResult.errors.some((error) => error.code === 'trackb-narrative-mismatch'));
  assert.equal(validateTrackBCommandResult('trackb.release.status', { ...valid, pointer: null }, { input }).valid, false);
  assert.equal(validateTrackBCommandResult('trackb.release.status', { ...valid, result: valid }, { input }).valid, false);
});

test('primitive helpers reject NONE outside base fields and unsafe paths', () => {
  assert.equal(isTrackBBaseHash('NONE'), true);
  assert.equal(isTrackBBaseHash(`sha256:${'a'.repeat(64)}`), true);
  assert.equal(isTrackBBaseHash('none'), false);
  for (const path of ['../secret', '/absolute', 'a\\b', 'a//b', 'a/./b', 'a/../b', '']) assert.equal(isTrackBSafeRelativePath(path), false, path);
  assert.equal(isTrackBSafeRelativePath('scenes/trackb/narrative.demo/file.json'), true);
});

test('every negative fixture executes a deterministic mutation and expected error code', async () => {
  const values = await loadExamples();
  const loaded = await registry();
  const artifactRegistries = await loadTrackBArtifactContractRegistries({ smcSchemaDir: 'contracts/schemas', sceneAssetCompilerSchemaDir: 'contracts/schemas/scene-asset-compiler' });
  assert.ok(Array.isArray(NEGATIVE_FIXTURES.cases) && NEGATIVE_FIXTURES.cases.length > 0);
  for (const item of NEGATIVE_FIXTURES.cases) {
    assert.equal(typeof item.target, 'string', `${item.name} target`);
    assert.equal(typeof item.mutation, 'string', `${item.name} mutation`);
    assert.match(item.expectedErrorCode, /^trackb-[a-z-]+$/i, `${item.name} expectedErrorCode`);
    const result = await executeNegativeFixture(item, values, loaded, artifactRegistries);
    assert.equal(result.valid, false, `${item.name} mutation unexpectedly validated`);
    assert.ok(result.errors.some((error) => error.code === item.expectedErrorCode), `${item.name}: expected ${item.expectedErrorCode}, observed ${JSON.stringify(result.errors)}`);
    markNegativeFixtures(item.name);
  }
});

test('negative fixture manifest is wired to executable assertions', () => {
  assert.equal(NEGATIVE_FIXTURES.schemaVersion, 1);
  assert.ok(Array.isArray(NEGATIVE_FIXTURES.cases) && NEGATIVE_FIXTURES.cases.length > 0);
  const names = NEGATIVE_FIXTURES.cases.map((item) => item.name);
  assert.equal(new Set(names).size, names.length);
  const unexecuted = names.filter((name) => !EXECUTED_NEGATIVE_FIXTURES.has(name));
  assert.deepEqual(unexecuted, []);
});
