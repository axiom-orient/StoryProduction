import { deflateRawSync } from 'node:zlib';
import { lstat, open, rename, unlink } from 'node:fs/promises';
import { dirname } from 'node:path';
import { compareCanonicalStrings } from '../../core/src/canonical.js';

const DOS_DATE_1980_01_01 = 0x21;
const DOS_TIME_00_00_00 = 0;
const DEFAULT_MAX_ENTRIES = 10_000;
const DEFAULT_MAX_ENTRY_BYTES = 128 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BYTES = 512 * 1024 * 1024;
const DEFAULT_MAX_ARCHIVE_BYTES = 512 * 1024 * 1024;

export async function writeDeterministicZip(outputPath, entries, limits = {}) {
  if (!Array.isArray(entries)) throw new Error('Zip entries must be an array');
  const maxEntries = positiveLimit(limits.maxEntries, DEFAULT_MAX_ENTRIES, 'maxEntries');
  const maxEntryBytes = positiveLimit(limits.maxEntryBytes, DEFAULT_MAX_ENTRY_BYTES, 'maxEntryBytes');
  const maxTotalBytes = positiveLimit(limits.maxTotalBytes, DEFAULT_MAX_TOTAL_BYTES, 'maxTotalBytes');
  const maxArchiveBytes = positiveLimit(limits.maxArchiveBytes, DEFAULT_MAX_ARCHIVE_BYTES, 'maxArchiveBytes');
  if (entries.length > maxEntries || entries.length > 0xffff) throw new Error(`Zip entry limit exceeded: ${entries.length}`);
  const normalized = entries.map(normalizeEntry).sort((a, b) => compareCanonicalStrings(a.path, b.path));
  if (normalized.some((entry, index) => index > 0 && entry.path === normalized[index - 1].path)) throw new Error('Duplicate zip path');
  let totalBytes = 0;
  for (const entry of normalized) {
    if (entry.bytes.length > maxEntryBytes) throw new Error(`Zip entry byte limit exceeded: ${entry.path}`);
    totalBytes += entry.bytes.length;
    if (totalBytes > maxTotalBytes) throw new Error(`Zip total byte limit exceeded: ${totalBytes}`);
  }
  const localParts = [];
  const centralParts = [];
  let offset = 0;
  for (const entry of normalized) {
    const name = Buffer.from(entry.path, 'utf8');
    if (name.length > 0xffff) throw new Error(`Zip path is too long: ${entry.path}`);
    const compressed = deflateRawSync(entry.bytes, { level: 9 });
    const crc = crc32(entry.bytes);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0); local.writeUInt16LE(20, 4); local.writeUInt16LE(0x0800, 6); local.writeUInt16LE(8, 8);
    local.writeUInt16LE(DOS_TIME_00_00_00, 10); local.writeUInt16LE(DOS_DATE_1980_01_01, 12); local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(compressed.length, 18); local.writeUInt32LE(entry.bytes.length, 22); local.writeUInt16LE(name.length, 26);
    localParts.push(local, name, compressed);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0); central.writeUInt16LE((3 << 8) | 20, 4); central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8); central.writeUInt16LE(8, 10); central.writeUInt16LE(DOS_TIME_00_00_00, 12); central.writeUInt16LE(DOS_DATE_1980_01_01, 14);
    central.writeUInt32LE(crc, 16); central.writeUInt32LE(compressed.length, 20); central.writeUInt32LE(entry.bytes.length, 24); central.writeUInt16LE(name.length, 28);
    central.writeUInt32LE(((0o100000 | entry.mode) << 16) >>> 0, 38); central.writeUInt32LE(offset, 42);
    centralParts.push(central, name);
    offset += local.length + name.length + compressed.length;
  }
  const centralDirectory = Buffer.concat(centralParts);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0); eocd.writeUInt16LE(normalized.length, 8); eocd.writeUInt16LE(normalized.length, 10);
  eocd.writeUInt32LE(centralDirectory.length, 12); eocd.writeUInt32LE(offset, 16);
  const zip = Buffer.concat([...localParts, centralDirectory, eocd]);
  if (zip.length > maxArchiveBytes) throw new Error(`Zip archive byte limit exceeded: ${zip.length}`);
  await atomicWrite(outputPath, zip);
  return zip;
}

async function atomicWrite(outputPath, bytes) {
  const parent = dirname(outputPath);
  let existing = null;
  try { existing = await lstat(outputPath); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  if (existing?.isSymbolicLink() || (existing && !existing.isFile())) throw new Error(`Zip output must be a regular non-symlink file: ${outputPath}`);
  const temp = `${outputPath}.tmp-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  let handle;
  try { handle = await open(temp, 'wx'); await handle.writeFile(bytes); await handle.sync(); await handle.close(); handle = null; await rename(temp, outputPath); await syncDirectory(parent); }
  finally { await handle?.close(); await unlink(temp).catch((error) => { if (error.code !== 'ENOENT') throw error; }); }
}
async function syncDirectory(path) { let handle; try { handle = await open(path, 'r'); await handle.sync(); } catch (error) { if (!['EINVAL', 'EISDIR', 'EPERM'].includes(error.code)) throw error; } finally { await handle?.close(); } }
function positiveLimit(value, fallback, label) { const normalized = value ?? fallback; if (!Number.isSafeInteger(normalized) || normalized < 1) throw new Error(`Invalid zip ${label}: ${normalized}`); return normalized; }
function normalizeEntry(entry) { const path = normalizePath(entry.path); const mode = entry.mode ?? 0o644; if (!Number.isInteger(mode) || mode < 0 || mode > 0o777) throw new Error(`Invalid zip mode for ${path}`); return { path, mode, bytes: Buffer.from(entry.bytes) }; }
function normalizePath(path) { const value = String(path).replaceAll('\\', '/'); if (!value || value.startsWith('/') || value.split('/').some((part) => part === '..' || part === '')) throw new Error(`Unsafe zip path: ${path}`); return value; }
const CRC_TABLE = (() => { const table = new Uint32Array(256); for (let n = 0; n < 256; n += 1) { let c = n; for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; table[n] = c >>> 0; } return table; })();
function crc32(buffer) { let c = 0xffffffff; for (const byte of buffer) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; }
