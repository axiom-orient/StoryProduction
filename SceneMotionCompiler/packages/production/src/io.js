// Local-workspace I/O only. This is not an OS sandbox or a provider credential store.
import { constants } from 'node:fs';
import { execFile } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { promisify } from 'node:util';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { lstat, link, mkdir, open, realpath, unlink } from 'node:fs/promises';
import { sha256HexStream, sha256Prefixed } from '../../core/src/canonical.js';
import { parseStrictJson } from '../../core/src/json-codec.js';
import { check, exact, integer } from './production-contract.js';

const POLICY = Object.freeze({
  maxJsonBytes: 8 * 1024 * 1024,
  maxMediaBytes: 2 * 1024 * 1024 * 1024,
  probeTimeoutMs: 60_000,
  probeOutputBytes: 1024 * 1024
});

export async function safePath(root, relative, { existing = true } = {}) {
  check(
    typeof relative === 'string'
      && relative.length > 0
      && !isAbsolute(relative)
      && !relative.includes('\\')
      && !relative.includes('\0')
      && relative.split('/').every((part) => part && part !== '.' && part !== '..'),
    'UNSAFE_PATH'
  );

  const originalRoot = resolve(root);
  const rootMetadata = await lstat(originalRoot);
  check(rootMetadata.isDirectory() && !rootMetadata.isSymbolicLink(), 'ROOT_NOT_DIRECTORY');
  const rootRealPath = await realpath(originalRoot);

  let current = rootRealPath;
  for (const part of relative.split('/')) {
    current = join(current, part);
    const metadata = await lstat(current).catch((error) => {
      if (error.code === 'ENOENT' && !existing) return null;
      throw error;
    });
    if (metadata) check(!metadata.isSymbolicLink(), 'SYMLINK_PATH');
  }

  if (existing) check((await lstat(current)).isFile(), 'FILE_REQUIRED');
  return current;
}

export async function readJson(root, relative) {
  const path = await safePath(root, relative);
  const file = await open(
    path,
    constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0)
  );

  let bytes;
  try {
    const before = await file.stat();
    check(before.isFile() && before.size <= POLICY.maxJsonBytes, 'JSON_TOO_LARGE');

    const buffer = Buffer.alloc(POLICY.maxJsonBytes + 1);
    let total = 0;
    while (total < buffer.length) {
      const { bytesRead } = await file.read(buffer, total, buffer.length - total, null);
      if (!bytesRead) break;
      total += bytesRead;
    }

    check(total <= POLICY.maxJsonBytes, 'JSON_TOO_LARGE');
    const after = await file.stat();
    check(
      total === before.size && after.size === before.size && after.mtimeMs === before.mtimeMs,
      'JSON_CHANGED_DURING_READ'
    );
    bytes = buffer.subarray(0, total);
  } finally {
    await file.close();
  }

  const value = parseStrictJson(bytes);
  check(value && typeof value === 'object' && !Array.isArray(value), 'JSON_OBJECT');
  return { value, sha256: sha256Prefixed(bytes), path: relative };
}

export async function hashFile(root, relative) {
  const path = await safePath(root, relative);
  const file = await open(
    path,
    constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0) | (constants.O_NONBLOCK ?? 0)
  );

  try {
    const before = await file.stat();
    check(before.size > 0 && before.size <= POLICY.maxMediaBytes, 'MEDIA_SIZE');

    const buffer = Buffer.alloc(1024 * 1024);
    let total = 0;
    async function* mediaChunks() {
      for (;;) {
        const { bytesRead } = await file.read(buffer, 0, buffer.length, null);
        if (!bytesRead) return;
        total += bytesRead;
        check(total <= POLICY.maxMediaBytes, 'MEDIA_SIZE');
        yield buffer.subarray(0, bytesRead);
      }
    }
    const digest = await sha256HexStream(mediaChunks());

    const after = await file.stat();
    check(
      total === before.size && after.size === before.size && after.mtimeMs === before.mtimeMs,
      'MEDIA_CHANGED_DURING_READ'
    );
    return `sha256:${digest}`;
  } finally {
    await file.close();
  }
}

export async function verifyMedia(root, ref) {
  exact(ref, 'path sha256', 'media');
  check(/^sha256:[a-f0-9]{64}$/.test(ref.sha256), 'MEDIA_HASH_FORMAT');
  check(await hashFile(root, ref.path) === ref.sha256, 'MEDIA_HASH_STALE');
}

export async function publish(root, relative, value) {
  const raw = Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
  check(raw.length <= POLICY.maxJsonBytes, 'OUTPUT_TOO_LARGE');
  const outputSha256 = sha256Prefixed(raw);

  let target = await safePath(root, relative, { existing: false });
  const existing = await lstat(target).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (existing) {
    check((await readJson(root, relative)).sha256 === outputSha256, 'IMMUTABLE_OUTPUT_CONFLICT');
    return outputSha256;
  }

  await mkdir(dirname(target), { recursive: true });
  target = await safePath(root, relative, { existing: false });
  const temporary = join(dirname(target), `.production-${randomUUID()}`);
  const file = await open(temporary, 'wx', 0o600);
  try {
    await file.writeFile(raw);
    await file.sync();
  } finally {
    await file.close();
  }

  try {
    try {
      await link(temporary, target);
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
      check((await readJson(root, relative)).sha256 === outputSha256, 'IMMUTABLE_OUTPUT_RACE');
    }
    const directory = await open(dirname(target), 'r');
    try {
      await directory.sync();
    } finally {
      await directory.close();
    }
  } finally {
    await unlink(temporary);
  }

  return outputSha256;
}

export async function probeClip(root, media, { signal } = {}) {
  await verifyMedia(root, media);
  check(/\.(mp4|mov)$/i.test(media.path), 'PROBE_SUPPORTS_MP4_MOV_ONLY');

  const path = await safePath(root, media.path);
  signal?.throwIfAborted();
  const env = { ...process.env };
  delete env.FFREPORT; // Read-only probing must not create a report via ambient configuration.

  const { stdout } = await promisify(execFile)(process.env.SMC_FFPROBE_BIN || 'ffprobe', [
    '-v', 'error',
    '-protocol_whitelist', 'file',
    '-f', 'mov',
    '-enable_drefs', '0',
    '-count_frames',
    '-show_streams',
    '-of', 'json',
    path
  ], {
    encoding: 'utf8',
    timeout: POLICY.probeTimeoutMs,
    maxBuffer: POLICY.probeOutputBytes,
    signal,
    env,
    windowsHide: true
  });

  await verifyMedia(root, media);
  const data = JSON.parse(stdout);
  const streams = data.streams ?? [];
  const video = streams.filter((stream) => stream.codec_type === 'video' && stream.disposition?.attached_pic !== 1);
  check(video.length === 1, 'EXACTLY_ONE_VIDEO_STREAM');
  const stream = video[0];
  const rate = /^(\d+)\/(\d+)$/.exec(stream.avg_frame_rate ?? '');
  check(rate, 'UNMEASURED_FPS');

  return {
    width: integer(stream.width, 1, 16_384, 'width'),
    height: integer(stream.height, 1, 16_384, 'height'),
    fpsNumerator: integer(Number(rate[1]), 1, Number.MAX_SAFE_INTEGER, 'fpsNumerator'),
    fpsDenominator: integer(Number(rate[2]), 1, Number.MAX_SAFE_INTEGER, 'fpsDenominator'),
    frameCount: integer(Number(stream.nb_read_frames), 1, Number.MAX_SAFE_INTEGER, 'frameCount'),
    audioStreamCount: integer(streams.filter((item) => item.codec_type === 'audio').length, 0, Number.MAX_SAFE_INTEGER, 'audioStreamCount')
  };
}
