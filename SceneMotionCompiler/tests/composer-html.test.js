import test from 'node:test';
import assert from 'node:assert/strict';
import { composeSceneHtml } from '../packages/composer/src/scene-html.js';
import { parseTrackBSemanticManifestHtml } from '../packages/contracts/src/trackb-contract.js';
import { canonicalHash } from '../packages/core/src/canonical.js';

const GRAPH = {
  schemaVersion: 3,
  sceneGraphId: 'scenegraph.test',
  sourceNarrativeSha256: `sha256:${'1'.repeat(64)}`, sourceSceneMapSha256: `sha256:${'2'.repeat(64)}`,
  directorPlanSha256: `sha256:${'3'.repeat(64)}`, continuityBibleSha256: `sha256:${'4'.repeat(64)}`, directorApprovalSha256: `sha256:${'5'.repeat(64)}`,
  sourceNarrativeId: 'narrative.test',
  characters: [
    { id: 'char.hero', name: '주인공', appearance: '검은 코트' },
    { id: 'char.mentor', name: '멘토', appearance: '은발' }
  ],
  environments: [
    { id: 'env.rooftop.night', name: '옥상' },
    { id: 'env.office.morning', name: '사무실' }
  ],
  shots: [
    {
      id: 'shot.001', sourceSceneId: 'SCN-001', environmentId: 'env.rooftop.night', action: '서 있다',
      characterIds: ['char.hero'], durationFrames: 96, timeOfDay: 'night', emotion: '결의',
      camera: { shotSize: 'WIDE', cameraAngle: 'EYE_LEVEL', cameraMovement: 'PUSH_IN', lensIntent: 'NORMAL' },
      dialogue: [{ characterId: 'char.hero', line: '이제 그만 끝내자' }]
    },
    {
      id: 'shot.002', sourceSceneId: 'SCN-002', environmentId: 'env.office.morning', action: '대화한다',
      characterIds: ['char.hero', 'char.mentor'], durationFrames: 72, timeOfDay: 'morning'
    }
  ],
  timeline: { fps: 24, totalDurationFrames: 168, transitions: [] },
  semanticManifest: buildSemanticManifest([
    {
      semanticId: 'semantic.action.SCN-001.0000', shotId: 'shot.001', sourceSceneId: 'SCN-001',
      kind: 'action', sourceText: '서 있다', sourceLocator: { sceneId: 'SCN-001', field: 'action', occurrenceIndex: 0 }
    },
    {
      semanticId: 'semantic.dialogue.SCN-001.0000', shotId: 'shot.001', sourceSceneId: 'SCN-001',
      kind: 'dialogue', sourceText: '이제 그만 끝내자',
      sourceLocator: { sceneId: 'SCN-001', field: 'dialogue', occurrenceIndex: 0, character: '주인공', dialogueIndex: 0 }
    },
    {
      semanticId: 'semantic.action.SCN-002.0000', shotId: 'shot.002', sourceSceneId: 'SCN-002',
      kind: 'action', sourceText: '대화한다', sourceLocator: { sceneId: 'SCN-002', field: 'action', occurrenceIndex: 0 }
    }
  ])
};

// Explicit test-only cinematic intent: never a production fallback.
for (const shot of GRAPH.shots) {
  shot.camera ??= { shotSize: 'MEDIUM', cameraAngle: 'EYE_LEVEL', cameraMovement: 'STATIC', lensIntent: 'NORMAL' };
  shot.narrativeRefs = [shot.sourceSceneId]; shot.continuityRefs = [shot.sourceSceneId];
  shot.semanticRefs = GRAPH.semanticManifest.entries.filter(e => e.shotId === shot.id).map(e => e.semanticId);
  shot.composition = 'Test composition'; shot.performanceBeat = 'Test beat'; shot.visualPriority = [];
  shot.referencePolicy = { requiresKeyframe: true, requiresCharacterReference: true, requiresEnvironmentReference: true };
}

function buildSemanticManifest(entries) {
  const normalizedEntries = entries.map((entry) => {
    const value = { ...entry };
    value.entryHash = canonicalHash(value);
    return value;
  });
  const manifest = {
    schemaVersion: 1,
    sourceSemanticManifestHash: `sha256:${'a'.repeat(64)}`,
    entries: normalizedEntries
  };
  manifest.semanticManifestHash = canonicalHash(manifest);
  return manifest;
}

test('composes self-contained HTML with offline CSS timeline', () => {
  const html = composeSceneHtml({ sceneGraph: GRAPH });

  assert.ok(html.startsWith('<!DOCTYPE html>'), 'must be a complete document');
  assert.ok(html.includes('scenegraph.test'), 'must reference the scene graph id');
  assert.ok(!html.includes('📍 undefined'), 'canonical environment fields must resolve without an undefined label');
  assert.ok(!html.includes('💬 undefined'), 'canonical dialogue fields must resolve without an undefined speaker');
  for (const shotId of ['shot.001', 'shot.002']) {
    assert.ok(html.includes(shotId), `must include ${shotId}`);
  }

  // environment layers
  assert.ok(html.includes('옥상 · night'));
  assert.ok(html.includes('사무실 · morning'));

  // characters
  assert.ok(html.includes('주인공'));
  assert.ok(html.includes('멘토'));

  // dialogue
  assert.ok(html.includes('이제 그만 끝내자'));
  assert.ok(html.includes('dialogue-overlay'));

  // Runtime/network gate: the moving scene must work without external code.
  assert.ok(!html.includes('gsap.min.js'), 'scene HTML must not load an external runtime');
  assert.ok(!html.includes('https://'), 'scene HTML must not make external network requests');
  assert.ok(html.includes('.shot.active .character'), 'character transition must be inline CSS');

  // camera push-in on shot.001
  assert.ok(html.includes('camera-push-in'), 'camera push-in must produce a CSS camera class');
  const embedding = parseTrackBSemanticManifestHtml(html);
  assert.equal(embedding.anchor, 'trackb-semantic-manifest');
  assert.equal(embedding.encoding, 'base64url-rfc4648');
  assert.equal(embedding.semanticManifestHash, GRAPH.semanticManifest.semanticManifestHash);
  assert.equal(embedding.semanticManifest.entries.length, 3);
});

test('semantic manifest anchor is one exact hidden non-executable div across the whole HTML', () => {
  const html = composeSceneHtml({ sceneGraph: GRAPH });
  for (const spoof of [
    '<span id="trackb-semantic-manifest">spoof</span>',
    '<script id=trackb-semantic-manifest></script>',
    '<span id="trackb&#45;semantic-manifest">spoof</span>'
  ]) {
    assert.throws(
      () => parseTrackBSemanticManifestHtml(html.replace('<!DOCTYPE html>', `<!DOCTYPE html>${spoof}`)),
      /anchor id must occur exactly once/
    );
  }
  const executable = html.replace(
    '<div id="trackb-semantic-manifest"',
    '<div onclick="alert(1)" id="trackb-semantic-manifest"'
  );
  assert.throws(() => parseTrackBSemanticManifestHtml(executable), /exact non-executable contract/);
});

test('empty shots rejects fail-closed', () => {
  assert.throws(() => composeSceneHtml({ sceneGraph: { ...GRAPH, shots: [] } }), /at least one shot/);
});

test('snake_case graph rejects without a compatibility fallback', () => {
  const snakeCaseGraph = {
    schema_version: GRAPH.schemaVersion,
    scene_graph_id: GRAPH.sceneGraphId,
    source_narrative_id: GRAPH.sourceNarrativeId,
    characters: GRAPH.characters,
    environments: GRAPH.environments.map(({ id, name, timeOfDay }) => ({ id, name, time_of_day: timeOfDay })),
    shots: GRAPH.shots.map((shot) => ({
      id: shot.id,
      environment_id: shot.environmentId,
      character_ids: shot.characterIds,
      action: shot.action,
      duration_frames: shot.durationFrames,
      emotion: shot.emotion,
      camera: shot.camera,
      dialogue: shot.dialogue?.map(({ characterId, line }) => ({ character_id: characterId, line }))
    })),
    timeline: { fps: GRAPH.timeline.fps, total_duration_frames: GRAPH.timeline.totalDurationFrames }
  };

  assert.throws(() => composeSceneHtml({ sceneGraph: snakeCaseGraph }), /canonical camelCase|snake_case/);
});

test('unknown environment reference rejects', () => {
  const sceneGraph = {
    ...GRAPH,
    shots: [{ ...GRAPH.shots[0], environmentId: 'env.unknown' }, GRAPH.shots[1]]
  };

  assert.throws(() => composeSceneHtml({ sceneGraph }), /unknown environment "env\.unknown"/);
});

test('unknown shot character reference rejects', () => {
  const sceneGraph = {
    ...GRAPH,
    shots: [{ ...GRAPH.shots[0], characterIds: ['char.unknown'] }, GRAPH.shots[1]]
  };

  assert.throws(() => composeSceneHtml({ sceneGraph }), /unknown character "char\.unknown"/);
});

test('unknown dialogue character reference rejects', () => {
  const sceneGraph = {
    ...GRAPH,
    shots: [{
      ...GRAPH.shots[0],
      dialogue: [{ ...GRAPH.shots[0].dialogue[0], characterId: 'char.unknown' }]
    }, GRAPH.shots[1]]
  };

  assert.throws(() => composeSceneHtml({ sceneGraph }), /dialogue references unknown character "char\.unknown"/);
});

test('unknown prop reference rejects', () => {
  const sceneGraph = {
    ...GRAPH,
    props: [{ id: 'prop.phone', name: '전화기' }],
    shots: [{ ...GRAPH.shots[0], propIds: ['prop.unknown'] }, GRAPH.shots[1]]
  };

  assert.throws(() => composeSceneHtml({ sceneGraph }), /unknown prop "prop\.unknown"/);
});

test('asset references replace placeholder sprites when provided', () => {
  const assets = { 'char.hero': '/assets/hero.png' };
  const html = composeSceneHtml({ sceneGraph: GRAPH, assets });
  assert.ok(html.includes('/assets/hero.png'), 'provided asset path must appear in HTML');
});

test('prop references render as a first-class visual layer', () => {
  const sceneGraph = {
    ...GRAPH,
    props: [{ id: 'prop.phone', name: '전화기' }],
    shots: [{ ...GRAPH.shots[0], propIds: ['prop.phone'] }, GRAPH.shots[1]]
  };
  const html = composeSceneHtml({ sceneGraph, assets: { 'prop.phone': 'data:image/png;base64,AAAA' } });
  assert.ok(html.includes('class="prop"'));
  assert.ok(html.includes('prop-visual'));
  assert.ok(html.includes('전화기'));
  assert.ok(html.includes('data:image/png;base64,AAAA'));
});

test('verified environment assets replace the generated backplate', () => {
  const html = composeSceneHtml({ sceneGraph: GRAPH, assets: { 'env.rooftop.night': 'data:image/png;base64,AAAA' } });
  assert.ok(html.includes('class="environment-image"'));
  assert.ok(html.includes('data:image/png;base64,AAAA'));
});

test('external asset URLs are rejected before HTML composition', () => {
  assert.throws(() => composeSceneHtml({ sceneGraph: GRAPH, assets: { 'char.hero': 'https://example.com/hero.png' } }), /must be local or a base64 image/);
});

test('declared timeline transitions are rendered and drive the offline runtime', () => {
  const sceneGraph = {
    ...GRAPH,
    timeline: {
      ...GRAPH.timeline,
      transitions: [{ fromShot: 'shot.001', toShot: 'shot.002', type: 'wipe', durationFrames: 12 }]
    }
  };
  const html = composeSceneHtml({ sceneGraph });
  assert.ok(html.includes('data-next-transition-type="wipe"'));
  assert.ok(html.includes('data-next-transition-duration-ms="500"'));
  assert.ok(html.includes('#stage[data-transition="wipe"]'));
  assert.ok(html.includes("stage.dataset.transition=previous?.dataset.nextTransitionType||'cut'"));
});

test('unknown timeline transition references fail closed', () => {
  const sceneGraph = {
    ...GRAPH,
    timeline: { ...GRAPH.timeline, transitions: [{ fromShot: 'shot.001', toShot: 'shot.999', type: 'cut', durationFrames: 0 }] }
  };
  assert.throws(() => composeSceneHtml({ sceneGraph }), /unknown shot/);
});

test('non-sequential and branching timeline transitions fail closed', () => {
  const nonSequential = {
    ...GRAPH,
    timeline: { ...GRAPH.timeline, transitions: [{ fromShot: 'shot.001', toShot: 'shot.001', type: 'cut', durationFrames: 0 }] }
  };
  assert.throws(() => composeSceneHtml({ sceneGraph: nonSequential }), /same shot/);

  const branching = {
    ...GRAPH,
    timeline: {
      ...GRAPH.timeline,
      transitions: [
        { fromShot: 'shot.001', toShot: 'shot.002', type: 'cut', durationFrames: 0 },
        { fromShot: 'shot.001', toShot: 'shot.002', type: 'wipe', durationFrames: 12 }
      ]
    }
  };
  assert.throws(() => composeSceneHtml({ sceneGraph: branching }), /duplicated|multiple outgoing/);
});
