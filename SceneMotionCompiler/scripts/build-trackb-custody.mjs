import { constants as fsConstants } from 'node:fs';
import { chmod, copyFile, lstat, mkdir, open, readFile, stat } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { TRACKB_CUSTODY_BINARY_NAME } from '../packages/store/src/trackb-custody-protocol.js';

const execFileAsync = promisify(execFile);
const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const targetTriple = argument('--target-triple') ?? await rustcHostTriple();
const outputRoot = resolve(argument('--out') ?? join(rootDir, 'runtime', 'native', targetTriple));
if (!/^[A-Za-z0-9._-]{1,127}$/.test(targetTriple)) throw new Error(`Invalid target triple: ${targetTriple}`);

const cargoArgs = ['build', '--locked', '--release', '--manifest-path', join(rootDir, 'Cargo.toml'), '--package', 'smc-trackb-custody', '--bin', TRACKB_CUSTODY_BINARY_NAME];
if (targetTriple !== await rustcHostTriple()) cargoArgs.push('--target', targetTriple);
await runCommand('cargo', cargoArgs, rootDir);
const builtBinary = targetTriple === await rustcHostTriple()
  ? join(rootDir, 'target', 'release', TRACKB_CUSTODY_BINARY_NAME)
  : join(rootDir, 'target', targetTriple, 'release', TRACKB_CUSTODY_BINARY_NAME);
const builtBytes = await readFile(builtBinary);
await mkdir(outputRoot, { recursive: true });
await assertDirectory(outputRoot);
const outputBinary = join(outputRoot, TRACKB_CUSTODY_BINARY_NAME);
const outputReceipt = join(outputRoot, `${TRACKB_CUSTODY_BINARY_NAME}.receipt.json`);
await assertAbsentOrRegular(outputBinary, 'runtime helper binary');
await assertAbsentOrRegular(outputReceipt, 'runtime helper receipt');
await copyFile(builtBinary, outputBinary);
await chmod(outputBinary, 0o755);
const metadata = await stat(outputBinary);
const binarySha256 = `sha256:${createHash('sha256').update(builtBytes).digest('hex')}`;
const rustVersion = await runCommand('rustc', ['-V'], rootDir);
const receipt = {
  schemaVersion: 1,
  targetTriple,
  binaryName: TRACKB_CUSTODY_BINARY_NAME,
  binarySha256,
  binaryBytes: metadata.size,
  toolVersion: rustVersion.stdout.trim().slice(0, 256),
};
const receiptBytes = Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`, 'utf8');
const handle = await open(outputReceipt, fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_TRUNC, 0o644);
try { await handle.writeFile(receiptBytes); await handle.sync(); } finally { await handle.close(); }
console.log(JSON.stringify({ status: 'PASS', targetTriple, binaryPath: outputBinary, receiptPath: outputReceipt, binarySha256, binaryBytes: metadata.size }, null, 2));

async function rustcHostTriple() {
  const result = await runCommand('rustc', ['-vV'], rootDir);
  const host = result.stdout.split('\n').find((line) => line.startsWith('host:'))?.slice('host:'.length).trim();
  if (!host) throw new Error('rustc -vV did not report a host target triple');
  return host;
}

async function runCommand(command, args, cwd) {
  try { return await execFileAsync(command, args, { cwd, maxBuffer: 2 * 1024 * 1024 }); }
  catch (error) { throw new Error(`${command} ${args.join(' ')} failed: ${error.stderr?.trim() || error.message}`, { cause: error }); }
}

function argument(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : null;
}

async function assertDirectory(path) {
  const metadata = await lstat(path);
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) throw new Error(`runtime output must be a regular non-symlink directory: ${path}`);
}

async function assertAbsentOrRegular(path, label) {
  try {
    const metadata = await lstat(path);
    if (metadata.isSymbolicLink() || !metadata.isFile()) throw new Error(`${label} must be a regular non-symlink file: ${path}`);
  } catch (error) {
    if (error.code !== 'ENOENT') throw error;
  }
}
