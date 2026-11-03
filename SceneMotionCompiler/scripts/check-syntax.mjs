import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const roots = ['apps', 'packages', 'scripts', 'tests'];
const files = [];
for (const root of roots) await collect(root);
files.sort();
const failures = [];
for (const file of files) {
  const result = spawnSync(process.execPath, ['--check', file], { encoding: 'utf8' });
  if (result.status !== 0) failures.push({ file, status: result.status, stderr: result.stderr });
}
if (failures.length) {
  console.error(JSON.stringify({ status: 'FAIL', failures }, null, 2));
  process.exit(1);
}
console.log(JSON.stringify({ status: 'PASS', files: files.length }, null, 2));

async function collect(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await collect(path);
    else if (entry.isFile() && (entry.name.endsWith('.js') || entry.name.endsWith('.mjs'))) files.push(path);
  }
}
