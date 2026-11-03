import { constants as fsConstants } from 'node:fs';
import { lstat, open, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, resolve } from 'node:path';
import {
  MAX_PROJECT_ROOT_BYTES,
  MAX_PROJECT_ROOT_COMPONENT_BYTES,
  storageIntegrity
} from './trackb-custody-protocol.js';

export async function validateProjectRoot(value) {
  if (typeof value !== 'string' || !isAbsolute(value)) throw storageIntegrity('projectRoot must be an absolute path');
  const path = resolve(value);
  if (Buffer.byteLength(path, 'utf8') > MAX_PROJECT_ROOT_BYTES) {
    throw storageIntegrity(`projectRoot exceeds ${MAX_PROJECT_ROOT_BYTES} bytes`);
  }
  // Walk from the filesystem root so an absolute path cannot enter through a
  // symlinked ancestor.  Every component is bounded and must be a directory;
  // realpath equality also rejects aliases such as a symlinked mount prefix.
  const components = path.split('/').filter(Boolean);
  let current = '/';
  for (const component of components) {
    if (component === '.' || component === '..' || component.includes('\0')
      || Buffer.byteLength(component, 'utf8') > MAX_PROJECT_ROOT_COMPONENT_BYTES) {
      throw storageIntegrity('projectRoot contains an unsafe or oversized path component');
    }
    current = current === '/' ? `/${component}` : `${current}/${component}`;
    const metadata = await lstatNoSymlink(current, 'project root ancestor');
    if (!metadata.isDirectory()) throw storageIntegrity('project root ancestor must be a directory');
    const canonical = await realpath(current).catch((error) => {
      throw storageIntegrity(`project root ancestor realpath failed: ${error.message}`, { cause: error });
    });
    if (canonical !== current) throw storageIntegrity('project root ancestor canonical path differs');
  }
  const metadata = await lstatNoSymlink(path, 'project root');
  if (!metadata.isDirectory()) throw storageIntegrity('projectRoot must be a regular non-symlink directory');
  return path;
}

export async function lstatNoSymlink(path, label) {
  let metadata;
  try { metadata = await lstat(path); }
  catch (error) { throw storageIntegrity(`${label} is unavailable: ${error.message}`, { cause: error }); }
  if (metadata.isSymbolicLink()) throw storageIntegrity(`${label} must not be a symlink`);
  return metadata;
}

export async function assertNoSymlinkAncestors(path, label) {
  let current = resolve(path);
  for (;;) {
    let metadata;
    try { metadata = await lstat(current); }
    catch (error) { throw storageIntegrity(`${label} ancestor is unavailable: ${error.message}`, { cause: error }); }
    if (metadata.isSymbolicLink()) throw storageIntegrity(`${label} ancestor must not be a symlink`);
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
}

export async function readBoundedStableFile(path, expectedMetadata, maximumBytes, label) {
  let handle;
  try {
    handle = await open(path, fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0));
  } catch (error) {
    throw storageIntegrity(`${label} cannot be opened safely: ${error.message}`, { cause: error });
  }
  try {
    const openedMetadata = await handle.stat();
    if (!openedMetadata.isFile()
      || Number(openedMetadata.dev) !== Number(expectedMetadata.dev)
      || Number(openedMetadata.ino) !== Number(expectedMetadata.ino)
      || openedMetadata.size > maximumBytes) {
      throw storageIntegrity(`${label} changed or exceeded its bound while opening`);
    }
    const chunks = [];
    let totalBytes = 0;
    while (true) {
      const remaining = maximumBytes - totalBytes;
      const buffer = Buffer.alloc(Math.min(64 * 1024, remaining + 1));
      const { bytesRead } = await handle.read(buffer, 0, buffer.length, null);
      if (bytesRead === 0) break;
      totalBytes += bytesRead;
      if (totalBytes > maximumBytes) throw storageIntegrity(`${label} exceeds its bounded size`);
      chunks.push(buffer.subarray(0, bytesRead));
    }
    const finalMetadata = await handle.stat();
    if (!finalMetadata.isFile()
      || Number(finalMetadata.dev) !== Number(openedMetadata.dev)
      || Number(finalMetadata.ino) !== Number(openedMetadata.ino)
      || finalMetadata.size !== openedMetadata.size
      || totalBytes !== openedMetadata.size) {
      throw storageIntegrity(`${label} changed while being read`);
    }
    return Buffer.concat(chunks, totalBytes);
  } finally {
    await handle.close();
  }
}

export async function readBoundedStableHandle(handle, expectedMetadata, maximumBytes, label, { preserveOffset = false } = {}) {
  if (expectedMetadata.size > maximumBytes) throw storageIntegrity(`${label} exceeds its bounded size`);
  const chunks = [];
  let totalBytes = 0;
  while (true) {
    const remaining = maximumBytes - totalBytes;
    const buffer = Buffer.alloc(Math.min(64 * 1024, remaining + 1));
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, preserveOffset ? totalBytes : null);
    if (bytesRead === 0) break;
    totalBytes += bytesRead;
    if (totalBytes > maximumBytes) throw storageIntegrity(`${label} exceeds its bounded size`);
    chunks.push(buffer.subarray(0, bytesRead));
  }
  const finalMetadata = await handle.stat();
  if (!finalMetadata.isFile()
    || Number(finalMetadata.dev) !== Number(expectedMetadata.dev)
    || Number(finalMetadata.ino) !== Number(expectedMetadata.ino)
    || finalMetadata.size !== expectedMetadata.size
    || totalBytes !== expectedMetadata.size) {
    throw storageIntegrity(`${label} changed while being read`);
  }
  return Buffer.concat(chunks, totalBytes);
}
