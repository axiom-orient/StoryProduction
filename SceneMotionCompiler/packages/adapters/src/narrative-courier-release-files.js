import { constants as fsConstants } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { lstat, mkdir, open, realpath, rename, unlink } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve, sep } from 'node:path';
import { sha256Hex } from '../../core/src/canonical.js';
import { NarrativeCourierReleaseError, contractError } from './narrative-courier-release-support.js';

const HEX_SHA256 = /^[0-9a-f]{64}$/;

export async function resolveDeclaredRoot(rootValue) {
  if (typeof rootValue !== 'string' || rootValue.trim().length === 0) {
    throw contractError('configuration-path-unsafe', 'projectRoot is required');
  }
  const declared = resolve(rootValue);
  let stat;
  try {
    stat = await lstat(declared);
  } catch (error) {
    throw contractError('input-unreadable', 'projectRoot is unreadable: ' + declared, { cause: error });
  }
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw contractError('configuration-path-unsafe', 'projectRoot must be a non-symlink directory');
  }
  const canonical = await realpath(declared);
  const canonicalStat = await lstat(canonical);
  if (canonicalStat.isSymbolicLink() || !canonicalStat.isDirectory()) {
    throw contractError('configuration-path-unsafe', 'projectRoot realpath is not a directory');
  }
  return canonical;
}

export async function readJsonArtifact(root, artifactRef, label, { requireHash = true } = {}) {
  if (!artifactRef || typeof artifactRef !== 'object' || Array.isArray(artifactRef)) {
    throw contractError('artifact-ref-invalid', label + ' reference must be an object');
  }
  const path = safeRelativePath(artifactRef.path, label + '.path');
  if (requireHash && !HEX_SHA256.test(String(artifactRef.sha256 ?? ''))) {
    throw contractError('artifact-ref-invalid', label + '.sha256 must be lowercase SHA-256');
  }
  const target = await resolveSafeFile(root, path, label);
  let bytes;
  try {
    bytes = await readBoundedJsonBytes(target);
  } catch (error) {
    throw contractError('input-unreadable', label + ' is unreadable: ' + path, { cause: error });
  }
  const actual = sha256Hex(bytes);
  if (requireHash && actual !== artifactRef.sha256) {
    throw contractError('artifact-hash-mismatch', label + ' SHA-256 mismatch', { details: { path, expected: artifactRef.sha256, actual } });
  }
  let value;
  try {
    value = JSON.parse(new TextDecoder('utf-8', {fatal:true}).decode(bytes));
  } catch (error) {
    throw contractError('artifact-invalid', label + ' is not valid UTF-8 JSON', { cause: error });
  }
  return { value, bytes, sha256: actual, path: target };
}

export function safeRelativePath(value, label) {
  if (typeof value !== 'string' || value.trim().length === 0 || value.includes('\0') || value.includes('\\') || isAbsolute(value) || value.includes(':')) {
    throw contractError('configuration-path-unsafe', label + ' must be a safe project-relative path');
  }
  const parts = value.split('/');
  if (parts.some((part) => part.length === 0 || part === '.' || part === '..')) {
    throw contractError('configuration-path-unsafe', label + ' must not contain traversal or empty path segments');
  }
  return value;
}

export async function resolveSafeFile(root, path, label) {
  let current = root;
  const parts = path.split('/');
  for (const [index, part] of parts.entries()) {
    current = join(current, part);
    let stat;
    try {
      stat = await lstat(current);
    } catch (error) {
      throw contractError('input-unreadable', label + ' is unreadable: ' + path, { cause: error });
    }
    if (stat.isSymbolicLink()) throw contractError('configuration-path-unsafe', label + ' cannot be a symlink: ' + path);
    if (index < parts.length - 1 && !stat.isDirectory()) {
      throw contractError('configuration-path-unsafe', label + ' parent is not a directory: ' + path);
    }
    if (index === parts.length - 1 && !stat.isFile()) {
      throw contractError('configuration-path-unsafe', label + ' must be a regular file: ' + path);
    }
  }
  const canonical = await realpath(current);
  if (canonical !== root && !canonical.startsWith(root + sep)) {
    throw contractError('configuration-path-unsafe', label + ' escapes projectRoot: ' + path);
  }
  const finalStat = await lstat(canonical);
  if (finalStat.isSymbolicLink() || !finalStat.isFile()) {
    throw contractError('configuration-path-unsafe', label + ' must resolve to a regular file: ' + path);
  }
  return canonical;
}

export async function ensureDirectory(root, path) {
  const safePath = safeRelativePath(path, 'projection directory');
  let current = root;
  for (const part of safePath.split('/')) {
    current = join(current, part);
    try {
      const stat = await lstat(current);
      if (stat.isSymbolicLink() || !stat.isDirectory()) {
        throw contractError('configuration-path-unsafe', 'projection directory is not a non-symlink directory: ' + safePath);
      }
    } catch (error) {
      if (error instanceof NarrativeCourierReleaseError) throw error;
      if (error.code !== 'ENOENT') throw error;
      await mkdir(current);
      const stat = await lstat(current);
      if (stat.isSymbolicLink() || !stat.isDirectory()) throw contractError('configuration-path-unsafe', 'projection directory creation was unsafe: ' + safePath);
    }
  }
  return current;
}

export async function atomicWrite(target, bytes) {
  const parent = dirname(target);
  const name = target.slice(parent.length + 1);
  const temporary = join(parent, '.' + name + '.' + randomUUID() + '.tmp');
  let handle;
  try {
    handle = await open(temporary, 'wx', 0o600);
    await handle.writeFile(bytes);
    await handle.sync();
    await handle.close();
    handle = null;
    try {
      const existing = await lstat(target);
      if (existing.isSymbolicLink() || !existing.isFile()) {
        throw contractError('configuration-path-unsafe', 'projection output is not a regular file: ' + target);
      }
    } catch (error) {
      if (error instanceof NarrativeCourierReleaseError) throw error;
      if (error.code !== 'ENOENT') throw error;
    }
    await rename(temporary, target);
  } finally {
    if (handle) await handle.close().catch(() => {});
    await unlink(temporary).catch((error) => { if (error.code !== 'ENOENT') throw error; });
  }
}

export async function readBoundedJsonBytes(path) {
  const limit = 8 * 1024 * 1024;
  const handle = await open(path, fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0) | (fsConstants.O_NONBLOCK ?? 0));
  try {
    const stat = await handle.stat();
    if (!stat.isFile() || stat.size > limit) throw contractError('artifact-size-invalid', 'JSON artifact must be a regular file of at most 8 MiB');
    const buffer = Buffer.alloc(limit + 1); let offset = 0;
    while (offset < buffer.length) { const {bytesRead} = await handle.read(buffer, offset, buffer.length - offset, null); if (!bytesRead) break; offset += bytesRead; }
    if (offset > limit) throw contractError('artifact-size-invalid', 'JSON artifact grew beyond 8 MiB');
    return buffer.subarray(0, offset);
  } finally { await handle.close(); }
}
