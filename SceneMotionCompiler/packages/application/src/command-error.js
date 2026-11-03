export class ApplicationCommandError extends Error {
  constructor(code, message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'ApplicationCommandError';
    this.code = code;
    this.details = options.details;
    this.retryable = options.retryable === true;
  }
}

