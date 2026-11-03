import { lstat, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { isAbsolute, join, relative, resolve, sep } from 'node:path';
import { runDoctor, packageSourceTree, verifySourcePackage } from '../packages/source-integrity/src/index.js';
import { sha256Prefixed } from '../packages/core/src/canonical.js';

const repositoryRoot = process.cwd();
const root = resolve(process.argv[2] ?? '.tmp/source-integrity');
assertScopedWorkspace(repositoryRoot, root);
await assertNoSymlinkComponents(resolve(repositoryRoot, '.tmp'), root);
await rm(root, { recursive: true, force: true });
await mkdir(root, { recursive: true });
const doctor = await runDoctor();
if (doctor.status !== 'PASS') throw new Error(`Doctor failed: ${JSON.stringify(doctor.checks)}`);
const firstPath = join(root, 'source-a.zip');
const secondPath = join(root, 'source-b.zip');
const first = await packageSourceTree({ rootDir: process.cwd(), outputPath: firstPath, packageName: 'scene-motion-compiler' });
const second = await packageSourceTree({ rootDir: process.cwd(), outputPath: secondPath, packageName: 'scene-motion-compiler' });
const firstBytes = await readFile(firstPath);
const secondBytes = await readFile(secondPath);
if (!firstBytes.equals(secondBytes)) throw new Error(`Source packages are not byte-deterministic: ${sha256Prefixed(firstBytes)} != ${sha256Prefixed(secondBytes)}`);
const isolatedWorkspace = await mkdtemp(join(await realpath(tmpdir()), 'smc-source-integrity-'));
let verification;
try {
  const extractionDir = join(isolatedWorkspace, 'clean-extract');
  verification = await verifySourcePackage({ zipPath: firstPath, extractDir: extractionDir });
} finally {
  await rm(isolatedWorkspace, { recursive: true, force: true });
}
const summary = {
  schemaVersion: 1,
  status: 'PASS',
  doctor,
  deterministic: true,
  packageHash: first.packageHash,
  sourceTreeHash: first.manifest.sourceTreeHash,
  fileCount: first.manifest.fileCount,
  isolatedManifestExtraction: {
    manifestVerified: verification.pass,
    outsideRepositoryTree: true,
    temporaryWorkspaceRemoved: true
  },
  cleanRuntimeVerification: 'NOT_RUN_USE_verify:source:clean'
};
await writeFile(join(root, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify({ status: 'PASS', root, deterministic: true, isolatedManifestExtraction: true, cleanRuntimeVerification: 'NOT_RUN_USE_verify:source:clean', packageHash: first.packageHash, sourceTreeHash: first.manifest.sourceTreeHash, fileCount: first.manifest.fileCount }, null, 2));

function assertScopedWorkspace(repository, workspace) {
  const allowedRoot = resolve(repository, '.tmp');
  const value = relative(allowedRoot, workspace);
  if (!value || value === '..' || value.startsWith(`..${sep}`) || isAbsolute(value)) {
    throw new Error(`Source-integrity workspace must be a child of ${allowedRoot}`);
  }
}

async function assertNoSymlinkComponents(allowedRoot, workspace) {
  const components = [allowedRoot];
  const child = relative(allowedRoot, workspace);
  let current = allowedRoot;
  for (const part of child.split(sep)) {
    current = join(current, part);
    components.push(current);
  }
  for (const path of components) {
    try {
      if ((await lstat(path)).isSymbolicLink()) throw new Error(`Source-integrity workspace must not use symlinks: ${path}`);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
}
