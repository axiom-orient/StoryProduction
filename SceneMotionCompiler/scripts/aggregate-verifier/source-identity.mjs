import { lstat, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { verifySourceManifest } from '../../packages/source-integrity/src/index.js';
import { VerifyError } from './errors.mjs';

export const SOURCE_MANIFEST_RELATIVE_PATH = 'SOURCE_MANIFEST.json';

const SOURCE_MANIFEST_MAX_BYTES = 16 * 1024 * 1024;
const SOURCE_IDENTITY_MAX_ERROR_LENGTH = 512;
const SOURCE_IDENTITY_MAX_MISMATCHES = 64;
const SOURCE_TREE_HASH_PATTERN = /^sha256:[0-9a-f]{64}$/;


function boundedString(value, maxLength) {
  if (typeof value !== 'string') return null;
  if (value.length <= maxLength) return value;
  return `${value.slice(0, Math.max(0, maxLength - 1))}…`;
}

function normalizeSourceTreeHash(value) {
  return typeof value === 'string' && SOURCE_TREE_HASH_PATTERN.test(value) ? value : null;
}

function normalizeSourceFileCount(value) {
  return Number.isSafeInteger(value) && value >= 0 ? value : null;
}

function sourceIdentityValues(value) {
  const manifest = value && typeof value === 'object' && !Array.isArray(value) ? value : null;
  return {
    sourceTreeHash: normalizeSourceTreeHash(manifest?.sourceTreeHash),
    fileCount: normalizeSourceFileCount(manifest?.fileCount),
  };
}

function sourceIdentityMismatch(field, expected, actual) {
  return {
    field,
    expected: expected ?? null,
    actual: actual ?? null,
  };
}

function boundedSourceMismatches(mismatches) {
  if (!Array.isArray(mismatches)) return [];
  return mismatches.slice(0, SOURCE_IDENTITY_MAX_MISMATCHES).map((mismatch) => {
    if (!mismatch || typeof mismatch !== 'object' || Array.isArray(mismatch)) {
      return { issue: boundedString(String(mismatch), 256) };
    }
    const result = {};
    for (const key of ['field', 'path', 'issue']) {
      if (typeof mismatch[key] === 'string') result[key] = boundedString(mismatch[key], 256);
    }
    for (const key of ['expected', 'actual']) {
      const value = mismatch[key];
      if (typeof value === 'string' || Number.isSafeInteger(value)) result[key] = value;
      else if (value && typeof value === 'object' && !Array.isArray(value)) {
        const scalar = {};
        for (const scalarKey of ['sourceTreeHash', 'fileCount', 'bytes', 'mode']) {
          if (typeof value[scalarKey] === 'string' || Number.isSafeInteger(value[scalarKey])) {
            scalar[scalarKey] = typeof value[scalarKey] === 'string'
              ? boundedString(value[scalarKey], 256)
              : value[scalarKey];
          }
        }
        if (Object.keys(scalar).length > 0) result[key] = scalar;
      }
    }
    return result;
  });
}

export function pendingSourceIdentity() {
  return {
    manifestPath: SOURCE_MANIFEST_RELATIVE_PATH,
    status: 'PENDING',
    declared: { sourceTreeHash: null, fileCount: null },
    observed: { sourceTreeHash: null, fileCount: null },
    verifiedAt: null,
    mismatches: [],
    error: null,
  };
}

function sourceManifestFileIdentity(stat) {
  return {
    dev: Number(stat.dev),
    ino: Number(stat.ino),
    size: Number(stat.size),
    mtimeNs: stat.mtimeNs === undefined ? null : Number(stat.mtimeNs),
  };
}

function sameSourceManifestFile(opened, observed) {
  return Boolean(
    opened
    && observed
    && opened.dev === observed.dev
    && opened.ino === observed.ino
    && opened.size === observed.size
    && (opened.mtimeNs === null || observed.mtimeNs === null || opened.mtimeNs === observed.mtimeNs),
  );
}

async function inspectSourceManifestFile(manifestPath) {
  let stat;
  try {
    stat = await lstat(manifestPath);
  } catch (error) {
    throw new VerifyError(
      'E_VERIFY_SOURCE_IDENTITY',
      `source manifest is unavailable: ${error.code ?? error.message}`,
      { manifestPath },
    );
  }
  if (stat.isSymbolicLink()) {
    throw new VerifyError('E_VERIFY_SOURCE_IDENTITY', 'source manifest must not be a symlink', { manifestPath });
  }
  if (!stat.isFile()) {
    throw new VerifyError('E_VERIFY_SOURCE_IDENTITY', 'source manifest must be a regular file', { manifestPath });
  }
  if (stat.size > SOURCE_MANIFEST_MAX_BYTES) {
    throw new VerifyError(
      'E_VERIFY_SOURCE_IDENTITY',
      `source manifest exceeds ${SOURCE_MANIFEST_MAX_BYTES} bytes`,
      { manifestPath, size: Number(stat.size) },
    );
  }
  return sourceManifestFileIdentity(stat);
}

export async function verifyProductionSourceIdentity(root) {
  const identity = pendingSourceIdentity();
  const manifestPath = join(root, SOURCE_MANIFEST_RELATIVE_PATH);
  try {
    const opened = await inspectSourceManifestFile(manifestPath);
    let manifest;
    try {
      // The path is checked with lstat before and after this bounded read.  The
      // final identity check below catches replacement while the tree is read.
      manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
    } catch (error) {
      throw new VerifyError(
        'E_VERIFY_SOURCE_IDENTITY',
        `source manifest is malformed: ${error.message}`,
        { manifestPath },
      );
    }
    identity.declared = sourceIdentityValues(manifest);
    const afterRead = await inspectSourceManifestFile(manifestPath);
    if (!sameSourceManifestFile(opened, afterRead)) {
      throw new VerifyError('E_VERIFY_SOURCE_IDENTITY', 'source manifest changed while it was read', { manifestPath });
    }
    if (!identity.declared.sourceTreeHash) {
      identity.mismatches.push(sourceIdentityMismatch('sourceTreeHash', 'sha256:<64 lowercase hex>', manifest?.sourceTreeHash));
    }
    if (identity.declared.fileCount === null) {
      identity.mismatches.push(sourceIdentityMismatch('fileCount', 'non-negative safe integer', manifest?.fileCount));
    }

    let verification;
    try {
      verification = await verifySourceManifest(root, manifest);
    } catch (error) {
      throw new VerifyError(
        'E_VERIFY_SOURCE_IDENTITY',
        `source manifest verification failed: ${error.message}`,
        { manifestPath },
      );
    }
    identity.observed = sourceIdentityValues(verification.actual);
    identity.mismatches.push(...boundedSourceMismatches(verification.mismatches));
    const afterVerification = await inspectSourceManifestFile(manifestPath);
    if (!sameSourceManifestFile(opened, afterVerification)) {
      throw new VerifyError('E_VERIFY_SOURCE_IDENTITY', 'source manifest changed during source verification', { manifestPath });
    }
    if (!identity.observed.sourceTreeHash) {
      identity.mismatches.push(sourceIdentityMismatch('observed.sourceTreeHash', 'sha256:<64 lowercase hex>', verification.actual?.sourceTreeHash));
    }
    if (identity.observed.fileCount === null) {
      identity.mismatches.push(sourceIdentityMismatch('observed.fileCount', 'non-negative safe integer', verification.actual?.fileCount));
    }
    if (identity.declared.sourceTreeHash !== identity.observed.sourceTreeHash) {
      identity.mismatches.push(sourceIdentityMismatch('sourceTreeHash', identity.declared.sourceTreeHash, identity.observed.sourceTreeHash));
    }
    if (identity.declared.fileCount !== identity.observed.fileCount) {
      identity.mismatches.push(sourceIdentityMismatch('fileCount', identity.declared.fileCount, identity.observed.fileCount));
    }
    identity.mismatches = boundedSourceMismatches(identity.mismatches);
    if (!verification.pass || identity.mismatches.length > 0) {
      throw new VerifyError(
        'E_VERIFY_SOURCE_IDENTITY',
        'source manifest identity does not match the current source tree',
        { manifestPath },
      );
    }
    identity.status = 'PASS';
    identity.verifiedAt = new Date().toISOString();
    return identity;
  } catch (error) {
    identity.status = 'FAIL';
    identity.verifiedAt = new Date().toISOString();
    identity.error = boundedString(error?.message ?? String(error), SOURCE_IDENTITY_MAX_ERROR_LENGTH);
    if (error instanceof VerifyError && error.details?.manifestPath) {
      identity.manifestPath = SOURCE_MANIFEST_RELATIVE_PATH;
    }
    throw new VerifyError(
      'E_VERIFY_SOURCE_IDENTITY',
      identity.error ?? 'source identity verification failed',
      { sourceIdentity: identity },
    );
  }
}

