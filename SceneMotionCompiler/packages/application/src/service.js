import { prepareProductionArtifact } from '../../production/src/index.js';
import { renderAnimatic, statusAnimatic, cancelAnimatic, recoverAnimatic } from '../../production/src/animatic.js';
import { NarrativeCourierReleaseError } from '../../adapters/src/narrative-courier-release.js';
import { lstat, realpath } from 'node:fs/promises';
import { dirname, relative, resolve, sep } from 'node:path';
import { ApplicationCommandError } from './command-error.js';
import { isPlainObject } from './trackb-application-primitives.js';
import { TRACK_B_HANDLER_IMPLEMENTATIONS } from './trackb-commands.js';
import {
  APPLICATION_COMMAND_METADATA,
  APPLICATION_COMMAND_REGISTRY
} from './command-registry.js';
import { approveTrackBRelease } from './trackb-release-runtime-approve.js';
import { prepareTrackBRelease } from './trackb-release-runtime-prepare.js';
import { promoteTrackBRelease } from './trackb-release-runtime-promote.js';
import { statusTrackBRelease } from './trackb-release-runtime-status.js';

export { APPLICATION_COMMAND_METADATA };

export function createApplicationCommandService(config) {
  const normalized = normalizeConfig(config);
  const publicConfig = {
    repoRoot: normalized.repoRoot,
    projectRoot: normalized.projectRoot
  };
  return Object.freeze({
    config: Object.freeze(publicConfig),
    metadata: APPLICATION_COMMAND_METADATA,
    execute(command, input = {}) {
      return executeNormalized(normalized, command, input);
    }
  });
}

/**
 * Execute one Track B command and preserve the shared application envelope.
 * Human approval is deliberately unavailable through this public dispatcher.
 */
export async function executeApplicationCommand(serviceOrConfig, command, input = {}) {
  try {
    const service = serviceOrConfig?.execute
      ? serviceOrConfig
      : createApplicationCommandService(serviceOrConfig);
    return service.execute(command, input);
  } catch (error) {
    return failureEnvelope(command, APPLICATION_COMMAND_METADATA[command] ?? null, normalizeCommandError(error));
  }
}

/** Internal Studio-only approval capability. */
export function createStudioApplicationCommandDispatcher(config, approvalHandler = approveTrackBRelease) {
  if (typeof approvalHandler !== 'function') {
    throw new ApplicationCommandError('configuration-invalid', 'Studio approval handler must be a function');
  }
  const normalized = normalizeConfig(config);
  return Object.freeze({
    executeApproval(input, executionContext) {
      return executeNormalized(normalized, 'trackb.release.approve', input, executionContext, { approvalHandler });
    }
  });
}

async function executeNormalized(config, command, input, executionContext, internal = null) {
  const metadata = APPLICATION_COMMAND_METADATA[command] ?? null;
  if (!metadata) return failureEnvelope(command, null, new ApplicationCommandError('command-unknown', `Unknown application command: ${command}`));
  try {
    if (!input || typeof input !== 'object' || Array.isArray(input)) {
      throw new ApplicationCommandError('command-input-invalid', 'Command input must be an object');
    }
    if (command === 'trackb.release.approve' && !internal?.approvalHandler) {
      throw new ApplicationCommandError('trackb-approval-forbidden', 'Track B release approval is available only through Studio');
    }
    const scopedConfig = command === 'trackb.release.status'
      ? await validateProjectRoot(config, false)
      : await validateProjectRoot(config, true);
    const result = await COMMANDS[command](scopedConfig, input, executionContext, internal);
    return { schemaVersion: 1, ok: true, command, metadata, result };
  } catch (error) {
    return failureEnvelope(command, metadata, normalizeCommandError(error));
  }
}

const HANDLER_IMPLEMENTATIONS = Object.freeze({
  animaticRender: (config, input) => renderAnimatic(config.projectRoot, input),
  animaticStatus: (config, input) => statusAnimatic(config.projectRoot, input),
  animaticCancel: (config, input) => cancelAnimatic(config.projectRoot, input),
  animaticRecover: (config, input) => recoverAnimatic(config.projectRoot, input),
  async prepareProduction(config, input) {
    if (Object.keys(input).length !== 2 || typeof input.requestPath !== 'string' || typeof input.outputPath !== 'string') throw new ApplicationCommandError('command-input-invalid', 'production.prepare requires exactly requestPath and outputPath');
    return prepareProductionArtifact({projectRoot:config.projectRoot,...input});
  },
  async prepareRelease(config, input) {
    return prepareTrackBRelease({ projectRoot: config.projectRoot, input }, config.trackBRuntimeDependencies);
  },
  async approveRelease(config, input, executionContext, internal) {
    return internal.approvalHandler(input, executionContext, {
      ...(config.trackBRuntimeDependencies ?? {}),
      projectRoot: config.projectRoot
    });
  },
  async promoteRelease(config, input) {
    return promoteTrackBRelease(input, {
      ...(config.trackBRuntimeDependencies ?? {}),
      projectRoot: config.projectRoot
    });
  },
  async statusRelease(config, input) {
    return statusTrackBRelease(input, {
      ...(config.trackBRuntimeDependencies ?? {}),
      projectRoot: config.projectRoot
    });
  },
  ...TRACK_B_HANDLER_IMPLEMENTATIONS
});

const COMMANDS = Object.freeze(Object.fromEntries(
  APPLICATION_COMMAND_REGISTRY.map(({ command, implementation }) => {
    const handler = HANDLER_IMPLEMENTATIONS[implementation];
    if (typeof handler !== 'function') throw new Error(`Application command has no handler: ${command}`);
    return [command, handler];
  })
));

async function validateProjectRoot(config, requireDirectory) {
  const root = resolve(config.projectRoot);
  const existing = await nearestExisting(root);
  const entry = await lstat(existing);
  if (entry.isSymbolicLink() || !entry.isDirectory()) {
    throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot must resolve through real directories');
  }
  const existingReal = await realpath(existing);
  const suffix = relative(existing, root);
  if (suffix.split(sep).includes('..')) {
    throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot contains an unsafe path component');
  }
  const resolvedRoot = resolve(existingReal, suffix);
  if (requireDirectory) {
    const target = await lstat(resolvedRoot).catch((error) => {
      throw new ApplicationCommandError('input-unreadable', `projectRoot is unavailable: ${root}`, { cause: error });
    });
    if (target.isSymbolicLink() || !target.isDirectory()) {
      throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot must be a real directory');
    }
  }
  return { ...config, projectRoot: resolvedRoot };
}

async function nearestExisting(path) {
  let current = path;
  while (true) {
    try {
      await lstat(current);
      return current;
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error;
      const parent = dirname(current);
      if (parent === current) throw error;
      current = parent;
    }
  }
}

function normalizeConfig(config) {
  if (!config || typeof config !== 'object' || Array.isArray(config)) {
    throw new ApplicationCommandError('configuration-invalid', 'Application command configuration is required');
  }
  if (Object.hasOwn(config, 'inputPaths')) {
    throw new ApplicationCommandError('configuration-invalid', 'Input protection is derived from verified artifact handoffs; inputPaths is not an Application option');
  }
  if (typeof config.repoRoot !== 'string' || !config.repoRoot || typeof config.projectRoot !== 'string' || !config.projectRoot) {
    throw new ApplicationCommandError('configuration-invalid', 'repoRoot and projectRoot are required');
  }
  const trackBRuntimeDependencies = config.trackBRuntimeDependencies ?? {};
  if (!isPlainObject(trackBRuntimeDependencies)) {
    throw new ApplicationCommandError('configuration-invalid', 'trackBRuntimeDependencies must be an object');
  }
  return {
    repoRoot: resolve(config.repoRoot),
    projectRoot: resolve(config.projectRoot),
    trackBRuntimeDependencies
  };
}

function normalizeCommandError(error) {
  if (error instanceof ApplicationCommandError) return error;
  if (error instanceof NarrativeCourierReleaseError) {
    return new ApplicationCommandError(error.code, error.message, {
      ...(error.cause === undefined ? {} : { cause: error.cause }),
      ...(error.details === undefined ? {} : { details: error.details }),
      retryable: error.retryable === true
    });
  }
  if (['ENOENT', 'EACCES', 'EPERM', 'EISDIR'].includes(error?.code)) {
    return new ApplicationCommandError('input-unreadable', error.message, { cause: error, details: { code: error.code } });
  }
  return new ApplicationCommandError('command-failed', error instanceof Error ? error.message : String(error), { cause: error });
}

function failureEnvelope(command, metadata, error) {
  return {
    schemaVersion: 1,
    ok: false,
    command,
    metadata,
    error: {
      code: error.code,
      message: error.message,
      retryable: error.retryable === true,
      ...(error.details === undefined ? {} : { details: error.details })
    }
  };
}
