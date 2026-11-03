import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFile, rm, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { writeReleaseFixture, singleSceneProjectionFixture } from './support/narrative-fixture.js';
import { cinematicContentHash } from '../packages/adapters/src/cinematic-contract.js';
import { importFromNarrativeCourierReleaseBundle, assertApprovedSceneProjection,
  loadNarrativeSceneProjection } from '../packages/adapters/src/narrative-courier-release.js';
import { buildSceneGraphFromNarrativeRelease } from '../packages/adapters/src/narrative-scene-projection.js';
import { composeSceneHtml } from '../packages/composer/src/scene-html.js';

function reseal(input) {
  for (const key of ['continuityBible', 'cinematicDirectorPlan', 'directorApproval']) input[key].contentHash = cinematicContentHash(input[key]);
  input.cinematicDirectorPlan.continuityBibleSha256 = input.continuityBible.contentHash;
  input.cinematicDirectorPlan.contentHash = cinematicContentHash(input.cinematicDirectorPlan);
  input.directorApproval.continuityBibleSha256 = input.continuityBible.contentHash;
  input.directorApproval.directorPlanSha256 = input.cinematicDirectorPlan.contentHash;
  input.directorApproval.contentHash = cinematicContentHash(input.directorApproval);
  return input;
}

test('2 scenes / 5 approved shots preserve frames, every semantic occurrence, camera, and dialogue', async () => {
  const f = await writeReleaseFixture({ multiShot:true });
  try {
    const graph = buildSceneGraphFromNarrativeRelease(f.input);
    const shots = f.input.cinematicDirectorPlan.scenes.flatMap(s=>s.shots);
    assert.equal(graph.schemaVersion,3); assert.equal(graph.shots.length,5);
    assert.equal(graph.timeline.totalDurationFrames,324);
    assert.deepEqual(graph.shots.map(s=>s.durationFrames),[48,72,60,90,54]);
    assert.deepEqual(graph.shots.map(s=>s.id),shots.map(s=>s.shotId));
    assert.deepEqual(graph.shots.map(s=>s.camera),shots.map(({shotSize,cameraAngle,cameraMovement,lensIntent})=>({shotSize,cameraAngle,cameraMovement,lensIntent})));
    assert.deepEqual(graph.semanticManifest.entries.map(e=>e.semanticId).sort(),f.map.semantic_manifest.entries.map(e=>e.semantic_id).sort());
    const html=composeSceneHtml({sceneGraph:graph});
    assert.ok(html.includes('이제 알겠어.') && html.includes('정말 끝이야.'));
    assert.match(html,/data-shot-id="shot\.005"/); assert.match(html,/requestAnimationFrame/);
    assert.doesNotMatch(html,/setTimeout\(.*(?:4000|4500)/);
    await importFromNarrativeCourierReleaseBundle(f.options);
    const receipt=JSON.parse(await readFile(join(f.root,'projection/narrative-scene-projection.v3.json')));
    assert.equal(receipt.sceneCount,2); assert.equal(receipt.shotCount,5);
    assert.equal(Object.keys(receipt.sourceArtifacts).length,4);
    assertApprovedSceneProjection(receipt,graph);
  } finally { await rm(f.root,{recursive:true,force:true}); }
});

const cases = [
 ['duplicate shot', i=>{i.cinematicDirectorPlan.scenes[0].shots[1].shotId=i.cinematicDirectorPlan.scenes[0].shots[0].shotId;}],
 ['missing occurrence', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].semanticRefs=[];i.cinematicDirectorPlan.scenes[0].shots[0].dialogueRefs=[];}],
 ['duplicate occurrence', i=>{i.cinematicDirectorPlan.scenes[0].shots[1].semanticRefs.push(i.cinematicDirectorPlan.scenes[0].shots[0].semanticRefs[0]);}],
 ['unknown entity', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].requiredEntities.push('char.unknown');}],
 ['zero duration', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].durationFrames=0;}],
 ['fractional frame', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].durationFrames=1.5;}],
 ['invalid camera', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].cameraAngle='GUESS';}],
 ['camera hidden field', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].implicitCamera=true;}],
 ['timeline sum', i=>{i.cinematicDirectorPlan.scenes[0].sceneDurationFrames++;}],
 ['scene order', i=>{i.cinematicDirectorPlan.scenes.reverse();}],
 ['false continuity fact', i=>{i.continuityBible.entities[0].name='invented';}],
 ['missing character reference', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].referencePolicy.requiresCharacterReference=false;}],
 ['missing approval', i=>{i.directorApproval.decision='CANDIDATE';}],
 ['invalid transition', i=>{i.cinematicDirectorPlan.scenes[0].shots[0].transitionOut.durationFrames=1;}]
];
for (const [name, mutate] of cases) test('rejects '+name+' even when an untrusted caller recomputes hashes',async()=>{
 const f=await writeReleaseFixture({multiShot:true});
 try { const input=structuredClone(f.input);mutate(input);assert.throws(()=>buildSceneGraphFromNarrativeRelease(reseal(input))); }
 finally {await rm(f.root,{recursive:true,force:true});}
});

test('stale plan approval and source drift fail closed',async()=>{
 const f=await writeReleaseFixture({multiShot:true});
 try {
  const i=structuredClone(f.input); i.cinematicDirectorPlan.scenes[0].shots[0].composition='new intent';
  i.cinematicDirectorPlan.contentHash=cinematicContentHash(i.cinematicDirectorPlan);
  assert.throws(()=>buildSceneGraphFromNarrativeRelease(i),/STALE/);
  assert.throws(()=>buildSceneGraphFromNarrativeRelease({...f.input,narrativePackageSha256:'sha256:'+'f'.repeat(64)}),/STALE/);
 }finally{await rm(f.root,{recursive:true,force:true});}
});

test('capsule bytes and reconstruction cannot be replaced by recomputing only receipt or graph hashes',()=>{
 const {sceneGraph,projection}=singleSceneProjectionFixture();
 const bad=structuredClone(projection);
 bad.sourceArtifacts.directorPlan.bytesBase64=Buffer.from('{}').toString('base64url');
 assert.throws(()=>assertApprovedSceneProjection(bad,sceneGraph),/CAPSULE_HASH/);
 const graph=structuredClone(sceneGraph); graph.shots[0].durationFrames++;graph.timeline.totalDurationFrames++;
 assert.throws(()=>assertApprovedSceneProjection(projection,graph));
});

test('actual NVC producer CLI and SMC consumer share hashes, immutable approval retries, and strict drafts',async()=>{
 const f=await writeReleaseFixture({multiShot:true});
 const cli=resolve('../NarrativeCourier/tools/cinematic.py');
 const base=['--project-root',f.root,'--release-bundle','release-bundle.json'];
  const python=process.env.STORY_TEST_PYTHON||process.env.STORY_NVC_TEST_PYTHON||'python3';
  const run=(command,...args)=>JSON.parse(execFileSync(python,[cli,command,...base,...args],{encoding:'utf8',timeout:15000,stdio:['ignore','pipe','pipe']}));
 try {
  const bible=run('continuity','--output','new/bible.json');
  assert.equal(bible.contentHash,f.input.continuityBible.contentHash);
  const {contentHash,...draft}=f.input.cinematicDirectorPlan;
  await writeFile(join(f.root,'draft.json'),JSON.stringify(draft));
  const plan=run('plan','--continuity','new/bible.json','--draft','draft.json','--output','new/plan.json');
  assert.equal(plan.contentHash,contentHash);
  const args=['--continuity','new/bible.json','--plan','new/plan.json','--reviewer','TEST_OPERATOR_NOT_REAL_REVIEW','--expected-plan-hash',contentHash,'--output','new/approval.json'];
  const first=run('approve',...args);const bytes=await readFile(join(f.root,'new/approval.json'));
  assert.deepEqual(run('approve',...args),first);assert.deepEqual(await readFile(join(f.root,'new/approval.json')),bytes);
  assert.equal(run('inspect','--continuity','new/bible.json','--plan','new/plan.json','--approval','new/approval.json').shots,5);
  await importFromNarrativeCourierReleaseBundle({...f.options,continuityBiblePath:'new/bible.json',directorPlanPath:'new/plan.json',directorApprovalPath:'new/approval.json'});
  await writeFile(join(f.root,'draft.json'),JSON.stringify({...draft,unknown:true}));
  assert.throws(()=>run('plan','--continuity','new/bible.json','--draft','draft.json','--output','bad-plan.json'));
 }finally{await rm(f.root,{recursive:true,force:true});}
});
