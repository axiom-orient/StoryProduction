// Deliberately internal application adapter for the trusted same-process
// Studio host. This module is not exported from packages/application/src/index.js.
import { approveTrackBRelease } from './trackb-release-runtime-approve.js';
import { createStudioApplicationCommandDispatcher } from './service.js';

export function createStudioApprovalDispatcher(config) {
  const dispatcher = createStudioApplicationCommandDispatcher(config, approveTrackBRelease);
  return Object.freeze({
    executeApproval: dispatcher.executeApproval
  });
}
