export const DEFAULT_EXCLUDED_DIRECTORIES = new Set([
  '.git',
  '.gjc',
  '.build',
  '.tmp',
  'SceneMotionCompiler Studio.app',
  'node_modules',
  'target',
  '__pycache__',
  '.pytest_cache'
]);
export const GENERATED_METADATA_FILES = new Set(['SOURCE_MANIFEST.json', 'VERIFY_LOCAL.md']);

export const DEFAULT_SOURCE_PACKAGE_LIMITS = Object.freeze({
  maxFiles: 10_000,
  maxFileBytes: 128 * 1024 * 1024,
  maxTotalBytes: 512 * 1024 * 1024,
  maxArchiveBytes: 512 * 1024 * 1024,
  maxMetadataBytes: 16 * 1024 * 1024,
  maxPathBytes: 4096,
  commandTimeoutMs: 15 * 60 * 1000
});

export function resolveLimits(input = {}) {
  const limits = { ...DEFAULT_SOURCE_PACKAGE_LIMITS, ...(input ?? {}) };
  for (const [name, value] of Object.entries(limits)) {
    if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`Invalid source package limit ${name}: ${value}`);
  }
  if (limits.maxFiles > 0xffff - GENERATED_METADATA_FILES.size) throw new Error('maxFiles exceeds ZIP32 entry capacity');
  if (limits.maxFileBytes > 0xffffffff || limits.maxTotalBytes > 0xffffffff || limits.maxArchiveBytes > 0xffffffff) {
    throw new Error('Source package limits exceed ZIP32 capacity');
  }
  return limits;
}

export function assertPackageName(packageName, limits) {
  if (typeof packageName !== 'string' || !packageName || packageName === '.' || packageName === '..' || packageName.includes('/') || packageName.includes('\\') || packageName.includes('\0')) {
    throw new Error(`Invalid source package name: ${packageName}`);
  }
  if (Buffer.byteLength(packageName, 'utf8') > limits.maxPathBytes) throw new Error('Source package name exceeds path byte limit');
}
