/** Bind Studio-provided approval context and immutable receipt to one candidate subject. */
import { canonicalHash } from '../../core/src/canonical.js';
import {
  hashTrackBApprovalReceipt,
  hashTrackBReleaseCandidate,
  isTrackBHash,
  isTrackBOpaqueId,
  validateTrackBApprovalReceipt,
  validateTrackBCommandInput,
  validateTrackBNarrativeAuthority,
  validateTrackBReleaseCandidate
} from '../../contracts/src/trackb-contract.js';
import {
  hashHex,
  hasExactKeys,
  mapRuntimeError,
  runtimeError,
  throwValidationIfInvalid
} from './trackb-release-runtime-support.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';

export function normalizeApprovalInput(input, now) {
  if (!isPlainObject(input)) throw runtimeError('trackb-input-invalid', 'Track B approval input must be an object');
  const validation = validateTrackBCommandInput('trackb.release.approve', input, { now });
  throwValidationIfInvalid(validation, 'trackb-input-invalid', 'Track B approval input');
  return input;
}

export function normalizeApprovalContext(context) {
  if (!isPlainObject(context) || !hasExactKeys(context, ['actorId', 'origin', 'reviewSessionId'])) {
    throw runtimeError('trackb-approval-forbidden', 'approval execution context must contain exactly actorId, origin, and reviewSessionId');
  }
  if (!isTrackBOpaqueId(context.actorId)) {
    throw runtimeError('trackb-approval-forbidden', 'approval actorId must be a valid opaque ID');
  }
  if (context.origin !== 'studio') {
    throw runtimeError('trackb-approval-forbidden', 'approval origin must be studio');
  }
  if (!isTrackBHash(context.reviewSessionId)) {
    throw runtimeError('trackb-approval-forbidden', 'approval reviewSessionId must be a sha256 hash');
  }
  return Object.freeze({
    actorId: context.actorId,
    origin: context.origin,
    reviewSessionId: context.reviewSessionId
  });
}

export function validateAuthorityForApproval(authority, input, candidate, registry) {
  if (!isPlainObject(authority)) throw runtimeError('trackb-storage-integrity', 'candidate-bound NarrativeAuthority must be an object');
  let validation;
  try {
    validation = validateTrackBNarrativeAuthority(authority, { registry });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'candidate-bound NarrativeAuthority validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-storage-integrity', `candidate-bound NarrativeAuthority is invalid: ${boundedMessage(first?.message ?? 'invalid authority')}`);
  }
  if (authority.authorityHash !== candidate.authorityHash) {
    throw runtimeError('trackb-storage-integrity', 'candidate-bound NarrativeAuthority hash differs from candidate');
  }
  if (authority.narrativeId !== input.narrativeId || authority.narrativeId !== candidate.narrativeId) {
    throw runtimeError('trackb-narrative-mismatch', 'candidate-bound NarrativeAuthority narrativeId differs from the approval subject');
  }
}

export function validateCandidateForApproval(candidate, input, registry, authority) {
  if (!isPlainObject(candidate)) throw runtimeError('trackb-candidate-invalid', 'Track B candidate must be an object');
  let validation;
  try {
    validation = validateTrackBReleaseCandidate(candidate, { registry, authority });
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-candidate-invalid', 'Track B candidate validation failed');
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-candidate-invalid', `Track B candidate validation failed: ${boundedMessage(first?.message ?? 'invalid candidate')}`);
  }
  let selfHash;
  try {
    selfHash = hashTrackBReleaseCandidate(candidate);
  } catch (error) {
    throw runtimeError('trackb-candidate-invalid', 'Track B candidate self-hash could not be computed', { cause: error });
  }
  if (selfHash !== candidate.candidateHash || candidate.candidateHash !== input.candidateHash) {
    throw runtimeError('trackb-candidate-invalid', 'candidateHash does not match the requested candidate bytes');
  }
  if (candidate.narrativeId !== input.narrativeId) {
    throw runtimeError('trackb-narrative-mismatch', 'candidate narrativeId differs from the approval input');
  }
  if (candidate.baseDeliverySetHash !== input.baseDeliverySetHash) {
    throw runtimeError('trackb-approval-stale', 'candidate baseDeliverySetHash differs from the approval input');
  }
}

export function buildApprovalReceipt({ input, context, candidate, registry, now }) {
  const approvalDomainHash = canonicalHash({
    narrativeId: input.narrativeId,
    candidateHash: input.candidateHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash: input.baseDeliverySetHash
  });
  const receipt = {
    schemaVersion: 2,
    approvalId: `approval.${hashHex(approvalDomainHash)}`,
    subject: {
      candidateHash: input.candidateHash,
      meaningQualificationHash: candidate.meaningQualificationHash,
      baseDeliverySetHash: input.baseDeliverySetHash
    },
    decision: input.decision,
    actor: {
      actorId: context.actorId,
      actorKind: 'human'
    },
    reason: input.reason,
    origin: context.origin,
    reviewSessionId: context.reviewSessionId,
    decidedAt: input.decidedAt
  };
  receipt.approvalHash = hashTrackBApprovalReceipt(receipt);
  validateStoredApprovalReceipt(receipt, registry, now, {
    candidateHash: input.candidateHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash: input.baseDeliverySetHash
  });
  return Object.freeze(receipt);
}

export function validateStoredApprovalReceipt(receipt, registry, now, { candidateHash, meaningQualificationHash, baseDeliverySetHash } = {}) {
  let validation;
  try {
    validation = validateTrackBApprovalReceipt(receipt, {
      registry,
      now,
      candidateHash,
      meaningQualificationHash,
      baseDeliverySetHash
    });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'stored Track B approval receipt validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    const code = first?.code === 'trackb-approval-stale' ? 'trackb-approval-stale' : 'trackb-storage-integrity';
    throw runtimeError(code, `stored Track B approval receipt is invalid: ${boundedMessage(first?.message ?? 'invalid receipt')}`);
  }
}
