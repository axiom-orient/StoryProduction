import { join, resolve } from 'node:path';
import { VerifyError } from './errors.mjs';

export const DEFAULT_TIMEOUT_MS = 120_000;
export const DEFAULT_TERM_GRACE_MS = 5_000;
export const DEFAULT_KILL_GRACE_MS = 2_000;
export const DEFAULT_MAX_LOG_BYTES = 10 * 1024 * 1024;
const LIMITS = Object.freeze({
  timeoutMs: { min: 100, max: 900_000 },
  termGraceMs: { min: 50, max: 30_000 },
  killGraceMs: { min: 50, max: 30_000 },
  maxLogBytes: { min: 1_024, max: 100 * 1024 * 1024 },
});

const GATE_DEFINITIONS = Object.freeze([
  ['doctor', ['npm', 'run', 'doctor'], 30_000],
  ['contracts', ['npm', 'run', 'contracts:verify'], 60_000],
  ['syntax', ['npm', 'run', 'syntax:check'], 60_000],
  ['tests', ['npm', 'test'], 900_000],
  ['source-manifest', ['npm', 'run', 'verify:manifest'], 60_000],
  ['source-integrity', ['npm', 'run', 'verify:source'], 180_000],
  ['trackb', ['npm', 'run', 'smoke:trackb'], 120_000],
  ['cli', ['npm', 'run', 'smoke:cli'], 120_000],
  ['studio-contract', ['npm', 'run', 'smoke:studio:contract'], 120_000],
  ['release', ['npm', 'run', 'smoke:release'], 120_000],
]);

export const DEFAULT_GATE_NAMES = Object.freeze(GATE_DEFINITIONS.map(([name]) => name));

const ENV_KEYS = Object.freeze({
  timeoutMs: 'SMC_VERIFY_GATE_TIMEOUT_MS',
  termGraceMs: 'SMC_VERIFY_TERM_GRACE_MS',
  killGraceMs: 'SMC_VERIFY_KILL_GRACE_MS',
  maxLogBytes: 'SMC_VERIFY_MAX_LOG_BYTES',
});

function selectedPackageManager(env) {
  const value = env?.SMC_PACKAGE_MANAGER ?? 'npm';
  if (value !== 'npm' && value !== 'pnpm') {
    throw new VerifyError('E_VERIFY_CONFIG', `SMC_PACKAGE_MANAGER must be "npm" or "pnpm": ${String(value)}`);
  }
  return value;
}

/**
 * Return the Track B production gates. The studio gate receives an output
 * directory inside the aggregate workspace so it cannot overwrite another
 * verification run's proof files.
 */
export function createDefaultGates(outputDir = join(process.cwd(), '.tmp', 'verify'), env = process.env) {
  const packageManager = selectedPackageManager(env);
  const absoluteOutputDir = resolve(outputDir);
  const studioProject = join(absoluteOutputDir, 'studio-project');
  const studioOutput = join(absoluteOutputDir, 'studio-contract');
  return GATE_DEFINITIONS.map(([name, command, timeoutMs]) => {
    const baseCommand = name === 'studio-contract'
      ? ['npm', 'run', 'smoke:studio:contract', '--', studioProject, studioOutput]
      : [...command];
    return {
      name,
      command: [packageManager, ...baseCommand.slice(1)],
      timeoutMs,
      termGraceMs: DEFAULT_TERM_GRACE_MS,
      killGraceMs: DEFAULT_KILL_GRACE_MS,
      maxLogBytes: DEFAULT_MAX_LOG_BYTES,
    };
  });
}

function validateInteger(name, value, limits) {
  if (!Number.isInteger(value) || value < limits.min || value > limits.max) {
    throw new VerifyError(
      'E_VERIFY_CONFIG',
      `${name} must be an integer between ${limits.min} and ${limits.max}: ${String(value)}`,
      { name, value, limits },
    );
  }
  return value;
}

function parseEnvInteger(env, key, fallback, limits) {
  const raw = env?.[key];
  if (raw === undefined || raw === '') return fallback;
  if (!/^[0-9]+$/.test(String(raw))) {
    throw new VerifyError('E_VERIFY_CONFIG', `${key} must contain only decimal digits`, { key, value: raw });
  }
  return validateInteger(key, Number(raw), limits);
}

export function resolveGateTimings(gate, env = process.env) {
  const timeoutMs = validateInteger('timeoutMs', gate.timeoutMs ?? DEFAULT_TIMEOUT_MS, LIMITS.timeoutMs);
  const termGraceMs = validateInteger('termGraceMs', gate.termGraceMs ?? DEFAULT_TERM_GRACE_MS, LIMITS.termGraceMs);
  const killGraceMs = validateInteger('killGraceMs', gate.killGraceMs ?? DEFAULT_KILL_GRACE_MS, LIMITS.killGraceMs);
  const maxLogBytes = validateInteger('maxLogBytes', gate.maxLogBytes ?? DEFAULT_MAX_LOG_BYTES, LIMITS.maxLogBytes);
  // SMC_VERIFY_GATE_TIMEOUT_MS is deliberately a global override.  It wins
  // over each gate's explicit default (including INPUT_TEST_TIMEOUT_MS), and
  // there is no per-gate environment escape hatch.  Bounds are validated
  // before any lock or child process is created.
  return {
    timeoutMs: parseEnvInteger(env, ENV_KEYS.timeoutMs, timeoutMs, LIMITS.timeoutMs),
    termGraceMs: parseEnvInteger(env, ENV_KEYS.termGraceMs, termGraceMs, LIMITS.termGraceMs),
    killGraceMs: parseEnvInteger(env, ENV_KEYS.killGraceMs, killGraceMs, LIMITS.killGraceMs),
    maxLogBytes: parseEnvInteger(env, ENV_KEYS.maxLogBytes, maxLogBytes, LIMITS.maxLogBytes),
  };
}

function validateGate(gate, index, env) {
  if (!gate || typeof gate !== 'object') {
    throw new VerifyError('E_VERIFY_CONFIG', `gate ${index} must be an object`);
  }
  if (typeof gate.name !== 'string' || !/^[a-z0-9][a-z0-9-]*$/.test(gate.name)) {
    throw new VerifyError('E_VERIFY_CONFIG', `gate ${index} has an invalid name`, { gate });
  }
  if (!Array.isArray(gate.command) || gate.command.length === 0 || gate.command.some((value) => typeof value !== 'string' || value.length === 0)) {
    throw new VerifyError('E_VERIFY_CONFIG', `gate ${gate.name} command must be a non-empty string array`, { gate });
  }
  const timings = resolveGateTimings(gate, env);
  return { ...gate, command: [...gate.command], ...timings };
}

export function validateGates(gates, env) {
  if (!Array.isArray(gates) || gates.length === 0) {
    throw new VerifyError('E_VERIFY_CONFIG', 'at least one aggregate gate is required');
  }
  const seen = new Set();
  return gates.map((gate, index) => {
    const normalized = validateGate(gate, index, env);
    if (seen.has(normalized.name)) {
      throw new VerifyError('E_VERIFY_CONFIG', `duplicate gate name: ${normalized.name}`);
    }
    seen.add(normalized.name);
    return normalized;
  });
}
