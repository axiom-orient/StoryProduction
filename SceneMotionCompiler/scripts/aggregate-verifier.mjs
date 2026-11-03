import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { VerifyError } from './aggregate-verifier/errors.mjs';
import { runGate } from './aggregate-verifier/gate-runner.mjs';
import { noCleanupEvidence } from './aggregate-verifier/process-group.mjs';
import {
  pendingSourceIdentity,
  verifyProductionSourceIdentity,
} from './aggregate-verifier/source-identity.mjs';
import {
  createDefaultGates,
  validateGates,
} from './aggregate-verifier/configuration.mjs';
import {
  prepareOutputDir,
  prepareWorkspace,
  releaseLock,
  writeAtomicJson,
} from './aggregate-verifier/workspace.mjs';

export {
  DEFAULT_GATE_NAMES,
  DEFAULT_TIMEOUT_MS,
  DEFAULT_TERM_GRACE_MS,
  DEFAULT_KILL_GRACE_MS,
  DEFAULT_MAX_LOG_BYTES,
  resolveGateTimings,
} from './aggregate-verifier/configuration.mjs';
export { createDefaultGates };

export { VerifyError };
export { SOURCE_MANIFEST_RELATIVE_PATH } from './aggregate-verifier/source-identity.mjs';

/**
 * Aggregate verification is intentionally a small delivery-infrastructure
 * boundary.  It owns only the child process groups it starts and the
 * `.tmp/verify` proof workspace.  Product/runtime contracts stay in the
 * individual gates.
 */

export const SUMMARY_SCHEMA_VERSION = 2;

function gateCommandText(gate) {
  return gate.command.map((value) => JSON.stringify(value)).join(' ');
}


function relativeProofPath(root, path) {
  const value = relative(root, path);
  return value && !value.startsWith('..') && !isAbsolute(value) ? value : path;
}

function blankGateRecord(gate, root, outputDir) {
  const logPath = join(outputDir, `${gate.name}.log`);
  return {
    gate: gate.name,
    command: gateCommandText(gate),
    status: 'NOT_RUN',
    exitCode: null,
    signal: null,
    timedOut: false,
    cancellationRequested: false,
    startedAt: null,
    endedAt: null,
    durationMs: 0,
    timeoutMs: gate.timeoutMs,
    termGraceMs: gate.termGraceMs,
    killGraceMs: gate.killGraceMs,
    maxLogBytes: gate.maxLogBytes,
    observedOutputBytes: 0,
    logBytes: 0,
    logTruncated: false,
    resourceLimitExceeded: false,
    logPath: relativeProofPath(root, logPath),
    logCreated: false,
    logIdentity: { opened: null, observed: null, valid: null, error: null },
    cleanup: noCleanupEvidence(),
    error: null,
  };
}


function overallStatus(records, cancelled = false) {
  if (records.some((record) => record.status === 'FAIL')) return 'FAIL';
  if (cancelled || records.some((record) => record.status === 'CANCELLED')) return 'CANCELLED';
  if (records.some((record) => record.status === 'TIMEOUT')) return 'TIMEOUT';
  if (records.length > 0 && records.every((record) => record.status === 'PASS')) return 'PASS';
  return 'FAIL';
}

function defaultSummary(root, outputDir, gates) {
  const records = gates.map((gate) => blankGateRecord(gate, root, outputDir));
  return {
    schemaVersion: SUMMARY_SCHEMA_VERSION,
    status: 'RUNNING',
    startedAt: new Date().toISOString(),
    endedAt: null,
    durationMs: 0,
    failFast: true,
    gateCount: records.length,
    gates: records,
    sourceIdentity: pendingSourceIdentity(),
    lock: { acquired: true, released: false, release: null },
    aggregateCleanup: { processGroupsChecked: 0, processGroupsAbsent: 0, survivorsNotProven: 0 },
    error: null,
  };
}

function updateCleanupTotals(summary) {
  const checked = summary.gates.filter((record) => record.cleanup?.processGroupChecked === true);
  summary.aggregateCleanup = {
    processGroupsChecked: checked.length,
    processGroupsAbsent: checked.filter((record) => record.cleanup.groupAbsent === true).length,
    survivorsNotProven: checked.filter((record) => record.cleanup.groupAbsent !== true).length,
  };
}

function resultExitCode(status) {
  if (status === 'PASS') return 0;
  if (status === 'TIMEOUT') return 124;
  if (status === 'CANCELLED') return 130;
  return 1;
}

/**
 * Run the aggregate verifier.  `gates` is a normal command-definition API,
 * not a test-mode switch; production callers use `createDefaultGates()` and
 * tests can exercise the same process-custody implementation with bounded
 * fixture commands.
 */
export async function runAggregateVerification({
  root = process.cwd(),
  outputDir = join(resolve(root), '.tmp', 'verify'),
  lockPath = join(resolve(root), '.tmp', 'verify.lock'),
  env = process.env,
  gates = createDefaultGates(outputDir, env),
  failFast = true,
  signal = null,
  summaryWriter = writeAtomicJson,
} = {}) {
  if (typeof summaryWriter !== 'function') {
    throw new VerifyError('E_VERIFY_CONFIG', 'summaryWriter must be a function');
  }
  const normalizedGates = validateGates(gates, env);
  const workspace = await prepareWorkspace(root, outputDir, lockPath);
  const { lock } = workspace;
  const summaryPath = join(workspace.outputDir, 'summary.json');
  const startedMs = Date.now();
  let summary;
  let finalError = null;
  let release = null;
  let summaryPublished = false;
  let summaryPublicationAttempted = false;

  const publishSnapshot = async () => {
    // Give an injected writer an immutable snapshot.  The production writer is
    // still the atomic fsync+rename implementation; this injection is a normal
    // I/O boundary for deterministic failure testing, not a production bypass.
    const snapshot = JSON.parse(JSON.stringify(summary));
    await summaryWriter(summaryPath, snapshot);
  };

  try {
    await prepareOutputDir(workspace.outputDir);
    summary = defaultSummary(workspace.root, workspace.outputDir, normalizedGates);
    summary.failFast = failFast;
    summary.lock = { acquired: true, acquiredAt: lock.acquiredAt, released: false, release: null };
    await publishSnapshot();

    const context = {
      root: workspace.root,
      outputDir: workspace.outputDir,
      env: { ...env },
      signal,
    };
    let cancelledBeforeStart = Boolean(signal?.aborted);
    for (let index = 0; index < normalizedGates.length; index += 1) {
      const gate = normalizedGates[index];
      const record = summary.gates[index];
      if (cancelledBeforeStart || signal?.aborted) {
        cancelledBeforeStart = true;
        record.error = 'aggregate signal received before gate start';
        record.cleanup = noCleanupEvidence('aggregate-signal-before-start');
        await publishSnapshot();
        continue;
      }
      await runGate(gate, context, record);
      updateCleanupTotals(summary);
      await publishSnapshot();
      if (record.status !== 'PASS' && failFast) {
        break;
      }
    }
    const cancelled = Boolean(signal?.aborted) || summary.gates.some((record) => record.status === 'CANCELLED');
    summary.status = overallStatus(summary.gates, cancelled);
    // A source-manifest gate failure is itself a source-identity precondition
    // failure.  Run the final verifier in that case too so production reports
    // the typed identity error instead of hiding it behind a generic gate exit.
    // Other failed gates keep identity PENDING because the final proof was not
    // reached; no failed aggregate may be promoted by this diagnostic pass.
    const sourceManifestGate = summary.gates.find((record) => record.gate === 'source-manifest');
    if (summary.status === 'PASS' || sourceManifestGate?.status === 'FAIL') {
      summary.sourceIdentity = await verifyProductionSourceIdentity(workspace.root);
    }
  } catch (error) {
    finalError = error;
    if (!summary) summary = defaultSummary(workspace.root, workspace.outputDir, normalizedGates);
    if (error?.code === 'E_VERIFY_SOURCE_IDENTITY' && error.details?.sourceIdentity) {
      summary.sourceIdentity = error.details.sourceIdentity;
    }
    summary.error = { code: error.code ?? 'E_VERIFY', message: error.message };
    const running = summary.gates.find((record) => record.startedAt && !record.endedAt);
    if (running) {
      running.status = signal?.aborted ? 'CANCELLED' : 'FAIL';
      running.error = error.message;
      running.endedAt = new Date().toISOString();
      running.durationMs = 0;
    }
    summary.status = signal?.aborted ? 'CANCELLED' : 'FAIL';
  } finally {
    if (summary) {
      updateCleanupTotals(summary);
      summary.endedAt = new Date().toISOString();
      summary.durationMs = Date.now() - startedMs;
    }
    release = await releaseLock(workspace.lockPath, lock.token);
    if (summary) {
      summary.lock = {
        ...summary.lock,
        released: release.lockAbsentAfterRelease === true,
        release,
      };
      if (release.lockAbsentAfterRelease !== true) {
        summary.status = 'FAIL';
        summary.error = summary.error ?? { code: 'E_VERIFY_LOCK_RELEASE', message: release.error ?? 'lock release not proven' };
      }
      updateCleanupTotals(summary);
      try {
        summaryPublicationAttempted = true;
        await publishSnapshot();
        summaryPublished = true;
      } catch (error) {
        const message = `final summary publication failed: ${error?.message ?? String(error)}`;
        const publicationError = new VerifyError(
          'E_VERIFY_SUMMARY_PUBLISH',
          message,
          { path: summaryPath },
        );
        // A failed final publication is the terminal delivery-infrastructure
        // failure, even when an earlier snapshot/write already failed.  Keep
        // any earlier error in the bounded summary cause, but expose this
        // typed error to callers so they cannot mistake the result for a
        // durably published final proof.
        finalError = publicationError;
        summary.status = 'FAIL';
        summary.error = {
          code: publicationError.code,
          message: publicationError.message,
          cause: summary.error ?? null,
        };
        summaryPublished = false;
      }
    }
  }

  if (!summary) {
    throw finalError ?? new VerifyError('E_VERIFY', 'aggregate verification did not produce a summary');
  }
  const status = summary.status;
  return {
    status,
    exitCode: resultExitCode(status),
    summary,
    // A path is only returned as the final summary when the final writer
    // completed.  A prior RUNNING snapshot may exist at summaryAttemptedPath,
    // but it is not a completed proof and must not be presented as one.
    summaryPath: summaryPublished ? summaryPath : null,
    summaryAttemptedPath: summaryPath,
    summaryPublished,
    summaryPublicationAttempted,
    error: finalError,
    lockRelease: release,
  };
}

async function main() {
  const controller = new AbortController();
  const onSignal = (signalName) => {
    if (!controller.signal.aborted) controller.abort(new Error(`received ${signalName}`));
  };
  process.on('SIGINT', () => onSignal('SIGINT'));
  process.on('SIGTERM', () => onSignal('SIGTERM'));

  try {
    const result = await runAggregateVerification({
      root: process.cwd(),
      outputDir: resolve(process.cwd(), '.tmp', 'verify'),
      lockPath: resolve(process.cwd(), '.tmp', 'verify.lock'),
      signal: controller.signal,
    });
    for (const record of result.summary.gates) {
      const seconds = Math.round(record.durationMs / 1000);
      const detail = record.error ?? (record.signal ? `signal=${record.signal}` : `exit=${record.exitCode ?? '-'}`);
      console.log(`${record.status.padEnd(9)} ${record.gate.padEnd(20)} ${String(seconds).padStart(4)}s  ${detail}`);
    }
    if (!result.summaryPublished) {
      const detail = String(result.error?.message ?? 'final summary was not durably published').slice(0, 512);
      console.error(
        `FAIL  aggregate verifier E_VERIFY_SUMMARY_PUBLISH: ${detail}; ` +
        `recovery path: ${relativeProofPath(process.cwd(), result.summaryAttemptedPath)}`,
      );
    }
    console.log(JSON.stringify({
      status: result.status,
      gateCount: result.summary.gates.length,
      summary: result.summaryPath ? relativeProofPath(process.cwd(), result.summaryPath) : null,
      summaryAttemptedPath: relativeProofPath(process.cwd(), result.summaryAttemptedPath),
      summaryPublished: result.summaryPublished,
      lockReleased: result.summary.lock.released,
    }, null, 2));
    process.exitCode = result.exitCode;
  } catch (error) {
    const code = error?.code ?? 'E_VERIFY';
    console.error(`FAIL  aggregate verifier ${code}: ${error?.message ?? String(error)}`);
    process.exitCode = code === 'E_VERIFY_LOCKED' ? 73 : code === 'E_VERIFY_SYMLINK' ? 74 : 1;
  }
}

const isMain = process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url));
if (isMain) await main();
