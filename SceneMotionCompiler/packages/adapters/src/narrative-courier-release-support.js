export class NarrativeCourierReleaseError extends Error {
  constructor(code, message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'NarrativeCourierReleaseError';
    this.code = code;
    this.details = options.details;
    this.retryable = options.retryable === true;
  }
}

export function contractError(code, message, options = {}) {
  return new NarrativeCourierReleaseError(code, message, options);
}
