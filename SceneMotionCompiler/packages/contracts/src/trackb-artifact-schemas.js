import { join } from 'node:path';
import { loadSceneAssetCompilerContractRegistry, loadSmcContractRegistry } from './closed-schema-set.js';

export const TRACK_B_ARTIFACT_SCHEMA_FILES = Object.freeze({
  projectionReceipt: 'narrative-scene-projection.schema.json',
  sceneGraph: 'scene-graph.schema.json',
  assetHandoff: 'scene-motion-asset-handoff-v2.schema.json'
});

/**
 * Load the closed SMC and SceneAssetCompiler contract sets needed by Track B artifact
 * binding. Archival and unclassified files are not registered or used as a
 * runtime fallback.
 */
export async function loadTrackBArtifactContractRegistries({
  smcSchemaDir = defaultSmcSchemaDir(),
  sceneAssetCompilerSchemaDir = defaultSceneAssetCompilerSchemaDir()
} = {}) {
  const [smcRegistry, sceneAssetCompilerRegistry] = await Promise.all([
    loadSmcContractRegistry(smcSchemaDir),
    loadSceneAssetCompilerContractRegistry(sceneAssetCompilerSchemaDir)
  ]);
  assertRegistryContains(smcRegistry, TRACK_B_ARTIFACT_SCHEMA_FILES.projectionReceipt, 'SMC projection receipt');
  assertRegistryContains(smcRegistry, TRACK_B_ARTIFACT_SCHEMA_FILES.sceneGraph, 'SMC SceneGraph');
  assertRegistryContains(sceneAssetCompilerRegistry, TRACK_B_ARTIFACT_SCHEMA_FILES.assetHandoff, 'SceneAssetCompiler asset handoff');
  return Object.freeze({ smcRegistry, sceneAssetCompilerRegistry });
}

/**
 * Validate supplied upstream artifacts against their actual registries before
 * any Track B cross-artifact/hash binding is evaluated. Missing registries are
 * errors, not an invitation to fall back to shape-only checks.
 */
export function validateTrackBArtifactSchemas({
  projectionReceipt,
  sceneGraph,
  handoff,
  artifactRegistries,
  registries,
  smcRegistry,
  sceneAssetCompilerRegistry
} = {}) {
  const errors = [];
  const suppliedRegistries = artifactRegistries ?? registries;
  const smc = smcRegistry ?? suppliedRegistries?.smcRegistry ?? suppliedRegistries?.smc;
  const sceneAssetCompiler = sceneAssetCompilerRegistry ?? suppliedRegistries?.sceneAssetCompilerRegistry;

  if (projectionReceipt !== undefined) {
    if (!smc) errors.push(schemaRegistryMissing('/projectionReceipt', 'SMC'));
    else errors.push(...validateRegisteredSchema(smc, TRACK_B_ARTIFACT_SCHEMA_FILES.projectionReceipt, projectionReceipt, '/projectionReceipt'));
  }
  if (sceneGraph !== undefined) {
    if (!smc) errors.push(schemaRegistryMissing('/sceneGraph', 'SMC'));
    else errors.push(...validateRegisteredSchema(smc, TRACK_B_ARTIFACT_SCHEMA_FILES.sceneGraph, sceneGraph, '/sceneGraph'));
  }
  if (handoff !== undefined) {
    if (!sceneAssetCompiler) errors.push(schemaRegistryMissing('/handoff', 'SceneAssetCompiler'));
    else errors.push(...validateRegisteredSchema(sceneAssetCompiler, TRACK_B_ARTIFACT_SCHEMA_FILES.assetHandoff, handoff, '/handoff'));
  }
  return { valid: errors.length === 0, errors };
}

function validateRegisteredSchema(registry, schemaId, value, rootPath) {
  if (!registry || typeof registry.schema !== 'function' || typeof registry.validate !== 'function') {
    return [schemaRegistryMissing(rootPath, schemaId)];
  }
  let result;
  try {
    registry.schema(schemaId);
    result = registry.validate(schemaId, value);
  } catch (error) {
    return [{
      code: 'trackb-artifact-schema-invalid',
      instancePath: rootPath,
      message: `${schemaId} registry validation failed: ${error.message}`
    }];
  }
  if (result.valid) return [];
  return result.errors.map((item) => ({
    ...item,
    code: 'trackb-artifact-schema-invalid',
    instancePath: `${rootPath}${item.instancePath || '/'}`
  }));
}

function assertRegistryContains(registry, schemaId, label) {
  try {
    registry.schema(schemaId);
  } catch (error) {
    const wrapped = new Error(`${label} schema is required: ${schemaId}`, { cause: error });
    wrapped.code = 'trackb-artifact-schema-registry-missing';
    throw wrapped;
  }
}

function schemaRegistryMissing(path, label) {
  return {
    code: 'trackb-artifact-schema-registry-missing',
    instancePath: path,
    message: `${label} artifact schema registry is required`
  };
}

function defaultSmcSchemaDir() {
  return new URL('../../../contracts/schemas/', import.meta.url).pathname;
}

function defaultSceneAssetCompilerSchemaDir() {
  return join(defaultSmcSchemaDir(), 'scene-asset-compiler');
}
