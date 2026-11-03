export class MacosStudioLaunchError extends Error {
  constructor(code, message, options = {}) {
    super(message, options);
    this.name = 'MacosStudioLaunchError';
    this.code = code;
  }
}
