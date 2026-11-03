import { resolve } from 'node:path';
import { packageSourceTree } from '../packages/source-integrity/src/index.js';

const rootDir = resolve(argument('--root') ?? '.');
const outputPath = resolve(argument('--out') ?? '.tmp/scene-motion-compiler-source.zip');
const result = await packageSourceTree({
  rootDir,
  outputPath,
  packageName: argument('--name') ?? 'scene-motion-compiler',
  limits: numericLimits()
});
console.log(JSON.stringify({ status: 'PASS', outputPath: result.outputPath, packageHash: result.packageHash, sourceTreeHash: result.manifest.sourceTreeHash, fileCount: result.manifest.fileCount, archiveBytes: result.archiveBytes }, null, 2));

function argument(name) { const index = process.argv.indexOf(name); return index >= 0 ? process.argv[index + 1] : null; }
function numericLimits() {
  const mapping = {
    '--max-files': 'maxFiles',
    '--max-file-bytes': 'maxFileBytes',
    '--max-total-bytes': 'maxTotalBytes',
    '--max-archive-bytes': 'maxArchiveBytes'
  };
  return Object.fromEntries(Object.entries(mapping).flatMap(([flag, key]) => {
    const value = argument(flag);
    if (value === null) return [];
    if (!/^\d+$/.test(value) || Number(value) <= 0) throw new Error(`${flag} must be a positive integer`);
    return [[key, Number(value)]];
  }));
}
