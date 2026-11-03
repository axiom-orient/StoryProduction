import { assertApprovedSceneProjection } from '../../adapters/src/narrative-courier-release.js';
import {
  deriveCandidateVerdict,
  hashTrackBRawBytes,
  hashTrackBEvidenceCell,
  hashTrackBReleaseCandidate,
  isTrackBBaseHash,
  isTrackBOpaqueId,
  validateTrackBNarrativeAuthority,
  validateTrackBProjectionReceiptShape,
  validateTrackBReleaseCandidate
} from '../../contracts/src/trackb-contract.js';
import { validateTrackBArtifactSchemas } from '../../contracts/src/trackb-artifact-schemas.js';
import { canonicalHash, canonicalizeJson } from '../../core/src/canonical.js';
import {
  AUTHORITY_VALIDATION_CODES,
  CANDIDATE_VALIDATION_CODES,
  assertMeasuredEvidenceBinding,
  assertNarrativeSourceBinding,
  assertProjectionGraphBinding,
  assertProvenanceBinding,
  assertRequiredArtifactContext,
  assertTrackBBinding,
  assertTrackBInput,
  deriveMeaningQualification,
  isByteSequence,
  invokeValidator,
  normalizeRuntimeErrorCode,
  throwValidationIfInvalid,
  trackBError,
  validateTrackBProjectionGraphBinding,
  validateTrackBProvenanceBinding
} from './trackb-release-bindings.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';

const TRACK_B_CURRENT_COMPOSER_MEANING_STATUS = 'SUPPORTED';

/**
 * Build and validate an immutable in-memory candidate. This function is pure
 * with respect to durable state: it only hashes supplied bytes/objects and
 * returns canonical candidate bytes. Measured evidence is accepted only as a
 * strict durable receipt whose hash and ordered shots are bound to every
 * upstream artifact before candidate construction.
 */
export function buildTrackBReleaseCandidate({
  candidateId,
  expectedBaseDeliverySetHash,
  authority,
  projectionReceipt,
  projectionReceiptBytes,
  sceneGraph,
  handoff,
  handoffBytes,
  htmlBytes,
  provenance,
  provenanceBytes,
  evidenceReceipt,
  evidenceReceiptBytes,
  registry,
  artifactRegistries
}) {
  assertTrackBInput(isTrackBOpaqueId(candidateId), 'candidateId must be a valid opaque ID');
  assertTrackBInput(isTrackBBaseHash(expectedBaseDeliverySetHash), 'expectedBaseDeliverySetHash must be NONE or a hash');

  const artifactBindings = {
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
    ...(provenanceBytes !== undefined ? { provenanceBytes } : {}),
    artifactRegistries
  };
  assertRequiredArtifactContext(artifactBindings);
  for (const [label, value] of [
    ['NarrativeAuthority', authority],
    ['projection receipt', projectionReceipt],
    ['SceneGraph', sceneGraph],
    ['SceneAssetCompiler handoff', handoff],
    ['provenance', provenance],
    ['measured evidence receipt', evidenceReceipt]
  ]) {
    assertTrackBBinding(isPlainObject(value), 'trackb-artifact-binding-invalid', `${label} must be an object`);
  }
  for (const [label, value] of [
    ['projection receipt', projectionReceiptBytes],
    ['handoff', handoffBytes],
    ['HTML', htmlBytes],
    ['measured evidence receipt', evidenceReceiptBytes]
  ]) {
    assertTrackBBinding(isByteSequence(value), 'trackb-artifact-binding-invalid', `${label} raw bytes are required`);
  }
  const authorityResult = invokeValidator(
    () => validateTrackBNarrativeAuthority(authority, { registry }),
    'trackb-authority-missing',
    'NarrativeAuthority'
  );
  throwValidationIfInvalid(authorityResult, 'trackb-authority-missing', 'NarrativeAuthority', AUTHORITY_VALIDATION_CODES);
  const artifactSchemaResult = invokeValidator(
    () => validateTrackBArtifactSchemas({ projectionReceipt, sceneGraph, handoff, artifactRegistries }),
    'trackb-artifact-binding-invalid',
    'Track B upstream artifact schema'
  );
  throwValidationIfInvalid(artifactSchemaResult, 'trackb-artifact-binding-invalid', 'Track B upstream artifact schema');
  const projectionShape = invokeValidator(
    () => validateTrackBProjectionReceiptShape(projectionReceipt),
    'trackb-artifact-binding-invalid',
    'NarrativeSceneProjection'
  );
  throwValidationIfInvalid(projectionShape, 'trackb-artifact-binding-invalid', 'NarrativeSceneProjection');
  assertNarrativeSourceBinding(authority, projectionReceipt);
  try { assertApprovedSceneProjection(projectionReceipt, sceneGraph); }
  catch (error) { throw trackBError('trackb-artifact-binding-invalid', 'approved cinematic source closure is invalid', { cause: error }); }
  assertProjectionGraphBinding(projectionReceipt, sceneGraph);
  const projectionReceiptHash = hashTrackBRawBytes(projectionReceiptBytes);
  const sceneGraphHash = canonicalHash(sceneGraph);
  const htmlHash = hashTrackBRawBytes(htmlBytes);
  const handoffHash = hashTrackBRawBytes(handoffBytes);
  assertProvenanceBinding(provenance, authority, projectionReceiptHash, sceneGraphHash, htmlHash, handoff, handoffHash, sceneGraph);
  assertMeasuredEvidenceBinding(evidenceReceipt, evidenceReceiptBytes, authority, provenance, htmlHash, sceneGraph, registry);

  const qualification = deriveMeaningQualification({
    authority,
    projectionReceipt,
    projectionReceiptBytes,
    sceneGraph,
    provenance,
    htmlBytes
  });

  const assetManifestHash = provenance.assetManifestHash;

  const artifacts = {
    projectionReceiptHash: hashTrackBRawBytes(projectionReceiptBytes),
    sceneGraphHash: canonicalHash(sceneGraph),
    assetHandoffHash: hashTrackBRawBytes(handoffBytes),
    assetHandoffSceneGraphHash: handoff.sceneGraphHash,
    htmlHash: hashTrackBRawBytes(htmlBytes),
    provenanceHash: provenance.provenanceHash
  };
  const measuredEvidenceByShot = new Map(evidenceReceipt.shots.map((shot) => [shot.shotId, {
    gateVerdict: shot.runtime.pass && shot.visual.pass ? 'PROMOTE' : 'FAIL',
    runtimeEvidence: {
      networkRequestCount: shot.runtime.networkRequestCount,
      exceptionCount: shot.runtime.exceptionCount,
      pass: shot.runtime.pass
    },
    assetEvidence: { assetManifestHash, pass: true },
    visualEvidence: { htmlHash: artifacts.htmlHash, pass: shot.visual.pass }
  }]));
  const meaningByShot = new Map();
  const authorityMeaningById = new Map(authority.meaningItems.map((item) => [item.meaningId, item]));
  for (const qualifiedItem of qualification.qualifiedItems) {
    const meaningItem = authorityMeaningById.get(qualifiedItem.meaningId);
    const items = meaningByShot.get(meaningItem.shotId) ?? [];
    items.push({
      meaningId: meaningItem.meaningId,
      entryHash: qualifiedItem.entryHash,
      status: TRACK_B_CURRENT_COMPOSER_MEANING_STATUS,
      representation: 'METADATA'
    });
    meaningByShot.set(meaningItem.shotId, items);
  }

  const evidenceCells = authority.shots.map((shot) => {
    const measuredEvidence = measuredEvidenceByShot.get(shot.shotId);
    const cell = {
      shotId: shot.shotId,
      gateVerdict: measuredEvidence.gateVerdict,
      meaningCoverage: meaningByShot.get(shot.shotId) ?? [],
      runtimeEvidence: measuredEvidence.runtimeEvidence,
      assetEvidence: measuredEvidence.assetEvidence,
      visualEvidence: measuredEvidence.visualEvidence
    };
    return { ...cell, evidenceCellHash: hashTrackBEvidenceCell(cell) };
  });

  const candidate = {
    schemaVersion: 4,
    candidateId,
    narrativeId: authority.narrativeId,
    authorityHash: authority.authorityHash,
    meaningQualificationHash: qualification.meaningQualificationHash,
    evidenceReceiptHash: evidenceReceipt.receiptHash,
    baseDeliverySetHash: expectedBaseDeliverySetHash,
    artifacts,
    evidenceCells,
    candidateVerdict: 'ITERATE'
  };
  candidate.candidateVerdict = deriveCandidateVerdict(candidate);
  candidate.candidateHash = hashTrackBReleaseCandidate(candidate);

  const candidateValidation = validatePreparedTrackBReleaseCandidate(candidate, artifactBindings);
  throwValidationIfInvalid(candidateValidation, 'trackb-candidate-invalid', 'Track B release candidate', CANDIDATE_VALIDATION_CODES);
  const candidateBytes = Buffer.from(canonicalizeJson(candidate), 'utf8');
  assertTrackBBinding(
    hashTrackBReleaseCandidate(candidate) === candidate.candidateHash,
    'trackb-candidate-invalid',
    'candidate canonical bytes do not reproduce candidateHash'
  );

  return Object.freeze({
    candidate,
    candidateBytes,
    candidateHash: candidate.candidateHash,
    authorityHash: candidate.authorityHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    evidenceReceiptHash: candidate.evidenceReceiptHash,
    baseDeliverySetHash: candidate.baseDeliverySetHash,
    candidateVerdict: candidate.candidateVerdict,
    evidenceCellCount: candidate.evidenceCells.length
  });
}

/**
 * Validate a candidate against the fully loaded artifact context. The result
 * is intentionally the contract validator result, allowing callers/tests to
 * inspect every typed error without mutating storage.
 */
function validatePreparedTrackBReleaseCandidate(candidate, context = {}) {
  const artifactBindings = context.artifactBindings ?? context;
  try {
    const validation = validateTrackBReleaseCandidate(candidate, {
      registry: context.registry ?? artifactBindings.registry,
      authority: context.authority ?? artifactBindings.authority,
      artifactBindings
    });
    const projectionBinding = validateTrackBProjectionGraphBinding(artifactBindings.projectionReceipt, artifactBindings.sceneGraph);
    const provenanceBinding = validateTrackBProvenanceBinding(
      artifactBindings.provenance,
      context.authority ?? artifactBindings.authority,
      isByteSequence(artifactBindings.projectionReceiptBytes)
        ? hashTrackBRawBytes(artifactBindings.projectionReceiptBytes)
        : undefined,
      isPlainObject(artifactBindings.sceneGraph) ? canonicalHash(artifactBindings.sceneGraph) : undefined,
      isByteSequence(artifactBindings.htmlBytes) ? hashTrackBRawBytes(artifactBindings.htmlBytes) : undefined,
      artifactBindings.handoff,
      isByteSequence(artifactBindings.handoffBytes) ? hashTrackBRawBytes(artifactBindings.handoffBytes) : undefined,
      artifactBindings.sceneGraph
    );
    return {
      valid: validation.valid && projectionBinding.valid && provenanceBinding.valid,
      errors: [...(validation.errors ?? []), ...projectionBinding.errors, ...provenanceBinding.errors]
    };
  } catch (error) {
    return {
      valid: false,
      errors: [{
        code: normalizeRuntimeErrorCode(error?.code, 'trackb-candidate-invalid'),
        instancePath: '/',
        message: boundedMessage(error?.message ?? 'Track B candidate validation failed')
      }]
    };
  }
}
