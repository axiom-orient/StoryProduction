/** Track B runtime error taxonomy and deterministic value helpers. */
import { ApplicationCommandError } from './command-error.js';
import { boundedMessage } from './trackb-application-primitives.js';
import { isTrackBHash } from '../../contracts/src/trackb-contract.js';

const TRACK_B_RUNTIME_ERROR_CODES = new Set([
  'trackb-input-invalid',
  'trackb-narrative-mismatch',
  'trackb-authority-missing',
  'trackb-candidate-missing',
  'trackb-candidate-invalid',
  'trackb-approval-required',
  'trackb-evidence-incomplete',
  'trackb-meaning-unsupported',
  'trackb-artifact-binding-invalid',
  'trackb-storage-integrity',
  'trackb-cas-conflict',
  'trackb-approval-conflict',
  'trackb-approval-stale',
  'trackb-approval-forbidden',
  'trackb-approval-not-promotable',
  'trackb-delivery-not-ready',
  'trackb-lock-busy',
  'trackb-lock-recovery-required',
  'trackb-pointer-reconcile-required',
  'trackb-upstream-failed'
]);

export function hasExactKeys(value, expectedKeys) {
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

export function throwValidationIfInvalid(result, fallbackCode, label) {
  if (result?.valid === true) return;
  const first = result?.errors?.[0];
  const code = TRACK_B_RUNTIME_ERROR_CODES.has(first?.code) ? first.code : fallbackCode;
  throw runtimeError(code, `${label} validation failed: ${boundedMessage(first?.message ?? 'invalid value')}`);
}

export function mapRuntimeError(error, fallbackCode, message) {
  if (error instanceof ApplicationCommandError && TRACK_B_RUNTIME_ERROR_CODES.has(error.code)) return error;
  if (TRACK_B_RUNTIME_ERROR_CODES.has(error?.code)) {
    return new ApplicationCommandError(error.code, boundedMessage(error.message), {
      retryable: error.retryable === true,
      details: error.details,
      cause: error
    });
  }
  return new ApplicationCommandError(fallbackCode, `${message}: ${boundedMessage(error?.message ?? 'unknown failure')}`, {
    retryable: false,
    cause: error
  });
}

export function runtimeError(code, message, options = {}) {
  return new ApplicationCommandError(
    TRACK_B_RUNTIME_ERROR_CODES.has(code) ? code : 'trackb-upstream-failed',
    boundedMessage(message),
    { ...options, retryable: options.retryable === true }
  );
}

export function hashHex(hash) {
  if (!isTrackBHash(hash)) throw runtimeError('trackb-artifact-binding-invalid', 'fixed artifact path requires a sha256 hash');
  return hash.slice('sha256:'.length);
}

export function hashBaseSegment(hash) {
  return hash === 'NONE' ? 'NONE' : hashHex(hash);
}
