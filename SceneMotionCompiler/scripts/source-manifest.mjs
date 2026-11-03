import { constants as fsConstants } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { lstat, mkdir, open, rename, unlink } from 'node:fs/promises';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildSourceManifest, collectSourceFiles } from '../packages/source-integrity/src/index.js';

/**
 * The manifest is consumed as a source-identity authority by the aggregate
 * verifier. Keep the payload bounded before opening the destination so a
 * malformed or unexpectedly large source tree cannot create an unbounded
 * metadata write.
 */
export const MAX_MANIFEST_BYTES = 16 * 1024 * 1024;
const MAX_TEMP_ATTEMPTS = 16;
const MANIFEST_TEMP_MARKER = '.smc-source-manifest-';

/**
 * Collect and deterministically build one source manifest.
 *
 * This function has no output side effects. Publishing is deliberately a
 * separate operation so the file-system boundary can be tested independently
 * from source inventory and hashing.
 */
export async function generateSourceManifest({ rootDir = '.', packageName = 'scene-motion-compiler' } = {}) {
  const root = resolve(rootDir);
  const files = await collectSourceFiles(root, { excludeGeneratedMetadata: true });
  return buildSourceManifest(files, { packageName });
}

/**
 * Serialize one manifest with a bounded, canonical JSON payload.
 */
export function serializeSourceManifest(manifest, maximumBytes = MAX_MANIFEST_BYTES) {
  if (!Number.isSafeInteger(maximumBytes) || maximumBytes < 1) {
    throw new Error(`Invalid source manifest byte limit: ${maximumBytes}`);
  }
  const text = `${JSON.stringify(manifest, null, 2)}\n`;
  const bytes = Buffer.from(text, 'utf8');
  if (bytes.length > maximumBytes) {
    throw new Error(`Source manifest exceeds ${maximumBytes} byte limit`);
  }
  return bytes;
}

/**
 * Publish a manifest using a same-directory temporary file and atomic rename.
 *
 * `beforeRename` is an in-process test seam. It is intentionally not exposed
 * by the CLI and runs only after the temporary file has been fully written,
 * fsynced. The descriptor remains open through rename/cleanup to prevent inode reuse. Any error before rename leaves the previous output
 * untouched and removes only the temporary file whose original regular-file
 * type/dev/ino identity is still proven; a replacement entry is preserved and
 * reported through a bounded cleanup diagnostic.
 */
export async function publishSourceManifest(outputPath, manifest, options = {}) {
  const targetPath = resolve(outputPath);
  const parentPath = dirname(targetPath);
  const beforeRename = options.beforeRename;
  const maximumBytes = options.maximumBytes ?? MAX_MANIFEST_BYTES;
  if (beforeRename !== undefined && typeof beforeRename !== 'function') {
    throw new TypeError('beforeRename must be a function when provided');
  }

  const payload = serializeSourceManifest(manifest, maximumBytes);
  await ensureSafeParentDirectory(parentPath);
  await assertManifestOutput(targetPath);

  let ownedTemporaryPath = null;
  let ownedTemporaryIdentity = null;
  let handle = null;
  let operationError = null;
  try {
    for (let attempt = 0; attempt < MAX_TEMP_ATTEMPTS; attempt += 1) {
      const candidate = joinTemporaryPath(parentPath, targetPath);
      try {
        handle = await open(
          candidate,
          fsConstants.O_WRONLY
            | fsConstants.O_CREAT
            | fsConstants.O_EXCL
            | (fsConstants.O_NOFOLLOW ?? 0),
          0o644,
        );
        ownedTemporaryPath = candidate;
        break;
      } catch (error) {
        if (error?.code !== 'EEXIST') throw error;
      }
    }
    if (!handle || !ownedTemporaryPath) {
      throw new Error(`Unable to allocate a unique source manifest temporary file after ${MAX_TEMP_ATTEMPTS} attempts`);
    }

    const temporaryMetadata = await handle.stat();
    if (!temporaryMetadata.isFile()) {
      throw new Error(`Source manifest temporary output is not a regular file: ${ownedTemporaryPath}`);
    }
    ownedTemporaryIdentity = {
      type: 'regular-file',
      dev: Number(temporaryMetadata.dev),
      ino: Number(temporaryMetadata.ino),
    };
    await handle.writeFile(payload);
    await handle.sync();
    // Keep the unlinked original inode alive: dev/ino can otherwise be recycled
    // before the identity comparison on filesystems that immediately reuse inodes.

    // Re-check after the injected seam and immediately before rename. POSIX
    // rename replaces the directory entry; it never follows an output symlink.
    await beforeRename?.({
      outputPath: targetPath,
      temporaryPath: ownedTemporaryPath,
      bytes: payload.length,
    });
    await assertSafeParentDirectory(parentPath);
    await assertManifestOutput(targetPath);
    await assertOwnedTemporary(ownedTemporaryPath, ownedTemporaryIdentity);
    await rename(ownedTemporaryPath, targetPath);
    ownedTemporaryPath = null;
    ownedTemporaryIdentity = null;
    await syncParentDirectory(parentPath);
    return { outputPath: targetPath, bytes: payload.length };
  } catch (error) {
    operationError = error;
    throw error;
  } finally {
    let cleanupError = null;
    let cleanupDiagnostic = null;
    if (ownedTemporaryPath) {
      cleanupDiagnostic = await removeOwnedTemporary(ownedTemporaryPath, ownedTemporaryIdentity);
      if (operationError) attachCleanupDiagnostic(operationError, cleanupDiagnostic);
    }
    if (handle) {
      try {
        await handle.close();
      } catch (error) {
        cleanupError = error;
      }
    }
    if (cleanupDiagnostic?.status === 'PRESERVED' && !operationError) {
      const error = new Error(`Source manifest temporary cleanup did not prove ownership: ${cleanupDiagnostic.reason}`);
      error.cleanupDiagnostic = cleanupDiagnostic;
      if (cleanupError) error.cause = cleanupError;
      throw error;
    }
    if (cleanupError && !operationError) throw cleanupError;
  }
}

export async function generateAndPublishSourceManifest({ rootDir = '.', outputPath = '.tmp/source-manifest.json', packageName = 'scene-motion-compiler' } = {}) {
  const manifest = await generateSourceManifest({ rootDir, packageName });
  const publication = await publishSourceManifest(outputPath, manifest);
  return { ...publication, manifest };
}

async function ensureSafeParentDirectory(parentPath) {
  try {
    await assertSafeParentDirectory(parentPath);
    return;
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }

  await mkdir(parentPath, { recursive: true });
  await assertSafeParentDirectory(parentPath);
}

async function assertSafeParentDirectory(parentPath) {
  const metadata = await lstat(parentPath);
  assertDirectory(parentPath, metadata, 'Source manifest output directory');
}

function assertDirectory(path, metadata, label) {
  if (metadata.isSymbolicLink()) throw new Error(`${label} must not be a symlink: ${path}`);
  if (!metadata.isDirectory()) throw new Error(`${label} must be a directory: ${path}`);
}

async function assertManifestOutput(path) {
  try {
    const metadata = await lstat(path);
    if (metadata.isSymbolicLink()) throw new Error(`Source manifest output must not be a symlink: ${path}`);
    if (!metadata.isFile()) throw new Error(`Source manifest output must be a regular file when it exists: ${path}`);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
}

async function assertOwnedTemporary(path, expectedIdentity) {
  let metadata;
  try {
    metadata = await lstat(path);
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`Source manifest temporary output disappeared before rename: ${path}`);
    throw error;
  }
  if (metadata.isSymbolicLink()) throw new Error(`Source manifest temporary output must not be a symlink: ${path}`);
  if (!metadata.isFile()) throw new Error(`Source manifest temporary output must be a regular file: ${path}`);
  const actualIdentity = {
    type: metadata.isFile() ? 'regular-file' : 'other',
    dev: Number(metadata.dev),
    ino: Number(metadata.ino),
  };
  if (
    actualIdentity.type !== expectedIdentity?.type
    || actualIdentity.dev !== expectedIdentity?.dev
    || actualIdentity.ino !== expectedIdentity?.ino
  ) {
    throw new Error(`Source manifest temporary output was replaced before rename: ${path}`);
  }
}

function joinTemporaryPath(parentPath, targetPath) {
  return join(parentPath, `${MANIFEST_TEMP_MARKER}${basename(targetPath)}-${process.pid}-${randomUUID()}.tmp`);
}

async function syncParentDirectory(parentPath) {
  const flags = fsConstants.O_RDONLY
    | (fsConstants.O_DIRECTORY ?? 0)
    | (fsConstants.O_NOFOLLOW ?? 0);
  const handle = await open(parentPath, flags);
  try {
    const metadata = await handle.stat();
    assertDirectory(parentPath, metadata, 'Source manifest output directory');
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function removeOwnedTemporary(path, expectedIdentity) {
  let metadata;
  try {
    metadata = await lstat(path);
  } catch (error) {
    if (error?.code === 'ENOENT') return cleanupDiagnostic(path, 'ABSENT', 'already-absent');
    return cleanupDiagnostic(path, 'PRESERVED', 'lstat-failed', { error: boundedErrorCode(error) });
  }
  const actualIdentity = {
    type: metadata.isSymbolicLink() ? 'symlink' : metadata.isFile() ? 'regular-file' : 'other',
    dev: Number(metadata.dev),
    ino: Number(metadata.ino),
  };
  if (
    actualIdentity.type !== expectedIdentity?.type
    || actualIdentity.dev !== expectedIdentity?.dev
    || actualIdentity.ino !== expectedIdentity?.ino
  ) {
    return cleanupDiagnostic(path, 'PRESERVED', 'identity-mismatch', {
      expected: boundedIdentity(expectedIdentity),
      observed: boundedIdentity(actualIdentity),
    });
  }
  try {
    // unlink removes only this verified directory entry, including a symlink;
    // unlike a recursive remove it never follows a final symlink to its target.
    await unlink(path);
    return cleanupDiagnostic(path, 'REMOVED', 'owned-entry');
  } catch (error) {
    if (error?.code === 'ENOENT') return cleanupDiagnostic(path, 'ABSENT', 'already-absent');
    return cleanupDiagnostic(path, 'PRESERVED', 'unlink-failed', { error: boundedErrorCode(error) });
  }
}

function cleanupDiagnostic(path, status, reason, details = {}) {
  return {
    status,
    reason,
    path: boundedString(path, 512),
    ...details,
  };
}

function boundedIdentity(identity) {
  return {
    type: typeof identity?.type === 'string' ? identity.type : 'unknown',
    dev: Number.isSafeInteger(identity?.dev) ? identity.dev : null,
    ino: Number.isSafeInteger(identity?.ino) ? identity.ino : null,
  };
}

function boundedErrorCode(error) {
  return boundedString(error?.code ?? error?.message ?? String(error), 128);
}

function boundedString(value, maximumLength) {
  const text = String(value ?? '');
  return text.length <= maximumLength ? text : `${text.slice(0, maximumLength - 1)}…`;
}

function attachCleanupDiagnostic(error, diagnostic) {
  try {
    error.cleanupDiagnostic = diagnostic;
  } catch {
    // Errors are normally extensible; never replace the original operation
    // error merely because a diagnostic property could not be attached.
  }
}

async function main() {
  const result = await generateAndPublishSourceManifest({
    rootDir: argument('--root') ?? '.',
    outputPath: argument('--out') ?? '.tmp/source-manifest.json',
    packageName: argument('--name') ?? 'scene-motion-compiler',
  });
  console.log(JSON.stringify({
    status: 'PASS',
    outputPath: result.outputPath,
    fileCount: result.manifest.fileCount,
    totalBytes: result.manifest.totalBytes,
    sourceTreeHash: result.manifest.sourceTreeHash,
  }, null, 2));
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

const isMain = process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url));
if (isMain) {
  main().catch((error) => {
    console.error(error?.stack ?? error?.message ?? String(error));
    process.exitCode = 1;
  });
}
