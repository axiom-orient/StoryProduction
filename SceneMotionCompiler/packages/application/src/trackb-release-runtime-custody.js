/** Own the Node-to-custody session lifecycle and strict protocol response checks. */
import { isAbsolute } from 'node:path';
import {
  TRACKB_CUSTODY_MAX_READ_BYTES,
  TRACKB_CUSTODY_MAX_WRITE_BYTES,
  TRACKB_CUSTODY_PROTOCOL_VERSION,
  openTrackBCustodySession
} from '../../store/src/index.js';
import {
  hashTrackBRawBytes,
  isTrackBHash,
  isTrackBOpaqueId,
  isTrackBSafeRelativePath,
  validateTrackBCommandResult
} from '../../contracts/src/trackb-contract.js';
import { decodeSnapshotBytes, parseCanonicalJsonBytes } from './trackb-release-runtime-artifacts.js';
import { loadTrackBOnlyRegistry } from './trackb-release-runtime-registry.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';
import {
  hasExactKeys,
  mapRuntimeError,
  runtimeError,
  throwValidationIfInvalid
} from './trackb-release-runtime-support.js';

export async function openNarrativeRuntime(dependencies, narrativeId) {
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'Track B narrative runtime dependencies must be an object');
  if (typeof dependencies.projectRoot !== 'string' || !isAbsolute(dependencies.projectRoot)) {
    throw runtimeError('trackb-input-invalid', 'narrative dependencies must provide an absolute projectRoot');
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
      'now'
    ].includes(key)));
  const registry = await loadTrackBOnlyRegistry(workflowDependencies);
  return Object.freeze({
    projectRoot: dependencies.projectRoot,
    operationId: dependencies.operationId,
    narrativeId,
    openNarrativeSession,
    registry
  });
}

export function assertNarrativeSession(session, { mutable }) {
  if (!session
    || typeof session.read !== 'function'
    || typeof session.snapshot !== 'function'
    || typeof session.close !== 'function'
    || (mutable && (typeof session.writeImmutable !== 'function' || typeof session.publishPointer !== 'function'))) {
    throw runtimeError('trackb-storage-integrity', `${mutable ? 'mutable' : 'read-only'} narrative custody session is incomplete`);
  }
}

export async function closeNarrativeSession(session, operationFailure, label) {
  if (!session) return;
  try {
    const closeResult = await session.close();
    verifyCloseResult(closeResult);
  } catch (error) {
    const closeFailure = mapRuntimeError(error, 'trackb-storage-integrity', `${label} custody session close failed`);
    if (!operationFailure) {
      throw runtimeError('trackb-storage-integrity', `${label} custody session close failed`, {
        cause: closeFailure,
        details: { closeCode: closeFailure.code, closeMessage: boundedMessage(closeFailure.message) }
      });
    }
    attachCloseFailure(operationFailure, closeFailure);
  }
}

export async function readNarrativeSnapshot(session, path, method = 'read') {
  if (!isTrackBSafeRelativePath(path)) throw runtimeError('trackb-storage-integrity', 'narrative artifact path is not safe');
  if (typeof session?.[method] !== 'function') throw runtimeError('trackb-storage-integrity', `custody session cannot ${method} narrative artifacts`);
  let snapshot;
  try {
    snapshot = await session[method](path);
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-storage-integrity', `custody could not ${method} ${path}`);
  }
  verifySnapshotResult(snapshot, { path });
  return snapshot;
}

export async function readRequiredNarrativeObject(session, path, { label, missingCode, parseCode }) {
  const snapshot = await readNarrativeSnapshot(session, path, 'read');
  if (snapshot.exists !== true) throw runtimeError(missingCode, `${label} is absent`);
  const bytes = decodeSnapshotBytes(snapshot, label);
  return parseCanonicalJsonBytes(bytes, label, parseCode);
}

export function validateCommandResult(command, result, input) {
  const validation = validateTrackBCommandResult(command, result, { input });
  throwValidationIfInvalid(validation, 'trackb-input-invalid', `${command} result`);
}

export function verifyPublishPointer(result, pointerBytes) {
  if (!isPlainObject(result)
    || !hasExactKeys(result, ['path', 'reused', 'snapshot'])
    || result.path !== 'delivery-pointer-v2.json'
    || typeof result.reused !== 'boolean'
    || !isPlainObject(result.snapshot)) {
    throw runtimeError('trackb-storage-integrity', 'custody pointer publish response is invalid');
  }
  verifySnapshotResult(result.snapshot);
  if (result.snapshot.exists !== true
    || result.snapshot.bytesBase64 !== pointerBytes.toString('base64url')
    || result.snapshot.byteLength !== pointerBytes.length
    || result.snapshot.sha256 !== hashTrackBRawBytes(pointerBytes)) {
    throw runtimeError('trackb-storage-integrity', 'custody pointer publish bytes do not match the requested pointer');
  }
}

export function verifyImmutableWrite(result, path, bytes) {
  if (!isPlainObject(result)
    || !hasExactKeys(result, ['path', 'reused', 'snapshot'])
    || !isTrackBSafeRelativePath(result.path)
    || result.path !== path
    || typeof result.reused !== 'boolean'
    || !isPlainObject(result.snapshot)) {
    throw runtimeError('trackb-storage-integrity', `custody immutable write response is invalid for ${path}`);
  }
  const snapshot = result.snapshot;
  verifySnapshotResult(snapshot);
  if (snapshot.exists !== true
    || snapshot.bytesBase64 !== bytes.toString('base64url')
    || snapshot.byteLength !== bytes.length
    || snapshot.sha256 !== hashTrackBRawBytes(bytes)) {
    throw runtimeError('trackb-storage-integrity', `custody immutable write bytes do not match ${path}`);
  }
}

export function verifyInitializeResult(result, narrativeId, mode = 'mutable') {
  if (!isPlainObject(result) || !hasExactKeys(result, ['protocolVersion', 'narrativeId', 'mode', 'namespacePresent'])
    || result.protocolVersion !== TRACKB_CUSTODY_PROTOCOL_VERSION
    || result.narrativeId !== narrativeId
    || result.mode !== mode
    || typeof result.namespacePresent !== 'boolean'
    || (mode === 'mutable' && result.namespacePresent !== true)) {
    throw runtimeError('trackb-storage-integrity', `custody initialize response does not match the requested ${mode} narrative`);
  }
}

export function verifySnapshotResult(result, { path = undefined } = {}) {
  if (!isPlainObject(result) || typeof result.exists !== 'boolean' || typeof result.snapshotToken !== 'string') {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot response identity is invalid');
  }
  const expectedKeys = result.exists
    ? (path === undefined
      ? ['exists', 'snapshotToken', 'bytesBase64', 'byteLength', 'sha256', 'device', 'inode']
      : ['path', 'exists', 'snapshotToken', 'bytesBase64', 'byteLength', 'sha256', 'device', 'inode'])
    : (path === undefined ? ['exists', 'snapshotToken'] : ['path', 'exists', 'snapshotToken']);
  if (!hasExactKeys(result, expectedKeys)) {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot response fields are invalid');
  }
  if (path !== undefined && (!isTrackBSafeRelativePath(result.path) || result.path !== path)) {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot path does not match the request');
  }
  if (result.exists === false) {
    if (result.snapshotToken !== 'NONE') throw runtimeError('trackb-storage-integrity', 'absent custody snapshot must use the NONE token');
    return result;
  }
  if (!isCanonicalOpaqueToken(result.snapshotToken)) {
    throw runtimeError('trackb-storage-integrity', 'present custody snapshot token is invalid');
  }
  if (typeof result.bytesBase64 !== 'string' || !/^[A-Za-z0-9_-]*$/.test(result.bytesBase64)) {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot bytesBase64 is invalid');
  }
  let bytes;
  try {
    bytes = Buffer.from(result.bytesBase64, 'base64url');
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot bytesBase64 could not be decoded', { cause: error });
  }
  if (bytes.toString('base64url') !== result.bytesBase64
    || !Number.isSafeInteger(result.byteLength)
    || result.byteLength < 0
    || result.byteLength > TRACKB_CUSTODY_MAX_READ_BYTES
    || result.byteLength !== bytes.length
    || !isTrackBHash(result.sha256)
    || result.sha256 !== hashTrackBRawBytes(bytes)
    || !Number.isSafeInteger(result.device)
    || result.device < 0
    || !Number.isSafeInteger(result.inode)
    || result.inode < 0) {
    throw runtimeError('trackb-storage-integrity', 'custody snapshot identity, hash, or byte length is invalid');
  }
  return result;
}

export function verifyCloseResult(result) {
  if (!isPlainObject(result) || !hasExactKeys(result, ['closed']) || result.closed !== true) {
    throw runtimeError('trackb-storage-integrity', 'custody close response is invalid');
  }
}

export function assertBoundedWriteBytes(bytes, label) {
  if (!Buffer.isBuffer(bytes) || bytes.length > TRACKB_CUSTODY_MAX_WRITE_BYTES) {
    throw runtimeError('trackb-storage-integrity', `${label} immutable bytes exceed the custody write bound`);
  }
}

function isCanonicalOpaqueToken(value) {
  if (typeof value !== 'string' || value === 'NONE' || value.length === 0 || value.length > 16 * 1024
    || !/^[A-Za-z0-9_-]+$/.test(value)) return false;
  try {
    return Buffer.from(value, 'base64url').toString('base64url') === value;
  } catch {
    return false;
  }
}

export function attachCloseFailure(primary, closeFailure) {
  const details = isPlainObject(primary.details) ? primary.details : {};
  primary.details = {
    ...details,
    custodyClose: {
      code: closeFailure.code,
      retryable: closeFailure.retryable === true,
      message: boundedMessage(closeFailure.message)
    }
  };
}
