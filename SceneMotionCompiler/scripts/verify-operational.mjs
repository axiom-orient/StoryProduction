import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';

const root = resolve('.');
const outputDir = join(root, '.tmp', 'operational');
await mkdir(outputDir, { recursive: true });
const gates = [
  { name: 'trackb', script: 'smoke:trackb', timeoutMs: 120_000, summaryPath: null },
  { name: 'cli', script: 'smoke:cli', timeoutMs: 120_000, summaryPath: null },
  { name: 'studio-contract', script: 'smoke:studio:contract', timeoutMs: 120_000, summaryPath: '.tmp/studio-contract/studio-smoke.json' },
  { name: 'release', script: 'smoke:release', timeoutMs: 120_000, summaryPath: null }
];
const results = [];
for (const gate of gates) {
  const started = Date.now();
  const logPath = join(outputDir, `${gate.name}.log`);
  const result = await runIsolatedNpm(gate.script, gate.timeoutMs);
  await writeFile(logPath, result.output);
  const diagnosticNotReady = result.status === 'PASS' && gate.summaryPath && await summaryIsProductNotReady(gate.summaryPath);
  const record = {
    gate: gate.name,
    status: diagnosticNotReady ? 'PRODUCT_NOT_READY' : result.status,
    exitCode: result.exitCode,
    timedOut: result.timedOut,
    durationMs: Date.now() - started,
    logPath: `.tmp/operational/${gate.name}.log`
  };
  results.push(record);
  console.log(`${record.status.padEnd(5)} ${gate.name.padEnd(22)} ${String(Math.round(record.durationMs / 1000)).padStart(4)}s  ${record.logPath}`);
  if (record.status === 'FAIL') {
    console.error(result.output.split(/\r?\n/).slice(-100).join('\n'));
    await writeSummary(results, 'FAIL');
    process.exit(result.exitCode || 1);
  }
}

async function summaryIsProductNotReady(path) {
  const summary = JSON.parse(await readFile(path, 'utf8'));
  return summary.productPass === false || summary.coveragePass === false || summary.coverage?.pass === false || ['PRODUCT_NOT_READY', 'LOCAL_DEMONSTRATION_PARTIAL'].includes(summary.status);
}
const productNotReady = results.some((record) => record.status === 'PRODUCT_NOT_READY');
const finalStatus = productNotReady ? 'PRODUCT_NOT_READY' : 'PASS';
await writeSummary(results, finalStatus);
console.log(JSON.stringify({ status: finalStatus, productPass: false, gateCount: results.length, summary: '.tmp/operational/summary.json' }, null, 2));
if (productNotReady) process.exit(2);

async function writeSummary(results, status) {
  await writeFile(join(outputDir, 'summary.json'), JSON.stringify({ schemaVersion: 1, status, gates: results }, null, 2) + '\n');
}

function runIsolatedNpm(script, timeoutMs) {
  return new Promise((resolveResult) => {
    const command = process.platform === 'win32' ? 'npm.cmd' : 'npm';
    const child = spawn(command, ['run', script], {
      cwd: root,
      env: { ...process.env },
      detached: process.platform !== 'win32',
      stdio: ['ignore', 'pipe', 'pipe']
    });
    const chunks = [];
    child.stdout.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
    child.stderr.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      terminate(child, 'SIGTERM');
      setTimeout(() => terminate(child, 'SIGKILL'), 5_000).unref();
    }, timeoutMs);
    child.once('error', (error) => {
      clearTimeout(timer);
      chunks.push(Buffer.from(`\nspawn error: ${error.stack ?? error.message}\n`));
      resolveResult({ status: 'FAIL', exitCode: 1, timedOut, output: Buffer.concat(chunks).toString('utf8') });
    });
    child.once('exit', (code, signal) => {
      clearTimeout(timer);
      const output = Buffer.concat(chunks).toString('utf8') + (signal ? `\nterminated by ${signal}\n` : '');
      resolveResult({ status: code === 0 && !timedOut ? 'PASS' : 'FAIL', exitCode: code ?? (timedOut ? 124 : 1), timedOut, output });
    });
  });
}

function terminate(child, signal) {
  try {
    if (process.platform !== 'win32' && child.pid) process.kill(-child.pid, signal);
    else child.kill(signal);
  } catch (error) {
    if (error.code !== 'ESRCH') throw error;
  }
}
