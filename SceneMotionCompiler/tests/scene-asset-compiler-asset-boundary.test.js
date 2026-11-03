import { approvedCinematicInput } from './support/narrative-fixture.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, mkdir, readFile, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { canonicalHash } from '../packages/core/src/canonical.js';
import { executeApplicationCommand } from '../packages/application/src/service.js';
import { loadVerifiedSceneAssetCompilerHandoff } from '../packages/application/src/scene-asset-compiler-handoff.js';

const SMC_CLI = resolve('packages/cli/src/main.js');
const SCENE_ASSET_COMPILER_CLI = resolve('../SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py');
const SCENE_ASSET_COMPILER_AVAILABLE = existsSync(SCENE_ASSET_COMPILER_CLI);
const PYTHON = process.env.STORY_TEST_PYTHON ?? process.env.SCENE_ASSET_COMPILER_PYTHON ?? 'python3';
const SHARED_CANONICAL_FIXTURE = resolve('contracts/examples/scene-asset-compiler-canonical-boundary-fixture.json');
const PNG_1X1 = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
  'base64'
);
const PSD_1X1 = Buffer.concat([
  Buffer.from('8BPS', 'ascii'),
  Buffer.from([0, 1]),
  Buffer.alloc(6),
  Buffer.from([0, 4]),
  Buffer.from([0, 0, 0, 1]),
  Buffer.from([0, 0, 0, 1]),
  Buffer.from([0, 8]),
  Buffer.from([0, 3])
]);

if (!SCENE_ASSET_COMPILER_AVAILABLE) {
  throw new Error(`SMC's complete functional-test suite requires the SceneAssetCompiler producer checkout at ${SCENE_ASSET_COMPILER_CLI}`);
}

const hex = (value) => createHash('sha256').update(Buffer.from(value, 'utf8')).digest('hex');
const rawHash = (bytes) => createHash('sha256').update(bytes).digest('hex');

function withDerivedId(value, field, prefix, blank = false) {
  const identity = { ...value };
  if (blank) identity[field] = '';
  else delete identity[field];
  return { ...value, [field]: prefix + canonicalHash(identity).slice('sha256:'.length) };
}

async function writeJson(path, value) {
  const bytes = Buffer.from(JSON.stringify(value, null, 2) + '\n', 'utf8');
  await mkdir(resolve(path, '..'), { recursive: true });
  await writeFile(path, bytes);
  return bytes;
}

function runJson(command, args) {
  const stdout = execFileSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  return JSON.parse(stdout);
}

function expectCliFailure(command, args, pattern) {
  try {
    execFileSync(command, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    const combined = `${error.stdout ?? ''}${error.stderr ?? ''}`;
    if (pattern) assert.match(combined, pattern);
    return combined;
  }
  assert.fail(`expected command failure: ${command} ${args.join(' ')}`);
}

function testWithSceneAssetCompiler(name, handler) {
  test(name, handler);
}

test('Python and SMC share the canonical boundary fixture and integer crop domain', async () => {
  const fixture = JSON.parse(await readFile(SHARED_CANONICAL_FIXTURE, 'utf8'));
  const { payload } = fixture;
  const crop = payload.crop;
  assert.equal(canonicalHash(payload), fixture.expectedCanonicalHash);
  assert.deepEqual(Object.keys(crop).sort(), ['height', 'width', 'x', 'y']);
  assert.ok(Object.values(crop).every((value) => Number.isSafeInteger(value)));
  assert.ok(crop.x >= 0 && crop.y >= 0 && crop.width >= 1 && crop.height >= 1);
  assert.equal(payload.alpha.unicode, '장면 🎬');
  assert.equal(payload.alpha.nullValue, null);
  assert.equal(payload.alpha.booleanFalse, false);
  assert.equal(payload.alpha.booleanTrue, true);
});

function sceneMapFixture() {
  const base = {
    schema_version: 2,
    map_id: '',
    narrative_id: 'e2e-project',
    screenplay_sha256: hex('screenplay'),
    scene_ir_sha256: hex('scene-ir'),
    scenes: [{
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
    }],
    semantic_manifest: {
      schema_version: 1,
      semantic_manifest_hash: '',
      entries: [
        { semantic_id: 'semantic.action.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'action', source_text: '지우가 문을 닫는다.', source_locator: { scene_id: 'SCN-001', field: 'action', occurrence_index: 0 } },
        { semantic_id: 'semantic.dialogue.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'dialogue', source_text: '이제 알겠어.', source_locator: { scene_id: 'SCN-001', field: 'dialogue', occurrence_index: 0, character: '지우', dialogue_index: 0 } },
        { semantic_id: 'semantic.dialogue.SCN-001.0001', entry_hash: '', scene_id: 'SCN-001', kind: 'dialogue', source_text: '정말 끝이야.', source_locator: { scene_id: 'SCN-001', field: 'dialogue', occurrence_index: 1, character: '지우', dialogue_index: 1 } },
        { semantic_id: 'semantic.emotion.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'emotion', source_text: '단호함', source_locator: { scene_id: 'SCN-001', field: 'emotion', occurrence_index: 0 } },
        { semantic_id: 'semantic.lightingTransition.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'lightingTransition', source_text: 'night-to-dawn', source_locator: { scene_id: 'SCN-001', field: 'lightingTransition', occurrence_index: 0 } },
        { semantic_id: 'semantic.cameraMovement.SCN-001.0000', entry_hash: '', scene_id: 'SCN-001', kind: 'cameraMovement', source_text: 'static', source_locator: { scene_id: 'SCN-001', field: 'cameraMovement', occurrence_index: 0 } }
      ]
    }
  };
  base.semantic_manifest.entries = base.semantic_manifest.entries.map((entry) => withDerivedId(entry, 'entry_hash', 'sha256:'));
  base.semantic_manifest = withDerivedId(base.semantic_manifest, 'semantic_manifest_hash', 'sha256:');
  return withDerivedId(base, 'map_id', 'nsm2_', true);
}

async function writeReleaseFixture() {
  const root = await mkdtemp(join(tmpdir(), 'smc-scene-asset-compiler-e2e-'));
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
  const input = approvedCinematicInput(map, { narrativePackageSha256: 'sha256:' + rawHash(packageBytes), sceneMapSha256: 'sha256:' + rawHash(mapBytes) });
  await writeJson(join(root, 'continuity-bible.json'), input.continuityBible);
  await writeJson(join(root, 'director-plan.json'), input.cinematicDirectorPlan);
  await writeJson(join(root, 'director-approval.json'), input.directorApproval);
  return { root, bundle };
}

async function createBoundaryFixture() {
  const fixture = await writeReleaseFixture();
  try {
    const projectionResult = runJson(process.execPath, [
      SMC_CLI,
      'project-narrative-bundle',
      '--project', fixture.root,
      '--bundle', 'release-bundle.json', '--continuity', 'continuity-bible.json', '--director-plan', 'director-plan.json', '--director-approval', 'director-approval.json'
    ]);
    const bootstrapPath = join(fixture.root, 'bootstrap-request.json');
    await writeJson(bootstrapPath, {
      schema_version: 1,
      project_id: 'e2e-project',
      goal: 'scene asset release',
      source: {
        kind: 'SMC_NARRATIVE_SCENE_PROJECTION',
        path: 'projection/narrative-scene-projection.v3.json',
        project_root: fixture.root,
        sha256: projectionResult.projectionReceiptHash.slice('sha256:'.length)
      },
      outputs: ['RUNTIME_ASSETS'],
      review_policy: { codex_visual_review: true, human_final_review: true }
    });
    const runDir = join(fixture.root, 'scene-asset-compiler-run');
    runJson(PYTHON, [SCENE_ASSET_COMPILER_CLI, 'init', bootstrapPath, runDir]);
    const graph = projectionResult.sceneGraph;
    const rights = { status: 'USER_OWNED', holder: 'e2e-user', source: 'immutable test fixture' };
    const specs = [
      { id: 'asset.char', entityId: graph.characters[0].id, requirementType: 'CHARACTER_SPRITE', kind: 'illustration', allowedUse: 'character sprite' },
      { id: 'asset.env', entityId: graph.environments[0].id, requirementType: 'ENVIRONMENT_PLATE', kind: 'illustration', allowedUse: 'environment plate' },
      { id: 'asset.prop', entityId: graph.props[0].id, requirementType: 'PROP_CUTOUT', kind: 'product', allowedUse: 'prop cutout' },
      { id: 'asset.shot', entityId: graph.shots[0].id, requirementType: 'SHOT_KEYFRAME', kind: 'photo', allowedUse: 'downstream shot reference only' }
    ];
    const assets = [];
    const requirements = [];
    for (const [index, spec] of specs.entries()) {
      const sourcePath = join(runDir, `inputs/${spec.id}.psd`);
      const runtimePath = join(runDir, `inputs/${spec.id}.png`);
      const sourceBytes = Buffer.concat([PSD_1X1, Buffer.from([index])]);
      await writeFile(sourcePath, sourceBytes);
      await writeFile(runtimePath, PNG_1X1);
      const sourceRelative = `inputs/${spec.id}.psd`;
      const runtimeRelative = `inputs/${spec.id}.png`;
      const sourceRegionId = `${spec.id}.full`;
      assets.push({
        assetId: spec.id,
        kind: spec.kind,
        rights,
        sourceRegionId,
        sourceArtifact: { path: sourceRelative, sha256: rawHash(sourceBytes), mime: 'image/vnd.adobe.photoshop', sizeBytes: sourceBytes.length },
        runtimeDerivative: { path: runtimeRelative, sha256: rawHash(PNG_1X1), mime: 'image/png', sizeBytes: PNG_1X1.length }
      });
      requirements.push({
        requirementId: `${spec.requirementType}:${spec.entityId}`,
        requirementType: spec.requirementType,
        sceneEntityId: spec.entityId,
        assetId: spec.id,
        kind: spec.kind,
        rights,
        sourceRegionId,
        allowedUse: spec.allowedUse,
        crop: { x: 0, y: 0, width: 1, height: 1 }
      });
    }
    const requestPath = join(runDir, 'scene-release-request.json');
    await writeJson(requestPath, {
      schemaVersion: 1,
      releaseId: 'release.e2e',
      createdAt: '2026-08-27T00:00:00Z',
      projectionReceiptPath: 'projection/narrative-scene-projection.v3.json',
      assets,
      requirements
    });
    const prepare = runJson(PYTHON, [SCENE_ASSET_COMPILER_CLI, 'scene-release', 'prepare', runDir, requestPath]);
    const approval = runJson(PYTHON, [SCENE_ASSET_COMPILER_CLI, 'scene-release', 'approve', runDir, '--reviewer', 'human-e2e', '--note', 'fixture approved']);
    const validation = runJson(PYTHON, [SCENE_ASSET_COMPILER_CLI, 'scene-release', 'validate', runDir]);
    const publication = runJson(PYTHON, [SCENE_ASSET_COMPILER_CLI, 'scene-release', 'publish', runDir]);
    return { fixture, runDir, graph, assets, requirements, prepare, approval, validation, publication };
  } catch (error) {
    await rm(fixture.root, { recursive: true, force: true });
    throw error;
  }
}

testWithSceneAssetCompiler('public NarrativeCourier projection → SceneAssetCompiler release → SMC HTML is a real offline E2E', async () => {
  const boundary = await createBoundaryFixture();
  try {
    assert.equal(boundary.prepare.status, 'PREPARED');
    assert.equal(boundary.approval.status, 'APPROVED');
    assert.equal(boundary.validation.status, 'VALID');
    assert.equal(boundary.publication.status, 'PUBLISHED');
    assert.match(boundary.publication.handoffPath, /^handoff\/scene-motion-asset-handoff\.v2\.json$/);
    const result = runJson(process.execPath, [
      SMC_CLI,
      'compile-scene-html',
      '--project', boundary.runDir,
      '--handoff', boundary.publication.handoffPath,
      '--out', 'out/e2e-scene.html'
    ]);
    assert.equal(result.status, 'PASS');
    assert.equal(result.shotCount, 1);
    assert.equal(result.characterCount, 1);
    assert.equal(result.environmentCount, 1);
    const assetPack = JSON.parse(await readFile(join(boundary.runDir, boundary.publication.handoff.assetPackPath), 'utf8'));
    assert.equal(canonicalHash(assetPack), boundary.validation.assetPackHash);
    assert.ok(assetPack.requirements.every(({ crop }) => Object.values(crop).every((value) => Number.isSafeInteger(value))));
    const html = await readFile(join(boundary.runDir, 'out/e2e-scene.html'), 'utf8');
    assert.match(html, /낡은 의자/);
    assert.match(html, /data:image\/png;base64,/);
    const provenance = JSON.parse(await readFile(join(boundary.runDir, 'out/e2e-scene.html.provenance.json'), 'utf8'));
    assert.equal(provenance.sceneGraphHash, boundary.validation.sceneGraphHash);
    assert.equal(provenance.assetManifestHash, result.assetManifestHash);
    assert.equal(provenance.authorityHash, undefined, 'ordinary HTML preview must not claim Track B authority binding');
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('SMC verified handoff loader accepts the durable v2 projection and all semantic pins', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const loaded = await loadVerifiedSceneAssetCompilerHandoff({
      projectRoot: boundary.runDir,
      handoffPath: boundary.publication.handoffPath
    });
    assert.equal(loaded.handoff.schemaVersion, 2);
    assert.equal(loaded.handoff.sceneGraphHash, loaded.projection.sceneGraphHash);
    assert.equal(loaded.handoff.sourceSemanticManifestHash, loaded.projection.sceneGraph.semanticManifest.sourceSemanticManifestHash);
    assert.equal(loaded.handoff.semanticManifestHash, loaded.projection.sceneGraph.semanticManifest.semanticManifestHash);
    assert.notEqual(loaded.handoff.sourceSemanticManifestHash, loaded.handoff.semanticManifestHash);
    assert.equal(loaded.provenance.sourceSemanticManifestHash, loaded.handoff.sourceSemanticManifestHash);
    assert.equal(loaded.provenance.semanticManifestHash, loaded.handoff.semanticManifestHash);
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('scene.compileHtml is handoff-only and rejects the retired authority path', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const compiled = await executeApplicationCommand({
      repoRoot: resolve('.'),
      projectRoot: boundary.runDir,
    }, 'scene.compileHtml', {
      sceneMotionAssetHandoffPath: boundary.publication.handoffPath,
      narrativeAuthorityPath: 'authority.json',
      outputPath: 'out/retired-authority-path.html'
    });
    assert.equal(compiled.ok, false);
    assert.equal(compiled.error.code, 'command-input-invalid');
    assert.match(compiled.error.message, /narrativeAuthorityPath/);
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('handoff-only HTML producer emits deterministic v2 provenance', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const config = { repoRoot: resolve('.'), projectRoot: boundary.runDir };
    const first = await executeApplicationCommand(config, 'scene.compileHtml', {
      sceneMotionAssetHandoffPath: boundary.publication.handoffPath,
      outputPath: 'out/trackb-v2.html'
    });
    const second = await executeApplicationCommand(config, 'scene.compileHtml', {
      sceneMotionAssetHandoffPath: boundary.publication.handoffPath,
      outputPath: 'out/trackb-v2-repeat.html'
    });
    assert.equal(first.ok, true);
    assert.equal(second.ok, true);
    assert.equal(first.result.htmlHash, second.result.htmlHash);
    assert.equal(first.result.provenance.provenanceHash, second.result.provenance.provenanceHash);
    assert.deepEqual(Object.keys(first.result.provenance).sort(), [
      'assetManifestHash', 'embedding', 'handoffHash', 'htmlHash', 'narrativeId',
      'projectionReceiptHash', 'provenanceHash', 'sceneGraphHash', 'schemaVersion',
      'semanticManifestHash', 'sourceSemanticManifestHash'
    ]);
    assert.equal(first.result.provenance.schemaVersion, 2);
    assert.equal(first.result.provenance.authorityHash, undefined);
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('SMC consumer rejects fractional and boolean crop before hash acceptance', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const packPath = join(boundary.runDir, boundary.publication.handoff.assetPackPath);
    const originalPack = JSON.parse(await readFile(packPath, 'utf8'));
    for (const [label, value] of [['fractional', 0.5], ['boolean', true]]) {
      const invalidPack = structuredClone(originalPack);
      invalidPack.requirements[0].crop.x = value;
      await writeJson(packPath, invalidPack);
      const error = expectCliFailure(process.execPath, [
        SMC_CLI,
        'compile-scene-html',
        '--project', boundary.runDir,
        '--handoff', boundary.publication.handoffPath,
        '--out', `out/invalid-crop-${label}.html`
      ], /scene-asset-compiler-handoff-invalid/);
      assert.match(error, /integer/);
      assert.doesNotMatch(error, /scene-asset-compiler-handoff-integrity-failed/);
      await writeJson(packPath, originalPack);
    }
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('SMC consumer revalidates all semantic projection pins in the v2 handoff', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const handoffPath = join(boundary.runDir, boundary.publication.handoffPath);
    const original = JSON.parse(await readFile(handoffPath, 'utf8'));
    for (const field of ['sceneGraphHash', 'sourceSemanticManifestHash', 'semanticManifestHash']) {
      const candidate = structuredClone(original);
      candidate[field] = `sha256:${'0'.repeat(64)}`;
      const identity = { ...candidate };
      delete identity.handoffId;
      candidate.handoffId = `SAC-SMC-${canonicalHash(identity).slice('sha256:'.length)}`;
      await writeJson(handoffPath, candidate);
      expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', boundary.publication.handoffPath, '--out', `out/stale-${field}.html`], /scene-asset-compiler-projection-stale/);
    }
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

testWithSceneAssetCompiler('SMC consumer rejects tampered, symlinked, traversal, stale-graph, and non-runtime inputs', async () => {
  const boundary = await createBoundaryFixture();
  try {
    const runtimePath = join(boundary.runDir, boundary.assets[0].runtimeDerivative.path);
    await writeFile(runtimePath, Buffer.from('tampered-runtime', 'utf8'));
    expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', boundary.publication.handoffPath, '--out', 'out/tampered.html'], /scene-asset-compiler-artifact-integrity-failed/);
    await writeFile(runtimePath, PNG_1X1);

    await unlink(runtimePath);
    await symlink('asset.char.psd', runtimePath);
    expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', boundary.publication.handoffPath, '--out', 'out/symlink.html'], /configuration-path-unsafe/);
    await unlink(runtimePath);
    await writeFile(runtimePath, PNG_1X1);

    expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', '../handoff/scene-motion-asset-handoff.v2.json', '--out', 'out/traversal.html'], /scene-asset-compiler-handoff-path-invalid|configuration-path-unsafe/);

    const approvalPath = join(boundary.runDir, 'handoff/scene-asset-release-approval.v1.json');
    const approval = JSON.parse(await readFile(approvalPath, 'utf8'));
    approval.subject.sceneGraphHash = `sha256:${'0'.repeat(64)}`;
    await writeJson(approvalPath, approval);
    const handoffPath = join(boundary.runDir, boundary.publication.handoffPath);
    const handoff = JSON.parse(await readFile(handoffPath, 'utf8'));
    handoff.approvalSha256 = rawHash(await readFile(approvalPath));
    const handoffIdentity = { ...handoff };
    delete handoffIdentity.handoffId;
    handoff.handoffId = `SAC-SMC-${canonicalHash(handoffIdentity).slice('sha256:'.length)}`;
    await writeJson(handoffPath, handoff);
    expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', boundary.publication.handoffPath, '--out', 'out/stale-approval.html'], /scene-asset-compiler-approval-stale/);

    const graphPath = join(boundary.runDir, 'projection/generations', (await readProjection(boundary.runDir)).sceneGraphPath.split('/').at(-2), 'scene-graph.json');
    const graph = JSON.parse(await readFile(graphPath, 'utf8'));
    graph.shots[0].action = 'stale graph';
    await writeJson(graphPath, graph);
    expectCliFailure(process.execPath, [SMC_CLI, 'compile-scene-html', '--project', boundary.runDir, '--handoff', boundary.publication.handoffPath, '--out', 'out/stale-graph.html'], /projection-graph-hash-mismatch|scene-asset-compiler-projection-stale/);
  } finally {
    await rm(boundary.fixture.root, { recursive: true, force: true });
  }
});

async function readProjection(runDir) {
  return JSON.parse(await readFile(join(runDir, 'projection/narrative-scene-projection.v3.json'), 'utf8'));
}
