import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { runDoctor } from '../packages/source-integrity/src/index.js';

const outputPath = resolve(argument('--out') ?? '.tmp/doctor.json');
const report = await runDoctor();
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify({ status: report.status, outputPath, checks: report.checks }, null, 2));
if (report.status !== 'PASS') process.exitCode = 1;

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}
