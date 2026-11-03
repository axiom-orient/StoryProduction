/** Decode and bind exact artifact bytes returned by the custody boundary. */
import { TRACKB_CUSTODY_MAX_READ_BYTES } from '../../store/src/index.js';
import { hashTrackBRawBytes, isTrackBHash, isTrackBSafeRelativePath } from '../../contracts/src/trackb-contract.js';
import { canonicalizeJson } from '../../core/src/canonical.js';
import { TRACK_B_PREPARE_MAX_ARTIFACT_BYTES } from './trackb-release-input-contract.js';
import { isPlainObject } from './trackb-application-primitives.js';
import { hasExactKeys, mapRuntimeError, runtimeError } from './trackb-release-runtime-support.js';

const TRACK_B_PROJECT_ARTIFACTS = Object.freeze([
  'narrativeAuthority',
  'projectionReceipt',
  'assetHandoff',
  'html',
  'provenance',
  'evidenceReceipt',
  'sceneGraph'
]);

export async function readProjectArtifact(session, kind, path, extra = {}) {
  if (!TRACK_B_PROJECT_ARTIFACTS.includes(kind)) throw runtimeError('trackb-input-invalid', `unsupported project artifact kind: ${kind}`);
  if (!isTrackBSafeRelativePath(path)) throw runtimeError('trackb-input-invalid', `${kind} path must be a safe project-relative path`);
  let result;
  try {
    result = await session.readProjectArtifact({ kind, path, ...extra });
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-storage-integrity', `custody could not read ${kind}`);
  }
  assertExactKeysForProjectArtifact(result, kind, path);
  if (result.exists !== true) {
    const missingCode = kind === 'narrativeAuthority' ? 'trackb-authority-missing' : 'trackb-artifact-binding-invalid';
    throw runtimeError(missingCode, `${kind} custody response is absent or has the wrong identity`);
  }
  if (typeof result.bytesBase64 !== 'string' || !/^[A-Za-z0-9_-]*$/.test(result.bytesBase64)) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response bytesBase64 is invalid`);
  }
  let bytes;
  try {
    bytes = Buffer.from(result.bytesBase64, 'base64url');
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response bytesBase64 could not be decoded`, { cause: error });
  }
  if (bytes.toString('base64url') !== result.bytesBase64
    || !Number.isSafeInteger(result.byteLength)
    || result.byteLength < 0
    || result.byteLength > TRACKB_CUSTODY_MAX_READ_BYTES
    || result.byteLength !== bytes.length
    || bytes.length > TRACK_B_PREPARE_MAX_ARTIFACT_BYTES) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response byte length or encoding is invalid`);
  }
  const actualHash = hashTrackBRawBytes(bytes);
  if (!isTrackBHash(result.sha256) || result.sha256 !== actualHash) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response raw hash is invalid`);
  }
  return bytes;
}

function assertExactKeysForProjectArtifact(result, kind, path) {
  if (!isPlainObject(result)
    || result.kind !== kind
    || result.path !== path
    || typeof result.exists !== 'boolean') {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response identity is invalid`);
  }
  const expected = result.exists
    ? ['kind', 'path', 'exists', 'bytesBase64', 'byteLength', 'sha256']
    : ['kind', 'path', 'exists'];
  if (!hasExactKeys(result, expected)) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response fields are invalid`);
  }
}

export function parseJsonBytes(bytes, label, code) {
  let text;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return JSON.parse(text);
  } catch (error) {
    throw runtimeError(code, `${label} returned bytes are not valid JSON`, { cause: error });
  }
}

export function parseCanonicalJsonBytes(bytes, label, code) {
  let value;
  try {
    value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    const canonicalBytes = Buffer.from(canonicalizeJson(value), 'utf8');
    if (!canonicalBytes.equals(bytes)) {
      throw new Error('canonical JSON bytes are required');
    }
  } catch (error) {
    throw runtimeError(code, `${label} bytes are not canonical valid JSON`, { cause: error });
  }
  return value;
}

export function decodeSnapshotBytes(snapshot, label) {
  if (!snapshot?.exists || typeof snapshot.bytesBase64 !== 'string') {
    throw runtimeError('trackb-storage-integrity', `${label} snapshot is absent`);
  }
  try {
    return Buffer.from(snapshot.bytesBase64, 'base64url');
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', `${label} snapshot bytes could not be decoded`, { cause: error });
  }
}
