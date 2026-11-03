import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { lstat, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { canonicalHash } from '../packages/core/src/canonical.js';
import {
  importFromNarrativeCourierReleaseBundle,
  loadNarrativeSceneProjection
} from '../packages/adapters/src/narrative-courier-release.js';
import { buildSceneGraphFromNarrativeRelease } from '../packages/adapters/src/narrative-scene-projection.js';

import { sceneMapFixture, rehashSceneMapSemantic, withDerivedId, writeJson, normalizeSourceLocatorForAssertion, writeReleaseFixture, approvedCinematicInput, CINEMATIC_PATHS } from './support/narrative-fixture.js';
const rawHash = bytes => createHash('sha256').update(bytes).digest('hex');

test('Completed NvcReleaseBundle projects structured scenes durably without screenplay parsing', async () => {
  const fixture = await writeReleaseFixture();
  try {
    const result = await importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS,
      releaseBundlePath: 'release-bundle.json',
      projectRoot: fixture.root
    });
    assert.equal(result.projectionReceipt.sceneCount, 2);
    assert.deepEqual(result.projectionReceipt.sceneIds, ['SCN-001', 'SCN-002']);
    assert.equal(result.sceneGraph.shots.length, 2);
    assert.equal(result.sceneGraph.shots[0].sourceSceneId, 'SCN-001');
    assert.equal(result.sceneGraph.shots[1].silent, true);
    assert.equal(result.sceneGraph.shots[0].timeOfDayLabel, '밤');
    assert.equal(result.sceneGraph.props.length, 1);
    assert.equal(result.sceneGraph.shots[0].propIds.length, 1);
    assert.equal(result.projectionReceiptPath, 'projection/narrative-scene-projection.v3.json');
    assert.equal(result.projectionReceipt.schemaVersion, 3);
    assert.equal(result.sceneGraph.semanticManifest.schemaVersion, 1);
    assert.equal(result.sceneGraph.semanticManifest.entries.length, 7);
    assert.deepEqual(
      [...new Set(result.sceneGraph.semanticManifest.entries.map((entry) => entry.kind))].sort(),
      ['action', 'cameraMovement', 'dialogue', 'emotion', 'lightingTransition']
    );
    const dialogueEntries = result.sceneGraph.semanticManifest.entries.filter((entry) => entry.kind === 'dialogue');
    assert.deepEqual(dialogueEntries.map((entry) => entry.sourceText), ['이제 알겠어.', '정말 끝이야.']);
    assert.deepEqual(dialogueEntries.map((entry) => entry.sourceLocator.dialogueIndex), [0, 1]);
    assert.deepEqual(dialogueEntries[0].sourceLocator, {
      sceneId: 'SCN-001', field: 'dialogue', occurrenceIndex: 0, character: '지우', dialogueIndex: 0
    });
    assert.equal('scene_id' in dialogueEntries[0].sourceLocator, false);
    assert.equal('dialogue_index' in dialogueEntries[0].sourceLocator, false);
    assert.deepEqual(dialogueEntries.map((entry) => entry.shotId), ['shot.001', 'shot.001']);
    const projectedEntries = new Map(result.sceneGraph.semanticManifest.entries.map((entry) => [entry.semanticId, entry]));
    for (const sourceEntry of fixture.map.semantic_manifest.entries) {
      const projectedEntry = projectedEntries.get(sourceEntry.semantic_id);
      assert.ok(projectedEntry, `projected semantic entry is missing: ${sourceEntry.semantic_id}`);
      assert.deepEqual(
        projectedEntry.sourceLocator,
        normalizeSourceLocatorForAssertion(sourceEntry.source_locator),
        `normalized locator changed a source value: ${sourceEntry.semantic_id}`
      );
    }
    assert.equal(result.projectionReceipt.sourceSemanticManifestHash, result.sceneGraph.semanticManifest.sourceSemanticManifestHash);
    assert.equal(result.projectionReceipt.semanticManifestHash, result.sceneGraph.semanticManifest.semanticManifestHash);
    assert.notEqual(
      result.sceneGraph.semanticManifest.sourceSemanticManifestHash,
      result.sceneGraph.semanticManifest.semanticManifestHash,
      'source and normalized semantic manifest hashes must remain distinct'
    );
    assert.match(result.projectionReceipt.sceneGraphPath, /^projection\/generations\/[A-Za-z0-9._-]+\/scene-graph\.json$/);
    assert.match(result.sceneGraphHash, /^sha256:[a-f0-9]{64}$/);
    assert.match(result.projectionReceiptHash, /^sha256:[a-f0-9]{64}$/);
    const persistedGraph = JSON.parse(await readFile(join(fixture.root, result.projectionReceipt.sceneGraphPath), 'utf8'));
    assert.deepEqual(persistedGraph, result.sceneGraph);
    const loaded = await loadNarrativeSceneProjection({
      projectRoot: fixture.root,
      projectionReceiptPath: result.projectionReceiptPath
    });
    assert.deepEqual(loaded.sceneGraph, result.sceneGraph);
    assert.equal(loaded.sceneGraphHash, result.sceneGraphHash);
    assert.equal(loaded.projectionReceiptHash, result.projectionReceiptHash);
    await assert.rejects(
      readFile(join(fixture.root, 'projection/scene-graph.json')),
      (error) => error.code === 'ENOENT'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('bundle ingress rejects unsafe paths, symlinks, non-regular files, and incomplete state', async () => {
  const fixture = await writeReleaseFixture();
  try {
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: resolve(fixture.root, 'release-bundle.json'), projectRoot: fixture.root }),
      (error) => error.code === 'configuration-path-unsafe'
    );
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: '../release-bundle.json', projectRoot: fixture.root }),
      (error) => error.code === 'configuration-path-unsafe'
    );
    await symlink('release-bundle.json', join(fixture.root, 'bundle-link.json'));
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'bundle-link.json', projectRoot: fixture.root }),
      (error) => error.code === 'configuration-path-unsafe'
    );
    await mkdir(join(fixture.root, 'bundle-directory.json'));
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'bundle-directory.json', projectRoot: fixture.root }),
      (error) => error.code === 'configuration-path-unsafe'
    );
    const statePath = join(fixture.root, 'state.json');
    const incompleteState = Buffer.from(JSON.stringify({ status: 'awaiting_human', nodes: { final_release: { status: 'completed' } } }, null, 2) + '\n');
    await writeFile(statePath, incompleteState);
    const incompleteBundle = {
      ...fixture.bundle,
      completed_state: { ...fixture.bundle.completed_state, sha256: rawHash(incompleteState) }
    };
    const rewritten = withDerivedId(incompleteBundle, 'bundle_id', 'nvcb_');
    await writeJson(join(fixture.root, 'incomplete-release-bundle.json'), rewritten);
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'incomplete-release-bundle.json', projectRoot: fixture.root }),
      (error) => error.code === 'nvc-release-incomplete'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('bundle ingress rejects package hash drift before projection', async () => {
  const fixture = await writeReleaseFixture();
  try {
    const packagePath = join(fixture.root, 'narrative-package.json');
    const packageBytes = await readFile(packagePath);
    await writeFile(packagePath, Buffer.concat([packageBytes, Buffer.from(' \n', 'utf8')]));
    await assert.rejects(
      importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'release-bundle.json', projectRoot: fixture.root }),
      (error) => error.code === 'artifact-hash-mismatch'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('scene map projection rejects duplicate SCN IDs, missing time, and scene loss', () => {
  const map = sceneMapFixture();
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: {
    ...map,
    scenes: [map.scenes[0], { ...map.scenes[0], order: 2 }]
  } }), (error) => error.code === 'nvc-scene-id-duplicate');
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: {
    ...map,
    scenes: [{ ...map.scenes[0], time_of_day: undefined }]
  } }), (error) => error.code === 'nvc-scene-time-invalid');
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: {
    ...map,
    scenes: [{ ...map.scenes[0], characters: [] }]
  } }), (error) => error.code === 'nvc-scene-character-loss');
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: { ...map, schema_version: 1 } }), (error) => error.code === 'nvc-scene-map-invalid');
});

test('scene map projection rejects semantic missing, duplicate, and hash drift without conversion', () => {
  const map = sceneMapFixture();
  const missing = structuredClone(map);
  missing.semantic_manifest.entries = missing.semantic_manifest.entries.filter((entry) => entry.semantic_id !== 'semantic.dialogue.SCN-001.0001');
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: rehashSceneMapSemantic(missing) }), (error) => error.code === 'nvc-semantic-occurrence-missing');

  const duplicateId = structuredClone(map);
  duplicateId.semantic_manifest.entries[1].semantic_id = duplicateId.semantic_manifest.entries[0].semantic_id;
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: rehashSceneMapSemantic(duplicateId) }), (error) => error.code === 'nvc-semantic-id-duplicate');

  const duplicateLocator = structuredClone(map);
  duplicateLocator.semantic_manifest.entries[2].source_locator = structuredClone(duplicateLocator.semantic_manifest.entries[1].source_locator);
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: rehashSceneMapSemantic(duplicateLocator) }), (error) => error.code === 'nvc-semantic-locator-duplicate');

  const sourceTextDrift = structuredClone(map);
  sourceTextDrift.semantic_manifest.entries[1].source_text = '바뀐 대사';
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: rehashSceneMapSemantic(sourceTextDrift) }), (error) => error.code === 'nvc-semantic-source-text-mismatch');

  const entryHashDrift = structuredClone(map);
  entryHashDrift.semantic_manifest.entries[0].entry_hash = `sha256:${'0'.repeat(64)}`;
  entryHashDrift.semantic_manifest = withDerivedId(entryHashDrift.semantic_manifest, 'semantic_manifest_hash', 'sha256:');
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: entryHashDrift }), (error) => error.code === 'nvc-semantic-entry-hash-mismatch');

  const manifestHashDrift = structuredClone(map);
  manifestHashDrift.semantic_manifest.semantic_manifest_hash = `sha256:${'0'.repeat(64)}`;
  assert.throws(() => buildSceneGraphFromNarrativeRelease({ sceneMap: manifestHashDrift }), (error) => error.code === 'nvc-semantic-manifest-hash-mismatch');
});

test('durable projection loader rejects graph hash drift and unsafe receipt paths', async () => {
  const fixture = await writeReleaseFixture();
  try {
    const result = await importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS,
      releaseBundlePath: 'release-bundle.json',
      projectRoot: fixture.root
    });
    const graphPath = join(fixture.root, result.projectionReceipt.sceneGraphPath);
    const graph = JSON.parse(await readFile(graphPath, 'utf8'));
    graph.shots[0].action = 'tampered';
    await writeFile(graphPath, JSON.stringify(graph, null, 2) + '\n');
    await assert.rejects(
      loadNarrativeSceneProjection({ projectRoot: fixture.root }),
      (error) => error.code === 'projection-graph-hash-mismatch'
    );

    await assert.rejects(
      loadNarrativeSceneProjection({ projectRoot: fixture.root, projectionReceiptPath: '../projection/narrative-scene-projection.v3.json' }),
      (error) => error.code === 'configuration-path-unsafe'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('projection graph and semantic hashes are deterministic across generation roots', async () => {
  const first = await writeReleaseFixture();
  const second = await writeReleaseFixture();
  try {
    const firstResult = await importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'release-bundle.json', projectRoot: first.root });
    const secondResult = await importFromNarrativeCourierReleaseBundle({ ...CINEMATIC_PATHS, releaseBundlePath: 'release-bundle.json', projectRoot: second.root });
    assert.equal(firstResult.sceneGraphHash, secondResult.sceneGraphHash);
    assert.equal(firstResult.projectionReceipt.sourceSemanticManifestHash, secondResult.projectionReceipt.sourceSemanticManifestHash);
    assert.equal(firstResult.projectionReceipt.semanticManifestHash, secondResult.projectionReceipt.semanticManifestHash);
    assert.equal(firstResult.projectionReceipt.sceneGraphPath === secondResult.projectionReceipt.sceneGraphPath, false);
  } finally {
    await rm(first.root, { recursive: true, force: true });
    await rm(second.root, { recursive: true, force: true });
  }
});
