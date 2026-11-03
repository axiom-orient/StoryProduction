import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { verifySourceManifest } from '../packages/source-integrity/src/index.js';

const rootDir = resolve(argument('--root') ?? '.');
const manifestPath = resolve(argument('--manifest') ?? 'SOURCE_MANIFEST.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const result = await verifySourceManifest(rootDir, manifest);
console.log(JSON.stringify({ status: result.pass ? 'PASS' : 'FAIL', rootDir, manifestPath, fileCount: result.actual.fileCount, sourceTreeHash: result.actual.sourceTreeHash, mismatches: result.mismatches }, null, 2));
if (!result.pass) process.exitCode = 1;

function argument(name) { const index = process.argv.indexOf(name); return index >= 0 ? process.argv[index + 1] : null; }
