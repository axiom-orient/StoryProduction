import { createHash } from 'node:crypto';

// Deliberately avoid locale-sensitive collation for persisted, hashed, or
// archived values. JavaScript code-unit comparison is stable across hosts.
export function compareCanonicalStrings(left, right) {
  const a = String(left);
  const b = String(right);
  return a === b ? 0 : a < b ? -1 : 1;
}

export function canonicalize(value) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new Error('Canonical JSON rejects non-finite numbers');
    return Object.is(value, -0) ? 0 : value;
  }
  if (Array.isArray(value)) return value.map((item, index) => {
    if (item === undefined) throw new Error(`Canonical JSON rejects undefined array item at ${index}`);
    return canonicalize(item);
  });
  if (typeof value === 'object') {
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) throw new Error('Canonical JSON accepts only plain objects');
    const output = {};
    for (const key of Object.keys(value).sort(compareCanonicalStrings)) {
      if (value[key] === undefined) throw new Error(`Canonical JSON rejects undefined property: ${key}`);
      output[key] = canonicalize(value[key]);
    }
    return output;
  }
  throw new Error(`Canonical JSON rejects type: ${typeof value}`);
}

export function canonicalizeJson(value) {
  return JSON.stringify(canonicalize(value));
}

/** Return unprefixed lowercase SHA-256 for an already-selected byte sequence. */
export function sha256Hex(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

/** Incrementally hash an async sequence of byte chunks without buffering it. */
export async function sha256HexStream(chunks) {
  const hash = createHash('sha256');
  for await (const chunk of chunks) {
    if (!(chunk instanceof Uint8Array)) throw new TypeError('SHA-256 stream chunks must be bytes');
    hash.update(chunk);
  }
  return hash.digest('hex');
}

/** Return prefixed SHA-256 for bytes, UTF-8 text, or canonical JSON identity. */
export function sha256Prefixed(value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(typeof value === 'string' ? value : canonicalizeJson(value));
  return `sha256:${sha256Hex(bytes)}`;
}

export function canonicalHash(value) {
  return sha256Prefixed(canonicalizeJson(value));
}
