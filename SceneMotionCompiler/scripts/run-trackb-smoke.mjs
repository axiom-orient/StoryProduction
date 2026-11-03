#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const result = spawnSync(process.execPath, [
  '--test',
  'tests/scene-asset-compiler-asset-boundary.test.js',
  '--test-name-pattern',
  '^public NarrativeCourier projection'
], { encoding: 'utf8', stdio: 'inherit' });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
console.log(JSON.stringify({
  status: 'PASS',
  path: 'NarrativeCourier release → SMC projection → SceneAssetCompiler handoff → SMC HTML'
}, null, 2));
