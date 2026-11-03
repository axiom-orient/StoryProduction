import { resolve } from 'node:path';
import { verifySourcePackage } from '../packages/source-integrity/src/index.js';

const zipPath = argument('--zip');
if (!zipPath) throw new Error('Usage: node scripts/verify-source-package.mjs --zip <archive> [--extract <dir>]');
const result = await verifySourcePackage({ zipPath: resolve(zipPath), extractDir: argument('--extract') ? resolve(argument('--extract')) : undefined });
console.log(JSON.stringify({
  status: 'PASS',
  zipPath: result.zipPath,
  packageHash: result.packageHash,
  sourceTreeHash: result.sourceTreeHash,
  fileCount: result.fileCount,
  archiveBytes: result.archiveBytes,
  extractedBytes: result.extractedBytes,
  extraction: result.extractionWasTemporary ? 'temporary-removed' : result.rootDir
}, null, 2));

function argument(name) { const index = process.argv.indexOf(name); return index >= 0 ? process.argv[index + 1] : null; }
