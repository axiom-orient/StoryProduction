import { decodeSnapshotBytes, parseCanonicalJsonBytes } from './trackb-release-runtime-artifacts.js';
import {
  assertNarrativeSession,
  closeNarrativeSession,
  openNarrativeRuntime,
  readNarrativeSnapshot,
  readRequiredNarrativeObject,
  validateCommandResult
} from './trackb-release-runtime-custody.js';
import {
  validateAuthorityForApproval,
  validateCandidateForApproval,
  validateStoredApprovalReceipt
} from './trackb-release-runtime-approval.js';
import {
  buildPointerProjection,
  deriveCandidateStatus,
  resolveCurrentDeliveryBase
} from './trackb-release-runtime-delivery.js';
import { hashBaseSegment, hashHex, mapRuntimeError, runtimeError, throwValidationIfInvalid } from './trackb-release-runtime-support.js';
import { isPlainObject } from './trackb-application-primitives.js';
import { validateTrackBCommandInput } from '../../contracts/src/trackb-contract.js';

/**
 * Read the verified current pointer and optional candidate state without
 * acquiring mutation custody or writing any stable artifact.
 */
export async function statusTrackBRelease(input, dependencies = {}) {
  const normalizedInput = normalizeStatusInput(input);
  const runtime = await openNarrativeRuntime(dependencies, normalizedInput.narrativeId);
  const { openNarrativeSession, registry } = runtime;
  let session;
  let operationFailure = null;

  try {
    const openOptions = {
      projectRoot: runtime.projectRoot,
      narrativeId: normalizedInput.narrativeId,
      readOnly: true
    };
    if (runtime.operationId !== undefined) openOptions.operationId = runtime.operationId;
    session = await openNarrativeSession(openOptions);
    assertNarrativeSession(session, { mutable: false });

    const current = await resolveCurrentDeliveryBase(session, normalizedInput.narrativeId, registry);
    const result = {
      narrativeId: normalizedInput.narrativeId,
      pointer: current.pointer
        ? buildPointerProjection(current.pointer)
        : { state: 'ABSENT' }
    };

    if (Object.hasOwn(normalizedInput, 'candidateHash')) {
      const candidatePath = `candidates/${hashHex(normalizedInput.candidateHash)}/trackb-release-candidate-v4.json`;
      const candidate = await readRequiredNarrativeObject(session, candidatePath, {
        label: 'Track B candidate',
        missingCode: 'trackb-candidate-missing',
        parseCode: 'trackb-candidate-invalid'
      });
      const candidateSubject = {
        narrativeId: normalizedInput.narrativeId,
        candidateHash: normalizedInput.candidateHash,
        baseDeliverySetHash: candidate.baseDeliverySetHash
      };
      validateCandidateForApproval(candidate, candidateSubject, registry);
      const authorityPath = `authority/${hashHex(candidate.authorityHash)}/trackb-narrative-authority-v2.json`;
      const authority = await readRequiredNarrativeObject(session, authorityPath, {
        label: 'Track B authority',
        missingCode: 'trackb-authority-missing',
        parseCode: 'trackb-storage-integrity'
      });
      validateAuthorityForApproval(authority, candidateSubject, candidate, registry);
      validateCandidateForApproval(candidate, candidateSubject, registry, authority);

      const approvalPath = `approvals/${hashHex(candidate.candidateHash)}/${hashBaseSegment(candidate.baseDeliverySetHash)}/trackb-release-approval-receipt-v2.json`;
      const approvalSnapshot = await readNarrativeSnapshot(session, approvalPath, 'snapshot');
      let approval;
      if (approvalSnapshot.exists === true) {
        const approvalBytes = decodeSnapshotBytes(approvalSnapshot, 'approval receipt');
        approval = parseCanonicalJsonBytes(approvalBytes, 'Track B approval receipt', 'trackb-storage-integrity');
        validateStoredApprovalReceipt(approval, registry, dependencies.now, {
          candidateHash: candidate.candidateHash,
          meaningQualificationHash: candidate.meaningQualificationHash,
          baseDeliverySetHash: candidate.baseDeliverySetHash
        });
      }
      result.candidateStatus = deriveCandidateStatus(candidate, approval, current, registry);
    }

    validateCommandResult('trackb.release.status', result, normalizedInput);
    return Object.freeze(result);
  } catch (error) {
    operationFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B release status failed');
    throw operationFailure;
  } finally {
    await closeNarrativeSession(session, operationFailure, 'Track B release status');
  }
}

function normalizeStatusInput(input) {
  if (!isPlainObject(input)) throw runtimeError('trackb-input-invalid', 'Track B status input must be an object');
  const validation = validateTrackBCommandInput('trackb.release.status', input);
  throwValidationIfInvalid(validation, 'trackb-input-invalid', 'Track B status input');
  return input;
}
