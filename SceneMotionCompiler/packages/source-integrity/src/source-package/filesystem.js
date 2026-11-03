import { constants as fsConstants } from 'node:fs';
import { lstat, open, readdir } from 'node:fs/promises';

export async function assertNotSymlink(path, label) {
  const metadata = await lstat(path);
  if (metadata.isSymbolicLink()) throw new Error(`${label} must not be a symlink: ${path}`);
}

export async function assertAbsentOrRegularFile(path, label) {
  try {
    const metadata = await lstat(path);
    if (metadata.isSymbolicLink()) throw new Error(`${label} must not be a symlink: ${path}`);
    if (!metadata.isFile()) throw new Error(`${label} must be a regular file when it exists: ${path}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}

export async function assertEmptyExtractionDirectory(path) {
  try {
    const metadata = await lstat(path);
    if (metadata.isSymbolicLink()) throw new Error(`Extraction directory must not be a symlink: ${path}`);
    if (!metadata.isDirectory()) throw new Error(`Extraction path must be a directory: ${path}`);
    const entries = await readdir(path);
    if (entries.length) throw new Error(`Extraction directory must be empty: ${path}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}

export async function readBoundedFile(path, maximumBytes, label) {
  const handle = await open(path, fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0));
  try {
    const metadata = await handle.stat();
    if (!metadata.isFile()) throw new Error(`${label} must be a regular file: ${path}`);
    if (metadata.size > maximumBytes) throw new Error(`${label} exceeds ${maximumBytes} byte limit`);
    const bytes = await handle.readFile();
    if (bytes.length > maximumBytes) throw new Error(`${label} exceeds ${maximumBytes} byte limit`);
    if (bytes.length !== metadata.size) throw new Error(`${label} changed while it was read`);
    return bytes;
  } finally {
    await handle.close();
  }
}

export async function syncFile(path) {
  const handle = await open(path, 'r');
  try { await handle.sync(); } finally { await handle.close(); }
}

export async function syncDirectory(path) {
  let handle;
  try {
    handle = await open(path, 'r');
    await handle.sync();
  } catch (error) {
    if (error.code !== 'EINVAL' && error.code !== 'ENOTSUP' && error.code !== 'EISDIR') throw error;
  } finally {
    await handle?.close();
  }
}
