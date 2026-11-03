import { isAbsolute } from 'node:path';
import { openTrackBCustodySession } from '../../store/src/index.js';
import {
  isTrackBOpaqueId,
  validateTrackBCommandResult
} from '../../contracts/src/trackb-contract.js';
import { throwValidationIfInvalid } from './trackb-release-bindings.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';
import { canonicalizeJson } from '../../core/src/canonical.js';
import { loadTrackBOnlyRegistry } from './trackb-release-runtime-registry.js';
import { decodeSnapshotBytes, parseCanonicalJsonBytes } from './trackb-release-runtime-artifacts.js';
import {
  assertBoundedWriteBytes,
  attachCloseFailure,
  verifyCloseResult,
  verifyImmutableWrite,
  verifySnapshotResult
} from './trackb-release-runtime-custody.js';
import {
  buildApprovalReceipt,
  normalizeApprovalContext,
  normalizeApprovalInput,
  validateAuthorityForApproval,
  validateCandidateForApproval,
  validateStoredApprovalReceipt
} from './trackb-release-runtime-approval.js';
import { resolveCurrentDeliveryBase } from './trackb-release-runtime-delivery.js';
import { hashBaseSegment, hashHex, mapRuntimeError, runtimeError } from './trackb-release-runtime-support.js';

/**
 * Record one human Studio decision against an immutable candidate.
 *
 * The execution context is deliberately separate from the command body. It
 * is supplied by the trusted Studio boundary and cannot be spoofed through
 * Track B input. This core opens a narrative custody session, never publishes
 * the stable pointer, and stages only the approval receipt for the exact
 * candidate/base subject.
 */
export async function approveTrackBRelease(input, executionContext, dependencies = {}) {
  const normalizedInput = normalizeApprovalInput(input, dependencies?.now);
  const context = normalizeApprovalContext(executionContext);
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'Track B approval dependencies must be an object');
  if (typeof dependencies.projectRoot !== 'string' || !isAbsolute(dependencies.projectRoot)) {
    throw runtimeError('trackb-input-invalid', 'approval dependencies must provide an absolute projectRoot');
  }
  if (dependencies.operationId !== undefined && !isTrackBOpaqueId(dependencies.operationId)) {
    throw runtimeError('trackb-input-invalid', 'operationId must be a valid opaque ID when provided');
  }

  const openNarrativeSession = dependencies.openNarrativeSession
    ?? dependencies.openSession
    ?? openTrackBCustodySession;
  if (typeof openNarrativeSession !== 'function') throw runtimeError('trackb-input-invalid', 'openNarrativeSession must be a function');
  const workflowDependencies = isPlainObject(dependencies.workflowDeps)
    ? dependencies.workflowDeps
    : Object.fromEntries(Object.entries(dependencies).filter(([key]) => ![
      'projectRoot',
      'operationId',
      'openNarrativeSession',
      'openSession',
      'workflowDeps',
      'now'
    ].includes(key)));
  const registry = await loadTrackBOnlyRegistry(workflowDependencies);

  let session;
  let operationFailure = null;
  try {
    const openOptions = {
      projectRoot: dependencies.projectRoot,
      narrativeId: normalizedInput.narrativeId,
      readOnly: false
    };
    if (dependencies.operationId !== undefined) openOptions.operationId = dependencies.operationId;
    session = await openNarrativeSession(openOptions);
    if (!session
      || typeof session.read !== 'function'
      || typeof session.snapshot !== 'function'
      || typeof session.writeImmutable !== 'function'
      || typeof session.close !== 'function') {
      throw runtimeError('trackb-storage-integrity', 'narrative custody session is incomplete');
    }

    const candidatePath = `candidates/${hashHex(normalizedInput.candidateHash)}/trackb-release-candidate-v4.json`;
    const candidateSnapshot = await session.read(candidatePath);
    verifySnapshotResult(candidateSnapshot, { path: candidatePath });
    if (candidateSnapshot.exists !== true) {
      throw runtimeError('trackb-candidate-missing', 'requested Track B candidate is absent');
    }
    const candidateBytes = decodeSnapshotBytes(candidateSnapshot, 'candidate');
    const candidate = parseCanonicalJsonBytes(candidateBytes, 'Track B candidate', 'trackb-candidate-invalid');
    // Validate the candidate shape and requested subject before deriving the
    // authority pathname from candidate-controlled fields. The second pass
    // below adds the authoritative NarrativeAuthority relation.
    validateCandidateForApproval(candidate, normalizedInput, registry);
    const authorityPath = `authority/${hashHex(candidate.authorityHash)}/trackb-narrative-authority-v2.json`;
    const authoritySnapshot = await session.read(authorityPath);
    verifySnapshotResult(authoritySnapshot, { path: authorityPath });
    if (authoritySnapshot.exists !== true) {
      throw runtimeError('trackb-authority-missing', 'candidate-bound NarrativeAuthority is absent');
    }
    const authorityBytes = decodeSnapshotBytes(authoritySnapshot, 'authority');
    const authority = parseCanonicalJsonBytes(authorityBytes, 'Track B authority', 'trackb-storage-integrity');
    validateAuthorityForApproval(authority, normalizedInput, candidate, registry);
    validateCandidateForApproval(candidate, normalizedInput, registry, authority);
    if (normalizedInput.decision === 'APPROVE' && candidate.candidateVerdict !== 'PROMOTE') {
      throw runtimeError('trackb-delivery-not-ready', 'only a PROMOTE candidate can receive APPROVE');
    }

    const currentBase = await resolveCurrentDeliveryBase(session, normalizedInput.narrativeId, registry);
    const baseDeliverySetHash = currentBase.baseDeliverySetHash;
    if (normalizedInput.baseDeliverySetHash !== baseDeliverySetHash) {
      throw runtimeError(
        'trackb-cas-conflict',
        'baseDeliverySetHash does not match the verified current delivery pointer'
      );
    }

    const approval = buildApprovalReceipt({
      input: normalizedInput,
      context,
      candidate,
      registry,
      now: dependencies.now
    });
    const approvalBytes = Buffer.from(canonicalizeJson(approval), 'utf8');
    assertBoundedWriteBytes(approvalBytes, 'approval');
    const approvalPath = `approvals/${hashHex(normalizedInput.candidateHash)}/${hashBaseSegment(baseDeliverySetHash)}/trackb-release-approval-receipt-v2.json`;
    const existingSnapshot = await session.snapshot(approvalPath);
    verifySnapshotResult(existingSnapshot, { path: approvalPath });

    let reused;
    if (existingSnapshot.exists === true) {
      const existingBytes = decodeSnapshotBytes(existingSnapshot, 'existing approval');
      const existingApproval = parseCanonicalJsonBytes(existingBytes, 'existing Track B approval', 'trackb-storage-integrity');
      validateStoredApprovalReceipt(existingApproval, registry, dependencies.now, {
        candidateHash: normalizedInput.candidateHash,
        meaningQualificationHash: candidate.meaningQualificationHash,
        baseDeliverySetHash
      });
      if (!existingBytes.equals(approvalBytes)) {
        throw runtimeError(
          'trackb-approval-conflict',
          'an approval receipt already exists for this candidate/base subject'
        );
      }
      reused = true;
    } else {
      const writeResult = await session.writeImmutable(approvalPath, approvalBytes);
      verifyImmutableWrite(writeResult, approvalPath, approvalBytes);
      reused = writeResult.reused;
    }

    const result = {
      approvalPath,
      approvalHash: approval.approvalHash,
      subject: {
        candidateHash: normalizedInput.candidateHash,
        meaningQualificationHash: candidate.meaningQualificationHash,
        baseDeliverySetHash
      },
      decision: normalizedInput.decision,
      reused
    };
    const resultValidation = validateTrackBCommandResult('trackb.release.approve', result, { input: normalizedInput });
    throwValidationIfInvalid(resultValidation, 'trackb-input-invalid', 'Track B approval result');
    return Object.freeze(result);
  } catch (error) {
    operationFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B release approval failed');
    throw operationFailure;
  } finally {
    if (session) {
      try {
        const closeResult = await session.close();
        verifyCloseResult(closeResult);
      } catch (error) {
        const closeFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B custody session close failed');
        if (!operationFailure) {
          throw runtimeError('trackb-storage-integrity', 'Track B custody session close failed', {
            cause: closeFailure,
            details: { closeCode: closeFailure.code, closeMessage: boundedMessage(closeFailure.message) }
          });
        }
        attachCloseFailure(operationFailure, closeFailure);
      }
    }
  }
}
