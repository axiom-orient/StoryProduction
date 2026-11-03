/** Build and reconcile the Track B DeliverySet and stable pointer projection. */
import { canonicalHash } from '../../core/src/canonical.js';
import {
  hashTrackBDeliveryPointer,
  hashTrackBDeliverySet,
  isTrackBSafeRelativePath,
  validateTrackBDeliveryPointer,
  validateTrackBDeliverySet
} from '../../contracts/src/trackb-contract.js';
import { decodeSnapshotBytes, parseCanonicalJsonBytes } from './trackb-release-runtime-artifacts.js';
import { verifySnapshotResult } from './trackb-release-runtime-custody.js';
import { boundedMessage } from './trackb-application-primitives.js';
import { hashHex, mapRuntimeError, runtimeError } from './trackb-release-runtime-support.js';

export function buildDeliverySet({ narrativeId, candidate, approval, baseDeliverySetHash, revision }) {
  const deliverySetId = `delivery.${hashHex(canonicalHash({
    narrativeId,
    candidateHash: candidate.candidateHash,
    approvalHash: approval.approvalHash,
    baseDeliverySetHash,
    revision
  }))}`;
  const deliverySet = {
    schemaVersion: 2,
    deliverySetId,
    narrativeId,
    candidateHash: candidate.candidateHash,
    authorityHash: candidate.authorityHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash,
    approvalHash: approval.approvalHash,
    artifacts: { ...candidate.artifacts },
    evidenceCellHashes: candidate.evidenceCells.map((cell) => cell.evidenceCellHash),
    revision,
    state: 'PROMOTED'
  };
  deliverySet.deliverySetHash = hashTrackBDeliverySet(deliverySet);
  return Object.freeze(deliverySet);
}

export function buildDeliveryPointer({ narrativeId, candidate, approval, deliverySet, deliverySetPath }) {
  const pointer = {
    schemaVersion: 2,
    pointerKey: pointerKeyForNarrative(narrativeId),
    narrativeId,
    revision: deliverySet.revision,
    state: 'PROMOTED',
    deliverySetHash: deliverySet.deliverySetHash,
    candidateHash: candidate.candidateHash,
    approvalHash: approval.approvalHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash: deliverySet.baseDeliverySetHash,
    deliverySetPath
  };
  pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
  return Object.freeze(pointer);
}

function pointerKeyForNarrative(narrativeId) {
  const readable = `pointer.${narrativeId}`;
  return readable.length <= 128
    ? readable
    : `pointer.${hashHex(canonicalHash({ narrativeId }))}`;
}

export function validateGeneratedDeliverySet(deliverySet, candidate, approval, registry) {
  let validation;
  try {
    validation = validateTrackBDeliverySet(deliverySet, { registry, candidate, approval });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'generated DeliverySet validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-storage-integrity', `generated DeliverySet is invalid: ${boundedMessage(first?.message ?? 'invalid delivery set')}`);
  }
}

export function validateGeneratedDeliveryPointer(pointer, deliverySet, candidate, approval, registry) {
  let validation;
  try {
    validation = validateTrackBDeliveryPointer(pointer, { registry, deliverySet, candidate, approval });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'generated delivery pointer validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-storage-integrity', `generated delivery pointer is invalid: ${boundedMessage(first?.message ?? 'invalid pointer')}`);
  }
}

export function validateExistingPromotion(current, candidate, approval, registry) {
  let deliveryValidation;
  let pointerValidation;
  try {
    deliveryValidation = validateTrackBDeliverySet(current.deliverySet, {
      registry,
      candidate,
      approval
    });
    pointerValidation = validateTrackBDeliveryPointer(current.pointer, {
      registry,
      deliverySet: current.deliverySet,
      candidate,
      approval
    });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'existing promotion binding validation failed', { cause: error });
  }
  if (deliveryValidation?.valid !== true || pointerValidation?.valid !== true) {
    throw runtimeError('trackb-storage-integrity', 'existing promotion is not bound to the requested candidate and approval');
  }
}

export function buildPromoteResultFromPointer(pointer, narrativeId, reused) {
  return {
    deliverySetPath: pointer.deliverySetPath,
    deliverySetHash: pointer.deliverySetHash,
    pointerPath: `scenes/trackb/${narrativeId}/delivery-pointer-v2.json`,
    pointerHash: pointer.pointerHash,
    meaningQualificationHash: pointer.meaningQualificationHash,
    revision: pointer.revision,
    state: pointer.state,
    reused
  };
}

export function buildPointerProjection(pointer) {
  return {
    state: 'PROMOTED',
    currentDeliverySetHash: pointer.deliverySetHash,
    candidateHash: pointer.candidateHash,
    approvalHash: pointer.approvalHash,
    meaningQualificationHash: pointer.meaningQualificationHash,
    pointerRevision: pointer.revision,
    pointerHash: pointer.pointerHash
  };
}

export function deriveCandidateStatus(candidate, approval, current, registry) {
  const pointerSubjectMatches = current.pointer
    && current.pointer.candidateHash === candidate.candidateHash
    && current.pointer.meaningQualificationHash === candidate.meaningQualificationHash
    && current.pointer.baseDeliverySetHash === candidate.baseDeliverySetHash;
  if (pointerSubjectMatches && !approval) {
    throw runtimeError('trackb-approval-required', 'stable pointer promotion requires its deterministic approval receipt');
  }
  if (pointerSubjectMatches && candidate.candidateVerdict !== 'PROMOTE') {
    throw runtimeError('trackb-storage-integrity', 'stable pointer cannot bind an ITERATE or FAIL candidate');
  }
  if (pointerSubjectMatches && approval && current.pointer.approvalHash !== approval.approvalHash) {
    throw runtimeError('trackb-storage-integrity', 'stable pointer approvalHash differs from the deterministic approval receipt');
  }
  if (pointerSubjectMatches && approval && approval.decision !== 'APPROVE') {
    throw runtimeError('trackb-storage-integrity', 'stable pointer cannot bind a non-APPROVE receipt');
  }
  if (approval?.decision === 'APPROVE' && candidate.candidateVerdict !== 'PROMOTE') {
    throw runtimeError('trackb-delivery-not-ready', 'an APPROVE receipt cannot describe an ITERATE or FAIL candidate');
  }
  if (candidate.candidateVerdict === 'ITERATE') {
    return {
      state: 'ITERATE',
      candidateHash: candidate.candidateHash,
      candidateVerdict: 'ITERATE'
    };
  }
  if (candidate.candidateVerdict === 'FAIL') {
    return {
      state: 'FAIL',
      candidateHash: candidate.candidateHash,
      candidateVerdict: 'FAIL'
    };
  }
  if (!approval) {
    return {
      state: 'READY',
      candidateHash: candidate.candidateHash,
      baseDeliverySetHash: candidate.baseDeliverySetHash,
      meaningQualificationHash: candidate.meaningQualificationHash,
      candidateVerdict: 'PROMOTE'
    };
  }
  if (approval.decision === 'APPROVE') {
    if (pointerSubjectMatches && current.pointer.approvalHash === approval.approvalHash) {
      validateExistingPromotion(current, candidate, approval, registry);
      return {
        state: 'PROMOTED',
        candidateHash: candidate.candidateHash,
        baseDeliverySetHash: candidate.baseDeliverySetHash,
        meaningQualificationHash: candidate.meaningQualificationHash,
        candidateVerdict: 'PROMOTE',
        deliverySetHash: current.pointer.deliverySetHash,
        approvalHash: approval.approvalHash
      };
    }
    return {
      state: 'APPROVED',
      candidateHash: candidate.candidateHash,
      baseDeliverySetHash: candidate.baseDeliverySetHash,
      meaningQualificationHash: candidate.meaningQualificationHash,
      candidateVerdict: 'PROMOTE',
      approvalHash: approval.approvalHash,
      decision: 'APPROVE'
    };
  }
  return {
    state: approval.decision === 'REVISE' ? 'REVISE' : 'REJECT',
    candidateHash: candidate.candidateHash,
    baseDeliverySetHash: candidate.baseDeliverySetHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    candidateVerdict: 'PROMOTE',
    approvalHash: approval.approvalHash,
    decision: approval.decision
  };
}

/**
 * Resolve the only current-base states admitted by the Track B runtime.
 *
 * An absent pointer is the explicit NONE base. A present pointer is not a
 * compare token until its canonical bytes, fixed delivery-set path, and
 * referenced immutable DeliverySet have all been verified in the same
 * narrative custody session.
 */
export async function resolveCurrentDeliveryBase(session, narrativeId, registry) {
  if (!session || typeof session.snapshot !== 'function') {
    throw runtimeError('trackb-storage-integrity', 'narrative custody session cannot snapshot the delivery pointer');
  }
  const pointerPath = 'delivery-pointer-v2.json';
  let pointerSnapshot;
  try {
    pointerSnapshot = await session.snapshot(pointerPath);
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-storage-integrity', 'custody could not read the delivery pointer');
  }
  verifySnapshotResult(pointerSnapshot, { path: pointerPath });
  if (pointerSnapshot.exists === false) {
    return Object.freeze({
      baseDeliverySetHash: 'NONE',
      pointerSnapshot
    });
  }

  const pointerBytes = decodeSnapshotBytes(pointerSnapshot, 'delivery pointer');
  const pointer = parseCanonicalJsonBytes(pointerBytes, 'Track B delivery pointer', 'trackb-storage-integrity');
  validateCurrentPointer(pointer, narrativeId, registry);

  const narrativePrefix = `scenes/trackb/${narrativeId}/`;
  const expectedDeliverySetPath = `${narrativePrefix}delivery-sets/${hashHex(pointer.deliverySetHash)}/trackb-delivery-set-v2.json`;
  if (pointer.deliverySetPath !== expectedDeliverySetPath) {
    throw runtimeError('trackb-storage-integrity', 'delivery pointer deliverySetPath is not the exact narrative-bound path');
  }
  // Only strip the exact, already-verified narrative prefix. Never resolve
  // or normalize a path supplied by the pointer.
  const deliverySetPath = pointer.deliverySetPath.slice(narrativePrefix.length);
  if (!isTrackBSafeRelativePath(deliverySetPath)) {
    throw runtimeError('trackb-storage-integrity', 'delivery pointer delivery-set path is not a safe narrative-relative path');
  }
  if (typeof session.read !== 'function') {
    throw runtimeError('trackb-storage-integrity', 'narrative custody session cannot read the referenced DeliverySet');
  }
  let deliverySetSnapshot;
  try {
    deliverySetSnapshot = await session.read(deliverySetPath);
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-storage-integrity', 'custody could not read the referenced DeliverySet');
  }
  verifySnapshotResult(deliverySetSnapshot, { path: deliverySetPath });
  if (deliverySetSnapshot.exists !== true) {
    throw runtimeError('trackb-storage-integrity', 'delivery pointer references a missing DeliverySet');
  }
  const deliverySetBytes = decodeSnapshotBytes(deliverySetSnapshot, 'delivery set');
  const deliverySet = parseCanonicalJsonBytes(deliverySetBytes, 'Track B delivery set', 'trackb-storage-integrity');
  validateCurrentDeliverySet(deliverySet, pointer, registry);

  return Object.freeze({
    baseDeliverySetHash: pointer.deliverySetHash,
    pointerSnapshot,
    deliverySetSnapshot,
    pointer,
    deliverySet
  });
}

function validateCurrentPointer(pointer, narrativeId, registry) {
  let validation;
  try {
    validation = validateTrackBDeliveryPointer(pointer, { registry });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'delivery pointer validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-storage-integrity', `delivery pointer is invalid: ${boundedMessage(first?.message ?? 'invalid pointer')}`);
  }
  if (pointer.narrativeId !== narrativeId) {
    throw runtimeError('trackb-storage-integrity', 'delivery pointer narrativeId differs from the locked narrative');
  }
}

function validateCurrentDeliverySet(deliverySet, pointer, registry) {
  let validation;
  try {
    validation = validateTrackBDeliverySet(deliverySet, { registry });
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'referenced DeliverySet validation failed', { cause: error });
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-storage-integrity', `referenced DeliverySet is invalid: ${boundedMessage(first?.message ?? 'invalid delivery set')}`);
  }
  for (const field of ['narrativeId', 'revision', 'state', 'deliverySetHash', 'candidateHash', 'approvalHash', 'meaningQualificationHash', 'baseDeliverySetHash']) {
    if (pointer[field] !== deliverySet[field]) {
      throw runtimeError('trackb-storage-integrity', `delivery pointer ${field} does not match the referenced DeliverySet`);
    }
  }
}
