import { constants as fsConstants } from 'node:fs';
import { open, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { hashTrackBRawBytes } from '../../contracts/src/trackb-contract.js';
import {
  TRACKB_CUSTODY_BINARY_NAME,
  TRACKB_CUSTODY_MAX_RECEIPT_BYTES,
  TRACKB_CUSTODY_MAX_BINARY_BYTES,
  TRACKB_CUSTODY_RECEIPT_NAME,
  TrackBCustodyError,
  assertSafeTargetTriple,
  storageIntegrity,
  validateReceipt
} from './trackb-custody-protocol.js';
import {
  assertNoSymlinkAncestors,
  lstatNoSymlink,
  readBoundedStableFile,
  readBoundedStableHandle
} from './trackb-custody-files.js';

const MODULE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const RECEIPT_NAME = TRACKB_CUSTODY_RECEIPT_NAME;

export function defaultTrackBCustodyTargetTriple({ platform = process.platform, arch = process.arch } = {}) {
  if (platform === 'darwin' && arch === 'arm64') return 'aarch64-apple-darwin';
  if (platform === 'darwin' && arch === 'x64') return 'x86_64-apple-darwin';
  if (platform === 'linux' && arch === 'arm64') return 'aarch64-unknown-linux-gnu';
  if (platform === 'linux' && arch === 'x64') return 'x86_64-unknown-linux-gnu';
  throw new TrackBCustodyError('trackb-storage-integrity', `unsupported custody target ${platform}/${arch}`);
}

/**
 * Resolve and verify the fixed helper receipt before any child process is
 * spawned.  No PATH lookup, cargo invocation, source-tree target fallback, or
 * FileRevisionStore fallback is permitted here.
 */
export async function resolveTrackBCustodyBinary(options = {}) {
  const targetTriple = options.targetTriple ?? defaultTrackBCustodyTargetTriple();
  assertSafeTargetTriple(targetTriple);
  if (Object.hasOwn(options, 'applicationConfigPath')) {
    throw storageIntegrity('applicationConfigPath is not a custody option; configure binaryPath');
  }
  const explicit = options.binaryPath ?? null;
  const resolvedExplicit = explicit === null ? null : await resolveConfiguredBinaryPath(explicit);
  const binaryPath = resolvedExplicit ?? join(resolve(options.runtimeRoot ?? MODULE_ROOT), 'runtime', 'native', targetTriple, TRACKB_CUSTODY_BINARY_NAME);
  const receiptPath = resolve(options.receiptPath ?? join(dirname(binaryPath), RECEIPT_NAME));
  const verified = await verifyTrackBCustodyReceipt({ binaryPath, receiptPath, targetTriple, retainBinaryHandle: options.retainBinaryHandle === true });
  return { ...verified, binaryPath: verified.binaryPath, receiptPath: verified.receiptPath, targetTriple };
}

export async function verifyTrackBCustodyReceipt({ binaryPath, receiptPath, targetTriple, retainBinaryHandle = false } = {}) {
  if (typeof binaryPath !== 'string' || !isAbsolute(binaryPath)) {
    throw storageIntegrity('custody binary path must be absolute');
  }
  if (typeof receiptPath !== 'string' || !isAbsolute(receiptPath)) {
    throw storageIntegrity('custody receipt path must be absolute');
  }
  assertSafeTargetTriple(targetTriple);
  const safeBinaryPath = resolve(binaryPath);
  const safeReceiptPath = resolve(receiptPath);
  await assertNoSymlinkAncestors(safeBinaryPath, 'custody helper binary');
  await assertNoSymlinkAncestors(safeReceiptPath, 'custody helper receipt');
  const metadata = await lstatNoSymlink(safeBinaryPath, 'custody helper binary');
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw storageIntegrity('custody helper binary must be a regular non-symlink file');
  }
  if ((metadata.mode & fsConstants.S_IXUSR) === 0 && (metadata.mode & fsConstants.S_IXGRP) === 0 && (metadata.mode & fsConstants.S_IXOTH) === 0) {
    throw storageIntegrity('custody helper binary is not executable');
  }
  const binaryRealPath = await realpath(safeBinaryPath).catch((error) => {
    throw storageIntegrity(`custody helper binary realpath failed: ${error.message}`, { cause: error });
  });
  if (binaryRealPath !== safeBinaryPath) throw storageIntegrity('custody helper binary realpath differs from configured path');
  const receiptMetadata = await lstatNoSymlink(safeReceiptPath, 'custody helper receipt');
  if (!receiptMetadata.isFile() || receiptMetadata.isSymbolicLink()) {
    throw storageIntegrity('custody helper receipt must be a regular non-symlink file');
  }
  const receiptRealPath = await realpath(safeReceiptPath).catch((error) => {
    throw storageIntegrity(`custody receipt realpath failed: ${error.message}`, { cause: error });
  });
  if (receiptRealPath !== safeReceiptPath) throw storageIntegrity('custody receipt realpath differs from configured path');
  if (receiptMetadata.size > TRACKB_CUSTODY_MAX_RECEIPT_BYTES) throw storageIntegrity('custody receipt exceeds its bounded size');
  if (metadata.size > TRACKB_CUSTODY_MAX_BINARY_BYTES) throw storageIntegrity('custody helper binary exceeds its bounded size');
  let receipt;
  try {
    const receiptBytes = await readBoundedStableFile(safeReceiptPath, receiptMetadata, TRACKB_CUSTODY_MAX_RECEIPT_BYTES, 'custody helper receipt');
    receipt = JSON.parse(receiptBytes.toString('utf8'));
  } catch (error) {
    if (error instanceof TrackBCustodyError) throw error;
    throw storageIntegrity(`custody helper receipt is not valid JSON: ${error.message}`, { cause: error });
  }
  validateReceipt(receipt, { targetTriple, binaryPath: safeBinaryPath, binaryBytes: metadata.size });
  let binaryHandle;
  try {
    binaryHandle = await open(safeBinaryPath, fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0));
    const openedMetadata = await binaryHandle.stat();
    if (!openedMetadata.isFile()
      || Number(openedMetadata.dev) !== Number(metadata.dev)
      || Number(openedMetadata.ino) !== Number(metadata.ino)) {
      throw storageIntegrity('custody helper binary changed before same-inode verification');
    }
    const bytes = await readBoundedStableHandle(binaryHandle, openedMetadata, TRACKB_CUSTODY_MAX_BINARY_BYTES, 'custody helper binary', { preserveOffset: retainBinaryHandle });
    const binarySha256 = hashTrackBRawBytes(bytes);
    if (binarySha256 !== receipt.binarySha256) {
      throw storageIntegrity('custody helper binary SHA-256 does not match its receipt');
    }
    if (Number(receipt.binaryBytes) !== bytes.length) {
      throw storageIntegrity('custody helper binary byte count does not match its receipt');
    }
    const result = {
      binaryPath: safeBinaryPath,
      binaryRealPath,
      receiptPath: safeReceiptPath,
      receiptRealPath,
      receipt,
      binarySha256,
      binaryBytes: bytes.length,
    };
    if (retainBinaryHandle) {
      result.binaryHandle = binaryHandle;
      binaryHandle = null;
    }
    return result;
  } finally {
    if (binaryHandle) await binaryHandle.close().catch(() => {});
  }
}

async function resolveConfiguredBinaryPath(value) {
  if (typeof value !== 'string' || !isAbsolute(value)) throw storageIntegrity('explicit custody binary/config path must be absolute');
  const path = resolve(value);
  const metadata = await lstatNoSymlink(path, 'explicit custody configuration');
  if (metadata.isFile() && !metadata.isSymbolicLink() && path.endsWith('.json')) {
    let config;
    try { config = JSON.parse((await readBoundedStableFile(path, metadata, TRACKB_CUSTODY_MAX_RECEIPT_BYTES, 'custody configuration')).toString('utf8')); }
    catch (error) { throw storageIntegrity(`explicit custody config is not valid JSON: ${error.message}`, { cause: error }); }
    const nested = config?.trackbCustodyBinaryPath ?? config?.binaryPath;
    if (typeof nested !== 'string' || !isAbsolute(nested)) throw storageIntegrity('explicit custody config must contain an absolute binaryPath');
    return resolve(nested);
  }
  return path;
}
