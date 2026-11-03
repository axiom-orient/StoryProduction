// Shared Track B-local value and identity rules. Public callers enter through trackb-contract.js.
import { canonicalHash, canonicalizeJson, sha256Prefixed } from '../../core/src/canonical.js';

export const TRACK_B_HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;
export const TRACK_B_BASE_HASH_PATTERN = /^(?:NONE|sha256:[0-9a-f]{64})$/;
export const TRACK_B_NARRATIVE_ID_PATTERN = /^narrative\.[A-Za-z0-9._:-]{1,127}$/;
export const TRACK_B_OPAQUE_ID_PATTERN = /^[A-Za-z][A-Za-z0-9._:-]{0,127}$/;
export const TRACK_B_SAFE_PATH_PATTERN = /^(?:[A-Za-z0-9._-]+\/)*[A-Za-z0-9._-]+$/;
export const TRACK_B_PROJECTION_RECEIPT_PATH = 'projection/narrative-scene-projection.v3.json';
export const TRACK_B_HANDOFF_PATH = 'handoff/scene-motion-asset-handoff.v2.json';
export const TRACK_B_RELEASE_CANDIDATE_FILENAME = 'trackb-release-candidate-v4.json';
export const TRACK_B_UTC_DATE_TIME_PATTERN = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$/;
export const TRACK_B_APPROVAL_MAX_FUTURE_MS = 5 * 60 * 1000;

export const TRACK_B_HASH_DOMAINS = Object.freeze({
  authorityHash: Object.freeze(['schemaVersion', 'authorityId', 'narrativeId', 'sourceRevision', 'sourceRefs', 'story', 'characters', 'environments', 'shots', 'meaningItems']),
  evidenceCellHash: Object.freeze(['shotId', 'gateVerdict', 'meaningCoverage', 'runtimeEvidence', 'assetEvidence', 'visualEvidence']),
  receiptHash: Object.freeze(['schemaVersion', 'receiptId', 'narrativeId', 'authorityHash', 'htmlHash', 'assetManifestHash', 'semanticManifestHash', 'rendererLockHash', 'navigationGuard', 'shotIds', 'shots']),
  meaningQualificationHash: Object.freeze(['authorityHash', 'projectionReceiptHash', 'sceneGraphHash', 'sourceSemanticManifestHash', 'semanticManifestHash', 'htmlHash', 'provenanceHash', 'qualifiedItems']),
  candidateHash: Object.freeze(['schemaVersion', 'candidateId', 'narrativeId', 'authorityHash', 'meaningQualificationHash', 'baseDeliverySetHash', 'artifacts', 'evidenceCells', 'candidateVerdict']),
  candidateV4Hash: Object.freeze(['schemaVersion', 'candidateId', 'narrativeId', 'authorityHash', 'meaningQualificationHash', 'evidenceReceiptHash', 'baseDeliverySetHash', 'artifacts', 'evidenceCells', 'candidateVerdict']),
  approvalHash: Object.freeze(['schemaVersion', 'approvalId', 'subject', 'decision', 'actor', 'reason', 'origin', 'reviewSessionId', 'decidedAt']),
  deliverySetHash: Object.freeze(['schemaVersion', 'deliverySetId', 'narrativeId', 'candidateHash', 'authorityHash', 'meaningQualificationHash', 'baseDeliverySetHash', 'approvalHash', 'artifacts', 'evidenceCellHashes', 'revision', 'state']),
  pointerHash: Object.freeze(['schemaVersion', 'pointerKey', 'narrativeId', 'revision', 'state', 'deliverySetHash', 'candidateHash', 'approvalHash', 'meaningQualificationHash', 'baseDeliverySetHash', 'deliverySetPath'])
});
export class TrackBContractError extends Error {
  constructor(code, message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'TrackBContractError';
    this.code = code;
    this.retryable = options.retryable === true;
    this.details = options.details;
  }
}

export function isTrackBHash(value) {
  return typeof value === 'string' && TRACK_B_HASH_PATTERN.test(value);
}

export function isTrackBBaseHash(value) {
  return typeof value === 'string' && TRACK_B_BASE_HASH_PATTERN.test(value);
}

export function isTrackBNarrativeId(value) {
  return typeof value === 'string' && TRACK_B_NARRATIVE_ID_PATTERN.test(value);
}

export function isTrackBOpaqueId(value) {
  return typeof value === 'string' && TRACK_B_OPAQUE_ID_PATTERN.test(value);
}

export function isTrackBDateTime(value) {
  return parseTrackBDateTime(value) !== null;
}

/**
 * Parse only the UTC RFC 3339 form owned by Track B. Date.parse is not used as
 * the acceptance test because it normalizes invalid calendar values.
 */
export function parseTrackBDateTime(value) {
  if (typeof value !== 'string' || !TRACK_B_UTC_DATE_TIME_PATTERN.test(value)) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?Z$/.exec(value);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const fraction = match[7] ?? '';
  const millisecond = fraction.length === 0 ? 0 : Number(fraction.slice(0, 3).padEnd(3, '0'));
  if (month < 1 || month > 12 || day < 1 || day > daysInUtcMonth(year, month)
    || hour > 23 || minute > 59 || second > 59) return null;
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(hour, minute, second, millisecond);
  if (date.getUTCFullYear() !== year || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day
    || date.getUTCHours() !== hour || date.getUTCMinutes() !== minute
    || date.getUTCSeconds() !== second || date.getUTCMilliseconds() !== millisecond) return null;
  return Object.freeze({ epochMilliseconds: date.getTime(), fractionDigits: fraction.length });
}

export function isTrackBSafeRelativePath(value) {
  if (typeof value !== 'string'
    || Buffer.byteLength(value, 'utf8') > 1024
    || value.includes('\\')
    || value.includes('\0')
    || value.startsWith('/')
    || !TRACK_B_SAFE_PATH_PATTERN.test(value)) return false;
  return value.split('/').every((segment) => segment !== '.' && segment !== '..' && segment.length > 0);
}

export function assertTrackBHash(value, label = 'hash', { allowNone = false } = {}) {
  const valid = allowNone ? isTrackBBaseHash(value) : isTrackBHash(value);
  if (!valid) throw new TrackBContractError('trackb-input-invalid', `${label} must match ${allowNone ? 'NONE or sha256:<64 lowercase hex>' : 'sha256:<64 lowercase hex>'}`);
  return value;
}

export function assertTrackBNarrativeId(value, label = 'narrativeId') {
  if (!isTrackBNarrativeId(value)) throw new TrackBContractError('trackb-input-invalid', `${label} must match ${TRACK_B_NARRATIVE_ID_PATTERN.source}`);
  return value;
}

export function assertTrackBOpaqueId(value, label = 'id') {
  if (!isTrackBOpaqueId(value)) throw new TrackBContractError('trackb-input-invalid', `${label} must match ${TRACK_B_OPAQUE_ID_PATTERN.source}`);
  return value;
}

export function assertTrackBDateTime(value, label = 'date-time', { now } = {}) {
  const parsed = parseTrackBDateTime(value);
  if (!parsed) throw new TrackBContractError('trackb-input-invalid', `${label} must be a valid UTC RFC 3339 date-time`);
  if (isTrackBDateBeyondFutureLimit(parsed, now)) throw new TrackBContractError('trackb-input-invalid', `${label} may not be more than five minutes in the future`);
  return value;
}

export function assertTrackBSafeRelativePath(value, label = 'path') {
  if (!isTrackBSafeRelativePath(value)) throw new TrackBContractError('trackb-input-invalid', `${label} must be a safe project-relative path`);
  return value;
}

export function hashTrackBRawBytes(value) {
  if (!(typeof value === 'string' || Buffer.isBuffer(value) || value instanceof Uint8Array)) throw new TrackBContractError('trackb-input-invalid', 'raw artifact hash input must be a UTF-8 string or byte sequence');
  const bytes = typeof value === 'string' ? Buffer.from(value, 'utf8') : Buffer.from(value);
  return sha256Prefixed(bytes);
}

export function canonicalTrackBHash(value) {
  return canonicalHash(value);
}

export function hashTrackBAuthority(value) {
  return hashDomain(value, 'authorityHash');
}

export function hashTrackBEvidenceCell(value) {
  return hashDomain(value, 'evidenceCellHash');
}

export function hashTrackBMeaningQualification(value) {
  return hashDomain(value, 'meaningQualificationHash');
}

export function hashTrackBMeasuredEvidenceReceipt(value) {
  return hashDomain(value, 'receiptHash');
}

export function hashTrackBReleaseCandidate(value) {
  if (value?.schemaVersion !== 4) {
    throw new TrackBContractError(
      'trackb-input-invalid',
      'Track B release candidates must use schemaVersion 4; legacy candidate hashes are not accepted'
    );
  }
  return hashDomain(value, 'candidateV4Hash', 'candidateHash');
}

export function hashTrackBApprovalReceipt(value) {
  return hashDomain(value, 'approvalHash');
}

export function hashTrackBDeliverySet(value) {
  return hashDomain(value, 'deliverySetHash');
}

export function hashTrackBDeliveryPointer(value) {
  return hashDomain(value, 'pointerHash');
}
export function hashDomain(value, domainName, selfField = domainName) {
  if (!isPlainObject(value)) throw new TrackBContractError('trackb-input-invalid', `${selfField} input must be an object`);
  const fields = TRACK_B_HASH_DOMAINS[domainName];
  if (!fields) throw new TrackBContractError('trackb-input-invalid', `unsupported Track B hash domain: ${domainName}`);
  const allowed = new Set([...fields, selfField]);
  for (const field of fields) if (!Object.hasOwn(value, field)) throw new TrackBContractError('trackb-input-invalid', `${selfField} domain is missing ${field}`);
  for (const field of Object.keys(value)) if (!allowed.has(field)) throw new TrackBContractError('trackb-input-invalid', `${selfField} domain has unsupported field ${field}`);
  const domain = Object.fromEntries(fields.map((field) => [field, value[field]]));
  return canonicalHash(domain);
}

export function hashCanonicalWithoutSelf(value, selfField) {
  if (!isPlainObject(value)) throw new TrackBContractError('trackb-input-invalid', 'canonical hash input must be an object');
  const clone = { ...value };
  delete clone[selfField];
  return canonicalHash(clone);
}
export function exactKeys(value, expected, errors, code, { optional = [] } = {}) {
  const allowed = new Set([...expected, ...optional]);
  for (const key of Object.keys(value)) if (!allowed.has(key)) errors.push(contractError('/', `unsupported key: ${key}`, code));
  for (const key of expected) if (!Object.hasOwn(value, key)) errors.push(contractError(`/${key}`, 'is required', code));
}

export function assertResult(result) {
  if (!result.valid) {
    const first = result.errors[0] ?? { code: 'trackb-input-invalid', message: 'Track B contract validation failed', instancePath: '/' };
    throw new TrackBContractError(first.code ?? 'trackb-input-invalid', first.message, { details: { errors: result.errors } });
  }
  return true;
}

export function resultWithErrors(errors) {
  return { valid: errors.length === 0, errors };
}

export function contractError(instancePath, message, code) {
  return { instancePath, message, code };
}
export function isTrackBNonEmptyText(value, maxLength) {
  return typeof value === 'string' && value.trim().length > 0 && [...value].length <= maxLength;
}

function daysInUtcMonth(year, month) {
  if (month === 2) return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : 28;
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

export function isTrackBDateBeyondFutureLimit(parsed, now) {
  const nowMilliseconds = now === undefined
    ? Date.now()
    : now instanceof Date
      ? now.getTime()
      : typeof now === 'number' && Number.isFinite(now)
        ? now
        : Number.NaN;
  return !Number.isFinite(nowMilliseconds) || parsed.epochMilliseconds > nowMilliseconds + TRACK_B_APPROVAL_MAX_FUTURE_MS;
}
export function canonicalTrackBDomainBytes(value, selfField) {
  const fields = TRACK_B_HASH_DOMAINS[selfField];
  if (!fields) throw new TrackBContractError('trackb-input-invalid', `unsupported Track B hash domain: ${selfField}`);
  return Buffer.from(canonicalizeJson(Object.fromEntries(fields.map((field) => [field, value[field]]))), 'utf8');
}

export function verifyTrackBSelfHash(value, selfField) {
  const actual = hashDomain(value, selfField);
  return actual === value[selfField];
}

export function isPlainObject(value) {
  return value !== null
    && typeof value === 'object'
    && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}
