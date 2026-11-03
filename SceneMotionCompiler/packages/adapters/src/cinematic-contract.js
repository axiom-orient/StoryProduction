// Consumer validation, not a director: all cinematic decisions arrive from NVC.
import { canonicalHash, sha256Hex, sha256Prefixed } from '../../core/src/canonical.js';
import { SchemaRegistry } from '../../contracts/src/schema-validator.js';
import bibleSchema from '../../../contracts/schemas/continuity-bible-v1.schema.json' with { type: 'json' };
import planSchema from '../../../contracts/schemas/cinematic-director-plan-v1.schema.json' with { type: 'json' };
import approvalSchema from '../../../contracts/schemas/cinematic-approval-v1.schema.json' with { type: 'json' };

const registry = new SchemaRegistry();
for (const schema of [bibleSchema, planSchema, approvalSchema]) registry.add(schema);
const require = (condition, message) => { if (!condition) throw new Error('CINEMATIC_CONTRACT:' + message); };
const equal = (left, right) => canonicalHash(left) === canonicalHash(right);

export function cinematicContentHash(value) {
  const { contentHash, ...body } = value;
  assertIntegerJson(body);
  return canonicalHash(body);
}

function assertIntegerJson(value) {
  if (typeof value === 'number') require(Number.isSafeInteger(value), 'UNSAFE_INTEGER');
  else if (typeof value === 'string') require(value.isWellFormed(), 'INVALID_UNICODE');
  else if (Array.isArray(value)) value.forEach(assertIntegerJson);
  else if (value && typeof value === 'object') {
    for (const [key, child] of Object.entries(value)) {
      require(key.isWellFormed(), 'INVALID_UNICODE');
      assertIntegerJson(child);
    }
  }
}

function sealed(schema, value) {
  registry.assert(schema.$id, value);
  require(value.contentHash === cinematicContentHash(value), 'CONTENT_HASH:' + schema.title);
}

function entityId(kind, identity) {
  return ({ CHARACTER: 'char', ENVIRONMENT: 'env', PROP: 'prop' })[kind] + '.h' +
    sha256Hex(Buffer.from(identity, 'utf8')).slice(0, 24);
}

function validateContinuityFacts(sceneMap, bible) {
  const expectedEntities = new Map();
  function add(kind, name, identity, sceneId) {
    const id = entityId(kind, identity);
    const entity = expectedEntities.get(id);
    if (entity) {
      require(entity.kind === kind && entity.name === name, 'ENTITY_ID_COLLISION');
      if (!entity.sourceSceneIds.includes(sceneId)) entity.sourceSceneIds.push(sceneId);
    } else expectedEntities.set(id, { entityId: id, kind, name, sourceSceneIds: [sceneId], visualState: 'UNRESOLVED_VISUAL' });
    return id;
  }
  const states = sceneMap.scenes.map(scene => ({
    sceneId: scene.scene_id,
    environmentId: add('ENVIRONMENT', scene.location, scene.setting + '|' + scene.location, scene.scene_id),
    characterIds: [...new Set(scene.characters)].map(name => add('CHARACTER', name, name, scene.scene_id)),
    propIds: [...new Set(scene.asset_requirements.map(r => add('PROP', r.description, r.kind + '|' + r.description, scene.scene_id)))],
    timeOfDay: scene.time_of_day
  }));
  require(equal(states, bible.sceneStates) && equal([...expectedEntities.values()], bible.entities), 'CONTINUITY_FACTS_CHANGED');
}

export function validateCinematicBundle({ sceneMap, continuityBible: bible, cinematicDirectorPlan: plan,
  directorApproval: approval, narrativePackageSha256, sceneMapSha256 }) {
  sealed(bibleSchema, bible); sealed(planSchema, plan); sealed(approvalSchema, approval);
  require(/^sha256:[a-f0-9]{64}$/.test(narrativePackageSha256) && /^sha256:[a-f0-9]{64}$/.test(sceneMapSha256), 'SOURCE_HASH_REQUIRED');
  require(bible.narrativeId === sceneMap.narrative_id, 'NARRATIVE_ID');
  for (const artifact of [bible, plan, approval]) {
    require(artifact.narrativePackageSha256 === narrativePackageSha256 && artifact.sceneMapSha256 === sceneMapSha256, 'SOURCE_STALE');
  }
  require(plan.continuityBibleSha256 === bible.contentHash && approval.continuityBibleSha256 === bible.contentHash &&
    approval.directorPlanSha256 === plan.contentHash, 'APPROVAL_OR_CONTINUITY_STALE');
  validateContinuityFacts(sceneMap, bible);
  require(equal(plan.scenes.map(s => s.sceneId), sceneMap.scenes.map(s => s.scene_id)), 'SCENE_ORDER');
  const sourceEntries = new Map(sceneMap.semantic_manifest.entries.map(e => [e.semantic_id, e]));
  const assigned = new Set(), shotIds = new Set();
  let order = 0, total = 0;
  plan.scenes.forEach((scene, sceneIndex) => {
    const state = bible.sceneStates[sceneIndex];
    const allowed = new Set([state.environmentId, ...state.characterIds, ...state.propIds]);
    require(scene.sceneDurationFrames === scene.shots.reduce((sum,s) => sum + s.durationFrames, 0), 'SCENE_DURATION_SUM');
    const dialogueOrder = [];
    for (const shot of scene.shots) {
      order += 1; total += shot.durationFrames;
      require(Number.isSafeInteger(total), 'TIMELINE_OVERFLOW');
      require(shot.order === order && !shotIds.has(shot.shotId), 'SHOT_ID_OR_ORDER');
      shotIds.add(shot.shotId);
      require(equal(shot.narrativeRefs, [scene.sceneId]) && equal(shot.continuityRefs, [scene.sceneId]), 'SHOT_SCENE_BINDING');
      require(shot.requiredEntities.includes(state.environmentId) && shot.requiredEntities.every(id => allowed.has(id)), 'SHOT_ENTITY_BINDING');
      for (const ref of shot.semanticRefs) {
        require(sourceEntries.has(ref) && sourceEntries.get(ref).scene_id === scene.sceneId && !assigned.has(ref), 'SEMANTIC_ASSIGNMENT');
        assigned.add(ref);
      }
      const dialogue = shot.semanticRefs.filter(ref => sourceEntries.get(ref).kind === 'dialogue');
      require(equal(shot.dialogueRefs, dialogue), 'DIALOGUE_ASSIGNMENT');
      for (const ref of dialogue) {
        const locator = sourceEntries.get(ref).source_locator;
        dialogueOrder.push(locator.dialogue_index);
        require(shot.requiredEntities.includes(entityId('CHARACTER', locator.character)), 'DIALOGUE_CHARACTER_MISSING');
      }
      if (shot.requiredEntities.some(id => id.startsWith('char.'))) {
        require(shot.referencePolicy.requiresKeyframe && shot.referencePolicy.requiresCharacterReference, 'CHARACTER_REFERENCE_REQUIRED');
      }
      require((shot.transitionOut.type === 'CUT') === (shot.transitionOut.durationFrames === 0), 'TRANSITION_DURATION');
    }
    require(equal(dialogueOrder, [...dialogueOrder].sort((a,b) => a-b)), 'DIALOGUE_ORDER_CHANGED');
  });
  require(assigned.size === sourceEntries.size, 'SEMANTIC_OCCURRENCE_LOSS');
  const shots = plan.scenes.flatMap(scene => scene.shots);
  shots.forEach((shot, index) => {
    const duration = shot.transitionOut.durationFrames;
    require(index + 1 === shots.length ? duration === 0 : duration <= Math.min(shot.durationFrames, shots[index+1].durationFrames), 'TRANSITION_EXCEEDS_SHOT');
  });
  return { bible, plan, approval, totalDurationFrames: total };
}

/** Decode exact producer bytes from the bounded, raw-hash-bound release capsule. */
export function decodeCinematicSources(receipt) {
  const sources = {};
  const names = ['sceneMap','continuityBible','directorPlan','directorApproval'];
  require(receipt.sourceArtifacts && equal(Object.keys(receipt.sourceArtifacts).sort(), [...names].sort()), 'SOURCE_CAPSULE_SET');
  for (const name of names) {
    const ref = receipt.sourceArtifacts[name];
    require(ref.encoding === 'base64url' && typeof ref.bytesBase64 === 'string' && /^[A-Za-z0-9_-]+$/.test(ref.bytesBase64), 'SOURCE_CAPSULE_ENCODING');
    const bytes = Buffer.from(ref.bytesBase64, 'base64url');
    require(bytes.length <= 8 * 1024 * 1024 && bytes.toString('base64url') === ref.bytesBase64, 'SOURCE_CAPSULE_SIZE_OR_ENCODING');
    require(sha256Prefixed(bytes) === ref.sha256, 'SOURCE_CAPSULE_HASH:' + name);
    const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    sources[name] = JSON.parse(text);
    assertIntegerJson(sources[name]);
  }
  require(receipt.sourceArtifacts.sceneMap.sha256 === 'sha256:' + receipt.sceneMapSha256, 'SOURCE_SCENE_MAP_HASH');
  return sources;
}
