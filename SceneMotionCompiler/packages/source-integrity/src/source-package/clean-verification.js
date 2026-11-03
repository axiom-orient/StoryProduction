import { spawnSync } from 'node:child_process';
import { lstat, mkdir, mkdtemp, realpath, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { resolveLimits } from './limits.js';
import { verifySourceManifest } from './manifest.js';
import { verifySourcePackage } from './archive.js';
import { readBoundedFile } from './filesystem.js';

export async function verifyCleanSourcePackage(input) {
  const limits = resolveLimits(input.limits);
  const osTemporaryRoot = await realpath(tmpdir());
  const workspace = await mkdtemp(join(osTemporaryRoot, 'smc-clean-source-'));
  let result;
  try {
    if (input.sourceRoot) {
      const sourceRoot = await realpath(resolve(input.sourceRoot));
      assertSeparateTrees(sourceRoot, workspace);
    }
    const extractionDirectory = join(workspace, 'extract');
    const packageVerification = await verifySourcePackage({
      zipPath: input.zipPath,
      extractDir: extractionDirectory,
      limits
    });
    await assertNoAncestorNodeModules(packageVerification.rootDir);
    const packageJson = JSON.parse((await readBoundedFile(join(packageVerification.rootDir, 'package.json'), limits.maxMetadataBytes, 'package.json')).toString('utf8'));
    const lockfile = JSON.parse((await readBoundedFile(join(packageVerification.rootDir, 'package-lock.json'), limits.maxMetadataBytes, 'package-lock.json')).toString('utf8'));
    if (!Number.isInteger(lockfile.lockfileVersion) || lockfile.lockfileVersion < 2) throw new Error('Clean source verification requires package-lock.json lockfileVersion >= 2');
    if (lockfile.name && packageJson.name && lockfile.name !== packageJson.name) throw new Error('package.json and package-lock.json names do not match');
    const npmCache = join(workspace, 'npm-cache');
    await mkdir(npmCache, { recursive: true });
    const environment = cleanNpmEnvironment(process.env, npmCache);
    const stdio = input.stdio ?? 'inherit';
    const npmExecutable = input.npmExecutable ?? (process.platform === 'win32' ? 'npm.cmd' : 'npm');
    runCleanCommand('locked install', npmExecutable, ['ci', '--include=dev', '--ignore-scripts', '--no-audit', '--no-fund', '--cache', npmCache], {
      cwd: packageVerification.rootDir,
      environment,
      stdio,
      timeout: limits.commandTimeoutMs
    });
    runCleanCommand('repository verification', npmExecutable, ['run', input.verifyScript ?? 'verify'], {
      cwd: packageVerification.rootDir,
      environment,
      stdio,
      timeout: limits.commandTimeoutMs
    });
    const finalManifestVerification = await verifySourceManifest(packageVerification.rootDir, packageVerification.manifest, { limits });
    if (!finalManifestVerification.pass) {
      throw new Error(`Extracted source changed during clean verification: ${JSON.stringify(finalManifestVerification.mismatches)}`);
    }
    result = {
      pass: true,
      packageHash: packageVerification.packageHash,
      sourceTreeHash: packageVerification.sourceTreeHash,
      fileCount: packageVerification.fileCount,
      lockfileVersion: lockfile.lockfileVersion,
      install: 'PASS',
      verify: 'PASS',
      freshNpmCache: true,
      inheritedNodeModules: false,
      workspaceRemoved: false
    };
  } finally {
    await rm(workspace, { recursive: true, force: true });
    if (result) result.workspaceRemoved = true;
  }
  return result;
}

async function assertNoAncestorNodeModules(rootDir) {
  let current = dirname(rootDir);
  for (;;) {
    try {
      const metadata = await lstat(join(current, 'node_modules'));
      if (metadata.isDirectory() || metadata.isSymbolicLink()) throw new Error(`Clean extraction would inherit node_modules from ${current}`);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
}

function assertSeparateTrees(first, second) {
  const firstToSecond = relative(first, second);
  const secondToFirst = relative(second, first);
  const secondInsideFirst = firstToSecond === '' || (!isAbsolute(firstToSecond) && firstToSecond !== '..' && !firstToSecond.startsWith(`..${sep}`));
  const firstInsideSecond = secondToFirst === '' || (!isAbsolute(secondToFirst) && secondToFirst !== '..' && !secondToFirst.startsWith(`..${sep}`));
  if (secondInsideFirst || firstInsideSecond) throw new Error('Clean extraction workspace must be outside the canonical source tree and its ancestry');
}

function cleanNpmEnvironment(source, npmCache) {
  const environment = { ...source };
  const resetNames = [
    'NODE_PATH',
    'npm_config_cache',
    'npm_config_prefix',
    'npm_config_offline',
    'npm_config_prefer_offline',
    'npm_config_prefer_online',
    'npm_config_omit',
    'npm_config_include',
    'npm_config_production',
    'npm_config_ignore_scripts',
    'npm_config_audit',
    'npm_config_fund'
  ];
  const reset = new Set(resetNames.flatMap((name) => [name, name.toUpperCase()]));
  for (const name of Object.keys(environment)) if (reset.has(name)) delete environment[name];
  environment.npm_config_cache = npmCache;
  return environment;
}

function runCleanCommand(stage, executable, args, options) {
  const result = spawnSync(executable, args, {
    cwd: options.cwd,
    env: options.environment,
    stdio: options.stdio,
    encoding: options.stdio === 'pipe' ? 'utf8' : undefined,
    maxBuffer: 8 * 1024 * 1024,
    timeout: options.timeout
  });
  if (result.error || result.status !== 0) {
    const status = result.status === null ? `signal ${result.signal ?? 'unknown'}` : `exit ${result.status}`;
    const detail = result.error?.message ?? boundedCommandOutput(result);
    throw new Error(`Clean source verification ${stage} failed (${status})${detail ? `: ${detail}` : ''}`);
  }
}

function boundedCommandOutput(result) {
  return `${result.stderr ?? ''}${result.stdout ?? ''}`.trim().slice(-4096);
}
