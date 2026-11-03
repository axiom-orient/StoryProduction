import { execFileSync } from 'node:child_process';
import { mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { writeReleaseFixture } from './narrative-fixture.js';
import { importFromNarrativeCourierReleaseBundle } from '../../packages/adapters/src/narrative-courier-release.js';

const SAC=resolve('../SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py');
const PYTHON=process.env.STORY_TEST_PYTHON||process.env.SCENE_ASSET_COMPILER_PYTHON||'python3';
// A byte-binding fixture, never an asserted character/keyframe quality result.
const PNG=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=','base64');
export async function visualFixture(){
 const f=await writeReleaseFixture({multiShot:true});
 await importFromNarrativeCourierReleaseBundle(f.options);
 const projection=JSON.parse(await readFile(join(f.root,'projection/narrative-scene-projection.v3.json')));
 const graph=JSON.parse(await readFile(join(f.root,projection.sceneGraphPath)));
 await mkdir(join(f.root,'refs'));await writeFile(join(f.root,'refs/image.png'),PNG);
 const artifact={path:'refs/image.png',mime:'image/png',sha256:'sha256:'+createHash('sha256').update(PNG).digest('hex')};
 const entityReferences=[['characters','CHARACTER'],['environments','ENVIRONMENT'],['props','PROP']].flatMap(([key,role])=>graph[key].map(e=>({entityId:e.id,role,images:[{view:role==='ENVIRONMENT'?'establishing':'front',artifact}],invariants:['FIXTURE_ONLY_NOT_VISUAL_QUALITY']})));
 const storyboardFrames=graph.shots.map(s=>({shotId:s.id,artifact,referenceEntityIds:[s.environmentId,...s.characterIds,...s.propIds]}));
 const draft={entityReferences,storyboardFrames};await writeFile(join(f.root,'reference-draft.json'),JSON.stringify(draft));
 const run=(command,...args)=>JSON.parse(execFileSync(PYTHON,[SAC,'visual-reference',command,f.root,...args],{encoding:'utf8',timeout:10000,stdio:['ignore','pipe','pipe']}));
 return {...f,projection,graph,draft,run};
}
