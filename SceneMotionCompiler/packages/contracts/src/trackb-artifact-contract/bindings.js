import { canonicalHash, canonicalizeJson } from '../../../core/src/canonical.js';
import {
  assertResult,
  contractError,
  exactKeys,
  hashCanonicalWithoutSelf,
  hashTrackBRawBytes,
  isPlainObject,
  isTrackBHash,
  isTrackBNarrativeId,
  resultWithErrors
} from '../trackb-contract-primitives.js';
import { validateTrackBArtifactSchemas } from '../trackb-artifact-schemas.js';
import {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  deriveTrackBMeaningQualification,
  parseTrackBSemanticManifestHtml
} from '../trackb-semantic-manifest-contract.js';
import { validateTrackBNarrativeAuthority, validateTrackBMeasuredEvidenceReceipt } from './authority.js';
import { validateTrackBProjectionReceiptShape } from './projection.js';
import { validateSchemaValue } from './schema.js';

export const TRACK_B_PROVENANCE_REQUIRED_KEYS = Object.freeze([
  'schemaVersion',
  'narrativeId',
  'projectionReceiptHash',
  'sceneGraphHash',
  'sourceSemanticManifestHash',
  'semanticManifestHash',
  'htmlHash',
  'embedding',
  'handoffHash',
  'assetManifestHash',
  'provenanceHash'
]);

export function validateTrackBArtifactBindings(candidate, {
  registry,
  authority,
  projectionReceipt,
  projectionReceiptBytes,
  sceneGraph,
  handoff,
  handoffBytes,
  htmlBytes,
  provenance,
  evidenceReceipt,
  evidenceReceiptBytes,
  artifactRegistries,
  registries,
  smcRegistry,
  sceneAssetCompilerRegistry
} = {}) {
  const errors = [];
  if (!isPlainObject(candidate)) return resultWithErrors([contractError('/', 'candidate must be an object', 'trackb-artifact-binding-invalid')]);
  const artifacts = candidate.artifacts;
  if (!isPlainObject(artifacts)) return resultWithErrors([contractError('/artifacts', 'candidate artifacts are required', 'trackb-artifact-binding-invalid')]);

  const suppliedRegistries = artifactRegistries ?? registries;
  const resolvedSmcRegistry = smcRegistry ?? suppliedRegistries?.smcRegistry ?? suppliedRegistries?.smc;
  const resolvedSceneAssetCompilerRegistry = sceneAssetCompilerRegistry ?? suppliedRegistries?.sceneAssetCompilerRegistry;
  for (const [path, value, label] of [
    ['/registry', registry, 'Track B'],
    ['/authority', authority, 'semantic authority'],
    ['/projectionReceipt', projectionReceipt, 'parsed projection receipt'],
    ['/projectionReceiptBytes', projectionReceiptBytes, 'projection receipt raw bytes'],
    ['/sceneGraph', sceneGraph, 'SceneGraph'],
    ['/handoff', handoff, 'parsed asset handoff'],
    ['/handoffBytes', handoffBytes, 'asset handoff raw bytes'],
    ['/htmlBytes', htmlBytes, 'HTML raw bytes'],
    ['/provenance', provenance, 'provenance'],
    ['/evidenceReceipt', evidenceReceipt, 'measured evidence receipt'],
    ['/evidenceReceiptBytes', evidenceReceiptBytes, 'measured evidence receipt raw bytes'],
    ['/artifactRegistries/smcRegistry', resolvedSmcRegistry, 'SMC artifact schema registry'],
    ['/artifactRegistries/sceneAssetCompilerRegistry', resolvedSceneAssetCompilerRegistry, 'SceneAssetCompiler artifact schema registry']
  ]) {
    if (value === undefined) errors.push(contractError(path, `${label} is required for Track B artifact binding`, 'trackb-artifact-binding-invalid'));
  }
  if (!isTrackBContractRegistryLike(registry)) errors.push(contractError('/registry', 'a loaded Track B registry is required for artifact binding', 'trackb-artifact-binding-invalid'));
  if (!isTrackBRegistryLike(resolvedSmcRegistry)) errors.push(contractError('/artifactRegistries/smcRegistry', 'a loaded SMC artifact schema registry is required', 'trackb-artifact-binding-invalid'));
  if (!isTrackBRegistryLike(resolvedSceneAssetCompilerRegistry)) errors.push(contractError('/artifactRegistries/sceneAssetCompilerRegistry', 'a loaded SceneAssetCompiler artifact schema registry is required', 'trackb-artifact-binding-invalid'));

  if (isTrackBContractRegistryLike(registry) && authority !== undefined) {
    const authorityResult = validateTrackBNarrativeAuthority(authority, { registry });
    errors.push(...authorityResult.errors);
  }
  const upstreamSchemas = validateTrackBArtifactSchemas({
    projectionReceipt,
    sceneGraph,
    handoff,
    artifactRegistries: { smcRegistry: resolvedSmcRegistry, sceneAssetCompilerRegistry: resolvedSceneAssetCompilerRegistry }
  });
  errors.push(...upstreamSchemas.errors);
  if (projectionReceipt === undefined && projectionReceiptBytes !== undefined) errors.push(contractError('/projectionReceipt', 'parsed projection receipt is required before raw-byte binding', 'trackb-artifact-binding-invalid'));
  if (handoff === undefined && handoffBytes !== undefined) errors.push(contractError('/handoff', 'parsed asset handoff is required before raw-byte binding', 'trackb-artifact-binding-invalid'));
  if (projectionReceipt) errors.push(...validateTrackBProjectionReceiptShape(projectionReceipt).errors);
  if (projectionReceiptBytes !== undefined) {
    try {
      const actual = hashTrackBRawBytes(projectionReceiptBytes);
      if (actual !== artifacts.projectionReceiptHash) errors.push(contractError('/artifacts/projectionReceiptHash', 'projection receipt raw-byte hash mismatch', 'trackb-artifact-binding-invalid'));
      if (projectionReceipt !== undefined) errors.push(...validateTrackBParsedRawJsonPair(projectionReceipt, projectionReceiptBytes, '/projectionReceiptBytes', 'projection receipt', 'trackb-artifact-binding-invalid'));
    } catch (error) {
      errors.push(contractError('/projectionReceiptBytes', `projection receipt raw bytes are invalid: ${error.message}`, 'trackb-artifact-binding-invalid'));
    }
  }
  if (authority) {
    if (candidate.narrativeId !== authority.narrativeId) errors.push(contractError('/narrativeId', 'candidate and authority narrativeId differ', 'trackb-narrative-mismatch'));
    if (candidate.authorityHash !== authority.authorityHash) errors.push(contractError('/authorityHash', 'candidate and authority hash differ', 'trackb-artifact-binding-invalid'));
    if (artifacts.projectionReceiptHash !== authority.sourceRefs?.projectionReceiptHash) errors.push(contractError('/artifacts/projectionReceiptHash', 'candidate projection receipt hash is not authority-bound', 'trackb-artifact-binding-invalid'));
    if (artifacts.sceneGraphHash !== authority.sourceRefs?.sceneGraphHash) errors.push(contractError('/artifacts/sceneGraphHash', 'candidate scene graph hash is not authority-bound', 'trackb-artifact-binding-invalid'));
    if (candidate.meaningQualificationHash !== undefined && !isTrackBHash(candidate.meaningQualificationHash)) errors.push(contractError('/meaningQualificationHash', 'candidate meaningQualificationHash is invalid', 'trackb-evidence-incomplete'));
  }
  if (projectionReceipt) {
    if (projectionReceipt.narrativeId !== undefined && projectionReceipt.narrativeId !== candidate.narrativeId) errors.push(contractError('/projectionReceipt/narrativeId', 'projection receipt narrativeId differs', 'trackb-narrative-mismatch'));
    if (projectionReceipt.sceneGraphHash !== undefined && projectionReceipt.sceneGraphHash !== artifacts.sceneGraphHash) errors.push(contractError('/projectionReceipt/sceneGraphHash', 'projection receipt sceneGraphHash differs', 'trackb-artifact-binding-invalid'));
    if (projectionReceipt.sourceSemanticManifestHash !== authority?.sourceRefs?.sourceSemanticManifestHash) errors.push(contractError('/projectionReceipt/sourceSemanticManifestHash', 'projection receipt source semantic manifest hash differs', 'trackb-artifact-binding-invalid'));
    if (projectionReceipt.semanticManifestHash !== authority?.sourceRefs?.semanticManifestHash) errors.push(contractError('/projectionReceipt/semanticManifestHash', 'projection receipt semantic manifest hash differs', 'trackb-artifact-binding-invalid'));
  }
  if (projectionReceipt && authority) {
    for (const [key, field] of [['nvcReleaseBundleHash', 'bundleSha256'], ['narrativePackageHash', 'packageSha256'], ['narrativeSceneMapHash', 'sceneMapSha256']]) {
      if (authority.sourceRefs?.[key] !== (key === 'nvcReleaseBundleHash' ? projectionReceipt[field] : `sha256:${projectionReceipt[field]}`)) errors.push(contractError(`/authority/sourceRefs/${key}`, 'upstream narrative source hash differs from projection', 'trackb-artifact-binding-invalid'));
    }
  }
  if (sceneGraph && canonicalHash(sceneGraph) !== artifacts.sceneGraphHash) errors.push(contractError('/artifacts/sceneGraphHash', 'scene graph canonical hash mismatch', 'trackb-artifact-binding-invalid'));
  if (handoff) {
    if (handoff.sceneGraphHash !== artifacts.sceneGraphHash) errors.push(contractError('/artifacts/assetHandoffSceneGraphHash', 'handoff sceneGraphHash differs from candidate', 'trackb-artifact-binding-invalid'));
    if (artifacts.assetHandoffSceneGraphHash !== handoff.sceneGraphHash) errors.push(contractError('/artifacts/assetHandoffSceneGraphHash', 'embedded handoff sceneGraphHash mismatch', 'trackb-artifact-binding-invalid'));
    if (handoff.projectionReceiptSha256 && artifacts.projectionReceiptHash !== `sha256:${handoff.projectionReceiptSha256}`) errors.push(contractError('/artifacts/projectionReceiptHash', 'candidate projection receipt hash differs from handoff raw-byte reference', 'trackb-artifact-binding-invalid'));
    if (handoff.narrativeId !== undefined && handoff.narrativeId !== candidate.narrativeId) errors.push(contractError('/handoff/narrativeId', 'handoff narrativeId differs', 'trackb-narrative-mismatch'));
    if (handoff.sourceSemanticManifestHash !== authority?.sourceRefs?.sourceSemanticManifestHash) errors.push(contractError('/handoff/sourceSemanticManifestHash', 'handoff source semantic manifest hash differs', 'trackb-artifact-binding-invalid'));
    if (handoff.semanticManifestHash !== authority?.sourceRefs?.semanticManifestHash) errors.push(contractError('/handoff/semanticManifestHash', 'handoff semantic manifest hash differs', 'trackb-artifact-binding-invalid'));
  }
  if (handoffBytes !== undefined) {
    try {
      if (hashTrackBRawBytes(handoffBytes) !== artifacts.assetHandoffHash) errors.push(contractError('/artifacts/assetHandoffHash', 'asset handoff raw-byte hash mismatch', 'trackb-artifact-binding-invalid'));
      if (handoff !== undefined) errors.push(...validateTrackBParsedRawJsonPair(handoff, handoffBytes, '/handoffBytes', 'asset handoff', 'trackb-artifact-binding-invalid'));
    } catch (error) {
      errors.push(contractError('/handoffBytes', `asset handoff raw bytes are invalid: ${error.message}`, 'trackb-artifact-binding-invalid'));
    }
  }
  if (evidenceReceipt !== undefined) {
    if (isTrackBContractRegistryLike(registry)) {
      const receiptResult = validateTrackBMeasuredEvidenceReceipt(evidenceReceipt, { registry });
      errors.push(...receiptResult.errors);
    } else {
      errors.push(contractError('/evidenceReceipt', 'a loaded Track B registry is required to validate the measured evidence receipt', 'trackb-artifact-binding-invalid'));
    }
    if (candidate.schemaVersion === 4 && candidate.evidenceReceiptHash !== evidenceReceipt.receiptHash) {
      errors.push(contractError('/evidenceReceiptHash', 'candidate evidenceReceiptHash does not match the measured evidence receipt', 'trackb-evidence-incomplete'));
    }
    if (authority && evidenceReceipt.narrativeId !== authority.narrativeId) {
      errors.push(contractError('/evidenceReceipt/narrativeId', 'measured evidence receipt narrativeId differs from authority', 'trackb-narrative-mismatch'));
    }
    if (authority && evidenceReceipt.authorityHash !== authority.authorityHash) {
      errors.push(contractError('/evidenceReceipt/authorityHash', 'measured evidence receipt authorityHash differs from authority', 'trackb-evidence-incomplete'));
    }
    if (provenance) {
      for (const field of ['htmlHash', 'assetManifestHash', 'semanticManifestHash']) {
        if (evidenceReceipt[field] !== provenance[field]) errors.push(contractError(`/evidenceReceipt/${field}`, `measured evidence receipt ${field} differs from provenance`, 'trackb-evidence-incomplete'));
      }
    }
    if (authority && Array.isArray(authority.shots)) {
      const expectedShotIds = authority.shots.map((shot) => shot.shotId);
      if (canonicalizeJson(evidenceReceipt.shotIds) !== canonicalizeJson(expectedShotIds)) {
        errors.push(contractError('/evidenceReceipt/shotIds', 'measured evidence receipt shot order does not match authority', 'trackb-evidence-incomplete'));
      }
    }
  }
  if (evidenceReceiptBytes !== undefined) {
    try {
      if (evidenceReceipt !== undefined) {
        errors.push(...validateTrackBParsedRawJsonPair(evidenceReceipt, evidenceReceiptBytes, '/evidenceReceiptBytes', 'measured evidence receipt', 'trackb-evidence-incomplete'));
        const rawText = typeof evidenceReceiptBytes === 'string' ? evidenceReceiptBytes : Buffer.from(evidenceReceiptBytes).toString('utf8');
        if (canonicalizeJson(evidenceReceipt) !== rawText) errors.push(contractError('/evidenceReceiptBytes', 'measured evidence receipt bytes must be canonical JSON', 'trackb-evidence-incomplete'));
      }
    } catch (error) {
      errors.push(contractError('/evidenceReceiptBytes', `measured evidence receipt raw bytes are invalid: ${error.message}`, 'trackb-evidence-incomplete'));
    }
  }
  if (htmlBytes !== undefined) {
    try {
      const actual = hashTrackBRawBytes(htmlBytes);
      if (actual !== artifacts.htmlHash) errors.push(contractError('/artifacts/htmlHash', 'HTML raw-byte hash mismatch', 'trackb-artifact-binding-invalid'));
      if (provenance?.htmlHash !== undefined && provenance.htmlHash !== actual) errors.push(contractError('/provenance/htmlHash', 'provenance htmlHash differs from HTML bytes', 'trackb-artifact-binding-invalid'));
      const embedded = parseTrackBSemanticManifestHtml(htmlBytes);
      if (sceneGraph?.semanticManifest && canonicalizeJson(embedded.semanticManifest) !== canonicalizeJson(sceneGraph.semanticManifest)) {
        errors.push(contractError('/htmlBytes', 'HTML semantic manifest does not equal SceneGraph semanticManifest', 'trackb-evidence-incomplete'));
      }
      if (embedded.semanticManifestHash !== sceneGraph?.semanticManifest?.semanticManifestHash) errors.push(contractError('/htmlBytes', 'HTML semanticManifestHash does not match SceneGraph', 'trackb-evidence-incomplete'));
      if (provenance?.embedding?.payloadHash !== embedded.payloadHash) errors.push(contractError('/provenance/embedding/payloadHash', 'provenance embedding payloadHash differs from HTML', 'trackb-artifact-binding-invalid'));
    } catch (error) {
      errors.push(contractError('/htmlBytes', `HTML semantic manifest is invalid: ${error.message}`, 'trackb-evidence-incomplete'));
    }
  }
  if (provenance) {
    validateTrackBProvenance(provenance, candidate, artifacts, errors, sceneGraph, handoffBytes);
  }
  // This domain is available only after the source, projection, SceneGraph,
  // HTML, measured evidence, and provenance bindings are all present.
  if (authority && sceneGraph?.semanticManifest && provenance && htmlBytes !== undefined) {
    const qualification = deriveTrackBMeaningQualification({ candidate, authority, projectionReceipt, projectionReceiptBytes, sceneGraph, provenance, htmlBytes });
    if (qualification.error) errors.push(contractError('/meaningQualificationHash', qualification.error, 'trackb-evidence-incomplete'));
    else if (candidate.meaningQualificationHash !== qualification.meaningQualificationHash) errors.push(contractError('/meaningQualificationHash', 'candidate meaningQualificationHash does not match the exact semantic qualification', 'trackb-evidence-incomplete'));
  }
  return resultWithErrors(errors);
}

export function assertTrackBArtifactBindings(candidate, options = {}) {
  assertResult(validateTrackBArtifactBindings(candidate, options));
  return candidate;
}

function validateTrackBProvenance(provenance, candidate, artifacts, errors, sceneGraph, handoffBytes) {
  if (!isPlainObject(provenance)) {
    errors.push(contractError('/provenance', 'provenance must be an object', 'trackb-artifact-binding-invalid'));
    return;
  }
  if (Object.hasOwn(provenance, 'authorityHash')) {
    errors.push(contractError('/provenance/authorityHash', 'provenance v2 must not contain authorityHash', 'trackb-artifact-binding-invalid'));
  }
  for (const field of TRACK_B_PROVENANCE_REQUIRED_KEYS) {
    if (!Object.hasOwn(provenance, field)) errors.push(contractError(`/provenance/${field}`, 'provenance field is required', 'trackb-artifact-binding-invalid'));
  }
  exactKeys(provenance, TRACK_B_PROVENANCE_REQUIRED_KEYS, errors, 'trackb-artifact-binding-invalid');
  if (provenance.schemaVersion !== 2) errors.push(contractError('/provenance/schemaVersion', 'provenance schemaVersion must be 2', 'trackb-artifact-binding-invalid'));
  if (!isTrackBNarrativeId(provenance.narrativeId)) errors.push(contractError('/provenance/narrativeId', 'provenance narrativeId is invalid', 'trackb-artifact-binding-invalid'));
  else if (provenance.narrativeId !== candidate.narrativeId) errors.push(contractError('/provenance/narrativeId', 'provenance narrativeId differs', 'trackb-narrative-mismatch'));
  for (const field of ['projectionReceiptHash', 'sceneGraphHash', 'sourceSemanticManifestHash', 'semanticManifestHash', 'htmlHash', 'handoffHash', 'assetManifestHash', 'provenanceHash']) {
    if (!isTrackBHash(provenance[field])) errors.push(contractError(`/provenance/${field}`, `${field} must be a hash`, 'trackb-artifact-binding-invalid'));
  }
  if (provenance.sourceSemanticManifestHash === provenance.semanticManifestHash) errors.push(contractError('/provenance', 'source and normalized semantic manifest hashes must differ', 'trackb-artifact-binding-invalid'));
  if (provenance.projectionReceiptHash !== artifacts.projectionReceiptHash) errors.push(contractError('/provenance/projectionReceiptHash', 'provenance projectionReceiptHash differs from candidate', 'trackb-artifact-binding-invalid'));
  if (provenance.sceneGraphHash !== artifacts.sceneGraphHash) errors.push(contractError('/provenance/sceneGraphHash', 'provenance sceneGraphHash differs from candidate', 'trackb-artifact-binding-invalid'));
  if (provenance.sourceSemanticManifestHash !== sceneGraph?.semanticManifest?.sourceSemanticManifestHash) errors.push(contractError('/provenance/sourceSemanticManifestHash', 'provenance sourceSemanticManifestHash differs from SceneGraph', 'trackb-artifact-binding-invalid'));
  if (provenance.semanticManifestHash !== sceneGraph?.semanticManifest?.semanticManifestHash) errors.push(contractError('/provenance/semanticManifestHash', 'provenance semanticManifestHash differs from SceneGraph', 'trackb-artifact-binding-invalid'));
  if (provenance.htmlHash !== artifacts.htmlHash) errors.push(contractError('/provenance/htmlHash', 'provenance htmlHash differs from candidate', 'trackb-artifact-binding-invalid'));
  if (provenance.handoffHash !== artifacts.assetHandoffHash) errors.push(contractError('/provenance/handoffHash', 'provenance handoffHash differs from candidate handoff bytes', 'trackb-artifact-binding-invalid'));
  if (handoffBytes !== undefined && hashTrackBRawBytes(handoffBytes) !== provenance.handoffHash) errors.push(contractError('/provenance/handoffHash', 'provenance handoffHash does not match raw handoff bytes', 'trackb-artifact-binding-invalid'));
  const expectedEmbeddingKeys = ['anchor', 'encoding', 'payloadHash'];
  if (!isPlainObject(provenance.embedding)) errors.push(contractError('/provenance/embedding', 'provenance embedding is required', 'trackb-artifact-binding-invalid'));
  else {
    exactKeys(provenance.embedding, expectedEmbeddingKeys, errors, 'trackb-artifact-binding-invalid');
    if (provenance.embedding.anchor !== TRACKB_SEMANTIC_MANIFEST_ANCHOR) errors.push(contractError('/provenance/embedding/anchor', 'provenance embedding anchor is invalid', 'trackb-artifact-binding-invalid'));
    if (provenance.embedding.encoding !== TRACKB_SEMANTIC_MANIFEST_ENCODING) errors.push(contractError('/provenance/embedding/encoding', 'provenance embedding encoding is invalid', 'trackb-artifact-binding-invalid'));
    if (!isTrackBHash(provenance.embedding.payloadHash)) errors.push(contractError('/provenance/embedding/payloadHash', 'provenance embedding payloadHash is invalid', 'trackb-artifact-binding-invalid'));
    if (sceneGraph?.semanticManifest) {
      const expectedPayloadHash = hashTrackBRawBytes(Buffer.from(canonicalizeJson(sceneGraph.semanticManifest), 'utf8'));
      if (provenance.embedding.payloadHash !== expectedPayloadHash) errors.push(contractError('/provenance/embedding/payloadHash', 'provenance embedding payloadHash does not match SceneGraph semantic manifest', 'trackb-evidence-incomplete'));
    }
  }
  try {
    const actualProvenanceHash = hashCanonicalWithoutSelf(provenance, 'provenanceHash');
    if (provenance.provenanceHash !== actualProvenanceHash || artifacts.provenanceHash !== actualProvenanceHash) errors.push(contractError('/provenance/provenanceHash', 'provenance canonical hash mismatch', 'trackb-artifact-binding-invalid'));
  } catch (error) {
    errors.push(contractError('/provenance', `provenance canonical hash cannot be computed: ${error.message}`, 'trackb-artifact-binding-invalid'));
  }
}

function isTrackBRegistryLike(value) {
  return value !== null
    && typeof value === 'object'
    && typeof value.schema === 'function'
    && typeof value.validate === 'function';
}

function isTrackBContractRegistryLike(value) {
  return isTrackBRegistryLike(value)
    && typeof value.definition === 'function'
    && Array.isArray(value.definitions)
    && value.definitions.length === 6;
}

function validateTrackBParsedRawJsonPair(parsedValue, rawBytes, path, label, code) {
  if (!(typeof rawBytes === 'string' || Buffer.isBuffer(rawBytes) || rawBytes instanceof Uint8Array)) {
    return [contractError(path, `${label} raw bytes must be a UTF-8 string or byte sequence`, code)];
  }
  try {
    const bytes = typeof rawBytes === 'string' ? Buffer.from(rawBytes, 'utf8') : Buffer.from(rawBytes);
    const decoded = JSON.parse(bytes.toString('utf8'));
    if (canonicalHash(decoded) !== canonicalHash(parsedValue)) {
      return [contractError(path, `${label} raw bytes do not parse to the supplied artifact`, code)];
    }
  } catch (error) {
    return [contractError(path, `${label} raw bytes must contain valid JSON: ${error.message}`, code)];
  }
  return [];
}
