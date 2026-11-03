import { lstat, readFile, realpath, stat } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { join, resolve, sep } from 'node:path';
import { sha256Hex } from '../../core/src/canonical.js';
import { loadSceneAssetCompilerContractRegistry, loadSmcContractRegistry } from '../../contracts/src/closed-schema-set.js';
import { ApplicationCommandError } from './command-error.js';

const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const SCENE_ASSET_COMPILER_SCHEMA_DIR = join(REPO_ROOT, 'contracts', 'schemas', 'scene-asset-compiler');
const SMC_SCHEMA_DIR = join(REPO_ROOT, 'contracts', 'schemas');
const IMAGE_MIME_TYPES = new Set(['image/png', 'image/jpeg', 'image/webp']);
const RUNTIME_EXTENSIONS = new Map([
  ['.png', 'image/png'],
  ['.jpg', 'image/jpeg'],
  ['.jpeg', 'image/jpeg'],
  ['.webp', 'image/webp']
]);

let sceneAssetCompilerRegistryPromise;
let sceneMotionCompilerRegistryPromise;

export function safeRelativePath(value, label) {
  if (typeof value !== 'string' || value.trim().length === 0 || value.includes('\0') || value.includes('\\') || value.startsWith('/') || value.includes(':')) {
    throw new ApplicationCommandError('configuration-path-unsafe', `${label} must be a safe project-relative path`);
  }
  const parts = value.split('/');
  if (parts.some((part) => part.length === 0 || part === '.' || part === '..')) throw new ApplicationCommandError('configuration-path-unsafe', `${label} must not contain traversal or empty path segments`);
  return value;
}

export async function resolveProjectRoot(projectRoot) {
  if (typeof projectRoot !== 'string' || projectRoot.trim().length === 0) throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot is required');
  const declared = resolve(projectRoot);
  let info;
  try { info = await lstat(declared); } catch (error) { throw new ApplicationCommandError('input-unreadable', `projectRoot is unreadable: ${declared}`, { cause: error }); }
  if (info.isSymbolicLink() || !info.isDirectory()) throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot must be a non-symlink directory');
  const root = await realpath(declared);
  const canonicalInfo = await lstat(root);
  if (canonicalInfo.isSymbolicLink() || !canonicalInfo.isDirectory()) throw new ApplicationCommandError('configuration-path-unsafe', 'projectRoot realpath is not a directory');
  return root;
}

export async function resolveSafeFile(root, path, label) {
  let current = root;
  const parts = path.split('/');
  for (const [index, part] of parts.entries()) {
    current = join(current, part);
    let info;
    try { info = await lstat(current); } catch (error) { throw new ApplicationCommandError('input-unreadable', `${label} is unreadable: ${path}`, { cause: error }); }
    if (info.isSymbolicLink()) throw new ApplicationCommandError('configuration-path-unsafe', `${label} cannot be a symlink: ${path}`);
    if (index < parts.length - 1 && !info.isDirectory()) throw new ApplicationCommandError('configuration-path-unsafe', `${label} parent is not a directory: ${path}`);
    if (index === parts.length - 1 && !info.isFile()) throw new ApplicationCommandError('configuration-path-unsafe', `${label} must be a regular file: ${path}`);
  }
  const canonical = await realpath(current);
  if (canonical !== root && !canonical.startsWith(`${root}${sep}`)) throw new ApplicationCommandError('configuration-path-unsafe', `${label} escapes projectRoot: ${path}`);
  return canonical;
}

export async function readJson(root, path, schemaName) {
  const target = await resolveSafeFile(root, safeRelativePath(path, schemaName), schemaName);
  let bytes;
  let value;
  try {
    bytes = await readFile(target);
    value = JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    throw new ApplicationCommandError('scene-asset-compiler-handoff-invalid', `cannot read ${schemaName}: ${error.message}`);
  }
  try {
    const registry = await sceneAssetCompilerRegistry();
    registry.assert(schemaName, value);
  } catch (error) {
    throw new ApplicationCommandError('scene-asset-compiler-handoff-invalid', `${schemaName} validation failed: ${error.message}`);
  }
  return { value, bytes, sha256: sha256Hex(bytes) };
}

export async function readBoundArtifact(root, reference, label, { runtime }) {
  if (!reference || typeof reference !== 'object') throw new ApplicationCommandError('scene-asset-compiler-artifact-ref-invalid', `${label} reference is invalid`);
  const path = safeRelativePath(reference.path, `${label}.path`);
  const target = await resolveSafeFile(root, path, label);
  const bytes = await readFile(target);
  const digest = sha256Hex(bytes);
  const info = await stat(target);
  if (digest !== reference.sha256 || info.size !== reference.sizeBytes) {
    throw new ApplicationCommandError('scene-asset-compiler-artifact-integrity-failed', `${label} hash or size mismatch`, {
      details: { path, expectedHash: reference.sha256, actualHash: digest, expectedSize: reference.sizeBytes, actualSize: info.size }
    });
  }
  if (!runtime) return { path, bytes, sha256: digest, sizeBytes: info.size, mime: reference.mime };
  const extension = path.slice(path.lastIndexOf('.')).toLowerCase();
  const extensionMime = RUNTIME_EXTENSIONS.get(extension);
  const detectedMime = detectRuntimeMime(bytes);
  if (!extensionMime || !IMAGE_MIME_TYPES.has(reference.mime) || extensionMime !== reference.mime || detectedMime !== reference.mime) {
    throw new ApplicationCommandError('scene-asset-compiler-runtime-format-invalid', `${label} must be a real PNG, JPEG, or WebP`, {
      details: { path, declaredMime: reference.mime, extensionMime, detectedMime }
    });
  }
  return { path, bytes, sha256: digest, sizeBytes: info.size, mime: reference.mime };
}

export async function validateGeneratedAssetManifest(assetManifest) {
  try {
    const registry = await sceneMotionCompilerRegistry();
    registry.assert('asset-manifest.schema.json', assetManifest);
  } catch (error) {
    throw new ApplicationCommandError('scene-asset-compiler-manifest-invalid', `generated asset manifest failed validation: ${error.message}`);
  }
}

function sceneAssetCompilerRegistry() {
  sceneAssetCompilerRegistryPromise ??= loadSceneAssetCompilerContractRegistry(SCENE_ASSET_COMPILER_SCHEMA_DIR);
  return sceneAssetCompilerRegistryPromise;
}

function sceneMotionCompilerRegistry() {
  sceneMotionCompilerRegistryPromise ??= loadSmcContractRegistry(SMC_SCHEMA_DIR);
  return sceneMotionCompilerRegistryPromise;
}

function detectRuntimeMime(bytes) {
  if (bytes.length >= 8 && bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) return 'image/png';
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return 'image/jpeg';
  if (bytes.length >= 12 && bytes.subarray(0, 4).toString('ascii') === 'RIFF' && bytes.subarray(8, 12).toString('ascii') === 'WEBP') return 'image/webp';
  return null;
}
