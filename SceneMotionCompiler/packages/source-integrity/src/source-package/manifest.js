import { constants as fsConstants } from 'node:fs';
import { lstat, open, readdir, realpath } from 'node:fs/promises';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { canonicalHash, compareCanonicalStrings, sha256Prefixed } from '../../../core/src/canonical.js';
import {
  DEFAULT_EXCLUDED_DIRECTORIES,
  GENERATED_METADATA_FILES,
  assertPackageName,
  resolveLimits
} from './limits.js';

export async function collectSourceFiles(rootDir, options = {}) {
  const root = resolve(rootDir);
  const limits = resolveLimits(options.limits);
  const rootMetadata = await lstat(root);
  if (rootMetadata.isSymbolicLink()) throw new Error(`Source root must not be a symlink: ${root}`);
  if (!rootMetadata.isDirectory()) throw new Error(`Source root must be a directory: ${root}`);
  const rootRealPath = await realpath(root);
  const excludedDirectories = new Set([...DEFAULT_EXCLUDED_DIRECTORIES, ...(options.excludedDirectories ?? [])]);
  const output = [];
  let totalBytes = 0;
  await walk(root);
  output.sort((a, b) => compareCanonicalStrings(a.path, b.path));
  return output;

  async function walk(directory) {
    await assertDirectoryInsideRoot(directory, rootRealPath);
    const entries = await readdir(directory, { withFileTypes: true });
    entries.sort((a, b) => compareCanonicalStrings(a.name, b.name));
    for (const entry of entries) {
      if (excludedDirectories.has(entry.name) && entry.isDirectory()) continue;
      if (entry.name === '.DS_Store') continue;
      const absolutePath = join(directory, entry.name);
      if (entry.isSymbolicLink()) throw new Error(`Symlinks are forbidden in source packages: ${absolutePath}`);
      if (entry.isDirectory()) {
        await walk(absolutePath);
        continue;
      }
      if (!entry.isFile()) continue;
      const path = safeRelative(root, absolutePath);
      if (options.excludeGeneratedMetadata === true && GENERATED_METADATA_FILES.has(path)) continue;
      if (Buffer.byteLength(path, 'utf8') > limits.maxPathBytes) throw new Error(`Source path exceeds ${limits.maxPathBytes} bytes: ${path}`);
      if (output.length >= limits.maxFiles) throw new Error(`Source file count exceeds limit ${limits.maxFiles}`);
      const realFilePath = await realpath(absolutePath);
      assertInsideRoot(rootRealPath, realFilePath, 'Source file');
      const handle = await open(absolutePath, fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0));
      try {
        const metadata = await handle.stat();
        if (!metadata.isFile()) throw new Error(`Source entry changed or is not a regular file: ${absolutePath}`);
        if (metadata.size > limits.maxFileBytes) throw new Error(`Source file exceeds ${limits.maxFileBytes} byte limit: ${path}`);
        if (totalBytes + metadata.size > limits.maxTotalBytes) throw new Error(`Source tree exceeds ${limits.maxTotalBytes} byte limit`);
        const bytes = await handle.readFile();
        if (bytes.length > limits.maxFileBytes) throw new Error(`Source file exceeds ${limits.maxFileBytes} byte limit: ${path}`);
        if (bytes.length !== metadata.size) throw new Error(`Source file changed while packaging: ${path}`);
        totalBytes += bytes.length;
        if (totalBytes > limits.maxTotalBytes) throw new Error(`Source tree exceeds ${limits.maxTotalBytes} byte limit`);
        output.push({
          path,
          bytes,
          sha256: sha256Prefixed(bytes),
          size: bytes.length,
          mode: metadata.mode & 0o111 ? 0o755 : 0o644
        });
      } finally {
        await handle.close();
      }
    }
  }
}

export function buildSourceManifest(files, options = {}) {
  const records = files.map(({ path, sha256, size, mode }) => ({
    path,
    sha256,
    bytes: size,
    mode: modeString(mode)
  })).sort((a, b) => compareCanonicalStrings(a.path, b.path));
  const manifest = {
    schemaVersion: 1,
    packageName: options.packageName ?? 'scene-motion-compiler',
    fileCount: records.length,
    totalBytes: records.reduce((sum, item) => sum + item.bytes, 0),
    excludedDirectories: [...DEFAULT_EXCLUDED_DIRECTORIES].sort(),
    generatedMetadataExcludedFromTreeHash: [...GENERATED_METADATA_FILES].sort(),
    files: records,
    sourceTreeHash: canonicalHash(records)
  };
  return manifest;
}

export async function verifySourceManifest(rootDir, manifest, options = {}) {
  const limits = resolveLimits(options.limits);
  assertManifestShape(manifest, limits);
  const files = await collectSourceFiles(rootDir, { excludeGeneratedMetadata: true, limits });
  const actual = buildSourceManifest(files, { packageName: manifest.packageName });
  const mismatches = [];
  if (actual.sourceTreeHash !== manifest.sourceTreeHash) mismatches.push({ field: 'sourceTreeHash', expected: manifest.sourceTreeHash, actual: actual.sourceTreeHash });
  if (actual.fileCount !== manifest.fileCount) mismatches.push({ field: 'fileCount', expected: manifest.fileCount, actual: actual.fileCount });
  if (actual.totalBytes !== manifest.totalBytes) mismatches.push({ field: 'totalBytes', expected: manifest.totalBytes, actual: actual.totalBytes });
  const expectedByPath = new Map((manifest.files ?? []).map((item) => [item.path, item]));
  const actualByPath = new Map(actual.files.map((item) => [item.path, item]));
  for (const [path, expected] of expectedByPath) {
    const value = actualByPath.get(path);
    if (!value) mismatches.push({ path, issue: 'missing' });
    else if (value.sha256 !== expected.sha256 || value.bytes !== expected.bytes || value.mode !== expected.mode) mismatches.push({ path, issue: 'metadata-mismatch', expected, actual: value });
  }
  for (const path of actualByPath.keys()) if (!expectedByPath.has(path)) mismatches.push({ path, issue: 'unexpected' });
  return { pass: mismatches.length === 0, mismatches, actual };
}

function assertManifestShape(manifest, limits) {
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) throw new Error('Source manifest must be an object');
  if (manifest.schemaVersion !== 1) throw new Error(`Unsupported source manifest schemaVersion: ${manifest.schemaVersion}`);
  assertPackageName(manifest.packageName, limits);
  if (!Number.isSafeInteger(manifest.fileCount) || manifest.fileCount < 0 || manifest.fileCount > limits.maxFiles) throw new Error('Source manifest fileCount is invalid or exceeds configured limit');
  if (!Number.isSafeInteger(manifest.totalBytes) || manifest.totalBytes < 0 || manifest.totalBytes > limits.maxTotalBytes) throw new Error('Source manifest totalBytes is invalid or exceeds configured limit');
  if (!Array.isArray(manifest.files) || manifest.files.length !== manifest.fileCount) throw new Error('Source manifest files do not match fileCount');
}

function safeRelative(root, path) {
  const value = relative(root, path).replaceAll('\\', '/');
  if (!value || value === '..' || value.startsWith('../') || value.startsWith('/')) throw new Error(`Path escapes source root: ${path}`);
  return value;
}

async function assertDirectoryInsideRoot(directory, rootRealPath) {
  const metadata = await lstat(directory);
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) throw new Error(`Source directory changed or is not a regular directory: ${directory}`);
  assertInsideRoot(rootRealPath, await realpath(directory), 'Source directory');
}

function assertInsideRoot(rootRealPath, candidateRealPath, label) {
  const value = relative(rootRealPath, candidateRealPath);
  if (value === '..' || value.startsWith(`..${sep}`) || isAbsolute(value)) throw new Error(`${label} escapes source root: ${candidateRealPath}`);
}

function modeString(mode) {
  return mode & 0o111 ? '100755' : '100644';
}
