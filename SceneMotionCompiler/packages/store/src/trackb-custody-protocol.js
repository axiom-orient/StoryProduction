import { hashTrackBRawBytes } from '../../contracts/src/trackb-contract.js';

export const TRACKB_CUSTODY_SHA256_PATTERN = /^sha256:[0-9a-f]{64}$/;
const SAFE_ID = /^[A-Za-z][A-Za-z0-9._:-]{0,127}$/;
const SAFE_TARGET_TRIPLE = /^[A-Za-z0-9._-]{1,127}$/;

export const TRACKB_CUSTODY_BINARY_NAME = 'smc-trackb-custody';
export const TRACKB_CUSTODY_PROTOCOL_VERSION = 1;
// Requests and responses are JSON-lines. Measurement returns exact HTML bytes
// as URL-safe base64, so a 16 MiB HTML artifact needs a line bound above 22
// MiB once JSON framing is included. Other operations remain constrained by
// their own smaller payload bounds.
export const TRACKB_CUSTODY_MAX_LINE_BYTES = 24 * 1024 * 1024;
export const TRACKB_CUSTODY_MAX_READ_BYTES = 8 * 1024 * 1024;
export const TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES = 8 * 1024 * 1024;
export const TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES = 16 * 1024 * 1024;
export const TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES = 1 * 1024 * 1024;
export const TRACKB_CUSTODY_MAX_MEASUREMENT_RECEIPT_BYTES = 64 * 1024;
export const TRACKB_CUSTODY_MAX_WRITE_BYTES = 1024 * 1024;
export const TRACKB_CUSTODY_MAX_POINTER_BYTES = 1024 * 1024;
export const TRACKB_CUSTODY_MAX_RECEIPT_BYTES = 64 * 1024;
export const TRACKB_CUSTODY_MAX_BINARY_BYTES = 128 * 1024 * 1024;
export const MAX_PROJECT_ROOT_BYTES = 1024;
export const MAX_PROJECT_ROOT_COMPONENT_BYTES = 255;
export const TRACKB_CUSTODY_RECEIPT_NAME = `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`;
export const TRACKB_PROJECT_ARTIFACT_KINDS = Object.freeze([
  'narrativeAuthority',
  'projectionReceipt',
  'assetHandoff',
  'html',
  'provenance',
  'evidenceReceipt',
  'sceneGraph',
]);
export const TRACKB_MEASUREMENT_ARTIFACT_KINDS = Object.freeze([
  'narrativeAuthority',
  'html',
  'provenance',
]);

export class TrackBCustodyError extends Error {
  constructor(code, message, { retryable = false, details = undefined, cause = undefined } = {}) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = 'TrackBCustodyError';
    this.code = code;
    this.retryable = retryable;
    if (details !== undefined) this.details = details;
  }
}

export function verifyProjectArtifactResult(result, { kind, path }) {
  if (!result || typeof result !== 'object' || Array.isArray(result)) {
    throw storageIntegrity('project artifact response must be an object');
  }
  if (result.kind !== kind || result.path !== path || typeof result.exists !== 'boolean') {
    throw storageIntegrity('project artifact response identity is invalid');
  }
  const expectedKeys = result.exists
    ? ['kind', 'path', 'exists', 'bytesBase64', 'byteLength', 'sha256']
    : ['kind', 'path', 'exists'];
  const actualKeys = Object.keys(result).sort();
  if (actualKeys.length !== expectedKeys.length || expectedKeys.some((key) => !actualKeys.includes(key))) {
    throw storageIntegrity('project artifact response fields are invalid');
  }
  if (!result.exists) return result;
  verifyHashedBytes(result, {
    context: 'project artifact response',
    maximumBytes: TRACKB_CUSTODY_MAX_READ_BYTES,
    byteLengthError: 'project artifact response byteLength is invalid',
    sha256Error: 'project artifact response sha256 is invalid',
  });
  return result;
}

export function verifyMeasurementArtifactResult(result, { kind, path } = {}) {
  assertObjectResponse(result, 'readMeasurementArtifact');
  if (!TRACKB_MEASUREMENT_ARTIFACT_KINDS.includes(kind)) {
    throw storageIntegrity('measurement artifact kind is invalid');
  }
  if (result.kind !== kind || result.path !== path || typeof result.exists !== 'boolean') {
    throw storageIntegrity('measurement artifact response identity is invalid');
  }
  const expectedKeys = result.exists
    ? ['kind', 'path', 'exists', 'bytesBase64', 'byteLength', 'sha256']
    : ['kind', 'path', 'exists'];
  assertExactKeys(result, expectedKeys, 'readMeasurementArtifact');
  if (!result.exists) return result;
  verifyHashedBytes(result, {
    context: 'measurement artifact response',
    maximumBytes: () => measurementArtifactMaximum(kind),
    byteLengthError: `measurement artifact response byteLength is invalid for ${kind}`,
    sha256Error: 'measurement artifact response sha256 is invalid',
  });
  return result;
}

export function verifyMeasurementPublishResult(result, { path, bytes } = {}) {
  assertObjectResponse(result, 'publishMeasurementReceipt');
  if (!(Buffer.isBuffer(bytes) || bytes instanceof Uint8Array)) {
    throw storageIntegrity('measurement receipt publish bytes must be bytes');
  }
  assertExactKeys(result, ['path', 'reused', 'snapshot'], 'publishMeasurementReceipt');
  if (result.path !== path || typeof result.reused !== 'boolean') {
    throw storageIntegrity('measurement receipt publish response identity is invalid');
  }
  verifySnapshotResult(result.snapshot);
  if (!result.snapshot.exists || result.snapshot.bytesBase64 !== Buffer.from(bytes).toString('base64url')) {
    throw storageIntegrity('measurement receipt publish response bytes do not match the request');
  }
  return result;
}

function measurementArtifactMaximum(kind) {
  switch (kind) {
    case 'narrativeAuthority': return TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES;
    case 'html': return TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES;
    case 'provenance': return TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES;
    default: throw storageIntegrity(`unsupported measurement artifact kind: ${kind}`);
  }
}

export function verifyInitializeResult(result, { narrativeId, mode }) {
  assertObjectResponse(result, 'initialize');
  assertExactKeys(result, ['protocolVersion', 'narrativeId', 'mode', 'namespacePresent'], 'initialize');
  if (result.protocolVersion !== TRACKB_CUSTODY_PROTOCOL_VERSION
    || result.narrativeId !== narrativeId
    || result.mode !== mode
    || typeof result.namespacePresent !== 'boolean'
    || (mode === 'mutable' && result.namespacePresent !== true)) {
    throw storageIntegrity('custody helper initialization response does not match the requested state');
  }
  return result;
}

export function verifyAcquireResult(result, { narrativeId, operationId }) {
  assertObjectResponse(result, 'acquire');
  assertExactKeys(result, ['protocolVersion', 'narrativeId', 'operationId', 'lockHeld'], 'acquire');
  if (result.protocolVersion !== TRACKB_CUSTODY_PROTOCOL_VERSION
    || result.narrativeId !== narrativeId
    || result.operationId !== operationId
    || result.lockHeld !== true) {
    throw storageIntegrity('custody helper acquire response does not match the initialized owner');
  }
  return result;
}

export function verifySnapshotResult(result, { path = undefined } = {}) {
  assertObjectResponse(result, 'snapshot');
  if (typeof result.exists !== 'boolean' || typeof result.snapshotToken !== 'string') {
    throw storageIntegrity('custody helper snapshot response identity is invalid');
  }
  const hasPath = path !== undefined;
  const expectedKeys = result.exists
    ? (hasPath
      ? ['path', 'exists', 'snapshotToken', 'bytesBase64', 'byteLength', 'sha256', 'device', 'inode']
      : ['exists', 'snapshotToken', 'bytesBase64', 'byteLength', 'sha256', 'device', 'inode'])
    : (hasPath ? ['path', 'exists', 'snapshotToken'] : ['exists', 'snapshotToken']);
  assertExactKeys(result, expectedKeys, 'snapshot');
  if (hasPath && result.path !== path) throw storageIntegrity('custody helper snapshot path does not match the request');
  validateResponseSnapshotToken(result.snapshotToken, result.exists);
  if (!result.exists) return result;
  verifyHashedBytes(result, {
    context: 'custody helper snapshot',
    maximumBytes: TRACKB_CUSTODY_MAX_READ_BYTES,
    byteLengthError: 'custody helper snapshot byteLength is invalid',
    sha256Error: 'custody helper snapshot sha256 is invalid',
  });
  if (!Number.isSafeInteger(result.device) || result.device < 0
    || !Number.isSafeInteger(result.inode) || result.inode < 0) {
    throw storageIntegrity('custody helper snapshot identity is invalid');
  }
  return result;
}

export function verifyWriteResult(result, { path, bytes }) {
  assertObjectResponse(result, 'writeImmutable');
  assertExactKeys(result, ['path', 'reused', 'snapshot'], 'writeImmutable');
  if (result.path !== path || typeof result.reused !== 'boolean') {
    throw storageIntegrity('custody helper immutable write response identity is invalid');
  }
  verifySnapshotResult(result.snapshot);
  if (!result.snapshot.exists || result.snapshot.bytesBase64 !== bytes.toString('base64url')) {
    throw storageIntegrity('custody helper immutable write response bytes do not match the request');
  }
  return result;
}

export function verifyPublishResult(result, { pointerBytes }) {
  assertObjectResponse(result, 'publishPointer');
  assertExactKeys(result, ['path', 'reused', 'snapshot'], 'publishPointer');
  if (result.path !== 'delivery-pointer-v2.json' || typeof result.reused !== 'boolean') {
    throw storageIntegrity('custody helper pointer publish response identity is invalid');
  }
  verifySnapshotResult(result.snapshot);
  if (!result.snapshot.exists || result.snapshot.bytesBase64 !== pointerBytes.toString('base64url')) {
    throw storageIntegrity('custody helper pointer publish response bytes do not match the request');
  }
  return result;
}

export function verifyCloseResult(result) {
  assertObjectResponse(result, 'close');
  assertExactKeys(result, ['closed'], 'close');
  if (result.closed !== true) throw storageIntegrity('custody helper close response is invalid');
  return result;
}

function assertObjectResponse(result, operation) {
  if (!result || typeof result !== 'object' || Array.isArray(result)) {
    throw storageIntegrity(`custody helper ${operation} response must be an object`);
  }
}

function assertExactKeys(result, expectedKeys, operation) {
  const expected = [...expectedKeys].sort();
  const actual = Object.keys(result).sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw storageIntegrity(`custody helper ${operation} response fields are invalid`);
  }
}

function verifyHashedBytes(result, { context, maximumBytes, byteLengthError, sha256Error }) {
  if (typeof result.bytesBase64 !== 'string' || !/^[A-Za-z0-9_-]*$/.test(result.bytesBase64)) {
    throw storageIntegrity(`${context} bytesBase64 is invalid`);
  }
  let bytes;
  try { bytes = Buffer.from(result.bytesBase64, 'base64url'); }
  catch (error) { throw storageIntegrity(`${context} bytesBase64 is invalid: ${error.message}`, { cause: error }); }
  if (bytes.toString('base64url') !== result.bytesBase64) {
    throw storageIntegrity(`${context} bytesBase64 is not canonical`);
  }
  const maximum = typeof maximumBytes === 'function' ? maximumBytes() : maximumBytes;
  if (!Number.isSafeInteger(result.byteLength)
    || result.byteLength < 0
    || result.byteLength > maximum
    || result.byteLength !== bytes.length) {
    throw storageIntegrity(byteLengthError);
  }
  if (typeof result.sha256 !== 'string'
    || !TRACKB_CUSTODY_SHA256_PATTERN.test(result.sha256)
    || hashTrackBRawBytes(bytes) !== result.sha256) {
    throw storageIntegrity(sha256Error);
  }
}

function validateResponseSnapshotToken(token, exists) {
  if (!exists) {
    if (token !== 'NONE') throw storageIntegrity('custody helper absent snapshot must use the NONE token');
    return;
  }
  if (token === 'NONE' || token.length === 0 || token.length > 16 * 1024 || !/^[A-Za-z0-9_-]+$/.test(token)) {
    throw storageIntegrity('custody helper present snapshot token is invalid');
  }
  let decoded;
  try { decoded = Buffer.from(token, 'base64url'); }
  catch (error) { throw storageIntegrity(`custody helper present snapshot token is invalid: ${error.message}`, { cause: error }); }
  if (decoded.toString('base64url') !== token) throw storageIntegrity('custody helper present snapshot token is not canonical');
}

export function validateReceipt(receipt, { targetTriple, binaryPath, binaryBytes }) {
  if (!receipt || typeof receipt !== 'object' || Array.isArray(receipt)) throw storageIntegrity('custody receipt must be an object');
  const required = ['schemaVersion', 'targetTriple', 'binaryName', 'binarySha256', 'binaryBytes', 'toolVersion'];
  const allowed = new Set(required);
  const unknown = Object.keys(receipt).filter((key) => !allowed.has(key));
  if (unknown.length) throw storageIntegrity(`custody receipt contains unknown field: ${unknown[0]}`);
  if (Object.keys(receipt).length !== required.length || required.some((key) => !Object.hasOwn(receipt, key))) throw storageIntegrity('custody receipt fields must be exactly schemaVersion,targetTriple,binaryName,binarySha256,binaryBytes,toolVersion');
  if (receipt.schemaVersion !== 1) throw storageIntegrity('custody receipt schemaVersion must be 1');
  if (receipt.targetTriple !== targetTriple) throw storageIntegrity('custody receipt targetTriple mismatch');
  if (receipt.binaryName !== TRACKB_CUSTODY_BINARY_NAME || receipt.binaryName !== basename(binaryPath)) throw storageIntegrity('custody receipt binaryName mismatch');
  if (!TRACKB_CUSTODY_SHA256_PATTERN.test(receipt.binarySha256)) throw storageIntegrity('custody receipt binarySha256 is invalid');
  if (!Number.isSafeInteger(receipt.binaryBytes) || receipt.binaryBytes < 1 || receipt.binaryBytes !== binaryBytes) throw storageIntegrity('custody receipt binaryBytes mismatch');
  if (typeof receipt.toolVersion !== 'string' || receipt.toolVersion.trim().length < 1 || receipt.toolVersion.length > 256) throw storageIntegrity('custody receipt toolVersion is invalid');
}

export function validateNarrativeId(value) {
  if (typeof value !== 'string' || !/^narrative\.[A-Za-z0-9._:-]{1,127}$/.test(value)) throw inputInvalid('narrativeId is invalid');
}

export function validateOpaqueId(value, label) {
  if (typeof value !== 'string' || !SAFE_ID.test(value)) throw inputInvalid(`${label} is invalid`);
}

export function validateRelativePath(value) {
  if (typeof value !== 'string' || value.length === 0 || Buffer.byteLength(value, 'utf8') > 1024 || value.startsWith('/') || value.endsWith('/') || value.includes('\\') || value.includes('\0')) throw inputInvalid('relative storage path is invalid');
  const parts = value.split('/');
  if (parts.some((part) => !/^[A-Za-z0-9._-]+$/.test(part) || part === '.' || part === '..')) throw inputInvalid('relative storage path is invalid');
  return value;
}

export function validateImmutablePath(value) {
  validateRelativePath(value);
  const parts = value.split('/');
  const hashSegment = (segment) => /^[0-9a-f]{64}$/.test(segment);
  const artifact = (prefix, filename) => parts.length === 3 && parts[0] === prefix && hashSegment(parts[1]) && parts[2] === filename;
  const approval = parts.length === 4 && parts[0] === 'approvals' && hashSegment(parts[1]) && (parts[2] === 'NONE' || hashSegment(parts[2])) && parts[3] === 'trackb-release-approval-receipt-v2.json';
  if (!(artifact('authority', 'trackb-narrative-authority-v2.json')
    || artifact('candidates', 'trackb-release-candidate-v4.json')
    || artifact('delivery-sets', 'trackb-delivery-set-v2.json')
    || approval)) throw inputInvalid('immutable path is outside the fixed Track B artifact namespace');
  return value;
}

export function boundedBuffer(value, maximum, label) {
  const bytes = Buffer.isBuffer(value) ? value : value instanceof Uint8Array ? Buffer.from(value) : typeof value === 'string' ? Buffer.from(value, 'utf8') : null;
  if (!bytes) throw inputInvalid(`${label} must be bytes or a string`);
  if (bytes.length > maximum) throw inputInvalid(`${label} exceeds ${maximum} bytes`);
  return bytes;
}

export function normalizeHelperError(error) {
  const code = typeof error?.code === 'string' ? error.code : 'trackb-storage-integrity';
  const message = typeof error?.message === 'string' ? error.message : 'custody helper returned an untyped error';
  return new TrackBCustodyError(code, message, { retryable: error?.retryable === true });
}

export function storageIntegrity(message, options = {}) {
  return new TrackBCustodyError('trackb-storage-integrity', message, options);
}

export function inputInvalid(message) {
  return new TrackBCustodyError('trackb-input-invalid', message);
}

export function assertSafeTargetTriple(value) {
  if (typeof value !== 'string' || !SAFE_TARGET_TRIPLE.test(value)) throw storageIntegrity('targetTriple is invalid');
}

export function basename(path) {
  return path.split(/[\\/]/).at(-1);
}
