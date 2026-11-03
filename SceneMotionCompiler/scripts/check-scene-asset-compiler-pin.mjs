#!/usr/bin/env node
// The SceneAssetCompiler -> SceneMotionCompiler boundary has one producer-owned schema
// source.  SMC vendors the same bytes for offline validation; a pin failure
// must stop the build before a contract can drift silently.
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = fileURLToPath(new URL('../', import.meta.url));
const SCENE_ASSET_COMPILER_ROOT = join(REPO_ROOT, '..', 'SceneAssetCompiler');
const SCHEMA_PINS = [
  ['scene-asset-pack-v2.schema.json', '736d61a59d1c68cf3aaed529b19713d2c7867ab69ef909ef7b8d63390b904765'],
  ['scene-asset-pack-audit-v1.schema.json', '0e3d39a4c89f312f257de7f6988cfe41d1d260c6b851e766d9a0c4fe35af40c0'],
  ['scene-asset-release-approval-v1.schema.json', 'ec0239bc0180cb62f5f53010ab73ee56a74c8e406af2ae413889746cb7292428'],
  ['scene-motion-asset-handoff-v2.schema.json', '67df73b3ea7d764b847c8ca3442fda5769d167aa581198c40f4b7fc1fb768e12'],
  ['binding-manifest-v1.schema.json', '97a1081626333a9a9ee00309bdf3acab0aeb79ecc4d0f76d17d621ae80dea521'],
  ['visual-reference-pack-v1.schema.json', '78efee1b1b943596b448209d2a660860e7dd9273b55df798b416d310558a5296'],
  ['visual-reference-approval-v1.schema.json', 'b3dbe29cb8d8ab88b1ec3b3252ec96e1757e70c3ad786c24e0fc57e3579f4845']
];

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

for (const [name, expected] of SCHEMA_PINS) {
  const producerPath = join(SCENE_ASSET_COMPILER_ROOT, 'docs', 'contracts', name);
  const vendoredPath = join(REPO_ROOT, 'contracts', 'schemas', 'scene-asset-compiler', name);
  const skillPath = join(SCENE_ASSET_COMPILER_ROOT, '.agents', 'skills', 'scene-asset-compiler', 'schemas', name);
  const producer = await readFile(producerPath);
  const vendored = await readFile(vendoredPath);
  const skill = await readFile(skillPath);
  const actual = sha256(producer);
  if (actual !== expected || !producer.equals(vendored) || !producer.equals(skill)) {
    console.error(JSON.stringify({
      status: 'FAIL',
      schema: name,
      expectedPin: expected,
      producerHash: actual,
      vendoredHash: sha256(vendored),
      skillHash: sha256(skill),
      action: 'review the producer schema change, then update every copy and this pin together'
    }, null, 2));
    process.exit(1);
  }
}

console.log(JSON.stringify({
  status: 'PASS',
  schemas: SCHEMA_PINS.map(([name, hash]) => ({ name, sha256: hash }))
}, null, 2));
