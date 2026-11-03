// Track B transport and Studio request contracts. Artifact state is validated by the artifact contract owner.
import {
  TRACK_B_HANDOFF_PATH,
  TRACK_B_PROJECTION_RECEIPT_PATH,
  TRACK_B_RELEASE_CANDIDATE_FILENAME,
  assertResult,
  contractError,
  exactKeys,
  isPlainObject,
  isTrackBBaseHash,
  isTrackBDateBeyondFutureLimit,
  isTrackBHash,
  isTrackBNarrativeId,
  isTrackBNonEmptyText,
  isTrackBOpaqueId,
  isTrackBSafeRelativePath,
  parseTrackBDateTime,
  resultWithErrors
} from './trackb-contract-primitives.js';

export const TRACK_B_LOOPBACK_ORIGIN_PATTERN = /^https?:\/\/(?:localhost|127\.0\.0\.1)(?::[0-9]{1,5})?$/;
export const TRACK_B_COMMAND_INPUT_KEYS = Object.freeze({
  'scene.projectNarrativeBundle': Object.freeze(['releaseBundlePath','continuityBiblePath','directorPlanPath','directorApprovalPath']),
  'scene.compileHtml': Object.freeze(['sceneMotionAssetHandoffPath']),
  'trackb.evidence.measure': Object.freeze(['narrativeAuthorityPath', 'htmlPath', 'provenancePath', 'outputPath']),
  'trackb.release.prepare': Object.freeze(['narrativeAuthorityPath', 'projectionReceiptPath', 'assetHandoffPath', 'htmlPath', 'provenancePath', 'evidenceReceiptPath', 'candidateId', 'expectedBaseDeliverySetHash']),
  'trackb.release.approve': Object.freeze(['narrativeId', 'candidateHash', 'baseDeliverySetHash', 'decision', 'reason', 'decidedAt']),
  'trackb.release.promote': Object.freeze(['narrativeId', 'candidateHash', 'approvalHash', 'expectedBaseDeliverySetHash']),
  'trackb.release.status': Object.freeze(['narrativeId'])
});

export const TRACK_B_COMMAND_OPTIONAL_INPUT_KEYS = Object.freeze({
  'scene.compileHtml': Object.freeze(['outputPath', 'width', 'height']),
  'trackb.release.status': Object.freeze(['candidateHash'])
});

export const TRACK_B_RELEASE_COMMANDS = Object.freeze([
  'trackb.evidence.measure',
  'trackb.release.prepare',
  'trackb.release.approve',
  'trackb.release.promote',
  'trackb.release.status'
]);
export const TRACK_B_STUDIO_EXECUTE_KEYS = Object.freeze(['command', 'input']);
export const TRACK_B_STUDIO_APPROVAL_KEYS = Object.freeze(['narrativeId', 'candidateHash', 'baseDeliverySetHash', 'decision', 'reason', 'decidedAt']);
export const TRACK_B_STUDIO_REQUEST_FRAME_KEYS = Object.freeze(['method', 'path', 'contentType', 'bodyBytes', 'origin', 'allowedOrigin', 'sessionValid']);
export const TRACK_B_STUDIO_MAX_BODY_BYTES = 8 * 1024 * 1024;
export const TRACK_B_STUDIO_ENDPOINT = '/api/execute';
export const TRACK_B_STUDIO_SESSION_ENDPOINT = '/api/review-session';
export const TRACK_B_STUDIO_COOKIE = Object.freeze({
  name: 'smc_trackb_review_session',
  path: '/api/execute',
  maxAgeSeconds: 900,
  sameSite: 'Strict',
  httpOnly: true,
  tokenBytes: 32
});
export const TRACK_B_COMMAND_RESULT_KEYS = Object.freeze({
  'trackb.evidence.measure': Object.freeze(['receiptPath', 'receiptHash', 'narrativeId', 'authorityHash', 'htmlHash', 'assetManifestHash', 'semanticManifestHash', 'rendererLockHash', 'shotIds', 'shotCount']),
  'trackb.release.prepare': Object.freeze(['candidatePath', 'candidateHash', 'authorityHash', 'meaningQualificationHash', 'evidenceReceiptHash', 'baseDeliverySetHash', 'candidateVerdict', 'evidenceCellCount']),
  'trackb.release.approve': Object.freeze(['approvalPath', 'approvalHash', 'subject', 'decision', 'reused']),
  'trackb.release.promote': Object.freeze(['deliverySetPath', 'deliverySetHash', 'pointerPath', 'pointerHash', 'meaningQualificationHash', 'revision', 'state', 'reused']),
  'trackb.release.status': Object.freeze(['narrativeId', 'pointer'])
});
export function isTrackBLoopbackOrigin(value) {
  if (typeof value !== 'string' || value.length === 0 || value.trim() !== value || !TRACK_B_LOOPBACK_ORIGIN_PATTERN.test(value)) return false;
  const portText = value.match(/:(\d+)$/)?.[1];
  return portText === undefined || (Number(portText) >= 1 && Number(portText) <= 65535);
}
export function validateTrackBCommandInput(command, input, { now } = {}) {
  const keys = TRACK_B_COMMAND_INPUT_KEYS[command];
  if (!keys) return resultWithErrors([contractError('/command', `unsupported Track B command: ${String(command)}`, 'trackb-transport-not-allowed')]);
  const errors = [];
  if (!isPlainObject(input)) return resultWithErrors([contractError('/input', 'command input must be an object', 'trackb-input-invalid')]);
  exactKeys(input, keys, errors, 'trackb-input-invalid', { optional: TRACK_B_COMMAND_OPTIONAL_INPUT_KEYS[command] ?? [] });
  if (command === 'scene.projectNarrativeBundle') {
    if (!isTrackBNonEmptyText(input.releaseBundlePath, 4096)) errors.push(contractError('/input/releaseBundlePath', 'releaseBundlePath must be a non-empty path', 'trackb-input-invalid'));
  } else if (command === 'scene.compileHtml') {
    if (!isTrackBNonEmptyText(input.sceneMotionAssetHandoffPath, 4096)) errors.push(contractError('/input/sceneMotionAssetHandoffPath', 'sceneMotionAssetHandoffPath must be a non-empty path', 'trackb-input-invalid'));
    if (input.outputPath !== undefined && !isTrackBNonEmptyText(input.outputPath, 4096)) errors.push(contractError('/input/outputPath', 'outputPath must be a non-empty path', 'trackb-input-invalid'));
    for (const key of ['width', 'height']) {
      if (input[key] !== undefined && (!Number.isSafeInteger(input[key]) || input[key] < 1 || input[key] > 8192)) errors.push(contractError(`/input/${key}`, `${key} must be an integer from 1 through 8192`, 'trackb-input-invalid'));
    }
  } else if (command === 'trackb.release.prepare') {
    for (const key of ['narrativeAuthorityPath', 'projectionReceiptPath', 'assetHandoffPath', 'htmlPath', 'provenancePath', 'evidenceReceiptPath']) if (!isTrackBSafeRelativePath(input[key])) errors.push(contractError(`/input/${key}`, `${key} must be a safe relative path`, 'trackb-input-invalid'));
    if (input.projectionReceiptPath !== TRACK_B_PROJECTION_RECEIPT_PATH) errors.push(contractError('/input/projectionReceiptPath', `projectionReceiptPath must be ${TRACK_B_PROJECTION_RECEIPT_PATH}`, 'trackb-input-invalid'));
    if (input.assetHandoffPath !== TRACK_B_HANDOFF_PATH) errors.push(contractError('/input/assetHandoffPath', `assetHandoffPath must be ${TRACK_B_HANDOFF_PATH}`, 'trackb-input-invalid'));
    if (!isTrackBOpaqueId(input.candidateId)) errors.push(contractError('/input/candidateId', 'candidateId is invalid', 'trackb-input-invalid'));
    if (!isTrackBBaseHash(input.expectedBaseDeliverySetHash)) errors.push(contractError('/input/expectedBaseDeliverySetHash', 'expectedBaseDeliverySetHash must be NONE or a hash', 'trackb-input-invalid'));
  } else if (command === 'trackb.evidence.measure') {
    for (const key of ['narrativeAuthorityPath', 'htmlPath', 'provenancePath', 'outputPath']) if (!isTrackBSafeRelativePath(input[key])) errors.push(contractError(`/input/${key}`, `${key} must be a safe relative path`, 'trackb-input-invalid'));
  } else if (command === 'trackb.release.approve') {
    if (!isTrackBNarrativeId(input.narrativeId)) errors.push(contractError('/input/narrativeId', 'narrativeId is invalid', 'trackb-input-invalid'));
    if (!isTrackBHash(input.candidateHash)) errors.push(contractError('/input/candidateHash', 'candidateHash is invalid', 'trackb-input-invalid'));
    if (!isTrackBBaseHash(input.baseDeliverySetHash)) errors.push(contractError('/input/baseDeliverySetHash', 'baseDeliverySetHash is invalid', 'trackb-input-invalid'));
    if (!['APPROVE', 'REVISE', 'REJECT'].includes(input.decision)) errors.push(contractError('/input/decision', 'decision is invalid', 'trackb-input-invalid'));
    if (!isTrackBNonEmptyText(input.reason, 2048)) errors.push(contractError('/input/reason', 'reason must be a non-empty string of at most 2048 characters', 'trackb-input-invalid'));
    const decidedAt = parseTrackBDateTime(input.decidedAt);
    if (!decidedAt) errors.push(contractError('/input/decidedAt', 'decidedAt must be a valid UTC RFC 3339 date-time', 'trackb-input-invalid'));
    else if (isTrackBDateBeyondFutureLimit(decidedAt, now)) errors.push(contractError('/input/decidedAt', 'decidedAt may not be more than five minutes in the future', 'trackb-input-invalid'));
  } else if (command === 'trackb.release.promote') {
    if (!isTrackBNarrativeId(input.narrativeId)) errors.push(contractError('/input/narrativeId', 'narrativeId is invalid', 'trackb-input-invalid'));
    if (!isTrackBHash(input.candidateHash)) errors.push(contractError('/input/candidateHash', 'candidateHash is invalid', 'trackb-input-invalid'));
    if (!isTrackBHash(input.approvalHash)) errors.push(contractError('/input/approvalHash', 'approvalHash is invalid', 'trackb-input-invalid'));
    if (!isTrackBBaseHash(input.expectedBaseDeliverySetHash)) errors.push(contractError('/input/expectedBaseDeliverySetHash', 'expectedBaseDeliverySetHash must be NONE or a hash', 'trackb-input-invalid'));
  } else if (command === 'trackb.release.status') {
    if (!isTrackBNarrativeId(input.narrativeId)) errors.push(contractError('/input/narrativeId', 'narrativeId is invalid', 'trackb-input-invalid'));
    if (input.candidateHash !== undefined && !isTrackBHash(input.candidateHash)) errors.push(contractError('/input/candidateHash', 'candidateHash is invalid', 'trackb-input-invalid'));
  }
  return resultWithErrors(errors);
}

export function validateTrackBCommandResult(command, result, { input } = {}) {
  const expected = TRACK_B_COMMAND_RESULT_KEYS[command];
  if (!expected) return resultWithErrors([contractError('/command', `unsupported Track B command: ${String(command)}`, 'trackb-transport-not-allowed')]);
  const errors = [];
  if (!isPlainObject(result)) return resultWithErrors([contractError('/result', 'command result must be an object', 'trackb-input-invalid')]);
  if (command === 'trackb.release.status') return validateTrackBStatusResult(result, { input });
  exactKeys(result, expected, errors, 'trackb-input-invalid');
  if (command === 'trackb.release.prepare') {
    for (const key of ['candidatePath']) if (!isTrackBSafeRelativePath(result[key])) errors.push(contractError(`/result/${key}`, `${key} must be a safe relative path`, 'trackb-input-invalid'));
    if (!result.candidatePath?.endsWith(`/${TRACK_B_RELEASE_CANDIDATE_FILENAME}`)) errors.push(contractError('/result/candidatePath', `candidatePath must end with ${TRACK_B_RELEASE_CANDIDATE_FILENAME}`, 'trackb-input-invalid'));
    for (const key of ['candidateHash', 'authorityHash', 'meaningQualificationHash', 'evidenceReceiptHash']) if (!isTrackBHash(result[key])) errors.push(contractError(`/result/${key}`, `${key} must be a hash`, 'trackb-input-invalid'));
    if (!isTrackBBaseHash(result.baseDeliverySetHash) || !['PROMOTE', 'ITERATE', 'FAIL'].includes(result.candidateVerdict) || !Number.isInteger(result.evidenceCellCount) || result.evidenceCellCount < 0 || result.evidenceCellCount > 2147483647) errors.push(contractError('/result', 'prepare result primitive is invalid', 'trackb-input-invalid'));
  } else if (command === 'trackb.evidence.measure') {
    if (!isTrackBSafeRelativePath(result.receiptPath) || !isTrackBHash(result.receiptHash) || !isTrackBNarrativeId(result.narrativeId) || !isTrackBHash(result.authorityHash) || !isTrackBHash(result.htmlHash) || !isTrackBHash(result.assetManifestHash) || !isTrackBHash(result.semanticManifestHash) || !isTrackBHash(result.rendererLockHash) || !Array.isArray(result.shotIds) || result.shotIds.length < 1 || result.shotIds.length > 64 || result.shotIds.some((id) => !isTrackBOpaqueId(id)) || !Number.isInteger(result.shotCount) || result.shotCount !== result.shotIds.length) errors.push(contractError('/result', 'measurement result shape is invalid', 'trackb-input-invalid'));
  } else if (command === 'trackb.release.approve') {
    if (!isTrackBSafeRelativePath(result.approvalPath) || !isTrackBHash(result.approvalHash) || !isPlainObject(result.subject) || !isTrackBHash(result.subject.candidateHash) || !isTrackBHash(result.subject.meaningQualificationHash) || !isTrackBBaseHash(result.subject.baseDeliverySetHash) || !['APPROVE', 'REVISE', 'REJECT'].includes(result.decision) || typeof result.reused !== 'boolean') errors.push(contractError('/result', 'approval result shape is invalid', 'trackb-input-invalid'));
    if (isPlainObject(result.subject)) exactKeys(result.subject, ['candidateHash', 'meaningQualificationHash', 'baseDeliverySetHash'], errors, 'trackb-input-invalid');
  } else if (command === 'trackb.release.promote') {
    if (!isTrackBSafeRelativePath(result.deliverySetPath) || !isTrackBSafeRelativePath(result.pointerPath) || !isTrackBHash(result.deliverySetHash) || !isTrackBHash(result.pointerHash) || !isTrackBHash(result.meaningQualificationHash) || !Number.isInteger(result.revision) || result.revision < 1 || result.revision > 2147483647 || result.state !== 'PROMOTED' || typeof result.reused !== 'boolean') errors.push(contractError('/result', 'promote result shape is invalid', 'trackb-input-invalid'));
  }
  return resultWithErrors(errors);
}

export function assertTrackBCommandInput(command, input, options = {}) {
  assertResult(validateTrackBCommandInput(command, input, options));
  return input;
}

export function validateTrackBStudioExecuteBody(body, { now } = {}) {
  const errors = [];
  if (!isPlainObject(body)) return resultWithErrors([contractError('/', 'Studio request body must be an object', 'trackb-input-invalid')]);
  exactKeys(body, TRACK_B_STUDIO_EXECUTE_KEYS, errors, 'trackb-input-invalid');
  if (body.command !== 'trackb.release.approve') errors.push(contractError('/command', 'Studio may invoke only trackb.release.approve', 'trackb-approval-forbidden'));
  if (isPlainObject(body.input)) {
    exactKeys(body.input, TRACK_B_STUDIO_APPROVAL_KEYS, errors, 'trackb-input-invalid');
    for (const forbidden of ['actorId', 'origin', 'reviewSessionId']) if (Object.hasOwn(body.input, forbidden)) errors.push(contractError(`/input/${forbidden}`, `${forbidden} is server session context and is not accepted from Studio body`, 'trackb-approval-forbidden'));
    errors.push(...validateTrackBCommandInput('trackb.release.approve', body.input, { now }).errors);
  } else errors.push(contractError('/input', 'Studio input must be an object', 'trackb-input-invalid'));
  return resultWithErrors(errors);
}

export function validateTrackBApplicationEnvelope(envelope, { command, input } = {}) {
  const errors = [];
  if (!isPlainObject(envelope)) return resultWithErrors([contractError('/', 'application envelope must be an object', 'trackb-input-invalid')]);
  const effectiveCommand = command ?? envelope.command;
  const allowed = envelope.ok === true ? ['schemaVersion', 'ok', 'command', 'metadata', 'result'] : ['schemaVersion', 'ok', 'command', 'metadata', 'error'];
  exactKeys(envelope, allowed, errors, 'trackb-input-invalid');
  if (envelope.schemaVersion !== 1 || typeof envelope.ok !== 'boolean' || (command !== undefined && envelope.command !== command)) errors.push(contractError('/', 'application envelope has invalid schemaVersion/ok/command', 'trackb-input-invalid'));
  if (!isPlainObject(envelope.metadata)) errors.push(contractError('/metadata', 'metadata must be an object', 'trackb-input-invalid'));
  if (envelope.ok === true && !Object.hasOwn(envelope, 'result')) errors.push(contractError('/result', 'successful envelope requires result', 'trackb-input-invalid'));
  if (effectiveCommand === 'trackb.release.status' && !isPlainObject(input)) errors.push(...trackBStatusInputContextErrors());
  if (envelope.ok === true && effectiveCommand !== undefined && Object.hasOwn(envelope, 'result')) errors.push(...validateTrackBCommandResult(effectiveCommand, envelope.result, { input }).errors);
  if (envelope.ok === false) {
    if (!isPlainObject(envelope.error)) errors.push(contractError('/error', 'failed envelope requires error object', 'trackb-input-invalid'));
    else {
      exactKeys(envelope.error, ['code', 'message', 'retryable'], errors, 'trackb-input-invalid', { optional: ['details'] });
      if (typeof envelope.error.code !== 'string' || typeof envelope.error.message !== 'string' || typeof envelope.error.retryable !== 'boolean') errors.push(contractError('/error', 'error code/message/retryable types are invalid', 'trackb-input-invalid'));
      if (Object.hasOwn(envelope.error, 'details') && envelope.error.details !== undefined && !isPlainObject(envelope.error.details)) errors.push(contractError('/error/details', 'details must be an object when supplied', 'trackb-input-invalid'));
    }
  }
  return resultWithErrors(errors);
}

export function validateTrackBStatusResult(value, { input } = {}) {
  const errors = [...validateTrackBStatusResultShape(value).errors];
  if (!isPlainObject(input)) return resultWithErrors([...errors, ...trackBStatusInputContextErrors()]);
  errors.push(...validateTrackBCommandInput('trackb.release.status', input).errors);
  if (isPlainObject(value) && value.narrativeId !== input.narrativeId) {
    errors.push(contractError('/narrativeId', 'status result narrativeId must equal status input narrativeId', 'trackb-narrative-mismatch'));
  }
  const inputHasCandidateHash = Object.hasOwn(input, 'candidateHash');
  const resultHasCandidateStatus = isPlainObject(value) && Object.hasOwn(value, 'candidateStatus');
  if (inputHasCandidateHash !== resultHasCandidateStatus) {
    errors.push(contractError('/candidateStatus', inputHasCandidateHash
      ? 'candidateStatus is required when status input includes candidateHash'
      : 'candidateStatus is forbidden when status input omits candidateHash', 'trackb-input-invalid'));
  }
  if (inputHasCandidateHash && resultHasCandidateStatus && value.candidateStatus?.candidateHash !== input.candidateHash) {
    errors.push(contractError('/candidateStatus/candidateHash', 'candidateStatus.candidateHash must equal status input candidateHash', 'trackb-input-invalid'));
  }
  return resultWithErrors(errors);
}

/**
 * Validate only the transport-shaped status projection. Callers validating a
 * command result must use validateTrackBStatusResult so the original request
 * input can bind candidateStatus presence and candidateHash.
 */
export function validateTrackBStatusResultShape(value) {
  const errors = [];
  if (!isPlainObject(value)) return resultWithErrors([contractError('/', 'status result must be an object', 'trackb-input-invalid')]);
  if (!isTrackBNarrativeId(value.narrativeId)) errors.push(contractError('/narrativeId', 'narrativeId is invalid', 'trackb-input-invalid'));
  if (!isPlainObject(value.pointer)) errors.push(contractError('/pointer', 'pointer projection is required', 'trackb-input-invalid'));
  else validatePointerProjection(value.pointer, errors);
  if (Object.hasOwn(value, 'candidateStatus')) validateCandidateStatus(value.candidateStatus, errors);
  exactKeys(value, Object.hasOwn(value, 'candidateStatus') ? ['narrativeId', 'pointer', 'candidateStatus'] : ['narrativeId', 'pointer'], errors, 'trackb-input-invalid');
  return resultWithErrors(errors);
}

export function assertTrackBStatusResult(value, options = {}) {
  assertResult(validateTrackBStatusResult(value, options));
  return value;
}
function validatePointerProjection(pointer, errors) {
  if (pointer.state === 'ABSENT') {
    exactKeys(pointer, ['state'], errors, 'trackb-input-invalid');
    return;
  }
  if (pointer.state !== 'PROMOTED') errors.push(contractError('/pointer/state', 'pointer state must be ABSENT or PROMOTED', 'trackb-input-invalid'));
  exactKeys(pointer, ['state', 'currentDeliverySetHash', 'candidateHash', 'approvalHash', 'meaningQualificationHash', 'pointerRevision', 'pointerHash'], errors, 'trackb-input-invalid');
  for (const field of ['currentDeliverySetHash', 'candidateHash', 'approvalHash', 'meaningQualificationHash', 'pointerHash']) if (!isTrackBHash(pointer[field])) errors.push(contractError(`/pointer/${field}`, `${field} must be a hash`, 'trackb-input-invalid'));
  if (!Number.isInteger(pointer.pointerRevision) || pointer.pointerRevision < 1 || pointer.pointerRevision > 2147483647) errors.push(contractError('/pointer/pointerRevision', 'pointerRevision must be an integer 1..2147483647', 'trackb-input-invalid'));
}

function validateCandidateStatus(status, errors) {
  if (!isPlainObject(status) || typeof status.state !== 'string') {
    errors.push(contractError('/candidateStatus', 'candidateStatus must be a strict variant', 'trackb-input-invalid'));
    return;
  }
  const common = ['state', 'candidateHash', 'candidateVerdict'];
  const variants = {
    READY: [...common, 'baseDeliverySetHash', 'meaningQualificationHash'],
    APPROVED: [...common, 'baseDeliverySetHash', 'meaningQualificationHash', 'approvalHash', 'decision'],
    REVISE: [...common, 'baseDeliverySetHash', 'meaningQualificationHash', 'approvalHash', 'decision'],
    REJECT: [...common, 'baseDeliverySetHash', 'meaningQualificationHash', 'approvalHash', 'decision'],
    PROMOTED: [...common, 'baseDeliverySetHash', 'meaningQualificationHash', 'deliverySetHash', 'approvalHash'],
    ITERATE: [...common],
    FAIL: [...common]
  };
  const keys = variants[status.state];
  if (!keys) {
    errors.push(contractError('/candidateStatus/state', 'candidateStatus.state is invalid', 'trackb-input-invalid'));
    return;
  }
  exactKeys(status, keys, errors, 'trackb-input-invalid');
  if (!isTrackBHash(status.candidateHash)) errors.push(contractError('/candidateStatus/candidateHash', 'candidateHash is invalid', 'trackb-input-invalid'));
  if (['READY', 'APPROVED', 'REVISE', 'REJECT', 'PROMOTED'].includes(status.state) && !isTrackBHash(status.meaningQualificationHash)) errors.push(contractError('/candidateStatus/meaningQualificationHash', 'meaningQualificationHash is invalid', 'trackb-input-invalid'));
  if (!['PROMOTE', 'ITERATE', 'FAIL'].includes(status.candidateVerdict)) errors.push(contractError('/candidateStatus/candidateVerdict', 'candidateVerdict is invalid', 'trackb-input-invalid'));
  const requiredVerdict = {
    READY: 'PROMOTE',
    APPROVED: 'PROMOTE',
    REVISE: 'PROMOTE',
    REJECT: 'PROMOTE',
    PROMOTED: 'PROMOTE',
    ITERATE: 'ITERATE',
    FAIL: 'FAIL'
  }[status.state];
  if (requiredVerdict && status.candidateVerdict !== requiredVerdict) errors.push(contractError('/candidateStatus/candidateVerdict', `${status.state} candidateStatus requires candidateVerdict=${requiredVerdict}`, 'trackb-input-invalid'));
  if (Object.hasOwn(status, 'baseDeliverySetHash') && !isTrackBBaseHash(status.baseDeliverySetHash)) errors.push(contractError('/candidateStatus/baseDeliverySetHash', 'baseDeliverySetHash is invalid', 'trackb-input-invalid'));
  if (['APPROVED', 'REVISE', 'REJECT'].includes(status.state)) {
    const expected = status.state === 'APPROVED' ? 'APPROVE' : status.state;
    if (status.decision !== expected || !isTrackBHash(status.approvalHash)) errors.push(contractError('/candidateStatus', `candidateStatus ${status.state} requires matching decision and approvalHash`, 'trackb-input-invalid'));
  }
  if (status.state === 'PROMOTED' && (!isTrackBHash(status.deliverySetHash) || !isTrackBHash(status.approvalHash))) errors.push(contractError('/candidateStatus', 'PROMOTED status requires deliverySetHash and approvalHash', 'trackb-input-invalid'));
}
function trackBStatusInputContextErrors() {
  return [contractError('/input', 'original status command input is required to validate the status result', 'trackb-input-invalid')];
}
