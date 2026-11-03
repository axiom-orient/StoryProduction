// Compatibility entrypoint for the existing SMC Node application. The native
// producer owns validation, worker claims, rendering, and terminal publication.
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';

const execute = promisify(execFile);
const OWNER_ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const BINARY = process.env.SMC_ANIMATIC_BIN
  || fileURLToPath(new URL('../../../target/release/smc-animatic', import.meta.url));

async function invoke(operation, root, input) {
  const arguments_ = [operation, '--project', root, '--owner-root', OWNER_ROOT];
  if (input.attemptId !== undefined) arguments_.push('--attempt', input.attemptId);
  if (operation === 'render' || operation === 'inspect') {
    if (Object.keys(input).some((key) => !['packetPath', 'assetPackPath', 'assetReviewPath', 'attemptId', 'reducedMotion'].includes(key))) throw new Error('UNKNOWN_ANIMATIC_OPTION');
    if (input.reducedMotion !== undefined && typeof input.reducedMotion !== 'boolean') throw new Error('REDUCED_MOTION_MUST_BE_BOOLEAN');
    arguments_.push('--packet', input.packetPath, '--assets', input.assetPackPath, '--asset-review', input.assetReviewPath,
      '--motion', input.reducedMotion ? 'reduced' : 'full');
  }
  let stdout;
  try {
    ({ stdout } = await execute(BINARY, arguments_, { timeout: 20 * 60 * 1000, maxBuffer: 16 * 1024 * 1024 }));
  } catch (error) {
    if (error.code === 3 && error.stdout) stdout = error.stdout;
    else throw error;
  }
  const value = JSON.parse(stdout);
  if (value.detail?.startsWith('OUTCOME_UNKNOWN:')) throw new Error(value.detail);
  return value;
}

export const loadAnimaticInput = (root, input) => invoke('inspect', root, input);
export const renderAnimatic = (root, input) => invoke('render', root, input);
export const statusAnimatic = (root, input) => invoke('status', root, input);
export const cancelAnimatic = (root, input) => invoke('cancel', root, input);
export const recoverAnimatic = (root, input) => invoke('recover', root, input);
