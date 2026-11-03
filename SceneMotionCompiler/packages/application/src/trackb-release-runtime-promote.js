import { canonicalizeJson } from '../../core/src/canonical.js';
import { decodeSnapshotBytes, parseCanonicalJsonBytes } from './trackb-release-runtime-artifacts.js';
import {
  assertBoundedWriteBytes,
  assertNarrativeSession,
  closeNarrativeSession,
  openNarrativeRuntime,
  readNarrativeSnapshot,
  readRequiredNarrativeObject,
  validateCommandResult,
  verifyImmutableWrite,
  verifyPublishPointer
} from './trackb-release-runtime-custody.js';
import {
  validateAuthorityForApproval,
  validateCandidateForApproval,
  validateStoredApprovalReceipt
} from './trackb-release-runtime-approval.js';
import {
  buildDeliveryPointer,
  buildDeliverySet,
  buildPromoteResultFromPointer,
  resolveCurrentDeliveryBase,
  validateExistingPromotion,
  validateGeneratedDeliveryPointer,
  validateGeneratedDeliverySet
} from './trackb-release-runtime-delivery.js';
import { hashBaseSegment, hashHex, mapRuntimeError, runtimeError, throwValidationIfInvalid } from './trackb-release-runtime-support.js';
import { isPlainObject } from './trackb-application-primitives.js';
import { validateTrackBCommandInput } from '../../contracts/src/trackb-contract.js';

/**
 * Promote one approved immutable candidate by staging a DeliverySet and
 * publishing the stable pointer through the same locked narrative custody
 * session.  This function intentionally performs no blind retry: a custody
 * post-rename uncertainty remains trackb-pointer-reconcile-required.
 */
export async function promoteTrackBRelease(input, dependencies = {}) {
  const normalizedInput = normalizePromoteInput(input);
  const runtime = await openNarrativeRuntime(dependencies, normalizedInput.narrativeId);
  const { openNarrativeSession, registry } = runtime;
  let session;
  let operationFailure = null;

  try {
    const openOptions = {
      projectRoot: runtime.projectRoot,
      narrativeId: normalizedInput.narrativeId,
      readOnly: false
    };
    if (runtime.operationId !== undefined) openOptions.operationId = runtime.operationId;
    session = await openNarrativeSession(openOptions);
    assertNarrativeSession(session, { mutable: true });

    // The lock is acquired by the mutable custody open before any of these
    // content-addressed paths are resolved.
    const current = await resolveCurrentDeliveryBase(session, normalizedInput.narrativeId, registry);
    const subject = {
      ...normalizedInput,
      baseDeliverySetHash: normalizedInput.expectedBaseDeliverySetHash
    };
    const candidatePath = `candidates/${hashHex(normalizedInput.candidateHash)}/trackb-release-candidate-v4.json`;
    const candidate = await readRequiredNarrativeObject(session, candidatePath, {
      label: 'Track B candidate',
      missingCode: 'trackb-candidate-missing',
      parseCode: 'trackb-candidate-invalid'
    });
    validateCandidateForApproval(candidate, subject, registry);

    const authorityPath = `authority/${hashHex(candidate.authorityHash)}/trackb-narrative-authority-v2.json`;
    const authority = await readRequiredNarrativeObject(session, authorityPath, {
      label: 'Track B authority',
      missingCode: 'trackb-authority-missing',
      parseCode: 'trackb-storage-integrity'
    });
    validateAuthorityForApproval(authority, subject, candidate, registry);
    validateCandidateForApproval(candidate, subject, registry, authority);

    const approvalPath = `approvals/${hashHex(normalizedInput.candidateHash)}/${hashBaseSegment(normalizedInput.expectedBaseDeliverySetHash)}/trackb-release-approval-receipt-v2.json`;
    const approvalSnapshot = await readNarrativeSnapshot(session, approvalPath, 'snapshot');
    if (approvalSnapshot.exists !== true) {
      throw runtimeError('trackb-approval-required', 'the exact candidate/base approval receipt is absent');
    }
    const approval = parseCanonicalJsonBytes(
      decodeSnapshotBytes(approvalSnapshot, 'Track B approval receipt'),
      'Track B approval receipt',
      'trackb-storage-integrity'
    );
    validateStoredApprovalReceipt(approval, registry, dependencies.now, {
      candidateHash: normalizedInput.candidateHash,
      meaningQualificationHash: candidate.meaningQualificationHash,
      baseDeliverySetHash: normalizedInput.expectedBaseDeliverySetHash
    });
    if (approval.approvalHash !== normalizedInput.approvalHash) {
      throw runtimeError('trackb-approval-stale', 'approvalHash does not identify the exact approval for this candidate/base subject');
    }
    if (approval.decision !== 'APPROVE') {
      throw runtimeError('trackb-approval-not-promotable', 'only an APPROVE receipt can promote a candidate');
    }
    if (candidate.candidateVerdict !== 'PROMOTE') {
      throw runtimeError('trackb-delivery-not-ready', 'only a PROMOTE candidate can produce a DeliverySet');
    }

    // A retry of an already committed subject must be recognized before
    // treating the old pointer base as a normal CAS conflict.
    if (current.pointer
      && current.pointer.candidateHash === normalizedInput.candidateHash
      && current.pointer.meaningQualificationHash === candidate.meaningQualificationHash
      && current.pointer.approvalHash === normalizedInput.approvalHash
      && current.pointer.baseDeliverySetHash === normalizedInput.expectedBaseDeliverySetHash) {
      validateExistingPromotion(current, candidate, approval, registry);
      const reusedResult = buildPromoteResultFromPointer(current.pointer, normalizedInput.narrativeId, true);
      validateCommandResult('trackb.release.promote', reusedResult, normalizedInput);
      return Object.freeze(reusedResult);
    }

    if (current.baseDeliverySetHash !== normalizedInput.expectedBaseDeliverySetHash) {
      throw runtimeError(
        'trackb-cas-conflict',
        'expectedBaseDeliverySetHash does not match the verified current delivery pointer'
      );
    }

    const revision = current.pointer === undefined ? 1 : current.pointer.revision + 1;
    if (!Number.isInteger(revision) || revision < 1 || revision > 2147483647) {
      throw runtimeError('trackb-storage-integrity', 'next DeliverySet revision exceeds the contract bound');
    }
    const deliverySet = buildDeliverySet({
      narrativeId: normalizedInput.narrativeId,
      candidate,
      approval,
      baseDeliverySetHash: normalizedInput.expectedBaseDeliverySetHash,
      revision
    });
    validateGeneratedDeliverySet(deliverySet, candidate, approval, registry);
    const deliverySetBytes = Buffer.from(canonicalizeJson(deliverySet), 'utf8');
    assertBoundedWriteBytes(deliverySetBytes, 'delivery set');
    const deliverySetRelativePath = `delivery-sets/${hashHex(deliverySet.deliverySetHash)}/trackb-delivery-set-v2.json`;
    const deliverySetPath = `scenes/trackb/${normalizedInput.narrativeId}/${deliverySetRelativePath}`;
    const pointer = buildDeliveryPointer({
      narrativeId: normalizedInput.narrativeId,
      candidate,
      approval,
      deliverySet,
      deliverySetPath
    });
    validateGeneratedDeliveryPointer(pointer, deliverySet, candidate, approval, registry);
    const pointerBytes = Buffer.from(canonicalizeJson(pointer), 'utf8');
    assertBoundedWriteBytes(pointerBytes, 'delivery pointer');

    // The DeliverySet is immutable and is staged before the pointer CAS.  A
    // failed stage therefore cannot alter the stable pointer.
    const deliveryWrite = await session.writeImmutable(deliverySetRelativePath, deliverySetBytes);
    verifyImmutableWrite(deliveryWrite, deliverySetRelativePath, deliverySetBytes);
    const publishResult = await session.publishPointer({
      expectedBaseDeliverySetHash: normalizedInput.expectedBaseDeliverySetHash,
      expectedPointerSnapshotToken: current.pointerSnapshot.snapshotToken,
      pointerBytes,
      deliverySetPath,
      deliverySetBytes,
      deliverySetSnapshotToken: deliveryWrite.snapshot.snapshotToken
    });
    verifyPublishPointer(publishResult, pointerBytes);

    const result = {
      deliverySetPath,
      deliverySetHash: deliverySet.deliverySetHash,
      pointerPath: `scenes/trackb/${normalizedInput.narrativeId}/delivery-pointer-v2.json`,
      pointerHash: pointer.pointerHash,
      meaningQualificationHash: deliverySet.meaningQualificationHash,
      revision: deliverySet.revision,
      state: 'PROMOTED',
      reused: publishResult.reused
    };
    validateCommandResult('trackb.release.promote', result, normalizedInput);
    return Object.freeze(result);
  } catch (error) {
    operationFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B release promote failed');
    throw operationFailure;
  } finally {
    await closeNarrativeSession(session, operationFailure, 'Track B release promote');
  }
}

function normalizePromoteInput(input) {
  if (!isPlainObject(input)) throw runtimeError('trackb-input-invalid', 'Track B promote input must be an object');
  const validation = validateTrackBCommandInput('trackb.release.promote', input);
  throwValidationIfInvalid(validation, 'trackb-input-invalid', 'Track B promote input');
  return input;
}
