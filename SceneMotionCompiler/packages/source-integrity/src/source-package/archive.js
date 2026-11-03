import { mkdir, mkdtemp, realpath, rename, rm } from 'node:fs/promises';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { sha256Prefixed } from '../../../core/src/canonical.js';
import { writeDeterministicZip } from '../../../export/src/deterministic-zip.js';
import {
  DEFAULT_EXCLUDED_DIRECTORIES,
  DEFAULT_SOURCE_PACKAGE_LIMITS,
  GENERATED_METADATA_FILES,
  assertPackageName,
  resolveLimits
} from './limits.js';
import { buildSourceManifest, collectSourceFiles, verifySourceManifest } from './manifest.js';
import {
  assertAbsentOrRegularFile,
  assertEmptyExtractionDirectory,
  assertNotSymlink,
  readBoundedFile,
  syncDirectory,
  syncFile
} from './filesystem.js';

const ZIP_LOCAL_FILE_HEADER = 0x04034b50;
const ZIP_CENTRAL_DIRECTORY_HEADER = 0x02014b50;
const ZIP_END_OF_CENTRAL_DIRECTORY = 0x06054b50;
const ZIP_EOCD_MINIMUM_BYTES = 22;
const ZIP_MAX_COMMENT_BYTES = 0xffff;

export async function packageSourceTree(input) {
  const rootDir = resolve(input.rootDir);
  const outputPath = resolve(input.outputPath);
  const packageName = input.packageName ?? basename(rootDir);
  const limits = resolveLimits(input.limits);
  assertPackageName(packageName, limits);
  assertOutputOutsideSourceInventory(rootDir, outputPath);
  const files = await collectSourceFiles(rootDir, { excludeGeneratedMetadata: true, limits });
  const manifest = buildSourceManifest(files, { packageName });
  const manifestBytes = Buffer.from(JSON.stringify(manifest, null, 2) + '\n');
  const verifyLocalBytes = Buffer.from(buildVerifyLocal(packageName));
  if (manifestBytes.length + verifyLocalBytes.length > limits.maxMetadataBytes) {
    throw new Error(`Generated package metadata exceeds ${limits.maxMetadataBytes} byte limit`);
  }
  const entries = files.map((file) => ({ path: `${packageName}/${file.path}`, bytes: file.bytes, mode: file.mode }));
  entries.push({ path: `${packageName}/SOURCE_MANIFEST.json`, bytes: manifestBytes, mode: 0o644 });
  entries.push({ path: `${packageName}/VERIFY_LOCAL.md`, bytes: verifyLocalBytes, mode: 0o644 });
  await mkdir(dirname(outputPath), { recursive: true });
  await assertNotSymlink(dirname(outputPath), 'Archive output directory');
  await assertAbsentOrRegularFile(outputPath, 'Archive output');
  const stagingDirectory = await mkdtemp(join(dirname(outputPath), '.smc-source-package-'));
  try {
    const stagedPath = join(stagingDirectory, 'source.zip');
    const zipBytes = await writeDeterministicZip(stagedPath, entries);
    if (zipBytes.length > limits.maxArchiveBytes) throw new Error(`Source archive exceeds ${limits.maxArchiveBytes} byte limit`);
    await syncFile(stagedPath);
    await assertAbsentOrRegularFile(outputPath, 'Archive output');
    await rename(stagedPath, outputPath);
    await syncDirectory(dirname(outputPath));
    return { outputPath, packageHash: sha256Prefixed(zipBytes), manifest, packageName, archiveBytes: zipBytes.length };
  } finally {
    await rm(stagingDirectory, { recursive: true, force: true });
  }
}

export async function verifySourcePackage(input) {
  const limits = resolveLimits(input.limits);
  const zipPath = resolve(input.zipPath);
  await assertNotSymlink(zipPath, 'Source archive');
  const zipBytes = await readBoundedFile(zipPath, limits.maxArchiveBytes, 'Source archive');
  const archive = inspectSourceZip(zipBytes, limits);
  const extractParent = input.extractDir ? resolve(input.extractDir) : await mkdtemp(join(await realpath(tmpdir()), 'smc-source-package-'));
  const cleanup = !input.extractDir;
  try {
    if (input.extractDir) await assertEmptyExtractionDirectory(extractParent);
    await mkdir(extractParent, { recursive: true });
    await assertNotSymlink(extractParent, 'Extraction directory');
    const extracted = spawnSync('unzip', ['-q', zipPath, '-d', extractParent], {
      encoding: 'utf8',
      maxBuffer: 1024 * 1024,
      timeout: 120_000
    });
    if (extracted.error) throw new Error(`Unable to extract source package: ${extracted.error.message}`);
    if (extracted.status !== 0) throw new Error(`Unable to extract source package: ${boundedCommandOutput(extracted)}`);
    const packageName = archive.packageName;
    const rootDir = join(extractParent, packageName);
    await assertNotSymlink(rootDir, 'Extracted source root');
    const manifestPath = join(rootDir, 'SOURCE_MANIFEST.json');
    const manifestBytes = await readBoundedFile(manifestPath, limits.maxMetadataBytes, 'Source manifest');
    const manifest = JSON.parse(manifestBytes.toString('utf8'));
    if (manifest.packageName !== packageName) throw new Error(`Source package root ${packageName} does not match manifest packageName ${manifest.packageName}`);
    const verification = await verifySourceManifest(rootDir, manifest, { limits });
    if (!verification.pass) throw new Error(`Source package manifest mismatch: ${JSON.stringify(verification.mismatches)}`);
    return {
      pass: true,
      zipPath,
      packageHash: sha256Prefixed(zipBytes),
      packageName,
      rootDir,
      extractionWasTemporary: cleanup,
      fileCount: manifest.fileCount,
      sourceTreeHash: manifest.sourceTreeHash,
      manifest,
      archiveBytes: zipBytes.length,
      extractedBytes: archive.totalUncompressedBytes
    };
  } finally {
    if (cleanup) await rm(extractParent, { recursive: true, force: true });
  }
}

function inspectSourceZip(bytes, limits) {
  const eocdStart = Math.max(0, bytes.length - ZIP_EOCD_MINIMUM_BYTES - ZIP_MAX_COMMENT_BYTES);
  const eocdOffset = bytes.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]), bytes.length - ZIP_EOCD_MINIMUM_BYTES);
  if (eocdOffset < eocdStart) throw new Error('Source archive has no valid end-of-central-directory record');
  if (bytes.readUInt32LE(eocdOffset) !== ZIP_END_OF_CENTRAL_DIRECTORY) throw new Error('Source archive has an invalid end-of-central-directory signature');
  const commentLength = bytes.readUInt16LE(eocdOffset + 20);
  if (eocdOffset + ZIP_EOCD_MINIMUM_BYTES + commentLength !== bytes.length) throw new Error('Source archive has trailing or truncated bytes');
  const diskNumber = bytes.readUInt16LE(eocdOffset + 4);
  const centralDisk = bytes.readUInt16LE(eocdOffset + 6);
  const diskEntries = bytes.readUInt16LE(eocdOffset + 8);
  const entryCount = bytes.readUInt16LE(eocdOffset + 10);
  const centralSize = bytes.readUInt32LE(eocdOffset + 12);
  const centralOffset = bytes.readUInt32LE(eocdOffset + 16);
  if (diskNumber !== 0 || centralDisk !== 0 || diskEntries !== entryCount) throw new Error('Multi-disk ZIP archives are forbidden');
  if (entryCount === 0) throw new Error('Source package is empty');
  if (entryCount > limits.maxFiles + GENERATED_METADATA_FILES.size) throw new Error(`Archive entry count exceeds limit ${limits.maxFiles + GENERATED_METADATA_FILES.size}`);
  if (centralOffset + centralSize !== eocdOffset) throw new Error('Source archive central directory bounds are invalid');
  let cursor = centralOffset;
  let totalUncompressedBytes = 0;
  const paths = [];
  const seenPaths = new Set();
  const localRanges = [];
  for (let index = 0; index < entryCount; index += 1) {
    if (cursor + 46 > eocdOffset || bytes.readUInt32LE(cursor) !== ZIP_CENTRAL_DIRECTORY_HEADER) throw new Error('Source archive central directory entry is invalid');
    const versionMadeBy = bytes.readUInt16LE(cursor + 4);
    const flags = bytes.readUInt16LE(cursor + 8);
    const method = bytes.readUInt16LE(cursor + 10);
    const compressedSize = bytes.readUInt32LE(cursor + 20);
    const uncompressedSize = bytes.readUInt32LE(cursor + 24);
    const nameLength = bytes.readUInt16LE(cursor + 28);
    const extraLength = bytes.readUInt16LE(cursor + 30);
    const entryCommentLength = bytes.readUInt16LE(cursor + 32);
    const entryDisk = bytes.readUInt16LE(cursor + 34);
    const externalAttributes = bytes.readUInt32LE(cursor + 38);
    const localOffset = bytes.readUInt32LE(cursor + 42);
    const entryEnd = cursor + 46 + nameLength + extraLength + entryCommentLength;
    if (entryEnd > eocdOffset) throw new Error('Source archive central directory entry exceeds archive bounds');
    if (nameLength === 0 || nameLength > limits.maxPathBytes) throw new Error(`Archive path exceeds ${limits.maxPathBytes} byte limit`);
    const nameBytes = bytes.subarray(cursor + 46, cursor + 46 + nameLength);
    const path = nameBytes.toString('utf8');
    if (!Buffer.from(path, 'utf8').equals(nameBytes)) throw new Error('Archive path is not valid UTF-8');
    assertSafeArchivePath(path);
    if (seenPaths.has(path)) throw new Error(`Duplicate archive path: ${path}`);
    seenPaths.add(path);
    if ((flags & 0x0001) !== 0) throw new Error(`Encrypted archive entries are forbidden: ${path}`);
    if ((flags & 0x0008) !== 0) throw new Error(`Data-descriptor archive entries are forbidden: ${path}`);
    if (method !== 0 && method !== 8) throw new Error(`Unsupported compression method ${method}: ${path}`);
    if (entryDisk !== 0) throw new Error(`Multi-disk archive entry is forbidden: ${path}`);
    const sourcePlatform = versionMadeBy >>> 8;
    const unixType = (externalAttributes >>> 16) & 0o170000;
    if (sourcePlatform === 3 && unixType === 0o120000) throw new Error(`Symlink archive entries are forbidden: ${path}`);
    if (uncompressedSize > limits.maxFileBytes && !path.endsWith('/SOURCE_MANIFEST.json') && !path.endsWith('/VERIFY_LOCAL.md')) {
      throw new Error(`Archive entry exceeds ${limits.maxFileBytes} byte limit: ${path}`);
    }
    totalUncompressedBytes += uncompressedSize;
    if (totalUncompressedBytes > limits.maxTotalBytes + limits.maxMetadataBytes) throw new Error('Archive uncompressed bytes exceed configured limit');
    const localRange = validateLocalZipEntry(bytes, {
      path,
      nameBytes,
      flags,
      method,
      compressedSize,
      uncompressedSize,
      localOffset,
      centralOffset
    });
    localRanges.push(localRange);
    paths.push(path);
    cursor = entryEnd;
  }
  if (cursor !== eocdOffset) throw new Error('Source archive central directory entry count is inconsistent');
  localRanges.sort((a, b) => a.start - b.start);
  for (let index = 1; index < localRanges.length; index += 1) {
    if (localRanges[index].start < localRanges[index - 1].end) throw new Error('Source archive contains overlapping local file records');
  }
  const topLevels = new Set(paths.map((path) => path.split('/')[0]));
  if (topLevels.size !== 1) throw new Error(`Source package must contain exactly one root directory, found ${[...topLevels].join(', ')}`);
  const packageName = [...topLevels][0];
  assertPackageName(packageName, limits);
  if (!seenPaths.has(`${packageName}/SOURCE_MANIFEST.json`)) throw new Error('Source package is missing SOURCE_MANIFEST.json');
  if (!seenPaths.has(`${packageName}/VERIFY_LOCAL.md`)) throw new Error('Source package is missing VERIFY_LOCAL.md');
  return { paths, packageName, totalUncompressedBytes };
}

function validateLocalZipEntry(bytes, entry) {
  if (entry.localOffset + 30 > entry.centralOffset || bytes.readUInt32LE(entry.localOffset) !== ZIP_LOCAL_FILE_HEADER) {
    throw new Error(`Archive local header is invalid: ${entry.path}`);
  }
  const flags = bytes.readUInt16LE(entry.localOffset + 6);
  const method = bytes.readUInt16LE(entry.localOffset + 8);
  const compressedSize = bytes.readUInt32LE(entry.localOffset + 18);
  const uncompressedSize = bytes.readUInt32LE(entry.localOffset + 22);
  const nameLength = bytes.readUInt16LE(entry.localOffset + 26);
  const extraLength = bytes.readUInt16LE(entry.localOffset + 28);
  const nameStart = entry.localOffset + 30;
  const dataStart = nameStart + nameLength + extraLength;
  const dataEnd = dataStart + entry.compressedSize;
  if (flags !== entry.flags || method !== entry.method || compressedSize !== entry.compressedSize || uncompressedSize !== entry.uncompressedSize) {
    throw new Error(`Archive local and central metadata disagree: ${entry.path}`);
  }
  if (!bytes.subarray(nameStart, nameStart + nameLength).equals(entry.nameBytes)) throw new Error(`Archive local and central paths disagree: ${entry.path}`);
  if (dataEnd > entry.centralOffset) throw new Error(`Archive entry data exceeds local-record bounds: ${entry.path}`);
  return { start: entry.localOffset, end: dataEnd };
}

function buildVerifyLocal(packageName) {
  return `# VERIFY_LOCAL\n\nThis archive is verified by content, not by its filename or modification time.\n\n1. Record the archive SHA-256.\n2. From a trusted checkout of the verifier, run \`node scripts/verify-source-package.mjs --zip /absolute/path/to/archive.zip\`.\n3. Run \`npm run verify:source:clean -- --zip /absolute/path/to/archive.zip\`. This creates an OS-temporary extraction, uses a fresh npm cache, installs exactly \`package-lock.json\` with \`npm ci --include=dev --ignore-scripts\`, runs \`npm run verify\`, rechecks the source manifest, and removes the temporary workspace. Registry access can be required when the fresh cache is empty.\n4. If inspecting manually, extract into an empty directory, enter \`${packageName}/\`, run \`node scripts/verify-source-tree.mjs\`, then \`npm ci --include=dev --ignore-scripts --no-audit --no-fund\` and \`npm run verify\`.\n\nDo not infer a test count, syntax-file count, product-readiness verdict, or external-provider result from this guide. Use the commands' current exit codes and generated reports. A successful manifest check proves archive identity only; the isolated install-and-verify command is the clean runtime proof.\n\n\`SOURCE_MANIFEST.json\` excludes itself and \`VERIFY_LOCAL.md\` from the source-tree hash to avoid self-reference. The ZIP hash still binds every archive byte.\n`;
}

function assertSafeArchivePath(path) {
  const value = String(path);
  if (!value || value.includes('\\') || value.includes('\0') || value.startsWith('/') || /^[A-Za-z]:/.test(value) || value.endsWith('/')) throw new Error(`Unsafe archive path: ${path}`);
  if (value.split('/').some((part) => part === '..' || part === '.' || part === '')) throw new Error(`Unsafe archive path: ${path}`);
}

function assertOutputOutsideSourceInventory(rootDir, outputPath) {
  const value = relative(rootDir, outputPath);
  if (!value || isAbsolute(value) || value === '..' || value.startsWith(`..${sep}`)) return;
  const firstPart = value.split(sep)[0];
  if (!DEFAULT_EXCLUDED_DIRECTORIES.has(firstPart)) throw new Error('Archive output must be outside the source inventory or inside an excluded directory');
}
