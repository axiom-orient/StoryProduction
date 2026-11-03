// Explicit test data only. Never imported by production code or used as generation quality evidence.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { canonicalHash } from '../../packages/core/src/canonical.js';
import { cinematicContentHash } from '../../packages/adapters/src/cinematic-contract.js';
import { buildSceneGraphFromNarrativeRelease } from '../../packages/adapters/src/narrative-scene-projection.js';
export const CINEMATIC_PATHS = { continuityBiblePath: 'continuity-bible.json', directorPlanPath: 'director-plan.json', directorApprovalPath: 'director-approval.json' };
const seal = value => ({ ...value, contentHash: cinematicContentHash(value) });

export function approvedCinematicInput(sceneMap, options = {}) {
  const source = { narrativePackageSha256: options.narrativePackageSha256 ?? 'sha256:' + 'a'.repeat(64),
    sceneMapSha256: options.sceneMapSha256 ?? canonicalHash(sceneMap) };
  const entities = new Map();
  function add(kind, name, identity, sceneId) {
    const id = ({CHARACTER:'char',ENVIRONMENT:'env',PROP:'prop'})[kind] + '.h' + createHash('sha256').update(identity).digest('hex').slice(0,24);
    if (!entities.has(id)) entities.set(id,{entityId:id,kind,name,sourceSceneIds:[],visualState:'UNRESOLVED_VISUAL'});
    if (!entities.get(id).sourceSceneIds.includes(sceneId)) entities.get(id).sourceSceneIds.push(sceneId);
    return id;
  }
  const states = sceneMap.scenes.map(scene => ({sceneId:scene.scene_id,
    environmentId:add('ENVIRONMENT',scene.location,scene.setting+'|'+scene.location,scene.scene_id),
    characterIds:[...new Set(scene.characters)].map(name => add('CHARACTER',name,name,scene.scene_id)),
    propIds:[...new Set(scene.asset_requirements.map(r => add('PROP',r.description,r.kind+'|'+r.description,scene.scene_id)))],timeOfDay:scene.time_of_day}));
  const continuityBible = seal({schemaVersion:1,narrativeId:sceneMap.narrative_id,...source,entities:[...entities.values()],sceneStates:states});
  let order=0;
  const scenes=sceneMap.scenes.map((scene,index) => {
    const state=states[index];
    const entries=sceneMap.semantic_manifest.entries.filter(e=>e.scene_id===scene.scene_id);
    const count=options.multiShot ? (index===0?3:2) : 1;
    const groups=Array.from({length:count},()=>[]);
    entries.forEach((e,i)=>groups[Math.min(count-1,Math.floor(i*count/entries.length))].push(e));
    const shots=groups.map((group,localIndex)=>{
      order++;
      return {shotId:'shot.'+String(order).padStart(3,'0'),order,narrativeRefs:[scene.scene_id],
        durationFrames: options.multiShot ? [48,72,60,90,54][order-1] : [72,120][index],
        shotSize:localIndex===0?'WIDE':'CLOSE_UP',cameraAngle:'EYE_LEVEL',cameraMovement:'STATIC',lensIntent:'NORMAL',
        composition:'Fixture composition '+order,subjectAction:scene.action.join(' ')||scene.heading,
        performanceBeat:'Fixture beat '+order,semanticRefs:group.map(e=>e.semantic_id),dialogueRefs:group.filter(e=>e.kind==='dialogue').map(e=>e.semantic_id),
        requiredEntities:[state.environmentId,...state.characterIds,...state.propIds],continuityRefs:[scene.scene_id],visualPriority:[],
        transitionOut:{type:'CUT',durationFrames:0},referencePolicy:{requiresKeyframe:true,requiresCharacterReference:true,requiresEnvironmentReference:true}};
    });
    return {sceneId:scene.scene_id,dramaticFunction:'Fixture function',sceneObjective:'Fixture objective',sceneDurationFrames:shots.reduce((n,s)=>n+s.durationFrames,0),shots};
  });
  const cinematicDirectorPlan=seal({schemaVersion:1,planId:'CDP-fixture',...source,continuityBibleSha256:continuityBible.contentHash,fps:24,scenes});
  const directorApproval=seal({schemaVersion:1,decision:'APPROVED',directorPlanSha256:cinematicDirectorPlan.contentHash,continuityBibleSha256:continuityBible.contentHash,...source,reviewedBy:'fixture-human-not-production',reviewedAt:'2026-09-22T00:00:00Z'});
  return {sceneMap,continuityBible,cinematicDirectorPlan,directorApproval,...source};
}

const hex = (value) => createHash('sha256').update(Buffer.from(value, 'utf8')).digest('hex');
const rawHash = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function withDerivedId(value, field, prefix, blank = false) {
  const identity = { ...value };
  if (blank) identity[field] = '';
  else delete identity[field];
  return { ...value, [field]: prefix + canonicalHash(identity).slice('sha256:'.length) };
}

export function sceneMapFixture() {
  const base = {
    schema_version: 2,
    map_id: '',
    narrative_id: 'demo-project',
    screenplay_sha256: hex('screenplay'),
    scene_ir_sha256: hex('scene-ir'),
    scenes: [
      {
        scene_id: 'SCN-001',
        order: 1,
        heading: 'SCN-001. INT. 방 - 밤',
        setting: 'INT',
        location: '방',
        time_of_day: 'night',
        time_of_day_label: '밤',
        silent: false,
        characters: ['지우'],
        action: ['지우가 문을 닫는다.'],
        dialogue: [
          { character: '지우', line: '이제 알겠어.' },
          { character: '지우', line: '정말 끝이야.' }
        ],
        asset_requirements: [{ kind: 'asset', description: '낡은 의자', source: '- Required props: 낡은 의자' }]
      },
      {
        scene_id: 'SCN-002',
        order: 2,
        heading: 'SCN-002. EXT. 옥상 - 새벽',
        setting: 'EXT',
        location: '옥상',
        time_of_day: 'dawn',
        time_of_day_label: '새벽',
        silent: true,
        characters: [],
        action: ['바람이 분다.'],
        dialogue: [],
        asset_requirements: []
      }
    ],
    semantic_manifest: {
      schema_version: 1,
      semantic_manifest_hash: '',
      entries: [
        { semantic_id: 'semantic.action.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'action', source_text: '지우가 문을 닫는다.', source_locator: { scene_id: 'SCN-001', field: 'action', occurrence_index: 0 } },
        { semantic_id: 'semantic.dialogue.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'dialogue', source_text: '이제 알겠어.', source_locator: { scene_id: 'SCN-001', field: 'dialogue', occurrence_index: 0, character: '지우', dialogue_index: 0 } },
        { semantic_id: 'semantic.dialogue.SCN-001.0001', entry_hash: '', scene_id: 'SCN-001', kind: 'dialogue', source_text: '정말 끝이야.', source_locator: { scene_id: 'SCN-001', field: 'dialogue', occurrence_index: 1, character: '지우', dialogue_index: 1 } },
        { semantic_id: 'semantic.emotion.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'emotion', source_text: '단호함', source_locator: { scene_id: 'SCN-001', field: 'emotion', occurrence_index: 0 } },
        { semantic_id: 'semantic.lightingTransition.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'lightingTransition', source_text: 'night-to-dawn', source_locator: { scene_id: 'SCN-001', field: 'lightingTransition', occurrence_index: 0 } },
        { semantic_id: 'semantic.action.SCN-002.0000', entry_hash: '', scene_id: 'SCN-002', kind: 'action', source_text: '바람이 분다.', source_locator: { scene_id: 'SCN-002', field: 'action', occurrence_index: 0 } },
        { semantic_id: 'semantic.cameraMovement.SCN-002.0000', entry_hash: '', scene_id: 'SCN-002', kind: 'cameraMovement', source_text: 'static', source_locator: { scene_id: 'SCN-002', field: 'cameraMovement', occurrence_index: 0 } }
      ]
    }
  };
  base.semantic_manifest.entries = base.semantic_manifest.entries.map((entry) => withDerivedId(entry, 'entry_hash', 'sha256:'));
  base.semantic_manifest = withDerivedId(base.semantic_manifest, 'semantic_manifest_hash', 'sha256:');
  return withDerivedId(base, 'map_id', 'nsm2_', true);
}

export function rehashSceneMapSemantic(map) {
  const next = structuredClone(map);
  next.semantic_manifest.entries = next.semantic_manifest.entries.map((entry) => withDerivedId(entry, 'entry_hash', 'sha256:'));
  next.semantic_manifest = withDerivedId(next.semantic_manifest, 'semantic_manifest_hash', 'sha256:');
  return withDerivedId(next, 'map_id', 'nsm2_', true);
}

export function normalizeSourceLocatorForAssertion(locator) {
  const keyMap = {
    scene_id: 'sceneId',
    field: 'field',
    occurrence_index: 'occurrenceIndex',
    character: 'character',
    dialogue_index: 'dialogueIndex'
  };
  return Object.fromEntries(Object.entries(locator).map(([key, value]) => {
    assert.ok(keyMap[key], `unexpected NVC source locator key: ${key}`);
    return [keyMap[key], value];
  }));
}

export async function writeJson(path, value) {
  const bytes = Buffer.from(JSON.stringify(value, null, 2) + '\n', 'utf8');
  await mkdir(join(path, '..'), { recursive: true });
  await writeFile(path, bytes);
  return bytes;
}

export async function writeReleaseFixture({ multiShot = false } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'smc-nvc-release-'));
  const sourceSnapshot = {
    config: { path: 'config.yaml', sha256: hex('config') },
    synopsis: { path: 'input/synopsis.md', sha256: hex('synopsis') },
    references: []
  };
  const state = { status: 'completed', nodes: { final_release: { status: 'completed' } }, source_snapshot: sourceSnapshot };
  const stateBytes = await writeJson(join(root, 'state.json'), state);
  const map = sceneMapFixture();
  const mapBytes = await writeJson(join(root, 'output/scene-map.json'), map);
  const packageBase = {
    schema_version: 2,
    package_id: '',
    narrative_id: map.narrative_id,
    sts_domain: { id: 'sts', version: '1.0.0', bundle_sha256: hex('sts') },
    source_snapshot_sha256: canonicalHash(sourceSnapshot).slice('sha256:'.length),
    canonical_narrative: {
      kind: 'SCREENPLAY',
      artifact: { kind: 'screenplay', path: 'output/final_screenplay.md', sha256: hex('screenplay'), mime: 'text/markdown' }
    },
    authority: Object.fromEntries(['source_ir', 'narrative_ir', 'execution_constraints', 'character_states', 'scene_ir', 'verification'].map((kind) => [
      kind,
      { kind, path: 'output/' + kind + '.md', sha256: hex(kind), mime: 'text/markdown' }
    ])),
    scene_map: { kind: 'narrative_scene_map_v2', path: 'output/scene-map.json', sha256: rawHash(mapBytes), mime: 'application/json' }
  };
  const pkg = withDerivedId(packageBase, 'package_id', 'nvc2_');
  const packageBytes = await writeJson(join(root, 'narrative-package.json'), pkg);
  const bundleBase = {
    schema_version: 1,
    bundle_id: '',
    project_name: map.narrative_id,
    status: 'COMPLETED',
    completed_state: { kind: 'completed_state', path: 'state.json', sha256: rawHash(stateBytes), mime: 'application/json' },
    narrative_package: { kind: 'narrative_package_v2', path: 'narrative-package.json', sha256: rawHash(packageBytes), mime: 'application/json' },
    scene_map: { kind: 'narrative_scene_map_v2', path: 'output/scene-map.json', sha256: rawHash(mapBytes), mime: 'application/json' },
    package_id: pkg.package_id,
    package_sha256: rawHash(packageBytes),
    source_snapshot_sha256: pkg.source_snapshot_sha256
  };
  const bundle = withDerivedId(bundleBase, 'bundle_id', 'nvcb_');
  await writeJson(join(root, 'release-bundle.json'), bundle);
  const input = approvedCinematicInput(map, { narrativePackageSha256: 'sha256:' + rawHash(packageBytes), sceneMapSha256: 'sha256:' + rawHash(mapBytes), multiShot });
  await writeJson(join(root, 'continuity-bible.json'), input.continuityBible);
  await writeJson(join(root, 'director-plan.json'), input.cinematicDirectorPlan);
  await writeJson(join(root, 'director-approval.json'), input.directorApproval);
  return { root, bundle, map, pkg, input, options: { projectRoot: root, releaseBundlePath: 'release-bundle.json', ...CINEMATIC_PATHS } };
}


// Custody tests use a real cinematic source capsule, not a graph with invented hash pins.
export function singleSceneProjectionFixture(narrativeId = 'narrative.demo') {
  let sceneMap = sceneMapFixture();
  sceneMap.narrative_id = narrativeId;
  sceneMap.scenes = [sceneMap.scenes[0]];
  sceneMap.semantic_manifest.entries = sceneMap.semantic_manifest.entries.filter(e => e.scene_id === 'SCN-001');
  sceneMap = rehashSceneMapSemantic(sceneMap);
  const sceneBytes = Buffer.from(JSON.stringify(sceneMap) + '\n');
  const input = approvedCinematicInput(sceneMap, { narrativePackageSha256: 'sha256:' + '2'.repeat(64), sceneMapSha256: 'sha256:' + rawHash(sceneBytes) });
  // Imported below to keep all fixture decisions explicitly outside production.
  const sceneGraph = buildSceneGraphFromNarrativeRelease(input);
  const sources = { sceneMap, continuityBible: input.continuityBible, directorPlan: input.cinematicDirectorPlan, directorApproval: input.directorApproval };
  const sourceArtifacts = Object.fromEntries(Object.entries(sources).map(([key,value]) => {
    const bytes = Buffer.from(JSON.stringify(value) + '\n');
    return [key,{encoding:'base64url',sha256:'sha256:'+rawHash(bytes),bytesBase64:bytes.toString('base64url')}];
  }));
  const projection = {schemaVersion:3,projectionId:'projection.fixture.v3',narrativeId,sourceNarrativeId:narrativeId,
    bundleSha256:'sha256:'+'1'.repeat(64),packageSha256:input.narrativePackageSha256.slice(7),sceneMapSha256:input.sceneMapSha256.slice(7),
    sourceSemanticManifestHash:sceneGraph.semanticManifest.sourceSemanticManifestHash,semanticManifestHash:sceneGraph.semanticManifest.semanticManifestHash,
    sceneGraphPath:'projection/generations/generation.001/scene-graph.json',sceneGraphHash:canonicalHash(sceneGraph),
    sceneCount:1,sceneIds:['SCN-001'],shotCount:1,sceneToShot:[{sceneId:'SCN-001',shotId:'shot.001',order:1}],
    sourceArtifacts,directorPlanSha256:sceneGraph.directorPlanSha256,continuityBibleSha256:sceneGraph.continuityBibleSha256,directorApprovalSha256:sceneGraph.directorApprovalSha256};
  return {sceneGraph,projection,input};
}
