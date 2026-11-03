import { visualFixture } from './support/visual-reference-fixture.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { writeReleaseFixture } from './support/narrative-fixture.js';
import { importFromNarrativeCourierReleaseBundle } from '../packages/adapters/src/narrative-courier-release.js';

const SAC=resolve('../SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py');
// A byte-binding fixture, never an asserted character/keyframe quality result.
const PNG=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64');

test('SAC owns reference pack / five keyframe approval and a rebuildable memory index',async()=>{
 const f=await visualFixture();
 try{
  const pack=f.run('prepare','--draft','reference-draft.json','--out','visual/pack.json');assert.equal(pack.status,'CANDIDATE');
  assert.equal(f.run('inspect','--pack','visual/pack.json').status,'VALID_CANDIDATE');
  assert.throws(()=>f.run('approve','--pack','visual/pack.json','--reviewer','fixture-human','--expected-pack-hash','sha256:'+'f'.repeat(64),'--out','visual/approval.json'));
  const args=['--pack','visual/pack.json','--reviewer','fixture-human','--expected-pack-hash',pack.packHash,'--out','visual/approval.json'];
  const approval=f.run('approve',...args);assert.deepEqual(f.run('approve',...args),approval);
  assert.equal(f.run('inspect','--pack','visual/pack.json','--approval','visual/approval.json').status,'VALID_APPROVED');
  const memory=f.run('index','--pack','visual/pack.json','--approval','visual/approval.json');
  assert.equal(memory.entries.filter(e=>e.view==='keyframe').length,5);assert.equal(memory.packHash,pack.packHash);
  await writeFile(join(f.root,'refs/image.png'),Buffer.concat([PNG,Buffer.from('changed')]));
  assert.throws(()=>f.run('inspect','--pack','visual/pack.json','--approval','visual/approval.json'));
 }finally{await rm(f.root,{recursive:true,force:true});}
});

test('SAC refuses unknown keyframes, omitted references, extra draft fields and path escape',async()=>{
 const f=await visualFixture();
 try{
  for(const mutation of [d=>{d.storyboardFrames[0].shotId='shot.foreign';},d=>{d.storyboardFrames.pop();},d=>{d.entityReferences=[];},d=>{d.autoApprove=true;},d=>{d.storyboardFrames[0].artifact.path='../escape.png';}]){
   const draft=structuredClone(f.draft);mutation(draft);await writeFile(join(f.root,'bad-draft.json'),JSON.stringify(draft));
   assert.throws(()=>f.run('prepare','--draft','bad-draft.json','--out','visual/bad.json'));
  }
 }finally{await rm(f.root,{recursive:true,force:true});}
});
