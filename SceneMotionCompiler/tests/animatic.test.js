// Explicit mechanical fixtures, including simulated human review records.
// These pixels/approvals are never evidence of a user's artwork or approval.
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { once } from 'node:events';
import { mkdir, writeFile, readFile, rm, rename, readdir, symlink } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { deflateSync } from 'node:zlib';
import { canonicalHash, sha256Prefixed } from '../packages/core/src/canonical.js';
import { writeReleaseFixture, writeJson } from './support/narrative-fixture.js';
import { frameState, totalFrames } from '../packages/production/src/animatic-runtime.js';
import { loadAnimaticInput, renderAnimatic, statusAnimatic, cancelAnimatic, recoverAnimatic } from '../packages/production/src/animatic.js';

const STORY_ROOT = fileURLToPath(new URL('../../', import.meta.url));
const NVC = process.env.STORY_NVC_BIN || join(STORY_ROOT, 'NarrativeCourier/target/release/narrative-courier-cli');
const SAC = process.env.STORY_SAC_ANIMATIC_BIN || join(STORY_ROOT, 'SceneAssetCompiler/target/release/sac-animatic');
const SMC_CLI = fileURLToPath(new URL('../packages/cli/src/main.js', import.meta.url));
const SMC_NATIVE = process.env.SMC_ANIMATIC_BIN || fileURLToPath(new URL('../target/release/smc-animatic', import.meta.url));
const execute = promisify(execFile);

async function owner(lane, root, tokens) {
  const executable = lane === SAC ? SAC : NVC;
  const arguments_ = lane === SAC ? [...tokens, '--owner-root', join(STORY_ROOT, 'SceneAssetCompiler')] : [lane, ...tokens];
  const { stdout } = await execute(executable, [...arguments_, '--project-root', root], { maxBuffer: 16 * 1024 * 1024 });
  return JSON.parse(stdout);
}

function testPng(width, height, transparent) {
  function crc(bytes) { let v = 0xffffffff; for (const byte of bytes) { v ^= byte; for (let k = 0; k < 8; k++) v = (v >>> 1) ^ ((v & 1) ? 0xedb88320 : 0); } return (v ^ 0xffffffff) >>> 0; }
  function chunk(name, bytes) { const type = Buffer.from(name); const length = Buffer.alloc(4); length.writeUInt32BE(bytes.length); const sum = Buffer.alloc(4); sum.writeUInt32BE(crc(Buffer.concat([type, bytes]))); return Buffer.concat([length, type, bytes, sum]); }
  const header = Buffer.alloc(13); header.writeUInt32BE(width); header.writeUInt32BE(height, 4); header[8] = 8; header[9] = 6;
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const pos = y * (width * 4 + 1) + 1 + x * 4;
    raw[pos] = x % 256; raw[pos + 1] = y % 256; raw[pos + 2] = transparent ? 230 : 60;
    raw[pos + 3] = transparent && x < width / 4 ? 0 : 255;
  }
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', header), chunk('IDAT', deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

async function fixture(t) {
  const source = await writeReleaseFixture();
  const { root, map } = source;
  t.after(async () => { if (!process.env.STORY_ANIMATIC_KEEP_FIXTURE) await rm(root, { recursive: true, force: true }); });
  // Complete the bytes pinned by this existing fixture's package/snapshot.
  await mkdir(join(root, 'input'), { recursive: true });
  for (const [path, text] of [['config.yaml', 'config'], ['input/synopsis.md', 'synopsis'], ['output/final_screenplay.md', 'screenplay'],
    ...['source_ir', 'narrative_ir', 'execution_constraints', 'character_states', 'scene_ir', 'verification'].map((k) => [`output/${k}.md`, k])]) await writeFile(join(root, path), text);
  const task = await owner('storyboard', root, ['prepare', '--task-id', 'animatic-fixture', '--brief', 'Mechanical fixture only', '--output', 'storyboards/task.json']);
  const scenes = map.scenes.map((scene, index) => ({ sceneId: scene.scene_id, intent: 'Mechanical source coverage', panels: [{ panelId: `panel-${index}`,
    composition: 'Explicit test composition', staging: 'Explicit test staging', sourceRefs: map.semantic_manifest.entries.filter((e) => e.scene_id === scene.scene_id).map((e) => ({ semanticId: e.semantic_id, entryHash: e.entry_hash })), soundNotes: 'No audio requested in this mechanical fixture', transition: 'CUT' }] }));
  await writeJson(join(root, 'board-draft.json'), { scenes, changeSummary: 'Mechanical fixture only', assumptions: [], openQuestions: [] });
  const board = await owner('storyboard', root, ['submit', '--task', task.path, '--draft', 'board-draft.json', '--output', 'storyboards/board.json']);
  await owner('storyboard', root, ['review', '--board', board.path, '--expected-hash', board.contentHash, '--reviewer', 'SIMULATED_TEST_ONLY', '--decision', 'approved', '--output', 'storyboards/review.json']);
  const boardArgs = ['--board', board.path, '--board-review', 'storyboards/review.json'];
  const current = await owner('animatic', root, ['source', ...boardArgs]);
  const camera = (frame, x = 0) => ({ frame, x, y: 0, scalePerMille: 1000, easing: 'linear' });
  const motion = (frame, x) => ({ ...camera(frame, x), y: 180, rotationMilliDeg: 0, opacityPerMille: 1000 });
  const draft = { schemaVersion: 'nvc.animatic-plan/1', source: current.source, boardHash: current.board.contentHash,
    boardReviewHash: current.boardReview.contentHash, fps: 12, canvas: { width: 640, height: 360, background: '#111827' },
    assets: [{ assetId: 'background', width: 640, height: 360, alpha: 'opaque', brief: 'Mechanical test gradient' },
      { assetId: 'actor', width: 96, height: 96, alpha: 'cutout', brief: 'Mechanical test alpha square' }],
    panels: scenes.map((scene) => ({ sceneId: scene.sceneId, panelId: scene.panels[0].panelId, durationFrames: 12,
      camera: [camera(0), camera(11, 16)], layers: [
        { layerId: 'background', depthPerMille: 400, poses: [{ frame: 0, assetId: 'background' }], motion: [motion(0, 320), motion(11, 320)] },
        { layerId: 'actor', depthPerMille: 1000, poses: [{ frame: 0, assetId: 'actor' }], motion: [motion(0, 260), motion(11, 380)] }],
      captions: [scene.panels[0].sourceRefs[0].semanticId] })), openQuestions: [] };
  await writeJson(join(root, 'direction-draft.json'), draft);
  const plan = await owner('animatic', root, ['submit', ...boardArgs, '--draft', 'direction-draft.json', '--output', 'animatic/plan.json']);
  const reviewArgs = ['review', ...boardArgs, '--plan', plan.path, '--expected-hash', plan.contentHash, '--reviewer', 'SIMULATED_TEST_ONLY', '--decision', 'approved', '--output', 'animatic/review.json'];
  const review = await owner('animatic', root, reviewArgs);
  assert.deepEqual(await owner('animatic', root, reviewArgs), review);
  const exportArgs = ['export', ...boardArgs, '--plan', plan.path, '--review', 'animatic/review.json'];
  const packet = await owner('animatic', root, [...exportArgs, '--output', 'animatic/packet.json']);
  const assets = [];
  for (const asset of draft.assets) {
    const bytes = testPng(asset.width, asset.height, asset.alpha === 'cutout');
    const path = `animatic/${asset.assetId}.png`; await writeFile(join(root, path), bytes);
    assets.push({ assetId: asset.assetId, path, sha256: sha256Prefixed(bytes), width: asset.width, height: asset.height, alpha: asset.alpha,
      provenance: { executor: 'user-supplied', source: 'TEST_ONLY_NOT_USER_ARTWORK' } });
  }
  await writeJson(join(root, 'assets-draft.json'), { schemaVersion: 'sac.animatic-assets/1', packetHash: packet.contentHash, assets });
  const pack = await owner(SAC, root, ['submit', '--packet', 'animatic/packet.json', '--draft', 'assets-draft.json', '--output', 'animatic/assets.json']);
  await owner(SAC, root, ['review', '--packet', 'animatic/packet.json', '--pack', pack.path, '--expected-hash', pack.contentHash,
    '--reviewer', 'SIMULATED_TEST_ONLY', '--decision', 'approved', '--output', 'animatic/assets-review.json']);
  return { root, packet, boardArgs, exportArgs, draft, input: { packetPath: 'animatic/packet.json', assetPackPath: 'animatic/assets.json', assetReviewPath: 'animatic/assets-review.json' } };
}

test('animatic owners preserve source, forbid wrong review/alpha/stale source, and expose exact frame state', async (t) => {
  const f = await fixture(t);
  assert.equal(totalFrames(f.packet.plan), 24);
  const end = frameState(f.packet.plan, 11);
  assert.equal(end.camera.x, 16); assert.equal(end.layers[1].x, 380);
  assert.equal(frameState(f.packet.plan, 12).panelId, 'panel-1');
  assert.equal(frameState(f.packet.plan, 11, true).layers[1].x, 260);
  assert.throws(() => frameState(f.packet.plan, 24), /FRAME_OUT_OF_RANGE/);
  await assert.rejects(owner('animatic', f.root, ['review', ...f.boardArgs, '--plan', 'animatic/plan.json',
    '--expected-hash', 'sha256:' + '0'.repeat(64), '--reviewer', 'TEST', '--decision', 'approved', '--output', 'animatic/bad-review.json']), /REVIEW_HASH_CHANGED/);
  await writeFile(join(f.root, 'animatic/actor.png'), testPng(96, 96, false));
  await assert.rejects(owner(SAC, f.root, ['inspect', '--packet', 'animatic/packet.json', '--pack', 'animatic/assets.json']), /PNG_HASH_CHANGED/);
  await assert.rejects(loadAnimaticInput(f.root, f.input), /PNG_HASH_CHANGED/);
  await writeFile(join(f.root, 'config.yaml'), 'changed');
  await assert.rejects(owner('animatic', f.root, f.exportArgs), /SOURCE_CHANGED/);
});

test('real locked Chromium -> frames -> VP9 WebM candidate exports editable source, verifies retry and rejects tampered output', async (t) => {
  const f = await fixture(t);
  const result = await renderAnimatic(f.root, { ...f.input, attemptId: 'runtime' });
  assert.equal(result.status, 'candidate', JSON.stringify(result));
  assert.equal(result.finalApproval, false); assert.equal(result.frameCount, 24); assert.equal(result.audio, 'none');
  assert.equal(result.artifacts.length, 5);
  assert.deepEqual(await renderAnimatic(f.root, { ...f.input, attemptId: 'runtime' }), result);
  await assert.rejects(renderAnimatic(f.root, { ...f.input, attemptId: 'runtime', reducedMotion: true }), /ATTEMPT_INPUT_CHANGED/);
  const frame0 = await readFile(join(f.root, 'animatic/renders/runtime/frames/000000.png'));
  const frame11 = await readFile(join(f.root, 'animatic/renders/runtime/frames/000011.png'));
  assert.notEqual(sha256Prefixed(frame0), sha256Prefixed(frame11));
  const html = await readFile(join(f.root, 'animatic/renders/runtime/composition.html'), 'utf8');
  assert.ok(html.includes('seekFrame')); assert.ok(html.includes('data:image/png;base64'));
  if (process.env.STORY_ANIMATIC_KEEP_FIXTURE) console.log(`ANIMATIC_MECHANICAL_ARTIFACT=${f.root}/animatic/renders/runtime/preview.webm`);
  const captions = await readFile(join(f.root, 'animatic/renders/runtime/captions.vtt'));
  assert.match(captions.toString('utf8'), /00:00:00.000 --> 00:00:01.000/);
  assert.ok(captions.toString('utf8').includes(f.packet.captions[0].lines[0].text));
  await writeFile(join(f.root, 'animatic/renders/runtime/captions.vtt'), 'corrupted');
  await assert.rejects(statusAnimatic(f.root, { attemptId: 'runtime' }), /OUTPUT_HASH_CHANGED/);
  await writeFile(join(f.root, 'animatic/renders/runtime/captions.vtt'), captions);
  assert.deepEqual(await statusAnimatic(f.root, { attemptId: 'runtime' }), result);
  const request = JSON.parse(await readFile(join(f.root, 'animatic/renders/runtime/request.json'), 'utf8'));
  const native = await execute(SMC_NATIVE, ['render', '--project', f.root, '--attempt', 'without-interpreters',
    '--packet', f.input.packetPath, '--assets', f.input.assetPackPath, '--asset-review', f.input.assetReviewPath], {
    timeout: 120000, maxBuffer: 16 * 1024 * 1024,
    env: { ...process.env, PATH: '/story-no-interpreters', SMC_FFMPEG_BIN: request.encoder.path,
      SMC_FFPROBE_BIN: request.probe.path, SMC_CHROMIUM_PATH: request.rendererLock.browser.binaryPath,
      SMC_FONT_PATH: request.rendererLock.font.path }
  });
  const nativeResult = JSON.parse(native.stdout);
  assert.equal(nativeResult.status, 'candidate'); assert.equal(nativeResult.frameCount, 24);
  assert.equal(nativeResult.finalApproval, false); assert.equal(nativeResult.audio, 'none');
});

test('cancellation is terminal and late successful commits cannot replace it; torn claims remain unknown', async (t) => {
  const f = await fixture(t);
  const worker = await startWorker(f, 'cancelled');
  const result = await cancelAnimatic(f.root, { attemptId: 'cancelled' });
  assert.equal(result.status, 'cancelled');
  assert.equal((await worker.closed)[0], 3);
  assert.equal(JSON.parse(worker.output()).status, 'cancelled');
  assert.deepEqual(await recoverAnimatic(f.root, { attemptId: 'cancelled' }), result);
  assert.deepEqual(await cancelAnimatic(f.root, { attemptId: 'cancelled' }), result);
  await mkdir(join(f.root, 'animatic/renders/torn'));
  assert.equal((await recoverAnimatic(f.root, { attemptId: 'torn' })).status, 'outcome_unknown');
  assert.equal((await statusAnimatic(f.root, { attemptId: 'absent' })).status, 'not_started');
});

async function startWorker(f, attemptId, native = false) {
  const child = spawn(native ? SMC_NATIVE : process.execPath, [...(native ? ['render'] : [SMC_CLI, 'animatic-render']), '--project', f.root, '--packet', f.input.packetPath,
    '--assets', f.input.assetPackPath, '--asset-review', f.input.assetReviewPath, '--attempt', attemptId], { detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  const closed = once(child, 'close');
  let output = ''; let errors = '';
  child.stdout.on('data', (bytes) => { output += bytes; }); child.stderr.on('data', (bytes) => { errors += bytes; });
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) {
    try { JSON.parse(await readFile(join(f.root, `animatic/renders/${attemptId}/request.json`), 'utf8'));
      return { child, closed, output: () => output }; }
    catch (error) { if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error; }
    if (child.exitCode !== null) throw new Error(`Worker exited before claim: ${output} ${errors}`);
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  process.kill(-child.pid, 'SIGKILL'); await closed;
  throw new Error('Worker claim deadline');
}

test('native render keeps frames, encoder output, and receipts on the selected root after its path is replaced', async (t) => {
  const f = await fixture(t);
  const moved = `${f.root}-moved`;
  t.after(() => rm(moved, { recursive: true, force: true }));
  const worker = await startWorker(f, 'root-replaced', true);
  await rename(f.root, moved); await mkdir(f.root);
  assert.equal((await worker.closed)[0], 0);
  const result = JSON.parse(worker.output());
  assert.equal(result.status, 'candidate');
  assert.equal(result.frameCount, 24);
  assert.deepEqual(await statusAnimatic(moved, { attemptId: 'root-replaced' }), result);
  assert.deepEqual(await readdir(f.root), []);
});

test('native render never writes through a replaced attempt ancestor and returns unknown', async (t) => {
  const f = await fixture(t);
  const outside = `${f.root}-outside`;
  await mkdir(outside); t.after(() => rm(outside, { recursive: true, force: true }));
  const worker = await startWorker(f, 'parent-replaced', true);
  await rename(join(f.root, 'animatic/renders'), join(f.root, 'animatic/original-renders'));
  await symlink(outside, join(f.root, 'animatic/renders'));
  assert.equal((await worker.closed)[0], 3);
  assert.equal(JSON.parse(worker.output()).status, 'outcome_unknown');
  assert.deepEqual(await readdir(outside), []);
  await assert.rejects(readFile(join(f.root, 'animatic/original-renders/parent-replaced/result.json')), { code: 'ENOENT' });
});

test('hard interruption remains unknown, recovery does not redispatch, and known renderer failure is distinct', async (t) => {
  const f = await fixture(t);
  const worker = await startWorker(f, 'interrupted');
  process.kill(-worker.child.pid, 'SIGKILL'); await worker.closed;
  assert.equal((await statusAnimatic(f.root, { attemptId: 'interrupted' })).status, 'outcome_unknown');
  const recovered = await recoverAnimatic(f.root, { attemptId: 'interrupted' });
  assert.equal(recovered.status, 'outcome_unknown');
  assert.deepEqual(await renderAnimatic(f.root, { ...f.input, attemptId: 'interrupted' }), recovered);
  assert.deepEqual(await recoverAnimatic(f.root, { attemptId: 'interrupted' }), recovered);
  // A real browser refusal is a known local failure; never an empty successful film.
  const packet = structuredClone(f.packet);
  packet.captions[0].lines[0].text = 'Lengthy source '.repeat(800);
  const reseal = (value) => { delete value.contentHash; value.contentHash = canonicalHash(value); return value; };
  reseal(packet);
  const pack = JSON.parse(await readFile(join(f.root, f.input.assetPackPath), 'utf8')); pack.packetHash = packet.contentHash; reseal(pack);
  const review = JSON.parse(await readFile(join(f.root, f.input.assetReviewPath), 'utf8')); review.packetHash = packet.contentHash; review.packHash = pack.contentHash; reseal(review);
  await writeJson(join(f.root, f.input.packetPath), packet); await writeJson(join(f.root, f.input.assetPackPath), pack); await writeJson(join(f.root, f.input.assetReviewPath), review);
  const failed = await renderAnimatic(f.root, { ...f.input, attemptId: 'caption-overflow' });
  assert.equal(failed.status, 'failed');
  assert.deepEqual(await renderAnimatic(f.root, { ...f.input, attemptId: 'caption-overflow' }), failed);
});
