import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { access, lstat, mkdir, mkdtemp, readFile, readdir, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import test from 'node:test';
import {
  generateSourceManifest,
  MAX_MANIFEST_BYTES,
  publishSourceManifest,
  serializeSourceManifest,
} from '../scripts/source-manifest.mjs';

const SCRIPT = resolve('scripts/source-manifest.mjs');
const TEMP_MARKER = '.smc-source-manifest-';

function fixtureManifest(extra = {}) {
  return {
    schemaVersion: 1,
    packageName: 'scene-motion-compiler',
    fileCount: 0,
    totalBytes: 0,
    excludedDirectories: ['.build', '.git', '.gjc', '.pytest_cache', '.tmp', 'SceneMotionCompiler Studio.app', '__pycache__', 'node_modules', 'target'],
    generatedMetadataExcludedFromTreeHash: ['SOURCE_MANIFEST.json', 'VERIFY_LOCAL.md'],
    files: [],
    sourceTreeHash: `sha256:${'0'.repeat(64)}`,
    ...extra,
  };
}

async function makeDirectory(prefix) {
  return mkdtemp(join(tmpdir(), prefix));
}

async function temporaryEntries(directory) {
  return (await readdir(directory)).filter((entry) => entry.startsWith(TEMP_MARKER));
}

test('successful manifest publish is parseable, deterministic, and leaves no temporary file', async () => {
  const root = await makeDirectory('smc-source-manifest-source-');
  const outputRoot = await makeDirectory('smc-source-manifest-output-');
  try {
    await writeFile(join(root, 'input.txt'), 'deterministic source\n');
    const first = await generateSourceManifest({ rootDir: root });
    const second = await generateSourceManifest({ rootDir: root });
    assert.deepEqual(second, first);
    assert.equal(serializeSourceManifest(second).toString('utf8'), serializeSourceManifest(first).toString('utf8'));

    const outputPath = join(outputRoot, 'SOURCE_MANIFEST.json');
    const repeatPath = join(outputRoot, 'SOURCE_MANIFEST.repeat.json');
    const publication = await publishSourceManifest(outputPath, first);
    await publishSourceManifest(repeatPath, second);
    assert.equal(publication.outputPath, outputPath);
    assert.equal(publication.bytes, serializeSourceManifest(first).length);
    const firstBytes = await readFile(outputPath);
    const repeatBytes = await readFile(repeatPath);
    assert.deepEqual(JSON.parse(firstBytes), first);
    assert.deepEqual(repeatBytes, firstBytes);
    assert.deepEqual(await temporaryEntries(outputRoot), []);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outputRoot, { recursive: true, force: true });
  }
});

test('missing parent is created and validated before a successful publish', async () => {
  const directory = await makeDirectory('smc-source-manifest-missing-parent-');
  try {
    const parent = join(directory, 'created', 'nested');
    const outputPath = join(parent, 'SOURCE_MANIFEST.json');
    const manifest = fixtureManifest();
    await publishSourceManifest(outputPath, manifest);
    const parentMetadata = await lstat(parent);
    assert.equal(parentMetadata.isDirectory(), true);
    assert.equal(parentMetadata.isSymbolicLink(), false);
    assert.deepEqual(JSON.parse(await readFile(outputPath, 'utf8')), manifest);
    assert.deepEqual(await temporaryEntries(parent), []);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('existing output symlink is rejected and its target bytes remain unchanged', async () => {
  const directory = await makeDirectory('smc-source-manifest-symlink-');
  try {
    const targetPath = join(directory, 'target.json');
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    const original = Buffer.from('trusted prior manifest bytes\n');
    await writeFile(targetPath, original);
    await symlink(targetPath, outputPath);

    await assert.rejects(
      () => publishSourceManifest(outputPath, fixtureManifest()),
      /output must not be a symlink/,
    );
    assert.deepEqual(await readFile(targetPath), original);
    assert.deepEqual(await temporaryEntries(directory), []);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('existing non-regular output and unsafe direct parents are rejected without writes', async () => {
  const directory = await makeDirectory('smc-source-manifest-boundary-');
  try {
    const nonRegularPath = join(directory, 'SOURCE_MANIFEST.json');
    await mkdir(nonRegularPath);
    await assert.rejects(
      () => publishSourceManifest(nonRegularPath, fixtureManifest()),
      /output must be a regular file/,
    );

    const realParent = join(directory, 'real-parent');
    const symlinkParent = join(directory, 'symlink-parent');
    await mkdir(realParent);
    await symlink(realParent, symlinkParent);
    await assert.rejects(
      () => publishSourceManifest(join(symlinkParent, 'SOURCE_MANIFEST.json'), fixtureManifest()),
      /output directory must not be a symlink/,
    );
    assert.deepEqual(await readdir(realParent), []);

    const nonDirectoryParent = join(directory, 'non-directory-parent');
    await writeFile(nonDirectoryParent, 'not a directory\n');
    await assert.rejects(
      () => publishSourceManifest(join(nonDirectoryParent, 'SOURCE_MANIFEST.json'), fixtureManifest()),
      /output directory must be a directory/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('pre-rename failure preserves prior bytes and removes only the owned temporary file', async () => {
  const directory = await makeDirectory('smc-source-manifest-failure-');
  try {
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    const original = Buffer.from('last good manifest bytes\n');
    await writeFile(outputPath, original);
    let temporaryPath = null;
    let failure = null;
    await assert.rejects(
      () => publishSourceManifest(outputPath, fixtureManifest({ sourceTreeHash: `sha256:${'1'.repeat(64)}` }), {
        beforeRename: async (context) => {
          temporaryPath = context.temporaryPath;
          assert.deepEqual(JSON.parse(await readFile(context.temporaryPath, 'utf8')).sourceTreeHash, `sha256:${'1'.repeat(64)}`);
          throw new Error('injected pre-rename interruption');
        },
      }),
      (error) => {
        failure = error;
        return /injected pre-rename interruption/.test(error.message)
          && error.cleanupDiagnostic?.status === 'REMOVED';
      },
    );
    assert.ok(temporaryPath);
    await assert.rejects(() => access(temporaryPath), (error) => error?.code === 'ENOENT');
    assert.deepEqual(await readFile(outputPath), original);
    assert.deepEqual(await temporaryEntries(directory), []);
    assert.equal(failure.cleanupDiagnostic.reason, 'owned-entry');
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('output replacement by a symlink immediately before rename is rejected without following the target', async () => {
  const directory = await makeDirectory('smc-source-manifest-race-');
  try {
    const targetPath = join(directory, 'target.json');
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    const original = Buffer.from('target must not be overwritten\n');
    await writeFile(targetPath, original);
    await writeFile(outputPath, 'prior output\n');

    await assert.rejects(
      () => publishSourceManifest(outputPath, fixtureManifest(), {
        beforeRename: async () => {
          await unlink(outputPath);
          await symlink(targetPath, outputPath);
        },
      }),
      /output must not be a symlink/,
    );
    assert.deepEqual(await readFile(targetPath), original);
    assert.deepEqual(await readFile(outputPath), original);
    assert.deepEqual(await temporaryEntries(directory), []);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('temporary symlink replacement before rename is rejected and its target remains unchanged', async () => {
  const directory = await makeDirectory('smc-source-manifest-temp-symlink-');
  try {
    const targetPath = join(directory, 'target.json');
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    const originalTarget = Buffer.from('temp symlink target must remain unchanged\n');
    const originalOutput = Buffer.from('last good output\n');
    await writeFile(targetPath, originalTarget);
    await writeFile(outputPath, originalOutput);
    let temporaryPath = null;
    let failure = null;

    await assert.rejects(
      () => publishSourceManifest(outputPath, fixtureManifest(), {
        beforeRename: async (context) => {
          temporaryPath = context.temporaryPath;
          await unlink(context.temporaryPath);
          await symlink(targetPath, context.temporaryPath);
        },
      }),
      (error) => {
        failure = error;
        return /temporary output must not be a symlink/.test(error.message)
          && error.cleanupDiagnostic?.status === 'PRESERVED'
          && error.cleanupDiagnostic.reason === 'identity-mismatch';
      },
    );
    assert.deepEqual(await readFile(targetPath), originalTarget);
    assert.deepEqual(await readFile(outputPath), originalOutput);
    assert.ok(temporaryPath);
    const replacementMetadata = await lstat(temporaryPath);
    assert.equal(replacementMetadata.isSymbolicLink(), true);
    assert.deepEqual(await readFile(temporaryPath), originalTarget);
    assert.equal(failure.cleanupDiagnostic.observed.type, 'symlink');
    assert.deepEqual(await temporaryEntries(directory), [basename(temporaryPath)]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('temporary regular-file replacement before rename is rejected and missing output stays missing', async () => {
  const directory = await makeDirectory('smc-source-manifest-temp-replacement-');
  try {
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    let temporaryPath = null;
    let failure = null;

    await assert.rejects(
      () => publishSourceManifest(outputPath, fixtureManifest(), {
        beforeRename: async (context) => {
          temporaryPath = context.temporaryPath;
          await unlink(context.temporaryPath);
          await writeFile(context.temporaryPath, 'different regular file\n');
        },
      }),
      (error) => {
        failure = error;
        return /temporary output was replaced before rename/.test(error.message)
          && error.cleanupDiagnostic?.status === 'PRESERVED'
          && error.cleanupDiagnostic.reason === 'identity-mismatch';
      },
    );
    assert.ok(temporaryPath);
    const replacementMetadata = await lstat(temporaryPath);
    assert.equal(replacementMetadata.isFile(), true);
    assert.equal(replacementMetadata.isSymbolicLink(), false);
    assert.equal(await readFile(temporaryPath, 'utf8'), 'different regular file\n');
    assert.equal(failure.cleanupDiagnostic.observed.type, 'regular-file');
    await assert.rejects(() => lstat(outputPath), (error) => error?.code === 'ENOENT');
    assert.deepEqual(await temporaryEntries(directory), [basename(temporaryPath)]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('oversized payload is rejected before any output or temporary file is created', async () => {
  const directory = await makeDirectory('smc-source-manifest-limit-');
  try {
    const outputPath = join(directory, 'SOURCE_MANIFEST.json');
    const manifest = fixtureManifest({ files: [{ path: 'x'.repeat(256), sha256: `sha256:${'a'.repeat(64)}`, bytes: 1, mode: '0644' }], fileCount: 1 });
    await assert.rejects(
      () => publishSourceManifest(outputPath, manifest, { maximumBytes: 64 }),
      /exceeds 64 byte limit/,
    );
    assert.deepEqual(await temporaryEntries(directory), []);
    await assert.rejects(() => lstat(outputPath), (error) => error?.code === 'ENOENT');
    assert.ok(MAX_MANIFEST_BYTES > 64);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test('CLI keeps its arguments and exits nonzero when output is unsafe', async () => {
  const root = await makeDirectory('smc-source-manifest-cli-');
  const outputRoot = await makeDirectory('smc-source-manifest-cli-output-');
  try {
    const outputTarget = join(outputRoot, 'target.json');
    const outputPath = join(outputRoot, 'SOURCE_MANIFEST.json');
    await writeFile(join(root, 'input.txt'), 'cli source\n');
    await writeFile(outputTarget, 'unchanged\n');
    await symlink(outputTarget, outputPath);
    assert.throws(
      () => execFileSync(process.execPath, [SCRIPT, '--root', root, '--out', outputPath, '--name', 'scene-motion-compiler'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }),
      (error) => error?.status !== 0 && /output must not be a symlink/.test(`${error.stdout ?? ''}${error.stderr ?? ''}`),
    );
    assert.equal(await readFile(outputTarget, 'utf8'), 'unchanged\n');
    assert.deepEqual(await temporaryEntries(root), []);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outputRoot, { recursive: true, force: true });
  }
});
