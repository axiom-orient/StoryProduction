import { assertApprovedSceneProjection } from '../../adapters/src/narrative-courier-release.js';
import {
  hashTrackBRawBytes,
  isTrackBNarrativeId,
  TRACK_B_PROJECTION_RECEIPT_PATH,
  TRACK_B_HANDOFF_PATH,
  validateTrackBNarrativeAuthority,
  validateTrackBMeasuredEvidenceReceipt,
  validateTrackBProjectionReceiptShape,
  validateTrackBCommandInput
} from '../../contracts/src/trackb-contract.js';
import { validateTrackBArtifactSchemas } from '../../contracts/src/trackb-artifact-schemas.js';
import { canonicalHash } from '../../core/src/canonical.js';
import {
  AUTHORITY_VALIDATION_CODES,
  assertMeasuredEvidenceBinding,
  assertNarrativeSourceBinding,
  assertProjectionGraphBinding,
  assertProvenanceBinding,
  assertTrackBBinding,
  assertTrackBInput,
  deriveMeaningQualification,
  isByteSequence,
  invokeValidator,
  mappedError,
  throwValidationIfInvalid,
  trackBError
} from './trackb-release-bindings.js';
import { isPlainObject } from './trackb-application-primitives.js';

export const TRACK_B_PREPARE_MAX_ARTIFACT_BYTES = 8 * 1024 * 1024;
export { TRACK_B_PROJECTION_RECEIPT_PATH, TRACK_B_HANDOFF_PATH };

export function loadTrackBReleaseInputsFromArtifacts({ input, artifacts, registry, artifactRegistries } = {}) {
  const inputValidation = validateTrackBCommandInput('trackb.release.prepare', input);
  throwValidationIfInvalid(inputValidation, 'trackb-input-invalid', 'Track B prepare input');
  assertTrackBBinding(isPlainObject(artifacts), 'trackb-artifact-binding-invalid', 'Track B raw artifact collection is required');
  const expectedArtifactKeys = [
    'authorityBytes',
    'projectionReceiptBytes',
    'handoffBytes',
    'htmlBytes',
    'provenanceBytes',
    'evidenceReceiptBytes',
    'sceneGraphBytes'
  ];
  const actualArtifactKeys = Object.keys(artifacts).sort();
  assertTrackBBinding(
    actualArtifactKeys.length === expectedArtifactKeys.length
      && expectedArtifactKeys.every((key) => actualArtifactKeys.includes(key)),
    'trackb-artifact-binding-invalid',
    'Track B raw artifact collection must contain exactly the seven declared byte fields'
  );

  const authorityArtifact = parseJsonBytesArtifact(
    artifacts.authorityBytes,
    'NarrativeAuthority',
    'trackb-authority-missing'
  );
  const projectionArtifact = parseJsonBytesArtifact(
    artifacts.projectionReceiptBytes,
    'NarrativeSceneProjection',
    'trackb-artifact-binding-invalid'
  );
  const handoffArtifact = parseJsonBytesArtifact(
    artifacts.handoffBytes,
    'SceneMotionAssetHandoff',
    'trackb-artifact-binding-invalid'
  );
  const htmlArtifact = normalizeRawBytesArtifact(
    artifacts.htmlBytes,
    'moving HTML',
    'trackb-artifact-binding-invalid'
  );
  const provenanceArtifact = parseJsonBytesArtifact(
    artifacts.provenanceBytes,
    'Track B provenance',
    'trackb-artifact-binding-invalid'
  );
  const evidenceReceiptArtifact = parseJsonBytesArtifact(
    artifacts.evidenceReceiptBytes,
    'Track B measured evidence receipt',
    'trackb-evidence-incomplete'
  );
  const sceneGraphArtifact = parseJsonBytesArtifact(
    artifacts.sceneGraphBytes,
    'SceneGraph projection',
    'trackb-artifact-binding-invalid'
  );

  const authorityResult = invokeValidator(
    () => validateTrackBNarrativeAuthority(authorityArtifact.value, { registry }),
    'trackb-authority-missing',
    'NarrativeAuthority'
  );
  throwValidationIfInvalid(authorityResult, 'trackb-authority-missing', 'NarrativeAuthority', AUTHORITY_VALIDATION_CODES);
  const projectionShape = invokeValidator(
    () => validateTrackBProjectionReceiptShape(projectionArtifact.value),
    'trackb-artifact-binding-invalid',
    'NarrativeSceneProjection'
  );
  throwValidationIfInvalid(projectionShape, 'trackb-artifact-binding-invalid', 'NarrativeSceneProjection');
  const evidenceReceiptResult = invokeValidator(
    () => validateTrackBMeasuredEvidenceReceipt(evidenceReceiptArtifact.value, { registry }),
    'trackb-evidence-incomplete',
    'Track B measured evidence receipt'
  );
  throwValidationIfInvalid(evidenceReceiptResult, 'trackb-evidence-incomplete', 'Track B measured evidence receipt');

  const artifactSchemaResult = invokeValidator(
    () => validateTrackBArtifactSchemas({
      projectionReceipt: projectionArtifact.value,
      sceneGraph: sceneGraphArtifact.value,
      handoff: handoffArtifact.value,
      artifactRegistries
    }),
    'trackb-artifact-binding-invalid',
    'Track B upstream artifact schema'
  );
  throwValidationIfInvalid(artifactSchemaResult, 'trackb-artifact-binding-invalid', 'Track B upstream artifact schema');

  assertTrackBBinding(
    input.projectionReceiptPath === handoffArtifact.value.projectionReceiptPath,
    'trackb-artifact-binding-invalid',
    'handoff projectionReceiptPath does not equal the requested projection receipt path'
  );
  assertTrackBBinding(
    input.projectionReceiptPath === TRACK_B_PROJECTION_RECEIPT_PATH,
    'trackb-artifact-binding-invalid',
    `projectionReceiptPath must be ${TRACK_B_PROJECTION_RECEIPT_PATH}`
  );
  assertTrackBBinding(
    input.assetHandoffPath === TRACK_B_HANDOFF_PATH && handoffArtifact.value.projectionReceiptPath === TRACK_B_PROJECTION_RECEIPT_PATH,
    'trackb-artifact-binding-invalid',
    `assetHandoffPath and handoff projectionReceiptPath must use the declared versioned paths ${TRACK_B_HANDOFF_PATH} and ${TRACK_B_PROJECTION_RECEIPT_PATH}`
  );
  const projectionReceiptHash = hashTrackBRawBytes(projectionArtifact.bytes);
  assertTrackBBinding(
    projectionReceiptHash === `sha256:${handoffArtifact.value.projectionReceiptSha256}`,
    'trackb-artifact-binding-invalid',
    'handoff projection receipt hash does not match the exact receipt bytes'
  );

  const sceneGraphHash = canonicalHash(sceneGraphArtifact.value);
  const projection = projectionArtifact.value;
  const authority = authorityArtifact.value;
  assertTrackBBinding(
    authority.sourceRefs.projectionReceiptHash === projectionReceiptHash,
    'trackb-artifact-binding-invalid',
    'NarrativeAuthority projectionReceiptHash does not match the exact receipt bytes'
  );
  assertTrackBBinding(
    authority.sourceRefs.sceneGraphHash === sceneGraphHash,
    'trackb-artifact-binding-invalid',
    'NarrativeAuthority sceneGraphHash does not match the canonical SceneGraph hash'
  );
  assertTrackBBinding(
    authority.sourceRefs.sourceSemanticManifestHash === sceneGraphArtifact.value.semanticManifest?.sourceSemanticManifestHash,
    'trackb-artifact-binding-invalid',
    'NarrativeAuthority sourceSemanticManifestHash does not match SceneGraph'
  );
  assertTrackBBinding(
    authority.sourceRefs.semanticManifestHash === sceneGraphArtifact.value.semanticManifest?.semanticManifestHash,
    'trackb-artifact-binding-invalid',
    'NarrativeAuthority semanticManifestHash does not match SceneGraph'
  );
  assertTrackBBinding(
    projection.narrativeId === authority.narrativeId,
    'trackb-narrative-mismatch',
    'projection narrativeId differs from NarrativeAuthority'
  );
  assertTrackBBinding(
    projection.sceneGraphHash === sceneGraphHash,
    'trackb-artifact-binding-invalid',
    'projection receipt sceneGraphHash does not match the canonical SceneGraph hash'
  );
  assertTrackBBinding(
    handoffArtifact.value.sceneGraphHash === sceneGraphHash,
    'trackb-artifact-binding-invalid',
    'SceneAssetCompiler handoff sceneGraphHash does not match the canonical SceneGraph hash'
  );
  assertTrackBBinding(
    handoffArtifact.value.sourceSemanticManifestHash === sceneGraphArtifact.value.semanticManifest?.sourceSemanticManifestHash
      && handoffArtifact.value.semanticManifestHash === sceneGraphArtifact.value.semanticManifest?.semanticManifestHash,
    'trackb-artifact-binding-invalid',
    'SceneAssetCompiler handoff semantic manifest hashes do not match SceneGraph'
  );
  assertNarrativeSourceBinding(authority, projection);
  assertProjectionGraphBinding(projection, sceneGraphArtifact.value);
  try { assertApprovedSceneProjection(projection, sceneGraphArtifact.value); }
  catch (error) { throw trackBError('trackb-artifact-binding-invalid', 'approved cinematic source closure is invalid', { cause: error }); }
  if (sceneGraphArtifact.value.sourceNarrativeId !== undefined
    && isTrackBNarrativeId(sceneGraphArtifact.value.sourceNarrativeId)) {
    assertTrackBBinding(
      sceneGraphArtifact.value.sourceNarrativeId === authority.narrativeId,
      'trackb-narrative-mismatch',
      'SceneGraph sourceNarrativeId differs from NarrativeAuthority'
    );
  }
  assertProvenanceBinding(
    provenanceArtifact.value,
    authority,
    projectionReceiptHash,
    sceneGraphHash,
    htmlArtifact.sha256,
    handoffArtifact.value,
    handoffArtifact.sha256,
    sceneGraphArtifact.value
  );
  assertMeasuredEvidenceBinding(
    evidenceReceiptArtifact.value,
    evidenceReceiptArtifact.bytes,
    authority,
    provenanceArtifact.value,
    htmlArtifact.sha256,
    sceneGraphArtifact.value,
    registry
  );
  const qualification = deriveMeaningQualification({
    authority,
    projectionReceipt: projection,
    projectionReceiptBytes: projectionArtifact.bytes,
    sceneGraph: sceneGraphArtifact.value,
    provenance: provenanceArtifact.value,
    htmlBytes: htmlArtifact.bytes
  });

  const artifactBindings = {
    registry,
    authority,
    projectionReceipt: projection,
    projectionReceiptBytes: projectionArtifact.bytes,
    sceneGraph: sceneGraphArtifact.value,
    handoff: handoffArtifact.value,
    handoffBytes: handoffArtifact.bytes,
    htmlBytes: htmlArtifact.bytes,
    provenance: provenanceArtifact.value,
    provenanceBytes: provenanceArtifact.bytes,
    evidenceReceipt: evidenceReceiptArtifact.value,
    evidenceReceiptBytes: evidenceReceiptArtifact.bytes,
    meaningQualificationHash: qualification.meaningQualificationHash,
    qualifiedMeaningItems: qualification.qualifiedItems,
    artifactRegistries
  };

  return Object.freeze({
    registry,
    artifactRegistries,
    authority,
    authorityBytes: authorityArtifact.bytes,
    projectionReceipt: projection,
    projectionReceiptBytes: projectionArtifact.bytes,
    sceneGraph: sceneGraphArtifact.value,
    sceneGraphBytes: sceneGraphArtifact.bytes,
    handoff: handoffArtifact.value,
    handoffBytes: handoffArtifact.bytes,
    htmlBytes: htmlArtifact.bytes,
    provenance: provenanceArtifact.value,
    provenanceBytes: provenanceArtifact.bytes,
    evidenceReceipt: evidenceReceiptArtifact.value,
    evidenceReceiptBytes: evidenceReceiptArtifact.bytes,
    meaningQualificationHash: qualification.meaningQualificationHash,
    qualifiedMeaningItems: qualification.qualifiedItems,
    artifactBindings,
    paths: Object.freeze({
      narrativeAuthorityPath: input.narrativeAuthorityPath,
      projectionReceiptPath: input.projectionReceiptPath,
      sceneGraphPath: projection.sceneGraphPath,
      assetHandoffPath: input.assetHandoffPath,
      htmlPath: input.htmlPath,
      provenancePath: input.provenancePath,
      evidenceReceiptPath: input.evidenceReceiptPath
    })
  });
}

/**
 * Complete the read/validate/build slice. No file is created or modified.
 */

function parseJsonBytesArtifact(value, label, errorCode) {
  const artifact = normalizeRawBytesArtifact(value, label, errorCode);
  let text;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(artifact.bytes);
  } catch (error) {
    throw mappedError(error, errorCode, `${label} is not valid UTF-8`, { cause: error });
  }
  try {
    return { ...artifact, value: JSON.parse(text) };
  } catch (error) {
    throw mappedError(error, errorCode, `${label} is not valid JSON`, { cause: error });
  }
}


function normalizeRawBytesArtifact(value, label, errorCode) {
  if (!isByteSequence(value)) throw trackBError(errorCode, `${label} raw bytes are required`);
  const bytes = Buffer.isBuffer(value)
    ? Buffer.from(value)
    : typeof value === 'string'
      ? Buffer.from(value, 'utf8')
      : Buffer.from(value);
  if (bytes.length > TRACK_B_PREPARE_MAX_ARTIFACT_BYTES) {
    throw trackBError(errorCode, `${label} exceeds the bounded ${TRACK_B_PREPARE_MAX_ARTIFACT_BYTES}-byte read limit`);
  }
  return { bytes, sha256: hashTrackBRawBytes(bytes), sizeBytes: bytes.length };
}
