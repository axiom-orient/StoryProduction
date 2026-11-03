/** Load producer-owned schemas without creating a second registry authority. */
import { loadTrackBArtifactContractRegistries } from '../../contracts/src/trackb-artifact-schemas.js';
import { loadTrackBContractRegistry } from '../../contracts/src/trackb-registry.js';
import { isRegistryLike } from './trackb-release-bindings.js';
import { isPlainObject } from './trackb-application-primitives.js';
import { mapRuntimeError, runtimeError } from './trackb-release-runtime-support.js';

export async function loadTrackBOnlyRegistry(dependencies) {
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'Track B approval workflow dependencies must be an object');
  let registry = dependencies.registry;
  if (registry === undefined) {
    try {
      registry = await loadTrackBContractRegistry(dependencies.trackBSchemaDir);
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-artifact-binding-invalid', 'Track B schema registry could not be loaded');
    }
  }
  if (!isRegistryLike(registry)
    || typeof registry.definition !== 'function'
    || !Array.isArray(registry.definitions)
    || registry.definitions.length !== 6) {
    throw runtimeError('trackb-artifact-binding-invalid', 'a loaded six-schema Track B registry is required');
  }
  return registry;
}

export async function loadRegistries(dependencies) {
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'workflow dependencies must be an object');
  let registry = dependencies.registry;
  if (registry === undefined) {
    try {
      registry = await loadTrackBContractRegistry(dependencies.trackBSchemaDir);
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-artifact-binding-invalid', 'Track B schema registry could not be loaded');
    }
  }
  if (!isRegistryLike(registry) || typeof registry.definition !== 'function'
    || !Array.isArray(registry.definitions) || registry.definitions.length !== 6) {
    throw runtimeError('trackb-artifact-binding-invalid', 'a loaded six-schema Track B registry is required');
  }
  let artifactRegistries = dependencies.artifactRegistries;
  if (artifactRegistries === undefined) {
    try {
      artifactRegistries = await loadTrackBArtifactContractRegistries(dependencies.artifactRegistryOptions ?? {});
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-artifact-binding-invalid', 'SMC/SceneAssetCompiler artifact schema registries could not be loaded');
    }
  }
  if (!isRegistryLike(artifactRegistries?.smcRegistry) || !isRegistryLike(artifactRegistries?.sceneAssetCompilerRegistry)) {
    throw runtimeError('trackb-artifact-binding-invalid', 'loaded SMC and SceneAssetCompiler artifact schema registries are required');
  }
  return { registry, artifactRegistries };
}
