import {
  assertResult,
  contractError,
  hashTrackBApprovalReceipt,
  hashTrackBDeliveryPointer,
  hashTrackBDeliverySet,
  isPlainObject,
  isTrackBDateBeyondFutureLimit,
  isTrackBNonEmptyText,
  isTrackBSafeRelativePath,
  parseTrackBDateTime,
  resultWithErrors
} from '../trackb-contract-primitives.js';
import { validateSchemaValue } from './schema.js';

export function validateTrackBApprovalReceipt(value, { registry, candidateHash, meaningQualificationHash, baseDeliverySetHash, now } = {}) {
  const result = validateSchemaValue('trackBReleaseApprovalReceipt', value, registry);
  if (!result.valid) return result;
  const errors = [];
  if (hashTrackBApprovalReceipt(value) !== value.approvalHash) errors.push(contractError('/approvalHash', 'approvalHash does not match its canonical self-hash', 'trackb-candidate-invalid'));
  if (candidateHash !== undefined && value.subject.candidateHash !== candidateHash) errors.push(contractError('/subject/candidateHash', 'approval subject candidateHash does not match candidate', 'trackb-approval-stale'));
  if (meaningQualificationHash !== undefined && value.subject.meaningQualificationHash !== meaningQualificationHash) errors.push(contractError('/subject/meaningQualificationHash', 'approval subject meaningQualificationHash does not match candidate', 'trackb-approval-stale'));
  if (baseDeliverySetHash !== undefined && value.subject.baseDeliverySetHash !== baseDeliverySetHash) errors.push(contractError('/subject/baseDeliverySetHash', 'approval subject baseDeliverySetHash does not match candidate base', 'trackb-approval-stale'));
  if (value.actor.actorKind !== 'human' || value.origin !== 'studio') errors.push(contractError('/', 'approval must be issued by a human Studio session', 'trackb-approval-forbidden'));
  if (!isTrackBNonEmptyText(value.reason, 2048)) errors.push(contractError('/reason', 'reason must be a non-empty string of at most 2048 characters', 'trackb-input-invalid'));
  const decidedAt = parseTrackBDateTime(value.decidedAt);
  if (!decidedAt) errors.push(contractError('/decidedAt', 'decidedAt must be a valid UTC RFC 3339 date-time', 'trackb-input-invalid'));
  else if (isTrackBDateBeyondFutureLimit(decidedAt, now)) errors.push(contractError('/decidedAt', 'decidedAt may not be more than five minutes in the future', 'trackb-input-invalid'));
  return resultWithErrors(errors);
}

export function assertTrackBApprovalReceipt(value, options = {}) {
  assertResult(validateTrackBApprovalReceipt(value, options));
  return value;
}

export function validateTrackBDeliverySet(value, { registry, candidate, approval } = {}) {
  const result = validateSchemaValue('trackBDeliverySet', value, registry);
  if (!result.valid) return result;
  const errors = [];
  if (hashTrackBDeliverySet(value) !== value.deliverySetHash) errors.push(contractError('/deliverySetHash', 'deliverySetHash does not match its canonical self-hash', 'trackb-candidate-invalid'));
  if (candidate) {
    if (!isPlainObject(candidate) || !isPlainObject(candidate.artifacts) || !Array.isArray(candidate.evidenceCells)) {
      errors.push(contractError('/candidate', 'candidate must contain artifacts and evidenceCells before delivery binding', 'trackb-candidate-invalid'));
    } else {
      if (value.candidateHash !== candidate.candidateHash || value.authorityHash !== candidate.authorityHash || value.meaningQualificationHash !== candidate.meaningQualificationHash || value.baseDeliverySetHash !== candidate.baseDeliverySetHash) errors.push(contractError('/', 'delivery set is not bound to candidate', 'trackb-candidate-invalid'));
      for (const field of ['projectionReceiptHash', 'sceneGraphHash', 'assetHandoffHash', 'assetHandoffSceneGraphHash', 'htmlHash', 'provenanceHash']) {
        if (value.artifacts[field] !== candidate.artifacts[field]) errors.push(contractError(`/artifacts/${field}`, `delivery set artifact ${field} does not match candidate`, 'trackb-artifact-binding-invalid'));
      }
      if (candidate.candidateVerdict !== 'PROMOTE') errors.push(contractError('/candidateHash', 'only a PROMOTE candidate can produce a delivery set', 'trackb-delivery-not-ready'));
      if (value.evidenceCellHashes.length !== candidate.evidenceCells.length || value.evidenceCellHashes.some((hash, index) => hash !== candidate.evidenceCells[index].evidenceCellHash)) errors.push(contractError('/evidenceCellHashes', 'delivery set evidence cell hashes are not in candidate order', 'trackb-evidence-incomplete'));
      if (value.narrativeId !== candidate.narrativeId) errors.push(contractError('/narrativeId', 'delivery set narrativeId does not match candidate', 'trackb-narrative-mismatch'));
    }
  }
  if (approval) {
    if (value.approvalHash !== approval.approvalHash || value.meaningQualificationHash !== approval.subject?.meaningQualificationHash) errors.push(contractError('/approvalHash', 'delivery set is not bound to approval and meaning qualification', 'trackb-approval-stale'));
    if (approval.decision !== 'APPROVE') errors.push(contractError('/approvalHash', 'only APPROVE receipts can produce a delivery set', 'trackb-approval-not-promotable'));
    if (approval.subject?.candidateHash !== value.candidateHash || approval.subject?.meaningQualificationHash !== value.meaningQualificationHash || approval.subject?.baseDeliverySetHash !== value.baseDeliverySetHash) errors.push(contractError('/approvalHash', 'approval subject does not match delivery set candidate/qualification/base', 'trackb-approval-stale'));
  }
  return resultWithErrors(errors);
}

export function assertTrackBDeliverySet(value, options = {}) {
  assertResult(validateTrackBDeliverySet(value, options));
  return value;
}

export function validateTrackBDeliveryPointer(value, { registry, deliverySet, candidate, approval } = {}) {
  const result = validateSchemaValue('trackBDeliveryPointer', value, registry);
  if (!result.valid) return result;
  const errors = [];
  if (!isTrackBSafeRelativePath(value.deliverySetPath)) errors.push(contractError('/deliverySetPath', 'deliverySetPath must be a safe project-relative path', 'trackb-storage-integrity'));
  if (hashTrackBDeliveryPointer(value) !== value.pointerHash) errors.push(contractError('/pointerHash', 'pointerHash does not match its canonical self-hash', 'trackb-storage-integrity'));
  if (deliverySet) {
    for (const field of ['narrativeId', 'revision', 'state', 'deliverySetHash', 'candidateHash', 'approvalHash', 'meaningQualificationHash', 'baseDeliverySetHash']) {
      if (value[field] !== deliverySet[field]) errors.push(contractError(`/${field}`, `pointer ${field} does not match delivery set`, 'trackb-storage-integrity'));
    }
  }
  if (candidate) {
    if (!isPlainObject(candidate)) errors.push(contractError('/candidate', 'candidate must be an object before pointer binding', 'trackb-storage-integrity'));
    else {
      if (value.candidateHash !== candidate.candidateHash || value.meaningQualificationHash !== candidate.meaningQualificationHash || value.baseDeliverySetHash !== candidate.baseDeliverySetHash) errors.push(contractError('/', 'pointer is not bound to candidate', 'trackb-storage-integrity'));
      if (candidate.candidateVerdict !== 'PROMOTE') errors.push(contractError('/candidateHash', 'pointer requires a PROMOTE candidate', 'trackb-delivery-not-ready'));
    }
  }
  if (approval) {
    if (value.approvalHash !== approval.approvalHash || value.meaningQualificationHash !== approval.subject?.meaningQualificationHash) errors.push(contractError('/approvalHash', 'pointer is not bound to approval and meaning qualification', 'trackb-storage-integrity'));
    if (approval.decision !== 'APPROVE') errors.push(contractError('/approvalHash', 'pointer requires an APPROVE receipt', 'trackb-approval-not-promotable'));
    if (approval.subject?.candidateHash !== value.candidateHash || approval.subject?.meaningQualificationHash !== value.meaningQualificationHash || approval.subject?.baseDeliverySetHash !== value.baseDeliverySetHash) errors.push(contractError('/approvalHash', 'pointer approval subject does not match candidate/qualification/base', 'trackb-approval-stale'));
  }
  return resultWithErrors(errors);
}

export function assertTrackBDeliveryPointer(value, options = {}) {
  assertResult(validateTrackBDeliveryPointer(value, options));
  return value;
}
