import { ApplicationCommandError } from './command-error.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';

const TRACK_B_MEASURE_RUNTIME_ERROR_CODES = new Set([
  'trackb-input-invalid',
  'trackb-narrative-mismatch',
  'trackb-authority-missing',
  'trackb-evidence-incomplete',
  'trackb-storage-integrity',
  'trackb-evidence-timeout',
  'trackb-evidence-cleanup',
  'trackb-lock-busy',
  'trackb-lock-recovery-required',
  'trackb-upstream-failed'
]);

export function throwValidationIfInvalid(result, fallbackCode, label) {
  if (result?.valid === true) return;
  const first = result?.errors?.[0];
  const code = TRACK_B_MEASURE_RUNTIME_ERROR_CODES.has(first?.code) ? first.code : fallbackCode;
  throw runtimeError(code, `${label} validation failed: ${boundedMessage(first?.message ?? 'invalid value')}`);
}

export function mapRuntimeError(error, fallbackCode, message) {
  if (error instanceof ApplicationCommandError && TRACK_B_MEASURE_RUNTIME_ERROR_CODES.has(error.code)) return error;
  if (TRACK_B_MEASURE_RUNTIME_ERROR_CODES.has(error?.code)) {
    return new ApplicationCommandError(error.code, boundedMessage(error.message, 'Track B measurement failed'), {
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
    TRACK_B_MEASURE_RUNTIME_ERROR_CODES.has(code) ? code : 'trackb-upstream-failed',
    boundedMessage(message, 'Track B measurement failed'),
    { ...options, retryable: options.retryable === true }
  );
}

export function attachCloseFailure(primary, closeFailure) {
  const details = isPlainObject(primary.details) ? primary.details : {};
  primary.details = {
    ...details,
    custodyClose: {
      code: closeFailure.code,
      retryable: closeFailure.retryable === true,
      message: boundedMessage(closeFailure.message, 'Track B measurement failed')
    }
  };
}
