import { lstat } from 'node:fs/promises';

export function fileIdentity(stat) {
  return {
    dev: Number(stat.dev),
    ino: Number(stat.ino),
    mode: Number(stat.mode),
    size: Number(stat.size),
  };
}

function sameFileIdentity(opened, observed) {
  return Boolean(opened && observed && opened.dev === observed.dev && opened.ino === observed.ino);
}

export async function inspectClosedLog(logPath, openedIdentity, writtenBytes, writeError, openError = null) {
  let stat;
  try {
    stat = await lstat(logPath);
  } catch (error) {
    return {
      opened: openedIdentity,
      observed: null,
      valid: false,
      error: `log path unavailable after close: ${error.code ?? error.message}`,
    };
  }
  const observed = fileIdentity(stat);
  const regular = stat.isFile() && !stat.isSymbolicLink();
  const identityMatches = sameFileIdentity(openedIdentity, observed);
  const sizeMatches = observed.size === writtenBytes;
  let error = null;
  if (!regular) error = 'log path is not a regular non-symlink file';
  else if (openError) error = `log descriptor identity unavailable: ${openError}`;
  else if (!identityMatches) error = 'log file identity changed while gate was running';
  else if (!sizeMatches) error = `log size mismatch: expected=${writtenBytes} observed=${observed.size}`;
  else if (writeError) error = `log stream error: ${writeError}`;
  return { opened: openedIdentity, observed, valid: !error, error };
}

export function createLogCapture(log, maxLogBytes, onOverflow) {
  const marker = Buffer.from(`\n[aggregate verifier log truncated after ${maxLogBytes} bytes]\n`);
  const contentLimit = Math.max(0, maxLogBytes - marker.length);
  const capture = {
    observedBytes: 0,
    writtenBytes: 0,
    truncated: false,
    writeError: null,
  };
  const addObservedBytes = (amount) => {
    capture.observedBytes = Math.min(Number.MAX_SAFE_INTEGER, capture.observedBytes + amount);
  };
  const write = (buffer) => {
    if (buffer.length === 0) return;
    try {
      log.write(buffer);
      capture.writtenBytes += buffer.length;
    } catch (error) {
      capture.writeError = capture.writeError ?? error.message;
    }
  };
  const onData = (chunk) => {
    const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    addObservedBytes(buffer.length);
    if (capture.truncated) return;
    const available = Math.max(0, contentLimit - capture.writtenBytes);
    const take = Math.min(available, buffer.length);
    if (take > 0) write(buffer.subarray(0, take));
    if (take < buffer.length) {
      write(marker);
      capture.truncated = true;
      onOverflow();
    }
  };
  return { capture, onData };
}

export function waitForReadableClose(stream) {
  if (!stream || stream.destroyed || stream.readableEnded) return Promise.resolve();
  return new Promise((resolveReadable) => {
    let settled = false;
    const settle = () => {
      if (settled) return;
      settled = true;
      stream.off('end', settle);
      stream.off('close', settle);
      stream.off('error', settle);
      resolveReadable();
    };
    stream.once('end', settle);
    stream.once('close', settle);
    stream.once('error', settle);
  });
}

export async function closeLog(stream) {
  if (!stream || stream.destroyed) return;
  await new Promise((resolveClose) => {
    if (stream.closed) {
      resolveClose();
      return;
    }
    const settle = () => {
      stream.off('close', settle);
      stream.off('error', settle);
      resolveClose();
    };
    stream.once('close', settle);
    stream.once('error', settle);
    stream.end();
  });
}
