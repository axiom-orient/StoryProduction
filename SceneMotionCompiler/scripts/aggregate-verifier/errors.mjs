export class VerifyError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = 'VerifyError';
    this.code = code;
    this.details = details;
  }
}
