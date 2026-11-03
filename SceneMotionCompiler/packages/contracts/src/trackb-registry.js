import { lstat, readFile, readdir } from 'node:fs/promises';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { SchemaRegistry, SchemaValidationError } from './schema-validator.js';

export const TRACK_B_REGISTRY_FILE = 'registry.json';

export const TRACK_B_SCHEMA_DEFINITIONS = Object.freeze([
  Object.freeze({
    key: 'trackBNarrativeAuthority',
    logicalName: 'TrackBNarrativeAuthority v2',
    file: 'trackb-narrative-authority-v2.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/narrative-authority/v2',
    version: 2
  }),
  Object.freeze({
    key: 'trackBReleaseCandidate',
    logicalName: 'TrackBReleaseCandidate v4',
    file: 'trackb-release-candidate-v4.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/release-candidate/v4',
    version: 4
  }),
  Object.freeze({
    key: 'trackBMeasuredEvidenceReceipt',
    logicalName: 'TrackBMeasuredEvidenceReceipt v2',
    file: 'trackb-measured-evidence-receipt-v2.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/measured-evidence-receipt/v2',
    version: 2
  }),
  Object.freeze({
    key: 'trackBReleaseApprovalReceipt',
    logicalName: 'TrackBReleaseApprovalReceipt v2',
    file: 'trackb-release-approval-receipt-v2.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/release-approval-receipt/v2',
    version: 2
  }),
  Object.freeze({
    key: 'trackBDeliverySet',
    logicalName: 'TrackBDeliverySet v2',
    file: 'trackb-delivery-set-v2.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/delivery-set/v2',
    version: 2
  }),
  Object.freeze({
    key: 'trackBDeliveryPointer',
    logicalName: 'TrackBDeliveryPointer v2',
    file: 'trackb-delivery-pointer-v2.schema.json',
    id: 'https://scene-motion-compiler.local/contracts/trackb/delivery-pointer/v2',
    version: 2
  })
]);

const EXPECTED_BY_FILE = new Map(TRACK_B_SCHEMA_DEFINITIONS.map((definition) => [definition.file, definition]));
const EXPECTED_BY_ID = new Map(TRACK_B_SCHEMA_DEFINITIONS.map((definition) => [definition.id, definition]));
const EXPECTED_BY_LOGICAL_NAME = new Map(TRACK_B_SCHEMA_DEFINITIONS.map((definition) => [definition.logicalName, definition]));
const EXPECTED_BY_KEY = new Map(TRACK_B_SCHEMA_DEFINITIONS.map((definition) => [definition.key, definition]));

export class TrackBRegistryError extends Error {
  constructor(message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'TrackBRegistryError';
    this.code = options.code ?? 'trackb-registry-invalid';
    this.details = options.details;
  }
}

/**
 * Load only the six Track B release schemas declared by registry.json.
 * This deliberately does not recurse through the generic Track A schema tree.
 */
export async function loadTrackBContractRegistry(schemaDir = defaultTrackBSchemaDir()) {
  const root = await assertDirectory(schemaDir, 'Track B schema directory');
  const registryPath = join(root, TRACK_B_REGISTRY_FILE);
  const registryValue = await readJsonFile(registryPath, 'Track B registry');
  const entries = assertRegistryShape(registryValue);
  await assertExactDirectoryContents(root, [TRACK_B_REGISTRY_FILE, ...entries.map((entry) => entry.path)]);
  const schemas = new SchemaRegistry();

  for (const entry of entries) {
    const definition = EXPECTED_BY_FILE.get(entry.path);
    if (!definition) {
      throw new TrackBRegistryError(`Track B registry path is not one of the six declared schema paths: ${entry.path}`);
    }
    if (entry.logicalName !== definition.logicalName || entry.$id !== definition.id || entry.version !== definition.version) {
      throw new TrackBRegistryError(`Track B registry entry does not match the immutable definition for ${entry.path}`);
    }
    const schemaPath = resolveRegistryPath(root, entry.path);
    const schema = await readJsonFile(schemaPath, `Track B schema ${entry.path}`);
    assertSchemaShape(schema, definition, entry.path);
    schemas.add(schema, { file: entry.path, path: schemaPath });
  }

  return new TrackBContractRegistry({ root, registry: registryValue, schemas });
}

export class TrackBContractRegistry {
  #root;
  #registry;
  #schemas;

  constructor({ root, registry, schemas }) {
    this.#root = root;
    this.#registry = registry;
    this.#schemas = schemas;
  }

  get root() { return this.#root; }
  get registry() { return structuredClone(this.#registry); }
  get definitions() { return TRACK_B_SCHEMA_DEFINITIONS.map((definition) => ({ ...definition })); }

  definition(identifier) {
    const definition = resolveDefinition(identifier);
    if (!definition) throw new TrackBRegistryError(`Unknown Track B schema identifier: ${String(identifier)}`);
    return { ...definition };
  }

  schema(identifier) {
    const definition = this.definition(identifier);
    return this.#schemas.schema(definition.id);
  }

  validate(identifier, value) {
    const definition = this.definition(identifier);
    return this.#schemas.validate(definition.id, value);
  }

  assert(identifier, value) {
    const definition = this.definition(identifier);
    try {
      this.#schemas.assert(definition.id, value);
    } catch (error) {
      if (error instanceof SchemaValidationError) throw error;
      throw error;
    }
    return value;
  }
}

export function isTrackBSchemaIdentifier(identifier) {
  return Boolean(resolveDefinition(identifier));
}

function resolveDefinition(identifier) {
  if (typeof identifier !== 'string') return null;
  return EXPECTED_BY_KEY.get(identifier)
    ?? EXPECTED_BY_ID.get(identifier)
    ?? EXPECTED_BY_LOGICAL_NAME.get(identifier)
    ?? EXPECTED_BY_FILE.get(identifier);
}

async function assertDirectory(path, label) {
  const resolved = resolve(path);
  let stat;
  try {
    stat = await lstat(resolved);
  } catch (error) {
    throw new TrackBRegistryError(`${label} is unavailable: ${resolved}`, { cause: error });
  }
  if (!stat.isDirectory() || stat.isSymbolicLink()) {
    throw new TrackBRegistryError(`${label} must be a real directory: ${resolved}`);
  }
  return resolved;
}

async function readJsonFile(path, label) {
  let stat;
  try {
    stat = await lstat(path);
  } catch (error) {
    throw new TrackBRegistryError(`${label} is unavailable: ${path}`, { cause: error });
  }
  if (!stat.isFile() || stat.isSymbolicLink()) throw new TrackBRegistryError(`${label} must be a regular non-symlink file: ${path}`);
  let text;
  try {
    text = await readFile(path, 'utf8');
  } catch (error) {
    throw new TrackBRegistryError(`${label} could not be read: ${path}`, { cause: error });
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new TrackBRegistryError(`${label} is not valid JSON: ${path}`, { cause: error });
  }
}

async function assertExactDirectoryContents(root, expectedNames) {
  const expected = new Set(expectedNames);
  const actual = new Set();
  let entries;
  try {
    entries = await readdir(root, { withFileTypes: true });
  } catch (error) {
    throw new TrackBRegistryError(`Track B schema directory could not be enumerated: ${root}`, { cause: error });
  }
  for (const entry of entries) {
    if (entry.isSymbolicLink()) {
      throw new TrackBRegistryError(`Track B schema directory contains a symlink: ${entry.name}`);
    }
    if (!entry.isFile()) {
      throw new TrackBRegistryError(`Track B schema directory contains a non-regular entry: ${entry.name}`);
    }
    if (!expected.has(entry.name)) {
      throw new TrackBRegistryError(`Track B schema directory contains an unclassified entry: ${entry.name}`);
    }
    actual.add(entry.name);
  }
  for (const expectedName of expected) {
    if (!actual.has(expectedName)) {
      throw new TrackBRegistryError(`Track B schema directory is missing the declared entry: ${expectedName}`);
    }
  }
}

function assertRegistryShape(value) {
  if (!isPlainObject(value)) throw new TrackBRegistryError('Track B registry must be an object');
  assertExactKeys(value, ['$schema', 'schemaVersion', 'registryId', 'schemas'], 'Track B registry');
  if (value.$schema !== 'https://json-schema.org/draft/2020-12/schema') throw new TrackBRegistryError('Track B registry must declare draft 2020-12');
  if (value.schemaVersion !== 2) throw new TrackBRegistryError('Track B registry schemaVersion must be 2');
  if (value.registryId !== 'trackb-release-contract-v4') throw new TrackBRegistryError('Track B registry registryId is invalid');
  if (!Array.isArray(value.schemas) || value.schemas.length !== TRACK_B_SCHEMA_DEFINITIONS.length) {
    throw new TrackBRegistryError('Track B registry must declare exactly six schemas');
  }
  const seenFiles = new Set();
  const seenIds = new Set();
  const seenNames = new Set();
  for (const entry of value.schemas) {
    if (!isPlainObject(entry)) throw new TrackBRegistryError('Track B registry schema entries must be objects');
    assertExactKeys(entry, ['logicalName', 'path', '$id', 'version'], 'Track B registry entry');
    assertSafeRegistryPath(entry.path);
    if (typeof entry.logicalName !== 'string' || entry.logicalName.trim() === '') throw new TrackBRegistryError('Track B registry logicalName must be non-empty');
    if (typeof entry.$id !== 'string' || !/^https:\/\/scene-motion-compiler\.local\/contracts\/trackb\/[a-z-]+\/v(?:2|4)$/.test(entry.$id)) throw new TrackBRegistryError(`Track B registry id is invalid: ${String(entry.$id)}`);
    if (!Number.isInteger(entry.version) || ![2, 4].includes(entry.version)) throw new TrackBRegistryError('Track B registry version must be integer 2 or 4');
    if (seenFiles.has(entry.path) || seenIds.has(entry.$id) || seenNames.has(entry.logicalName)) throw new TrackBRegistryError('Track B registry paths, ids, and logical names must be unique');
    seenFiles.add(entry.path); seenIds.add(entry.$id); seenNames.add(entry.logicalName);
  }
  const expectedFiles = new Set(TRACK_B_SCHEMA_DEFINITIONS.map((definition) => definition.file));
  if (seenFiles.size !== expectedFiles.size || [...expectedFiles].some((file) => !seenFiles.has(file))) throw new TrackBRegistryError('Track B registry must declare the exact six immutable schema paths');
  return value.schemas;
}

function assertSchemaShape(schema, definition, file) {
  if (!isPlainObject(schema)) throw new TrackBRegistryError(`Track B schema must be an object: ${file}`);
  if (schema.$schema !== 'https://json-schema.org/draft/2020-12/schema') throw new TrackBRegistryError(`Track B schema must use draft 2020-12: ${file}`);
  if (schema.$id !== definition.id) throw new TrackBRegistryError(`Track B schema $id does not match registry: ${file}`);
  if (schema.title !== definition.logicalName) throw new TrackBRegistryError(`Track B schema title does not match registry: ${file}`);
  if (schema.schemaVersion !== definition.version) throw new TrackBRegistryError(`Track B schema schemaVersion must be ${definition.version}: ${file}`);
  visitSchemaNodes(schema, '$', file, new Set());
}

function visitSchemaNodes(node, path, file, seen) {
  if (!isPlainObject(node) && !Array.isArray(node)) return;
  if (isPlainObject(node)) {
    if (seen.has(node)) return;
    seen.add(node);
    if (Object.hasOwn(node, '$ref')) throw new TrackBRegistryError(`Track B schema may not use implicit or remote $ref (${file}${path})`);
    if (Object.hasOwn(node, 'default')) throw new TrackBRegistryError(`Track B schema may not define implicit defaults (${file}${path})`);
    if (node.type === 'object' && node.additionalProperties !== false) throw new TrackBRegistryError(`Every Track B object schema must set additionalProperties=false (${file}${path})`);
    for (const [key, value] of Object.entries(node)) visitSchemaNodes(value, `${path}/${key}`, file, seen);
  } else {
    for (let index = 0; index < node.length; index += 1) visitSchemaNodes(node[index], `${path}/${index}`, file, seen);
  }
}

function assertSafeRegistryPath(value) {
  if (typeof value !== 'string' || value.length === 0 || value.length > 1024 || isAbsolute(value) || value.includes('\\') || value.includes('\0') || value.includes('/')) {
    throw new TrackBRegistryError(`Track B schema path must be a single safe relative filename: ${String(value)}`);
  }
  if (!/^[A-Za-z0-9._-]+$/.test(value) || value === '.' || value === '..') throw new TrackBRegistryError(`Track B schema path contains unsafe characters: ${value}`);
}

function resolveRegistryPath(root, value) {
  assertSafeRegistryPath(value);
  const resolved = resolve(root, value);
  const escaped = relative(root, resolved);
  if (escaped === '' || escaped === '..' || escaped.startsWith(`..${sep}`) || isAbsolute(escaped)) {
    throw new TrackBRegistryError(`Track B schema path escapes its registry directory: ${value}`);
  }
  return resolved;
}

function assertExactKeys(value, allowed, label) {
  const actual = Object.keys(value).sort();
  const expected = [...allowed].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new TrackBRegistryError(`${label} has unsupported or missing keys`);
  }
}

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value) && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}

function defaultTrackBSchemaDir() {
  return new URL('../../../contracts/schemas/trackb/', import.meta.url).pathname;
}
