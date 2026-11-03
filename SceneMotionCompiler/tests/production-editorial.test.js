import test from 'node:test';
import assert from 'node:assert/strict';
import { sha256Prefixed } from '../packages/core/src/canonical.js';
import { execFileSync } from 'node:child_process';
import { writeFile,readFile,rm } from 'node:fs/promises';
import {join,resolve} from 'node:path';
import {visualFixture} from './support/visual-reference-fixture.js';
import {prepareMotionRequest,ingestClip,reviewClip,buildEditorialTimeline,exportEditorialOTIO,prepareProductionArtifact} from '../packages/production/src/index.js';

const sourcePaths={projectionReceiptPath:'projection/narrative-scene-projection.v3.json',visualReferencePackPath:'visual/pack.json',visualReferenceApprovalPath:'visual/approval.json'};
async function write(root,path,value){await writeFile(join(root,path),JSON.stringify(value,null,2)+'\n');}
async function fixture(){
 const f=await visualFixture();const pack=f.run('prepare','--draft','reference-draft.json','--out','visual/pack.json');
 f.run('approve','--pack','visual/pack.json','--reviewer','fixture-not-real-approval','--expected-pack-hash',pack.packHash,'--out','visual/approval.json');
 return f;
}

test('real ffprobe / candidate / QC / 5-shot editorial / OTIO wiring, with NOT_RUN distinct from PASS',async()=>{
 const f=await fixture();
 try{
  // Synthetic encoding fixture only. This does not establish generation or visual quality.
  execFileSync(process.env.FFMPEG_BIN||'ffmpeg',['-v','error','-f','lavfi','-i','color=c=black:s=64x64:r=24','-frames:v','120','-c:v','mpeg4','-y',join(f.root,'fixture.mp4')],{timeout:20000,stdio:'pipe'});
  const media={path:'fixture.mp4',sha256:sha256Prefixed(await readFile(join(f.root,'fixture.mp4')))};const clips=[];
  for(let i=0;i<f.graph.shots.length;i++){
   const shot=f.graph.shots[i],requestDraft={kind:'MotionRequestDraft',sources:sourcePaths,shotId:shot.id,width:64,height:64,providerTransferAllowed:false};
   const request=await prepareMotionRequest(f.root,requestDraft);assert.equal(request.durationFrames,shot.durationFrames);assert.deepEqual(request.camera,shot.camera);
   await write(f.root,`motion-${i}.json`,request);
   await write(f.root,`provider-${i}.json`,{schemaVersion:1,kind:'MotionProviderReceipt',requestHash:request.contentHash,providerId:'LOCAL_ENCODING_FIXTURE_NOT_GENERATION',providerJobId:'fixture.'+i,state:'SUCCEEDED',transferred:false,mediaSha256:media.sha256});
   const draft={kind:'ClipIngestDraft',requestPath:`motion-${i}.json`,providerReceiptPath:`provider-${i}.json`,media};
   const candidate=await ingestClip(f.root,draft);assert.equal(candidate.observation.frameCount,120);assert.equal(candidate.observation.fpsNumerator,24);await write(f.root,`candidate-${i}.json`,candidate);
   const blocked=await reviewClip(f.root,{kind:'ClipReviewDraft',candidatePath:`candidate-${i}.json`,reviewEvidencePath:null});
   assert.equal(blocked.verdict,'BLOCKED');assert.equal(blocked.observed.motion,'NOT_RUN');assert.ok(Object.values(blocked.contractChecks).every(Boolean));
   // Simulated review receipt tests binding, NOT a Human/media quality result.
   await write(f.root,`review-${i}.json`,{schemaVersion:1,kind:'ShotReviewEvidence',candidateHash:candidate.contentHash,reviewer:'SIMULATED_OPERATOR_FOR_WIRING_TEST',basis:'OPERATOR_REVIEW',observed:{visualContinuity:'PASS',motion:'PASS',narrative:'PASS'}});
   const qc=await reviewClip(f.root,{kind:'ClipReviewDraft',candidatePath:`candidate-${i}.json`,reviewEvidencePath:`review-${i}.json`});assert.equal(qc.verdict,'PASS');await write(f.root,`qc-${i}.json`,qc);
   clips.push({shotId:shot.id,candidatePath:`candidate-${i}.json`,qcPath:`qc-${i}.json`,sourceStartFrame:0});
  }
  const draft={kind:'EditorialDraft',projectionReceiptPath:sourcePaths.projectionReceiptPath,timelineId:'TEST-2-SCENES-5-SHOTS',clips,audioPlan:[],captions:[{text:'WIRING FIXTURE — NOT GENERATION QUALITY',startFrame:0,durationFrames:24}]};
  const timeline=await buildEditorialTimeline(f.root,draft);assert.equal(timeline.state,'CANDIDATE');assert.equal(timeline.totalDurationFrames,324);assert.deepEqual(timeline.clips.map(c=>c.timelineStartFrame),[0,48,120,180,270]);
  await write(f.root,'timeline.json',timeline);const otio=await exportEditorialOTIO(f.root,{kind:'OTIOExportDraft',timelinePath:'timeline.json'});
  assert.equal(otio.OTIO_SCHEMA,'Timeline.1');assert.equal(otio.tracks.children[0].children.length,5);assert.equal(otio.metadata.StoryProduction.finalApproval,false);assert.match(otio.tracks.children[0].children[0].media_reference.target_url,/^file:\/\//);
  await assert.rejects(()=>buildEditorialTimeline(f.root,{...draft,clips:[...clips].reverse()}),/SHOT_ORDER/);
  const invalid=structuredClone(draft);invalid.clips[0].sourceStartFrame=100;await assert.rejects(()=>buildEditorialTimeline(f.root,invalid),/TRIM_OUT_OF_RANGE/);
  // The public CLI shares the application registry and writes the same candidate contract.
  await write(f.root,'cli-request.json',{kind:'MotionRequestDraft',sources:sourcePaths,shotId:'shot.001',width:64,height:64,providerTransferAllowed:false});
  const result=JSON.parse(execFileSync('node',[resolve('packages/cli/src/main.js'),'production-prepare','--project',f.root,'--request','cli-request.json','--out','motion-cli.json'],{encoding:'utf8',timeout:20000}));
  assert.equal(result.finalApproval,false);assert.equal(result.artifactKind,'ShotMotionRequest');
  assert.deepEqual(await prepareProductionArtifact({projectRoot:f.root,requestPath:'cli-request.json',outputPath:'motion-cli.json'}),Object.fromEntries(Object.entries(result).filter(([k])=>k!=='status')));
  const review=JSON.parse(await readFile(join(f.root,'review-0.json')));review.observed.motion='FAIL';await write(f.root,'review-0.json',review);
  await assert.rejects(()=>buildEditorialTimeline(f.root,draft),/EVIDENCE_CHANGED/);
 }finally{await rm(f.root,{recursive:true,force:true});}
});

test('failed/cancelled/outcome-unknown states never adopt media or receive an implicit retry',async()=>{
 const f=await fixture();
 try{
  const request=await prepareMotionRequest(f.root,{kind:'MotionRequestDraft',sources:sourcePaths,shotId:'shot.001',width:64,height:64,providerTransferAllowed:false});await write(f.root,'motion.json',request);
  for(const state of ['FAILED','CANCELLED','OUTCOME_UNKNOWN']){
   const r={schemaVersion:1,kind:'MotionProviderReceipt',requestHash:request.contentHash,providerId:'test',providerJobId:'test.'+state,state,transferred:false,mediaSha256:null};await write(f.root,'provider.json',r);
   const candidate=await ingestClip(f.root,{kind:'ClipIngestDraft',requestPath:'motion.json',providerReceiptPath:'provider.json',media:null});assert.equal(candidate.state,state);assert.equal(candidate.observation,null);
   await assert.rejects(()=>ingestClip(f.root,{kind:'ClipIngestDraft',requestPath:'motion.json',providerReceiptPath:'provider.json',media:{path:'x.mp4',sha256:'sha256:'+'a'.repeat(64)}}));
   await write(f.root,'candidate.json',candidate);const qc=await reviewClip(f.root,{kind:'ClipReviewDraft',candidatePath:'candidate.json',reviewEvidencePath:null});assert.equal(qc.verdict,'BLOCKED');
   if(['OUTCOME_UNKNOWN','CANCELLED'].includes(state))assert.equal(qc.repairs[0].decision,'BLOCKED');
   r.transferred=true;await write(f.root,'provider.json',r);await assert.rejects(()=>ingestClip(f.root,{kind:'ClipIngestDraft',requestPath:'motion.json',providerReceiptPath:'provider.json',media:null}),/TRANSFER_NOT_ALLOWED/);
  }
 }finally{await rm(f.root,{recursive:true,force:true});}
});
