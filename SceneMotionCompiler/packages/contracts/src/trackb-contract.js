//! Stable public surface for Track B contracts.

export { loadTrackBArtifactContractRegistries } from './trackb-artifact-schemas.js';
export {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  deriveTrackBMeaningQualification,
  parseTrackBSemanticManifestHtml,
  validateTrackBSemanticManifestObject
} from './trackb-semantic-manifest-contract.js';

export {
  TRACK_B_APPROVAL_MAX_FUTURE_MS,
  TRACK_B_BASE_HASH_PATTERN,
  TRACK_B_HANDOFF_PATH,
  TRACK_B_HASH_DOMAINS,
  TRACK_B_HASH_PATTERN,
  TRACK_B_NARRATIVE_ID_PATTERN,
  TRACK_B_OPAQUE_ID_PATTERN,
  TRACK_B_PROJECTION_RECEIPT_PATH,
  TRACK_B_RELEASE_CANDIDATE_FILENAME,
  TRACK_B_SAFE_PATH_PATTERN,
  TRACK_B_UTC_DATE_TIME_PATTERN,
  TrackBContractError,
  assertTrackBDateTime,
  assertTrackBHash,
  assertTrackBNarrativeId,
  assertTrackBOpaqueId,
  assertTrackBSafeRelativePath,
  canonicalTrackBDomainBytes,
  canonicalTrackBHash,
  hashTrackBApprovalReceipt,
  hashTrackBAuthority,
  hashTrackBDeliveryPointer,
  hashTrackBDeliverySet,
  hashTrackBEvidenceCell,
  hashTrackBMeasuredEvidenceReceipt,
  hashTrackBMeaningQualification,
  hashTrackBRawBytes,
  hashTrackBReleaseCandidate,
  isTrackBBaseHash,
  isTrackBDateTime,
  isTrackBHash,
  isTrackBNarrativeId,
  isTrackBOpaqueId,
  isTrackBSafeRelativePath,
  parseTrackBDateTime,
  verifyTrackBSelfHash
} from './trackb-contract-primitives.js';

export {
  TRACK_B_COMMAND_INPUT_KEYS,
  TRACK_B_COMMAND_OPTIONAL_INPUT_KEYS,
  TRACK_B_COMMAND_RESULT_KEYS,
  TRACK_B_LOOPBACK_ORIGIN_PATTERN,
  TRACK_B_RELEASE_COMMANDS,
  TRACK_B_STUDIO_APPROVAL_KEYS,
  TRACK_B_STUDIO_COOKIE,
  TRACK_B_STUDIO_ENDPOINT,
  TRACK_B_STUDIO_EXECUTE_KEYS,
  TRACK_B_STUDIO_MAX_BODY_BYTES,
  TRACK_B_STUDIO_REQUEST_FRAME_KEYS,
  TRACK_B_STUDIO_SESSION_ENDPOINT,
  assertTrackBCommandInput,
  assertTrackBStatusResult,
  isTrackBLoopbackOrigin,
  validateTrackBApplicationEnvelope,
  validateTrackBCommandInput,
  validateTrackBCommandResult,
  validateTrackBStatusResult,
  validateTrackBStatusResultShape,
  validateTrackBStudioExecuteBody
} from './trackb-command-contract.js';

export {
  TRACK_B_PROVENANCE_REQUIRED_KEYS,
  assertTrackBApprovalReceipt,
  assertTrackBArtifactBindings,
  assertTrackBDeliveryPointer,
  assertTrackBDeliverySet,
  assertTrackBMeasuredEvidenceReceipt,
  assertTrackBNarrativeAuthority,
  assertTrackBProjectionReceiptShape,
  assertTrackBReleaseCandidate,
  deriveCandidateVerdict,
  validateTrackBApprovalReceipt,
  validateTrackBArtifactBindings,
  validateTrackBDeliveryPointer,
  validateTrackBDeliverySet,
  validateTrackBMeasuredEvidenceReceipt,
  validateTrackBNarrativeAuthority,
  validateTrackBProjectionReceiptShape,
  validateTrackBReleaseCandidate
} from './trackb-artifact-contract.js';
