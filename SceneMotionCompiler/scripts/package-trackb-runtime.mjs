import { chmod, copyFile, lstat, mkdir, realpath } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  defaultTrackBCustodyTargetTriple,
  verifyTrackBCustodyReceipt,
} from '../packages/store/src/trackb-custody-binary.js';
import { TRACKB_CUSTODY_BINARY_NAME } from '../packages/store/src/trackb-custody-protocol.js';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetTriple = argument('--target-triple') ?? defaultTrackBCustodyTargetTriple();
if (!/^[A-Za-z0-9._-]{1,127}$/.test(targetTriple)) throw new Error(`Invalid target triple: ${targetTriple}`);
const sourceRoot = resolve(argument('--runtime-root') ?? join(rootDir, 'runtime', 'native', targetTriple));
const outputArgument = argument('--out');
if (outputArgument === null) throw new Error('package:runtime requires --out outside the source runtime/native tree');
const sourceBinary = join(sourceRoot, TRACKB_CUSTODY_BINARY_NAME);
const sourceReceipt = join(sourceRoot, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`);
const outputRoot = resolve(outputArgument);
const sourceNativeRoot = resolve(join(rootDir, 'runtime', 'native'));
const sourceNative = await inspectDirectoryPath(sourceNativeRoot, 'source runtime/native', { allowMissing: false });
assertOutsideSource(outputRoot, sourceNativeRoot, sourceNative.realPath);
await assertOutputPath(outputRoot, sourceNativeRoot, sourceNative.realPath, { allowMissing: true });
const sourceVerification = await verifyTrackBCustodyReceipt({ binaryPath: sourceBinary, receiptPath: sourceReceipt, targetTriple });
await ensureDirectoryTree(outputRoot, 'runtime package output');
await assertOutputPath(outputRoot, sourceNativeRoot, sourceNative.realPath, { allowMissing: false });
const destination = join(outputRoot, targetTriple);
assertOutsideSource(destination, sourceNativeRoot, sourceNative.realPath);
await assertOutputPath(destination, sourceNativeRoot, sourceNative.realPath, { allowMissing: true });
await ensureDirectoryTree(destination, 'runtime package destination');
await assertOutputPath(destination, sourceNativeRoot, sourceNative.realPath, { allowMissing: false });
const binaryPath = join(destination, TRACKB_CUSTODY_BINARY_NAME);
const receiptPath = join(destination, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`);
await assertOutputFile(binaryPath, 'runtime package binary');
await assertOutputFile(receiptPath, 'runtime package receipt');
await copyFile(sourceBinary, binaryPath);
await chmod(binaryPath, 0o755);
await copyFile(sourceReceipt, receiptPath);
await assertOutputPath(destination, sourceNativeRoot, sourceNative.realPath, { allowMissing: false });
await assertOutputFile(binaryPath, 'runtime package binary');
await assertOutputFile(receiptPath, 'runtime package receipt');
await verifyTrackBCustodyReceipt({ binaryPath, receiptPath, targetTriple });
console.log(JSON.stringify({ status: 'PASS', targetTriple, binaryPath, receiptPath, binarySha256: sourceVerification.binarySha256, binaryBytes: sourceVerification.binaryBytes, sourceTreeFallback: false }, null, 2));

async function assertOutputPath(path, sourceLexicalPath, sourceRealPath, { allowMissing }) {
  const info = await inspectDirectoryPath(path, 'runtime package output', { allowMissing });
  assertOutsideSource(info.realPath, sourceLexicalPath, sourceRealPath);
  return info;
}

async function inspectDirectoryPath(path, label, { allowMissing }) {
  const safePath = resolve(path);
  const components = safePath.split(sep).filter(Boolean);
  let lexical = sep;
  let real = sep;
  const missing = [];
  for (const component of components) {
    const next = lexical === sep ? `${sep}${component}` : join(lexical, component);
    if (missing.length > 0) {
      missing.push(component);
      continue;
    }
    let metadata;
    try {
      metadata = await lstat(next);
    } catch (error) {
      if (allowMissing && error?.code === 'ENOENT') {
        missing.push(component);
        continue;
      }
      throw new Error(`${label} is unavailable: ${error.message}`);
    }
    if (metadata.isSymbolicLink()) throw new Error(`${label} ancestor must not be a symlink: ${next}`);
    if (!metadata.isDirectory()) throw new Error(`${label} must be a non-symlink directory: ${next}`);
    const canonical = await realpath(next).catch((error) => {
      throw new Error(`${label} realpath failed: ${error.message}`);
    });
    if (canonical !== next) throw new Error(`${label} canonical path differs: ${next}`);
    lexical = next;
    real = canonical;
  }
  if (missing.length > 0 && !allowMissing) {
    throw new Error(`${label} is unavailable: ${safePath}`);
  }
  let prospective = real;
  for (const component of missing) prospective = join(prospective, component);
  return { lexicalPath: safePath, realPath: prospective, exists: missing.length === 0 };
}

async function ensureDirectoryTree(path, label) {
  const safePath = resolve(path);
  const components = safePath.split(sep).filter(Boolean);
  let lexical = sep;
  for (const component of components) {
    const next = lexical === sep ? `${sep}${component}` : join(lexical, component);
    let metadata;
    try {
      metadata = await lstat(next);
    } catch (error) {
      if (error?.code !== 'ENOENT') throw new Error(`${label} is unavailable: ${error.message}`);
      try {
        await mkdir(next);
      } catch (mkdirError) {
        if (mkdirError?.code !== 'EEXIST') throw new Error(`${label} cannot be created: ${mkdirError.message}`);
      }
      metadata = await lstat(next).catch((lstatError) => {
        throw new Error(`${label} cannot be revalidated: ${lstatError.message}`);
      });
    }
    if (metadata.isSymbolicLink()) throw new Error(`${label} ancestor must not be a symlink: ${next}`);
    if (!metadata.isDirectory()) throw new Error(`${label} must be a non-symlink directory: ${next}`);
    const canonical = await realpath(next).catch((error) => {
      throw new Error(`${label} realpath failed: ${error.message}`);
    });
    if (canonical !== next) throw new Error(`${label} canonical path differs: ${next}`);
    lexical = next;
  }
}

async function assertOutputFile(path, label) {
  let metadata;
  try {
    metadata = await lstat(path);
  } catch (error) {
    if (error?.code === 'ENOENT') return;
    throw new Error(`${label} is unavailable: ${error.message}`);
  }
  if (metadata.isSymbolicLink()) throw new Error(`${label} must not be a symlink: ${path}`);
  if (!metadata.isFile()) throw new Error(`${label} must be a regular file: ${path}`);
}

function assertOutsideSource(candidate, sourceLexicalPath, sourceRealPath) {
  const safeCandidate = resolve(candidate);
  if (isSameOrInside(safeCandidate, resolve(sourceLexicalPath))
    || isSameOrInside(safeCandidate, resolve(sourceRealPath))) {
    throw new Error('package output must be outside the source runtime/native tree');
  }
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

function isSameOrInside(candidate, parent) {
  const child = relative(parent, candidate);
  return child === '' || (child !== '..' && !child.startsWith(`..${sep}`) && !isAbsolute(child));
}
