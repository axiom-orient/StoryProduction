import { createWriteStream, fstatSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { join } from 'node:path';
import {
  closeLog,
  createLogCapture,
  fileIdentity,
  inspectClosedLog,
  waitForReadableClose,
} from './log-capture.mjs';
import {
  delay,
  noCleanupEvidence,
  processGroupProbe,
  terminateOwnedProcessGroup,
} from './process-group.mjs';

export async function runGate(gate, context, record) {
  const startedMs = Date.now();
  const startedAt = new Date(startedMs).toISOString();
  const logAbsolute = join(context.outputDir, `${gate.name}.log`);
  record.startedAt = startedAt;

  let log;
  try {
    log = createWriteStream(logAbsolute, { flags: 'w', mode: 0o600 });
    await new Promise((resolveOpen, rejectOpen) => {
      const onOpen = () => { log.off('error', onError); resolveOpen(); };
      const onError = (error) => { log.off('open', onOpen); rejectOpen(error); };
      log.once('open', onOpen);
      log.once('error', onError);
    });
  } catch (error) {
    record.status = 'FAIL';
    record.error = `log-open: ${error.message}`;
    record.endedAt = new Date().toISOString();
    record.durationMs = Date.now() - startedMs;
    record.cleanup = noCleanupEvidence('spawn-not-started');
    return record;
  }
  record.logCreated = true;

  let openedIdentity = null;
  let openedIdentityError = null;
  try {
    if (!Number.isInteger(log.fd)) throw new Error('log descriptor is unavailable after open');
    openedIdentity = fileIdentity(fstatSync(log.fd));
  } catch (error) {
    openedIdentityError = error.message;
  }

  let child;
  try {
    const [command, ...args] = gate.command;
    child = spawn(command, args, {
      cwd: context.root,
      env: {
        ...context.env,
        SMC_VERIFY_GATE_LOG_PATH: logAbsolute,
        SMC_VERIFY_GATE_MAX_LOG_BYTES: String(gate.maxLogBytes),
      },
      detached: process.platform !== 'win32',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });
  } catch (error) {
    await closeLog(log);
    record.status = 'FAIL';
    record.error = `spawn: ${error.message}`;
    record.logIdentity = await inspectClosedLog(logAbsolute, openedIdentity, 0, null, openedIdentityError);
    record.endedAt = new Date().toISOString();
    record.durationMs = Date.now() - startedMs;
    record.cleanup = noCleanupEvidence('spawn-not-started');
    return record;
  }

  const childPid = child.pid ?? null;
  let logError = null;
  log.on('error', (error) => { logError = error; });

  let resolveLogOverflow;
  const logOverflowPromise = new Promise((resolveOverflow) => {
    resolveLogOverflow = resolveOverflow;
  });
  const logCapture = createLogCapture(log, gate.maxLogBytes, () => {
    if (child.stdout) child.stdout.pause();
    if (child.stderr) child.stderr.pause();
    resolveLogOverflow({ kind: 'RESOURCE_LIMIT' });
  });
  child.stdout?.on('data', logCapture.onData);
  child.stderr?.on('data', logCapture.onData);
  const streamsDone = Promise.all([
    waitForReadableClose(child.stdout),
    waitForReadableClose(child.stderr),
  ]);

  const exitPromise = new Promise((resolveExit) => {
    let settled = false;
    const settle = (value) => {
      if (settled) return;
      settled = true;
      resolveExit(value);
    };
    child.once('error', (error) => settle({ kind: 'ERROR', error }));
    child.once('exit', (code, signal) => settle({ kind: 'EXIT', code, signal }));
  });

  let timer;
  let abortHandler;
  const triggerPromise = new Promise((resolveTrigger) => {
    timer = setTimeout(() => resolveTrigger({ kind: 'TIMEOUT' }), gate.timeoutMs);
    if (context.signal) {
      abortHandler = () => resolveTrigger({ kind: 'CANCELLED', reason: context.signal.reason?.message ?? 'aggregate verification cancelled' });
      if (context.signal.aborted) abortHandler();
      else context.signal.addEventListener('abort', abortHandler, { once: true });
    }
  });

  let first;
  try {
    first = await Promise.race([
      exitPromise,
      triggerPromise,
      logOverflowPromise,
    ]);
  } finally {
    clearTimeout(timer);
    if (context.signal && abortHandler) context.signal.removeEventListener('abort', abortHandler);
  }

  let cleanup = null;
  let exit = first;
  if (first.kind === 'TIMEOUT' || first.kind === 'CANCELLED' || first.kind === 'RESOURCE_LIMIT') {
    record.timedOut = first.kind === 'TIMEOUT';
    record.cancellationRequested = first.kind === 'CANCELLED';
    record.resourceLimitExceeded = first.kind === 'RESOURCE_LIMIT';
    cleanup = childPid
      ? await terminateOwnedProcessGroup(
        childPid,
        gate,
        first.kind === 'TIMEOUT' ? 'gate-timeout' : first.kind === 'CANCELLED' ? 'aggregate-signal' : 'log-byte-limit',
      )
      : noCleanupEvidence('spawn-without-pid');
    exit = await Promise.race([
      exitPromise,
      delay(gate.termGraceMs + gate.killGraceMs + 1_000).then(() => ({ kind: 'CLEANUP_DEADLINE' })),
    ]);
  }

  await Promise.race([streamsDone, delay(1_000)]);
  if (!child.stdout?.readableEnded && !child.stdout?.destroyed) child.stdout?.destroy();
  if (!child.stderr?.readableEnded && !child.stderr?.destroyed) child.stderr?.destroy();

  if (exit.kind === 'ERROR') {
    record.error = `spawn: ${exit.error.message}`;
    record.status = record.cancellationRequested ? 'CANCELLED' : record.timedOut ? 'TIMEOUT' : 'FAIL';
  } else if (exit.kind === 'CLEANUP_DEADLINE') {
    record.error = 'child exit was not observed before bounded cleanup deadline';
    record.status = record.cancellationRequested ? 'CANCELLED' : record.timedOut ? 'TIMEOUT' : 'FAIL';
  } else {
    record.exitCode = exit.code;
    record.signal = exit.signal;
    if (record.resourceLimitExceeded || logCapture.capture.truncated) record.status = 'FAIL';
    else if (record.cancellationRequested) record.status = 'CANCELLED';
    else if (record.timedOut) record.status = 'TIMEOUT';
    else if (exit.code === 0 && !exit.signal) record.status = 'PASS';
    else record.status = 'FAIL';
  }
  if (record.resourceLimitExceeded || logCapture.capture.truncated) {
    record.resourceLimitExceeded = true;
    record.status = 'FAIL';
    record.error = record.error ?? `log byte limit exceeded (${gate.maxLogBytes} bytes)`;
  }

  // A normally exiting gate must also have no owned group left behind.  If a
  // gate leaves one, terminate that group and downgrade the gate to FAIL.
  if (!cleanup && childPid) {
    const probe = processGroupProbe(childPid);
    if (probe.checked && probe.absent) {
      cleanup = {
        ...noCleanupEvidence('normal-exit'),
        attempted: true,
        ownerPid: childPid,
        processGroupId: childPid,
        processGroupChecked: true,
        finalProbe: probe,
        groupAbsent: true,
        survivorStatus: 'ABSENT',
      };
    } else {
      cleanup = await terminateOwnedProcessGroup(childPid, gate, 'gate-exit-cleanup');
      if (record.status === 'PASS') record.status = 'FAIL';
      if (!cleanup.groupAbsent) record.error = record.error ?? 'owned process group absence not proven';
    }
  }
  if (!cleanup) cleanup = noCleanupEvidence('spawn-not-started');

  try {
    await closeLog(log);
  } catch (error) {
    record.error = record.error ?? `log-close: ${error.message}`;
    record.status = 'FAIL';
  }
  record.observedOutputBytes = logCapture.capture.observedBytes;
  record.logBytes = logCapture.capture.writtenBytes;
  record.logTruncated = logCapture.capture.truncated;
  record.logIdentity = await inspectClosedLog(
    logAbsolute,
    openedIdentity,
    logCapture.capture.writtenBytes,
    logError ?? logCapture.capture.writeError,
    openedIdentityError,
  );
  if (!record.logIdentity.valid) {
    record.status = 'FAIL';
    record.error = record.error
      ? `${record.error}; log-integrity: ${record.logIdentity.error}`
      : `log-integrity: ${record.logIdentity.error}`;
  }
  record.cleanup = cleanup;
  record.exitCode = exit.kind === 'EXIT' ? exit.code : record.exitCode;
  record.signal = exit.kind === 'EXIT' ? exit.signal : record.signal;
  record.endedAt = new Date().toISOString();
  record.durationMs = Date.now() - startedMs;
  if (logError) record.error = record.error ?? `log: ${logError.message}`;
  return record;
}
