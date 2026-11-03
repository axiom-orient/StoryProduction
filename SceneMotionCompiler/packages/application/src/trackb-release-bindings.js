/**
 * Pure Track B cross-artifact binding and error normalization rules.
 * These functions compare producer-owned bytes/claims; they never read or
 * publish project state.
 */
import { ApplicationCommandError } from './command-error.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';
import {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  deriveTrackBMeaningQualification,
  hashTrackBRawBytes,
  isTrackBHash,
  isTrackBNarrativeId,
  TRACK_B_PROVENANCE_REQUIRED_KEYS,
  validateTrackBMeasuredEvidenceReceipt
} from '../../contracts/src/trackb-contract.js';
import { canonicalHash, canonicalizeJson } from '../../core/src/canonical.js';

const TRACK_B_RUNTIME_ERROR_CODES = new Set([
  'trackb-input-invalid',
  'trackb-narrative-mismatch',
  'trackb-authority-missing',
  'trackb-candidate-invalid',
  'trackb-evidence-incomplete',
  'trackb-meaning-unsupported',
  'trackb-artifact-binding-invalid',
  'trackb-storage-integrity',
  'trackb-upstream-failed'
]);

export const AUTHORITY_VALIDATION_CODES = new Set([
  'trackb-narrative-mismatch',
  'trackb-input-invalid'
]);

export const CANDIDATE_VALIDATION_CODES = new Set([
  'trackb-candidate-invalid',
  'trackb-evidence-incomplete',
  'trackb-meaning-unsupported',
  'trackb-artifact-binding-invalid',
  'trackb-narrative-mismatch',
  'trackb-input-invalid'
]);

export function validateTrackBProjectionGraphBinding(projection, sceneGraph) {
  const errors = [];
  if (!isPlainObject(projection)) {
    errors.push(bindingValidationError('/', 'projection receipt must be an object'));
    return { valid: false, errors };
  }
  if (!isPlainObject(sceneGraph) || !Array.isArray(sceneGraph.shots)) {
    errors.push(bindingValidationError('/sceneGraph/shots', 'SceneGraph shots are required for projection binding'));
    return { valid: false, errors };
  }
  const graphSceneIds = [...new Set(sceneGraph.shots.map((shot) => shot?.sourceSceneId))];
  if (projection.sceneCount !== graphSceneIds.length || projection.shotCount !== sceneGraph.shots.length
    || !Array.isArray(projection.sceneIds)
    || projection.sceneIds.length !== graphSceneIds.length
    || projection.sceneIds.some((id, index) => id !== graphSceneIds[index])) {
    errors.push(bindingValidationError('/', 'projection receipt scene identity/order does not match SceneGraph'));
  }
  if (!Array.isArray(projection.sceneToShot) || projection.sceneToShot.length !== sceneGraph.shots.length) {
    errors.push(bindingValidationError('/sceneToShot', 'projection receipt sceneToShot count does not match SceneGraph'));
  } else {
    for (const [index, mapping] of projection.sceneToShot.entries()) {
      const shot = sceneGraph.shots[index];
      if (!isPlainObject(mapping)
        || mapping.sceneId !== shot?.sourceSceneId
        || mapping.shotId !== shot?.id
        || mapping.order !== index + 1) {
        errors.push(bindingValidationError(`/sceneToShot/${index}`, 'projection receipt sceneToShot mapping does not match SceneGraph'));
      }
    }
  }
  return { valid: errors.length === 0, errors };
}

export function assertProjectionGraphBinding(projection, sceneGraph) {
  const result = validateTrackBProjectionGraphBinding(projection, sceneGraph);
  if (result.valid) return;
  const first = result.errors[0];
  throw trackBError(first.code, first.message, { details: { instancePath: first.instancePath } });
}

export function deriveMeaningQualification({ authority, projectionReceipt, projectionReceiptBytes, sceneGraph, provenance, htmlBytes }) {
  const result = deriveTrackBMeaningQualification({ authority, projectionReceipt, projectionReceiptBytes, sceneGraph, provenance, htmlBytes });
  if (result.valid !== true) {
    throw trackBError('trackb-evidence-incomplete', `Track B semantic qualification failed: ${boundedMessage(result.error ?? 'semantic occurrence bijection is invalid')}`);
  }
  return result;
}

export function validateTrackBProvenanceBinding(provenance, authority, projectionReceiptHash, sceneGraphHash, htmlHash, handoff, handoffHash, sceneGraph) {
  const errors = [];
  if (!isPlainObject(provenance)) {
    errors.push(bindingValidationError('/provenance', 'provenance must be an object'));
    return { valid: false, errors };
  }
  if (!isPlainObject(authority)) errors.push(bindingValidationError('/authority', 'NarrativeAuthority is required for provenance binding'));
  if (!isPlainObject(handoff)) errors.push(bindingValidationError('/handoff', 'SceneMotionAssetHandoff is required for provenance binding'));
  if (Object.hasOwn(provenance, 'authorityHash')) errors.push(bindingValidationError('/provenance/authorityHash', 'provenance v2 must not contain authorityHash'));
  for (const field of TRACK_B_PROVENANCE_REQUIRED_KEYS) {
    if (!Object.hasOwn(provenance, field)) errors.push(bindingValidationError(`/provenance/${field}`, `provenance ${field} is required`));
  }
  const actualKeys = Object.keys(provenance).sort();
  const expectedKeys = [...TRACK_B_PROVENANCE_REQUIRED_KEYS].sort();
  if (actualKeys.length !== expectedKeys.length || actualKeys.some((key, index) => key !== expectedKeys[index])) {
    errors.push(bindingValidationError('/provenance', 'provenance v2 contains unsupported or missing fields'));
  }
  if (provenance.schemaVersion !== 2) errors.push(bindingValidationError('/provenance/schemaVersion', 'provenance schemaVersion must be 2'));
  if (!isTrackBNarrativeId(provenance.narrativeId)) {
    errors.push(bindingValidationError('/provenance/narrativeId', 'provenance narrativeId is invalid'));
  } else if (isPlainObject(authority) && provenance.narrativeId !== authority.narrativeId) {
    errors.push({ code: 'trackb-narrative-mismatch', instancePath: '/provenance/narrativeId', message: 'provenance narrativeId differs from NarrativeAuthority' });
  }
  for (const [field, expected] of [
    ['projectionReceiptHash', projectionReceiptHash],
    ['sceneGraphHash', sceneGraphHash],
    ['htmlHash', htmlHash],
    ['sourceSemanticManifestHash', sceneGraph?.semanticManifest?.sourceSemanticManifestHash],
    ['semanticManifestHash', sceneGraph?.semanticManifest?.semanticManifestHash],
    ['handoffHash', handoffHash]
  ]) {
    if (!isTrackBHash(provenance[field])) errors.push(bindingValidationError(`/provenance/${field}`, `provenance ${field} must be a hash`));
    if (expected !== undefined && provenance[field] !== expected) errors.push(bindingValidationError(`/provenance/${field}`, `provenance ${field} does not match its bound artifact`));
  }
  if (provenance.sourceSemanticManifestHash === provenance.semanticManifestHash) errors.push(bindingValidationError('/provenance', 'source and normalized semantic manifest hashes must differ'));
  if (!isPlainObject(provenance.embedding)) errors.push(bindingValidationError('/provenance/embedding', 'provenance embedding is required'));
  else {
    const embeddingKeys = Object.keys(provenance.embedding).sort();
    if (embeddingKeys.join(',') !== 'anchor,encoding,payloadHash') errors.push(bindingValidationError('/provenance/embedding', 'provenance embedding contains unsupported or missing fields'));
    if (provenance.embedding.anchor !== TRACKB_SEMANTIC_MANIFEST_ANCHOR) errors.push(bindingValidationError('/provenance/embedding/anchor', 'provenance embedding anchor is invalid'));
    if (provenance.embedding.encoding !== TRACKB_SEMANTIC_MANIFEST_ENCODING) errors.push(bindingValidationError('/provenance/embedding/encoding', 'provenance embedding encoding is invalid'));
    if (!isTrackBHash(provenance.embedding.payloadHash)) errors.push(bindingValidationError('/provenance/embedding/payloadHash', 'provenance embedding payloadHash is invalid'));
    if (sceneGraph?.semanticManifest) {
      const expectedPayloadHash = hashTrackBRawBytes(Buffer.from(canonicalizeJson(sceneGraph.semanticManifest), 'utf8'));
      if (provenance.embedding.payloadHash !== expectedPayloadHash) errors.push(bindingValidationError('/provenance/embedding/payloadHash', 'provenance embedding payloadHash does not match SceneGraph semantic manifest', 'trackb-evidence-incomplete'));
    }
  }
  if (!isTrackBHash(provenance.provenanceHash)) {
    errors.push(bindingValidationError('/provenance/provenanceHash', 'provenance provenanceHash must be a hash'));
  } else {
    try {
      const withoutSelf = { ...provenance };
      delete withoutSelf.provenanceHash;
      if (canonicalHash(withoutSelf) !== provenance.provenanceHash) {
        errors.push(bindingValidationError('/provenance/provenanceHash', 'provenance provenanceHash does not match its canonical self-hash'));
      }
    } catch (error) {
      errors.push(bindingValidationError('/provenance/provenanceHash', `provenance canonical hash could not be computed: ${boundedMessage(error.message)}`));
    }
  }
  if (!isTrackBHash(provenance.assetManifestHash)) errors.push(bindingValidationError('/provenance/assetManifestHash', 'provenance assetManifestHash is required to bind asset evidence'));
  if (isPlainObject(handoff)) {
    if (handoff.schemaVersion !== 2) errors.push(bindingValidationError('/handoff/schemaVersion', 'SceneAssetCompiler handoff schemaVersion must be 2'));
    if (provenance.projectionReceiptHash !== `sha256:${handoff.projectionReceiptSha256}`) {
      errors.push(bindingValidationError('/provenance/projectionReceiptHash', 'provenance projectionReceiptHash does not equal handoff projectionReceiptSha256'));
    }
    if (provenance.sceneGraphHash !== handoff.sceneGraphHash) {
      errors.push(bindingValidationError('/provenance/sceneGraphHash', 'provenance sceneGraphHash does not equal handoff sceneGraphHash'));
    }
    if (sceneGraph?.semanticManifest
      && (handoff.sourceSemanticManifestHash !== sceneGraph.semanticManifest.sourceSemanticManifestHash
        || handoff.semanticManifestHash !== sceneGraph.semanticManifest.semanticManifestHash)) {
      errors.push(bindingValidationError('/handoff', 'SceneAssetCompiler handoff semantic manifest hashes do not match SceneGraph'));
    }
  }
  return { valid: errors.length === 0, errors };
}

export function assertProvenanceBinding(provenance, authority, projectionReceiptHash, sceneGraphHash, htmlHash, handoff, handoffHash, sceneGraph) {
  const result = validateTrackBProvenanceBinding(provenance, authority, projectionReceiptHash, sceneGraphHash, htmlHash, handoff, handoffHash, sceneGraph);
  if (result.valid) return;
  const first = result.errors[0];
  throw trackBError(first.code, first.message, { details: { instancePath: first.instancePath } });
}

export function assertMeasuredEvidenceBinding(receipt, receiptBytes, authority, provenance, htmlHash, sceneGraph, registry) {
  const validation = validateTrackBMeasuredEvidenceReceipt(receipt, { registry });
  if (validation.valid !== true) {
    const first = validation.errors?.[0];
    throw trackBError(first?.code ?? 'trackb-evidence-incomplete', first?.message ?? 'measured evidence receipt is invalid');
  }
  const receiptText = Buffer.from(receiptBytes).toString('utf8');
  assertTrackBBinding(canonicalizeJson(receipt) === receiptText, 'trackb-evidence-incomplete', 'measured evidence receipt bytes must be canonical JSON');
  assertTrackBBinding(receipt.narrativeId === authority.narrativeId, 'trackb-narrative-mismatch', 'measured evidence receipt narrativeId differs from authority');
  assertTrackBBinding(receipt.authorityHash === authority.authorityHash, 'trackb-evidence-incomplete', 'measured evidence receipt authorityHash differs from authority');
  for (const field of ['htmlHash', 'assetManifestHash', 'semanticManifestHash']) {
    assertTrackBBinding(receipt[field] === provenance[field], 'trackb-evidence-incomplete', `measured evidence receipt ${field} differs from provenance`);
  }
  assertTrackBBinding(receipt.htmlHash === htmlHash, 'trackb-evidence-incomplete', 'measured evidence receipt htmlHash differs from HTML bytes');
  assertTrackBBinding(receipt.semanticManifestHash === sceneGraph.semanticManifest?.semanticManifestHash, 'trackb-evidence-incomplete', 'measured evidence receipt semanticManifestHash differs from SceneGraph');
  const expectedShotIds = authority.shots.map((shot) => shot.shotId);
  assertTrackBBinding(canonicalizeJson(receipt.shotIds) === canonicalizeJson(expectedShotIds), 'trackb-evidence-incomplete', 'measured evidence receipt shot order differs from authority');
}

export function assertRequiredArtifactContext(context) {
  const required = ['registry', 'authority', 'projectionReceipt', 'projectionReceiptBytes', 'sceneGraph', 'handoff', 'handoffBytes', 'htmlBytes', 'provenance', 'evidenceReceipt', 'evidenceReceiptBytes', 'artifactRegistries'];
  for (const key of required) {
    if (context[key] === undefined || context[key] === null) throw trackBError('trackb-artifact-binding-invalid', `Track B artifact context is missing ${key}`);
  }
}

export function invokeValidator(callback, fallbackCode, label) {
  try {
    return callback();
  } catch (error) {
    throw mappedError(error, fallbackCode, `${label} validation could not run`);
  }
}

export function throwValidationIfInvalid(result, fallbackCode, label, preserveCodes = new Set()) {
  if (result?.valid === true) return;
  const first = result?.errors?.[0];
  const candidateCode = first?.code;
  const code = preserveCodes.has(candidateCode) ? candidateCode : fallbackCode;
  throw trackBError(code, `${label} validation failed: ${boundedMessage(first?.message ?? 'invalid artifact')}`, {
    details: first?.instancePath ? { instancePath: first.instancePath } : undefined
  });
}

export function assertTrackBInput(condition, message) {
  if (!condition) throw trackBError('trackb-input-invalid', message);
}

export function assertTrackBBinding(condition, code, message) {
  if (!condition) throw trackBError(code, message);
}

function bindingValidationError(instancePath, message, code = 'trackb-artifact-binding-invalid') {
  return { code, instancePath, message };
}

export function trackBError(code, message, options = {}) {
  return new ApplicationCommandError(
    normalizeRuntimeErrorCode(code, 'trackb-upstream-failed'),
    boundedMessage(message),
    { ...options, retryable: false }
  );
}

export function mappedError(error, fallbackCode, message, options = {}) {
  if (error instanceof ApplicationCommandError && TRACK_B_RUNTIME_ERROR_CODES.has(error.code)) return error;
  const code = normalizeRuntimeErrorCode(error?.code, fallbackCode);
  return trackBError(code, `${message}: ${boundedMessage(error?.message ?? 'unknown failure')}`, {
    ...options,
    cause: error,
    details: options.details
  });
}

export function normalizeRuntimeErrorCode(code, fallbackCode) {
  return TRACK_B_RUNTIME_ERROR_CODES.has(code) ? code : fallbackCode;
}

export function isRegistryLike(value) {
  return value !== null && typeof value === 'object' && typeof value.schema === 'function' && typeof value.validate === 'function';
}

export function isByteSequence(value) {
  return typeof value === 'string' || Buffer.isBuffer(value) || value instanceof Uint8Array;
}

export function assertNarrativeSourceBinding(authority, projection) {
  for (const [key, field] of [
    ['nvcReleaseBundleHash', 'bundleSha256'],
    ['narrativePackageHash', 'packageSha256'],
    ['narrativeSceneMapHash', 'sceneMapSha256']
  ]) {
    assertTrackBBinding(authority.sourceRefs[key] === (key === 'nvcReleaseBundleHash' ? projection[field] : `sha256:${projection[field]}`),
      'trackb-artifact-binding-invalid', `NarrativeAuthority ${key} does not match projection ${field}`);
  }
}
