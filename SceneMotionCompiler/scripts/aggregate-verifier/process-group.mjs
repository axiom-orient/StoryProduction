export function processGroupProbe(pid) {
  const checkedAt = new Date().toISOString();
  if (process.platform === 'win32') {
    return { checked: false, absent: null, checkedAt, error: 'process-group probing is unsupported on Windows' };
  }
  try {
    process.kill(-pid, 0);
    return { checked: true, absent: false, checkedAt, error: null };
  } catch (error) {
    if (error?.code === 'ESRCH') return { checked: true, absent: true, checkedAt, error: null };
    return { checked: true, absent: false, checkedAt, error: error?.message ?? String(error) };
  }
}

function signalProcessGroup(pid, signal) {
  if (process.platform === 'win32') {
    return { sent: false, checked: false, error: 'process-group signalling is unsupported on Windows' };
  }
  try {
    process.kill(-pid, signal);
    return { sent: true, checked: true, error: null };
  } catch (error) {
    if (error?.code === 'ESRCH') return { sent: false, checked: true, alreadyAbsent: true, error: null };
    return { sent: false, checked: true, error: error?.message ?? String(error) };
  }
}

export function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

async function waitForProcessGroupAbsence(pid, timeoutMs) {
  const startedAt = Date.now();
  let lastProbe = processGroupProbe(pid);
  while (lastProbe.checked && !lastProbe.absent && Date.now() - startedAt < timeoutMs) {
    await delay(Math.min(50, Math.max(1, timeoutMs - (Date.now() - startedAt))));
    lastProbe = processGroupProbe(pid);
  }
  return {
    ...lastProbe,
    waitedMs: Date.now() - startedAt,
    timedOut: lastProbe.checked && !lastProbe.absent,
  };
}

export async function terminateOwnedProcessGroup(pid, timings, reason) {
  const cleanup = {
    attempted: true,
    reason,
    ownerPid: pid,
    processGroupId: pid,
    processGroupChecked: true,
    term: null,
    afterTerm: null,
    kill: null,
    afterKill: null,
    finalProbe: null,
    groupAbsent: false,
    survivorStatus: 'NOT_PROVEN',
    error: null,
  };

  const before = processGroupProbe(pid);
  if (!before.checked) {
    cleanup.processGroupChecked = false;
    cleanup.finalProbe = before;
    cleanup.error = before.error;
    return cleanup;
  }
  if (before.absent) {
    cleanup.finalProbe = before;
    cleanup.groupAbsent = true;
    cleanup.survivorStatus = 'ABSENT';
    return cleanup;
  }

  cleanup.term = { ...signalProcessGroup(pid, 'SIGTERM'), sentAt: new Date().toISOString() };
  cleanup.afterTerm = await waitForProcessGroupAbsence(pid, timings.termGraceMs);
  if (!cleanup.afterTerm.absent) {
    cleanup.kill = { ...signalProcessGroup(pid, 'SIGKILL'), sentAt: new Date().toISOString() };
    cleanup.afterKill = await waitForProcessGroupAbsence(pid, timings.killGraceMs);
    cleanup.finalProbe = cleanup.afterKill;
  } else {
    cleanup.finalProbe = cleanup.afterTerm;
  }
  cleanup.groupAbsent = cleanup.finalProbe?.checked === true && cleanup.finalProbe.absent === true;
  cleanup.survivorStatus = cleanup.groupAbsent ? 'ABSENT' : 'NOT_PROVEN';
  if (!cleanup.groupAbsent) cleanup.error = cleanup.finalProbe?.error ?? 'owned process group remains or could not be proven absent';
  return cleanup;
}

export function noCleanupEvidence(reason = 'not-started') {
  return {
    attempted: false,
    reason,
    ownerPid: null,
    processGroupId: null,
    processGroupChecked: false,
    term: null,
    afterTerm: null,
    kill: null,
    afterKill: null,
    finalProbe: null,
    groupAbsent: null,
    survivorStatus: 'NOT_APPLICABLE',
    error: null,
  };
}
