import { canonicalHash, sha256Hex } from '../../core/src/canonical.js';
import { decodeCinematicSources, validateCinematicBundle } from './cinematic-contract.js';
import { contractError } from './narrative-courier-release-support.js';

const PREF_SHA256 = /^sha256:[0-9a-f]{64}$/;
const SCENE_ID = /^SCN-[0-9]{3,}$/;
const SEMANTIC_ID = /^semantic\.[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/;
const TIME_OF_DAY = new Set(['dawn', 'morning', 'noon', 'day', 'afternoon', 'dusk', 'night']);
const SEMANTIC_KINDS = new Set(['action', 'dialogue', 'emotion', 'lightingTransition', 'cameraMovement']);
const SCENE_GRAPH_SCHEMA_VERSION = 3;
const SEMANTIC_MANIFEST_SCHEMA_VERSION = 1;
const SCENE_GRAPH_ID_SEGMENT_MAX_LENGTH = 127;
const CANONICAL_NARRATIVE_ID = /^narrative\.[A-Za-z0-9._:-]+$/;

/** Pure release-custody revalidation: no path re-opening and no extra authority. */
export function assertApprovedSceneProjection(projectionReceipt, sceneGraph) {
  const sources = decodeCinematicSources(projectionReceipt);
  const expectedGraph = buildSceneGraphFromNarrativeRelease({ sceneMap: sources.sceneMap,
    continuityBible: sources.continuityBible, cinematicDirectorPlan: sources.directorPlan,
    directorApproval: sources.directorApproval, narrativePackageSha256: 'sha256:' + projectionReceipt.packageSha256,
    sceneMapSha256: 'sha256:' + projectionReceipt.sceneMapSha256 });
  if (canonicalHash(expectedGraph) !== canonicalHash(sceneGraph)) {
    throw contractError('projection-approved-plan-mismatch', 'graph is not the exact approved cinematic projection');
  }
}

export function buildSceneGraphFromNarrativeRelease(input) {
  const { sceneMap } = input;
  assertSceneMapIntegrity(sceneMap);
  const { bible, plan, approval, totalDurationFrames } = validateCinematicBundle(input);
  const characterByName = new Map(bible.entities.filter(e => e.kind === 'CHARACTER').map(e => [e.name, e.entityId]));
  const characters = bible.entities.filter(e => e.kind === 'CHARACTER').map(e => ({ id: e.entityId, name: e.name }));
  const environments = bible.entities.filter(e => e.kind === 'ENVIRONMENT').map(e => ({ id: e.entityId, name: e.name }));
  const props = bible.entities.filter(e => e.kind === 'PROP').map(e => ({ id: e.entityId, name: e.name }));
  const entries = new Map(sceneMap.semantic_manifest.entries.map(e => [e.semantic_id, e]));
  const semanticToShot = new Map();
  const shots = plan.scenes.flatMap((scenePlan, index) => {
    const scene = sceneMap.scenes[index], state = bible.sceneStates[index];
    return scenePlan.shots.map(shot => {
      shot.semanticRefs.forEach(ref => semanticToShot.set(ref, shot.shotId));
      const result = {
        id: shot.shotId, sourceSceneId: scene.scene_id,
        heading: scene.heading, setting: scene.setting, location: scene.location,
        timeOfDay: scene.time_of_day, timeOfDayLabel: scene.time_of_day_label, silent: shot.dialogueRefs.length === 0,
        environmentId: state.environmentId,
        characterIds: shot.requiredEntities.filter(id => id.startsWith('char.')),
        propIds: shot.requiredEntities.filter(id => id.startsWith('prop.')),
        action: shot.subjectAction, durationFrames: shot.durationFrames,
        camera: { shotSize: shot.shotSize, cameraAngle: shot.cameraAngle, cameraMovement: shot.cameraMovement, lensIntent: shot.lensIntent },
        narrativeRefs: shot.narrativeRefs, semanticRefs: shot.semanticRefs,
        continuityRefs: shot.continuityRefs, composition: shot.composition,
        performanceBeat: shot.performanceBeat, visualPriority: shot.visualPriority,
        referencePolicy: shot.referencePolicy
      };
      if (shot.dialogueRefs.length) result.dialogue = shot.dialogueRefs.map(ref => ({
        characterId: characterByName.get(entries.get(ref).source_locator.character), line: entries.get(ref).source_text
      }));
      return result;
    });
  });
  const sourceShots = plan.scenes.flatMap(scene => scene.shots);
  const transitionTypes = { CUT: 'cut', CROSS_DISSOLVE: 'cross-dissolve', FADE_TO_BLACK: 'fade-to-black', FADE_FROM_BLACK: 'fade-from-black', WIPE: 'wipe' };
  return {
    schemaVersion: SCENE_GRAPH_SCHEMA_VERSION,
    sceneGraphId: 'scenegraph.' + deriveInternalNarrativeId(sceneMap.narrative_id),
    sourceNarrativeId: sceneMap.narrative_id,
    sourceNarrativeSha256: input.narrativePackageSha256, sourceSceneMapSha256: input.sceneMapSha256,
    directorPlanSha256: plan.contentHash, continuityBibleSha256: bible.contentHash,
    directorApprovalSha256: approval.contentHash,
    characters, environments, props, shots,
    timeline: { fps: plan.fps, totalDurationFrames, transitions: sourceShots.slice(0, -1).map((shot,index) => ({
      fromShot: shot.shotId, toShot: sourceShots[index+1].shotId,
      type: transitionTypes[shot.transitionOut.type], durationFrames: shot.transitionOut.durationFrames
    })) },
    semanticManifest: buildSceneGraphSemanticManifest(sceneMap.semantic_manifest, semanticToShot)
  };
}

function deriveInternalNarrativeId(rawId) {
  return deriveSmcNarrativeIdentifier(rawId);
}

export function deriveSmcNarrativeIdentifier(rawId) {
  if (typeof rawId !== 'string' || rawId.length === 0) {
    throw contractError('nvc-narrative-id-invalid', 'NarrativeSceneMap narrative_id must be a non-empty string');
  }
  if (rawId.length <= SCENE_GRAPH_ID_SEGMENT_MAX_LENGTH && CANONICAL_NARRATIVE_ID.test(rawId)) return rawId;
  return `narrative.nvc-${sha256Hex(Buffer.from(rawId, 'utf8'))}`;
}

export function assertSceneMapIntegrity(sceneMap) {
  if (!sceneMap || sceneMap.schema_version !== 2 || !Array.isArray(sceneMap.scenes) || sceneMap.scenes.length === 0) {
    throw contractError('nvc-scene-map-invalid', 'NarrativeSceneMap must contain at least one scene');
  }
  const seen = new Set();
  const sceneById = new Map();
  sceneMap.scenes.forEach((scene, index) => {
    if (!SCENE_ID.test(scene.scene_id)) throw contractError('nvc-scene-id-invalid', 'invalid scene id: ' + scene.scene_id);
    if (seen.has(scene.scene_id)) throw contractError('nvc-scene-id-duplicate', 'duplicate scene id: ' + scene.scene_id);
    seen.add(scene.scene_id);
    sceneById.set(scene.scene_id, scene);
    if (scene.order !== index + 1) throw contractError('nvc-scene-order-invalid', 'scene order must be contiguous at ' + scene.scene_id);
    if (!TIME_OF_DAY.has(scene.time_of_day) || typeof scene.time_of_day_label !== 'string' || scene.time_of_day_label.trim().length === 0) {
      throw contractError('nvc-scene-time-invalid', 'scene time is missing or unsupported at ' + scene.scene_id);
    }
    const dialogueCharacters = new Set(scene.dialogue.map((dialogue) => dialogue.character));
    if (scene.silent !== (scene.dialogue.length === 0)) {
      throw contractError('nvc-scene-silence-invalid', 'silent flag does not match dialogue at ' + scene.scene_id);
    }
    for (const character of dialogueCharacters) {
      if (!scene.characters.includes(character)) {
        throw contractError('nvc-scene-character-loss', 'dialogue character is missing from scene characters at ' + scene.scene_id);
      }
    }
  });
  assertNvcSemanticManifestIntegrity(sceneMap.semantic_manifest, sceneById);
  return sceneMap;
}

function assertNvcSemanticManifestIntegrity(manifest, sceneById) {
  if (!manifest || manifest.schema_version !== SEMANTIC_MANIFEST_SCHEMA_VERSION || !Array.isArray(manifest.entries)) {
    throw contractError('nvc-semantic-manifest-invalid', 'NarrativeSceneMap semantic_manifest is invalid');
  }
  assertExactKeys(manifest, new Set(['schema_version', 'semantic_manifest_hash', 'entries']), 'nvc-semantic-manifest-invalid');
  if (!PREF_SHA256.test(manifest.semantic_manifest_hash)) {
    throw contractError('nvc-semantic-manifest-invalid', 'NarrativeSceneMap semantic_manifest_hash is invalid');
  }
  const expectedManifestHash = hashWithoutField(manifest, 'semantic_manifest_hash');
  if (manifest.semantic_manifest_hash !== expectedManifestHash) {
    throw contractError('nvc-semantic-manifest-hash-mismatch', 'NarrativeSceneMap semantic_manifest_hash does not match its entries', {
      details: { expected: expectedManifestHash, actual: manifest.semantic_manifest_hash }
    });
  }

  const semanticIds = new Set();
  const locators = new Set();
  const actionOccurrences = new Map();
  const dialogueOccurrences = new Map();
  for (const [index, entry] of manifest.entries.entries()) {
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
      throw contractError('nvc-semantic-entry-invalid', `semantic entry ${index} is not an object`);
    }
    assertExactKeys(entry, new Set(['semantic_id', 'entry_hash', 'scene_id', 'kind', 'source_text', 'source_locator']), 'nvc-semantic-entry-invalid');
    if (typeof entry.semantic_id !== 'string' || !SEMANTIC_ID.test(entry.semantic_id) || !PREF_SHA256.test(entry.entry_hash) ||
        typeof entry.scene_id !== 'string' || !SCENE_ID.test(entry.scene_id) || !SEMANTIC_KINDS.has(entry.kind) ||
        typeof entry.source_text !== 'string' || entry.source_text.length === 0) {
      throw contractError('nvc-semantic-entry-invalid', `semantic entry ${index} has invalid fields`);
    }
    const expectedEntryHash = hashWithoutField(entry, 'entry_hash');
    if (entry.entry_hash !== expectedEntryHash) {
      throw contractError('nvc-semantic-entry-hash-mismatch', `semantic entry hash mismatch at index ${index}`, {
        details: { semanticId: entry.semantic_id, expected: expectedEntryHash, actual: entry.entry_hash }
      });
    }
    if (semanticIds.has(entry.semantic_id)) {
      throw contractError('nvc-semantic-id-duplicate', `duplicate semantic_id: ${entry.semantic_id}`);
    }
    semanticIds.add(entry.semantic_id);
    const locator = entry.source_locator;
    if (!locator || typeof locator !== 'object' || Array.isArray(locator)) {
      throw contractError('nvc-semantic-locator-invalid', `semantic entry source_locator is invalid: ${entry.semantic_id}`);
    }
    const locatorKeys = new Set(Object.keys(locator));
    const requiredLocatorKeys = new Set(['scene_id', 'field', 'occurrence_index']);
    const dialogueLocatorKeys = new Set(['scene_id', 'field', 'occurrence_index', 'character', 'dialogue_index']);
    if (![...requiredLocatorKeys].every((key) => locatorKeys.has(key)) ||
        [...locatorKeys].some((key) => !(entry.kind === 'dialogue' ? dialogueLocatorKeys : requiredLocatorKeys).has(key)) ||
        typeof locator.scene_id !== 'string' || !SCENE_ID.test(locator.scene_id) ||
        !SEMANTIC_KINDS.has(locator.field) || !Number.isInteger(locator.occurrence_index) || locator.occurrence_index < 0 ||
        (entry.kind === 'dialogue' && (typeof locator.character !== 'string' || locator.character.length === 0 ||
          !Number.isInteger(locator.dialogue_index) || locator.dialogue_index !== locator.occurrence_index))) {
      throw contractError('nvc-semantic-locator-invalid', `semantic entry source_locator has invalid fields: ${entry.semantic_id}`);
    }
    if (entry.kind !== 'dialogue' && (locator.character !== undefined || locator.dialogue_index !== undefined)) {
      throw contractError('nvc-semantic-locator-invalid', `non-dialogue source_locator has dialogue fields: ${entry.semantic_id}`);
    }
    const locatorKey = [locator.scene_id, locator.field, locator.occurrence_index].join('|');
    if (locators.has(locatorKey)) {
      throw contractError('nvc-semantic-locator-duplicate', `duplicate source_locator: ${locatorKey}`);
    }
    locators.add(locatorKey);
    if (entry.scene_id !== locator.scene_id) {
      throw contractError('nvc-semantic-binding-mismatch', `semantic entry scene_id differs from source_locator: ${entry.semantic_id}`);
    }
    if (entry.kind !== locator.field || !SEMANTIC_KINDS.has(entry.kind)) {
      throw contractError('nvc-semantic-kind-mismatch', `semantic entry kind differs from source_locator: ${entry.semantic_id}`);
    }
    const scene = sceneById.get(entry.scene_id);
    if (!scene) throw contractError('nvc-semantic-scene-unknown', `semantic entry references unknown scene: ${entry.scene_id}`);
    if (locator.field === 'dialogue') {
      if (locator.dialogue_index !== locator.occurrence_index) {
        throw contractError('nvc-semantic-dialogue-index-mismatch', `dialogue locator index mismatch: ${entry.semantic_id}`);
      }
      const occurrence = locator.dialogue_index;
      if (!scene.dialogue[occurrence]) {
        throw contractError('nvc-semantic-occurrence-missing', `dialogue occurrence is missing: ${entry.semantic_id}`);
      }
      if (locator.character !== scene.dialogue[occurrence].character || entry.source_text !== scene.dialogue[occurrence].line) {
        throw contractError('nvc-semantic-source-text-mismatch', `dialogue source text or character drift: ${entry.semantic_id}`);
      }
      addOccurrence(dialogueOccurrences, entry.scene_id, occurrence);
    } else if (locator.field === 'action') {
      const occurrence = locator.occurrence_index;
      if (occurrence >= scene.action.length || entry.source_text !== scene.action[occurrence]) {
        throw contractError('nvc-semantic-occurrence-missing', `action occurrence or source text drift: ${entry.semantic_id}`);
      }
      addOccurrence(actionOccurrences, entry.scene_id, occurrence);
    }
  }

  for (const [sceneId, scene] of sceneById.entries()) {
    assertOccurrenceBijection(actionOccurrences.get(sceneId), scene.action.length, `action at ${sceneId}`);
    assertOccurrenceBijection(dialogueOccurrences.get(sceneId), scene.dialogue.length, `dialogue at ${sceneId}`);
  }
}

function addOccurrence(byScene, sceneId, occurrence) {
  const values = byScene.get(sceneId) ?? new Set();
  if (values.has(occurrence)) {
    throw contractError('nvc-semantic-locator-duplicate', `duplicate semantic occurrence: ${sceneId}:${occurrence}`);
  }
  values.add(occurrence);
  byScene.set(sceneId, values);
}

function assertOccurrenceBijection(values, expectedLength, label) {
  const actual = values ?? new Set();
  if (actual.size !== expectedLength || [...Array(expectedLength).keys()].some((index) => !actual.has(index))) {
    throw contractError('nvc-semantic-occurrence-missing', `${label} semantic manifest is not a bijection`);
  }
}

function buildSceneGraphSemanticManifest(sourceManifest, sceneToShot) {
  const entries = sourceManifest.entries.map((sourceEntry) => {
    const shotId = sceneToShot.get(sourceEntry.semantic_id);
    if (!shotId) {
      throw contractError('projection-scene-map-mismatch', `semantic entry references an unmapped scene: ${sourceEntry.scene_id}`);
    }
    const entry = {
      semanticId: sourceEntry.semantic_id,
      entryHash: '',
      shotId,
      sourceSceneId: sourceEntry.scene_id,
      kind: sourceEntry.kind,
      sourceText: sourceEntry.source_text,
      // NVC owns the snake_case source manifest.  SceneGraph is the SMC
      // canonical graph, so translate only the field names at this boundary
      // while preserving every locator value exactly.
      sourceLocator: normalizeSourceLocator(sourceEntry.source_locator)
    };
    entry.entryHash = hashWithoutField(entry, 'entryHash');
    return entry;
  });
  const manifest = {
    schemaVersion: SEMANTIC_MANIFEST_SCHEMA_VERSION,
    sourceSemanticManifestHash: sourceManifest.semantic_manifest_hash,
    semanticManifestHash: '',
    entries
  };
  manifest.semanticManifestHash = hashWithoutField(manifest, 'semanticManifestHash');
  return manifest;
}

function normalizeSourceLocator(locator) {
  const normalized = {
    sceneId: locator.scene_id,
    field: locator.field,
    occurrenceIndex: locator.occurrence_index
  };
  if (locator.character !== undefined) normalized.character = locator.character;
  if (locator.dialogue_index !== undefined) normalized.dialogueIndex = locator.dialogue_index;
  return normalized;
}

function hashWithoutField(value, field) {
  const copy = structuredClone(value);
  delete copy[field];
  return canonicalHash(copy);
}

function assertExactKeys(value, keys, code) {
  const actual = new Set(Object.keys(value));
  if (actual.size !== keys.size || [...actual].some((key) => !keys.has(key))) {
    throw contractError(code, 'semantic contract contains an unexpected or missing field');
  }
}

export function assertSceneGraphReferences(sceneGraph) {
  if (!sceneGraph || sceneGraph.schemaVersion !== SCENE_GRAPH_SCHEMA_VERSION) {
    throw contractError('projection-graph-version-unsupported', 'SceneGraph schemaVersion must be 3');
  }
  const environments = new Set(sceneGraph.environments.map((item) => item.id));
  const characters = new Set(sceneGraph.characters.map((item) => item.id));
  const props = new Set(sceneGraph.props.map((item) => item.id));
  const shotIds = new Set();
  sceneGraph.shots.forEach((shot) => {
    if (shotIds.has(shot.id)) throw contractError('projection-shot-duplicate', 'duplicate projected shot id: ' + shot.id);
    shotIds.add(shot.id);
    if (!environments.has(shot.environmentId)) throw contractError('projection-reference-invalid', 'unknown environment: ' + shot.environmentId);
    for (const id of shot.characterIds ?? []) if (!characters.has(id)) throw contractError('projection-reference-invalid', 'unknown character: ' + id);
    for (const id of shot.propIds ?? []) if (!props.has(id)) throw contractError('projection-reference-invalid', 'unknown prop: ' + id);
    for (const dialogue of shot.dialogue ?? []) if (!characters.has(dialogue.characterId)) throw contractError('projection-reference-invalid', 'unknown dialogue character: ' + dialogue.characterId);
  });
  assertGraphSemanticManifestIntegrity(sceneGraph.semanticManifest, sceneGraph.shots);
}

function assertGraphSemanticManifestIntegrity(manifest, shots) {
  if (!manifest || manifest.schemaVersion !== SEMANTIC_MANIFEST_SCHEMA_VERSION || !Array.isArray(manifest.entries)) {
    throw contractError('projection-semantic-manifest-invalid', 'SceneGraph semanticManifest is invalid');
  }
  const expectedManifestHash = hashWithoutField(manifest, 'semanticManifestHash');
  if (manifest.semanticManifestHash !== expectedManifestHash) {
    throw contractError('projection-semantic-manifest-hash-mismatch', 'SceneGraph semanticManifestHash does not match its entries', {
      details: { expected: expectedManifestHash, actual: manifest.semanticManifestHash }
    });
  }
  const shotIds = new Set(shots.map((shot) => shot.id));
  const semanticIds = new Set();
  const locators = new Set();
  for (const [index, entry] of manifest.entries.entries()) {
    const expectedEntryHash = hashWithoutField(entry, 'entryHash');
    if (entry.entryHash !== expectedEntryHash) {
      throw contractError('projection-semantic-entry-hash-mismatch', `SceneGraph semantic entry hash mismatch at index ${index}`, {
        details: { semanticId: entry.semanticId, expected: expectedEntryHash, actual: entry.entryHash }
      });
    }
    if (semanticIds.has(entry.semanticId)) throw contractError('projection-semantic-id-duplicate', `duplicate semanticId: ${entry.semanticId}`);
    semanticIds.add(entry.semanticId);
    const locator = entry.sourceLocator;
    const locatorKey = [locator.sceneId, locator.field, locator.occurrenceIndex].join('|');
    if (locators.has(locatorKey)) throw contractError('projection-semantic-locator-duplicate', `duplicate semantic sourceLocator: ${locatorKey}`);
    locators.add(locatorKey);
    if (!shotIds.has(entry.shotId)) throw contractError('projection-semantic-shot-unknown', `semantic entry references unknown shot: ${entry.shotId}`);
    if (entry.sourceSceneId !== locator.sceneId || entry.kind !== locator.field) {
      throw contractError('projection-semantic-binding-mismatch', `semantic entry source locator does not match its identity: ${entry.semanticId}`);
    }
  }
}

export function assertProjectionBinding(projectionReceipt, sceneGraph) {
  if (projectionReceipt.shotCount !== sceneGraph.shots.length) {
    throw contractError('projection-scene-count-mismatch', 'projection receipt shotCount does not match SceneGraph shots');
  }
  const graphSceneIds = [...new Set(sceneGraph.shots.map((shot) => shot.sourceSceneId))];
  if (projectionReceipt.sceneCount !== graphSceneIds.length ||
      JSON.stringify(projectionReceipt.sceneIds) !== JSON.stringify(graphSceneIds)) {
    throw contractError('projection-scene-id-mismatch', 'projection receipt does not preserve SCN ids and order');
  }
  if (!Array.isArray(projectionReceipt.sceneToShot) || projectionReceipt.sceneToShot.length !== sceneGraph.shots.length) {
    throw contractError('projection-scene-map-mismatch', 'projection receipt sceneToShot count does not match SceneGraph');
  }
  projectionReceipt.sceneToShot.forEach((mapping, index) => {
    const shot = sceneGraph.shots[index];
    if (mapping.sceneId !== shot.sourceSceneId || mapping.shotId !== shot.id || mapping.order !== index + 1) {
      throw contractError('projection-scene-map-mismatch', 'projection receipt sceneToShot mapping is not ordered or hash-bound');
    }
  });
  for (const key of ['directorPlanSha256', 'continuityBibleSha256', 'directorApprovalSha256']) {
    if (projectionReceipt[key] !== sceneGraph[key]) throw contractError('projection-cinematic-binding-mismatch', key);
  }
  if (projectionReceipt.sourceSemanticManifestHash !== sceneGraph.semanticManifest.sourceSemanticManifestHash ||
      projectionReceipt.semanticManifestHash !== sceneGraph.semanticManifest.semanticManifestHash) {
    throw contractError('projection-semantic-binding-mismatch', 'projection receipt semantic hashes do not match the durable SceneGraph', {
      details: {
        expectedSourceSemanticManifestHash: sceneGraph.semanticManifest.sourceSemanticManifestHash,
        receivedSourceSemanticManifestHash: projectionReceipt.sourceSemanticManifestHash,
        expectedSemanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
        receivedSemanticManifestHash: projectionReceipt.semanticManifestHash
      }
    });
  }
}
