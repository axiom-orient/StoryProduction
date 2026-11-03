//! Stable facade for Track B artifact relation validators.
//!
//! The public import path remains `trackb-artifact-contract.js`; each
//! validator family delegates to its owner module, while cross-artifact raw
//! byte and provenance binding remains a single implementation owner.

export {
  TRACK_B_PROVENANCE_REQUIRED_KEYS,
  assertTrackBArtifactBindings,
  validateTrackBArtifactBindings
} from './trackb-artifact-contract/bindings.js';
export {
  assertTrackBNarrativeAuthority,
  assertTrackBMeasuredEvidenceReceipt,
  validateTrackBNarrativeAuthority,
  validateTrackBMeasuredEvidenceReceipt
} from './trackb-artifact-contract/authority.js';
export {
  assertTrackBReleaseCandidate,
  deriveCandidateVerdict,
  validateTrackBReleaseCandidate
} from './trackb-artifact-contract/candidate.js';
export {
  assertTrackBApprovalReceipt,
  assertTrackBDeliveryPointer,
  assertTrackBDeliverySet,
  validateTrackBApprovalReceipt,
  validateTrackBDeliveryPointer,
  validateTrackBDeliverySet
} from './trackb-artifact-contract/release.js';
export {
  assertTrackBProjectionReceiptShape,
  validateTrackBProjectionReceiptShape
} from './trackb-artifact-contract/projection.js';
