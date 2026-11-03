import { resolve } from 'node:path';
import { verifyCleanSourcePackage } from '../packages/source-integrity/src/index.js';

const zipPath = argument('--zip');
if (!zipPath) throw new Error('Usage: node scripts/verify-clean-source-package.mjs --zip <archive>');

const result = await verifyCleanSourcePackage({
  zipPath: resolve(zipPath),
  sourceRoot: process.cwd()
});

console.log(JSON.stringify({ status: 'PASS', ...result }, null, 2));

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}
