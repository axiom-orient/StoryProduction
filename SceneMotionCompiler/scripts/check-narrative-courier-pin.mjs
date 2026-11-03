#!/usr/bin/env node
// Verifies the vendored NarrativeCourier producer schema against its recorded pin,
// and — when a local NarrativeCourier checkout is available — against upstream bytes.
// Drift policy: docs/BOUNDARY_NARRATIVECOURIER.md §핵심 규칙 5.
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO_ROOT = fileURLToPath(new URL('../', import.meta.url));
const SCHEMA_PINS = [
  {
    name: 'NarrativePackage v2',
    vendored: join(REPO_ROOT, 'contracts', 'schemas', 'narrative-courier-narrative-package-v2.schema.json'),
    upstream: join('contracts', 'narrative-package.schema.json'),
    hash: '313072cf3cc57e33fa951615b6c12ee76fb842c3537e5ef12b6a196e3f6250a5'
  },
  {
    name: 'NarrativeSceneMap v2',
    vendored: join(REPO_ROOT, 'contracts', 'schemas', 'narrative-scene-map.schema.json'),
    upstream: join('contracts', 'narrative-scene-map-v2.schema.json'),
    hash: 'd18181b216c01a74c93802fee31000a970e8f27b7267fbcd2678af443ed94ff2'
  },
  {
    name: 'NvcReleaseBundle v1',
    vendored: join(REPO_ROOT, 'contracts', 'schemas', 'narrative-release-bundle.schema.json'),
    upstream: join('contracts', 'release-bundle.schema.json'),
    hash: '860dcb26479ae0b0a7ef7e666483f30e9d1a0fc48520399967d3f49cee2a4fc3'
  },
  {name: 'continuity-bible-v1', vendored: join(REPO_ROOT, 'contracts', 'schemas', 'continuity-bible-v1.schema.json'), upstream: join('contracts', 'continuity-bible-v1.schema.json'), hash: '8bbceabdc8d51a5244ef37e7ab7aab5b81129082961fa9fafc938345fa626bf8'},
  {name: 'cinematic-director-plan-v1', vendored: join(REPO_ROOT, 'contracts', 'schemas', 'cinematic-director-plan-v1.schema.json'), upstream: join('contracts', 'cinematic-director-plan-v1.schema.json'), hash: 'f2dda4ad8d890904d5ed3d0395ebd4c3fd8a72ee88e483e1e6282a168639a0ea'},
  {name: 'cinematic-approval-v1', vendored: join(REPO_ROOT, 'contracts', 'schemas', 'cinematic-approval-v1.schema.json'), upstream: join('contracts', 'cinematic-approval-v1.schema.json'), hash: 'f453360a08b799c1d8c7a3dadba1fc3ea363c35942d09d4b4f8c369d1f5b495a'}
];
const CANDIDATE_SIBLINGS = ['../NarrativeCourier', '../../NarrativeCourier'];

function sha256Hex(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

for (const schema of SCHEMA_PINS) {
  const vendoredHash = sha256Hex(await readFile(schema.vendored));
  if (vendoredHash !== schema.hash) {
    console.error(JSON.stringify({
      status: 'FAIL',
      reason: `${schema.name} vendored schema no longer matches recorded upstream pin`,
      expectedPin: schema.hash,
      actual: vendoredHash,
      action: 're-copy from a reviewed NarrativeCourier release tag and update the schema pin'
    }, null, 2));
    process.exit(1);
  }
}

for (const candidate of CANDIDATE_SIBLINGS) {
  try {
    const upstreamHashes = [];
    for (const schema of SCHEMA_PINS) {
      const upstreamHash = sha256Hex(await readFile(resolve(REPO_ROOT, candidate, schema.upstream)));
      upstreamHashes.push(upstreamHash.slice(0, 12));
      if (upstreamHash !== schema.hash) {
        console.error(JSON.stringify({
          status: 'DRIFT',
          reason: `${schema.name} upstream checkout has moved past the vendored pin`,
          checkedPath: resolve(REPO_ROOT, candidate, schema.upstream),
          expectedPin: schema.hash,
          upstreamNow: upstreamHash,
          action: 'review upstream changes, re-copy schema, bump supported checks, update pin'
        }, null, 2));
        process.exit(1);
      }
    }
    console.log(JSON.stringify({ status: 'PASS', vendored: SCHEMA_PINS.map((schema) => schema.hash.slice(0, 12)), upstreamChecked: candidate, upstreamMatchesPin: true, upstreamHashes }));
    process.exit(0);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}

console.log(JSON.stringify({ status: 'PASS', vendored: SCHEMA_PINS.map((schema) => schema.hash.slice(0, 12)), upstreamChecked: null, note: 'no local NarrativeCourier checkout found; pins verified' }));
