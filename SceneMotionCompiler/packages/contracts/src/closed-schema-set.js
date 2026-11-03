import { lstat, readdir, readFile } from 'node:fs/promises';
import { isAbsolute, join, resolve } from 'node:path';
import { loadContractRegistry } from './schema-validator.js';

export const SMC_ACTIVE_SCHEMA_FILES = Object.freeze([
  'continuity-bible-v1.schema.json',
  'cinematic-director-plan-v1.schema.json',
  'cinematic-approval-v1.schema.json',
  'asset-manifest.schema.json',
  'narrative-courier-narrative-package-v2.schema.json',
  'narrative-release-bundle.schema.json',
  'narrative-scene-map.schema.json',
  'narrative-scene-projection.schema.json',
  'scene-graph.schema.json'
]);

export const SMC_ARCHIVAL_SCHEMA_FILES = Object.freeze([
  'evaluation-report.schema.json'
]);

export const ANIMATIC_ACTIVE_SCHEMA_FILES = Object.freeze([
  'animatic-plan-v1.schema.json', 'animatic-review-v1.schema.json',
  'animatic-packet-v1.schema.json', 'animatic-assets-v1.schema.json',
  'animatic-assets-review-v1.schema.json', 'animatic-result-v1.schema.json'
]);

export async function loadAnimaticContractRegistry(schemaDir) {
  return loadClosedSchemaRegistry({ schemaDir, setName: 'Animatic', activeFiles: ANIMATIC_ACTIVE_SCHEMA_FILES });
}

export const SCENE_ASSET_COMPILER_ACTIVE_SCHEMA_FILES = Object.freeze([
  'visual-reference-pack-v1.schema.json',
  'visual-reference-approval-v1.schema.json',
  'binding-manifest-v1.schema.json',
  'scene-asset-pack-audit-v1.schema.json',
  'scene-asset-pack-v2.schema.json',
  'scene-asset-release-approval-v1.schema.json',
  'scene-motion-asset-handoff-v2.schema.json'
]);

export class ClosedSchemaSetError extends Error {
  constructor(message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'ClosedSchemaSetError';
    this.code = options.code ?? 'closed-schema-set-invalid';
    this.details = options.details;
  }
}

export async function loadSmcContractRegistry(schemaDir = defaultSmcSchemaDir()) {
  return loadClosedSchemaRegistry({
    schemaDir,
    setName: 'SMC',
    activeFiles: SMC_ACTIVE_SCHEMA_FILES,
    archivalFiles: SMC_ARCHIVAL_SCHEMA_FILES,
    allowedDirectories: ['scene-asset-compiler', 'trackb', 'production', 'animatic']
  });
}

export async function loadSceneAssetCompilerContractRegistry(schemaDir = defaultSceneAssetCompilerSchemaDir()) {
  return loadClosedSchemaRegistry({
    schemaDir,
    setName: 'SceneAssetCompiler',
    activeFiles: SCENE_ASSET_COMPILER_ACTIVE_SCHEMA_FILES
  });
}

/**
 * Load exactly the declared active files. Directory enumeration is used only
 * to reject drift; it is never used to choose schemas for the registry.
 */
export async function loadClosedSchemaRegistry({
  schemaDir,
  setName,
  activeFiles,
  archivalFiles = [],
  allowedDirectories = []
} = {}) {
  if (typeof setName !== 'string' || setName.trim() === '') throw new ClosedSchemaSetError('schema set name is required');
  const active = assertFileList(activeFiles, `${setName} active schema set`);
  const archival = assertFileList(archivalFiles, `${setName} archival schema set`, { allowEmpty: true });
  const activeSet = new Set(active);
  const archivalSet = new Set(archival);
  const overlap = active.find((file) => archivalSet.has(file));
  if (overlap) throw new ClosedSchemaSetError(`${setName} schema is both active and archival: ${overlap}`);

  const root = await assertRealDirectory(schemaDir, `${setName} schema directory`);
  const entries = await readdir(root, { withFileTypes: true });
  const allowedDirectorySet = new Set(allowedDirectories);
  const allowedFiles = new Set([...activeSet, ...archivalSet]);
  for (const entry of entries) {
    if (entry.isSymbolicLink()) throw new ClosedSchemaSetError(`${setName} schema directory contains a symlink: ${entry.name}`);
    if (entry.isDirectory()) {
      if (!allowedDirectorySet.has(entry.name)) throw unclassified(setName, entry.name);
      continue;
    }
    if (!entry.isFile() || !allowedFiles.has(entry.name)) throw unclassified(setName, entry.name);
  }
  for (const file of active) await assertRealFile(join(root, file), `${setName} active schema`);
  for (const file of archival) {
    const path = join(root, file);
    try {
      await assertRealFile(path, `${setName} archival schema`);
    } catch (error) {
      if (error?.cause?.code === 'ENOENT' || error?.code === 'ENOENT') continue;
      throw error;
    }
  }

  const registry = await loadContractRegistry(root, active);
  assertReferenceClosure(registry, active, setName);
  return registry;
}

function assertReferenceClosure(registry, activeFiles, setName) {
  for (const file of activeFiles) {
    const root = registry.schema(file).schema;
    visitSchema(root, { registry, currentFile: file, currentRoot: root, setName }, new Set());
  }
}

function visitSchema(node, context, seen) {
  if (!node || typeof node !== 'object') return;
  if (seen.has(node)) return;
  seen.add(node);
  if (Array.isArray(node)) {
    for (const child of node) visitSchema(child, context, seen);
    return;
  }
  if (typeof node.$ref === 'string') assertReference(node.$ref, context);
  for (const child of Object.values(node)) visitSchema(child, context, seen);
}

function assertReference(reference, { registry, currentFile, currentRoot, setName }) {
  const [location = '', fragment = ''] = reference.split('#', 2);
  let targetRoot = currentRoot;
  let targetLabel = currentFile;
  if (location) {
    let target;
    try {
      target = registry.schema(location);
    } catch (error) {
      throw new ClosedSchemaSetError(`${setName} schema has an unresolved $ref: ${reference}`, { cause: error });
    }
    targetRoot = target.schema;
    targetLabel = target.source?.file ?? location;
  }
  if (!fragment) return;
  if (!fragment.startsWith('/')) throw new ClosedSchemaSetError(`${setName} schema has an unsupported $ref fragment: ${reference}`);
  let node = targetRoot;
  for (const token of fragment.slice(1).split('/').map(unescapePointer)) {
    if (!node || typeof node !== 'object' || !Object.hasOwn(node, token)) {
      throw new ClosedSchemaSetError(`${setName} schema has a dangling $ref: ${reference}`, {
        details: { source: targetLabel }
      });
    }
    node = node[token];
  }
}

function assertFileList(files, label, { allowEmpty = false } = {}) {
  if (!Array.isArray(files) || (!allowEmpty && files.length === 0)) throw new ClosedSchemaSetError(`${label} must be a non-empty array`);
  const result = [];
  const seen = new Set();
  for (const file of files ?? []) {
    if (typeof file !== 'string' || file.length === 0 || isAbsolute(file) || file.includes('\\') || file.includes('\0') || file.includes('/') || !/^[A-Za-z0-9._-]+$/.test(file) || file === '.' || file === '..') {
      throw new ClosedSchemaSetError(`${label} contains an unsafe file name: ${String(file)}`);
    }
    if (seen.has(file)) throw new ClosedSchemaSetError(`${label} contains a duplicate: ${file}`);
    seen.add(file);
    result.push(file);
  }
  return result;
}

async function assertRealDirectory(path, label) {
  if (typeof path !== 'string' || path.trim() === '') throw new ClosedSchemaSetError(`${label} is required`);
  const resolved = resolve(path);
  let info;
  try {
    info = await lstat(resolved);
  } catch (error) {
    throw new ClosedSchemaSetError(`${label} is unavailable: ${resolved}`, { cause: error });
  }
  if (!info.isDirectory() || info.isSymbolicLink()) throw new ClosedSchemaSetError(`${label} must be a real directory: ${resolved}`);
  return resolved;
}

async function assertRealFile(path, label) {
  let info;
  try {
    info = await lstat(path);
  } catch (error) {
    throw new ClosedSchemaSetError(`${label} is unavailable: ${path}`, { cause: error });
  }
  if (!info.isFile() || info.isSymbolicLink()) throw new ClosedSchemaSetError(`${label} must be a regular non-symlink file: ${path}`);
  // Parse once here so malformed archival files are reported as contract
  // errors even though archival schemas are intentionally not registered.
  try {
    JSON.parse(await readFile(path, 'utf8'));
  } catch (error) {
    throw new ClosedSchemaSetError(`${label} is not valid JSON: ${path}`, { cause: error });
  }
}

function unclassified(setName, name) {
  return new ClosedSchemaSetError(`${setName} schema set contains an unclassified entry: ${name}`, {
    code: 'closed-schema-set-unclassified'
  });
}

function unescapePointer(value) {
  return value.replace(/~1/g, '/').replace(/~0/g, '~');
}

function defaultSmcSchemaDir() {
  return new URL('../../../contracts/schemas/', import.meta.url).pathname;
}

function defaultSceneAssetCompilerSchemaDir() {
  return join(defaultSmcSchemaDir(), 'scene-asset-compiler');
}
