// Deterministic semantic-manifest parsing and qualification for Track B evidence.
import { canonicalHash, canonicalizeJson } from '../../core/src/canonical.js';
import {
  hashTrackBMeaningQualification,
  hashTrackBRawBytes,
  isPlainObject,
  isTrackBHash
} from './trackb-contract-primitives.js';

export const TRACKB_SEMANTIC_MANIFEST_ANCHOR = 'trackb-semantic-manifest';
export const TRACKB_SEMANTIC_MANIFEST_ENCODING = 'base64url-rfc4648';

export function deriveTrackBMeaningQualification({ authority, projectionReceipt, projectionReceiptBytes, projectionReceiptHash, sceneGraph, provenance, htmlBytes } = {}) {
  try {
    if (!isPlainObject(authority) || !isPlainObject(projectionReceipt) || !isPlainObject(sceneGraph)
      || !isPlainObject(sceneGraph.semanticManifest) || !isPlainObject(provenance)) {
      return { valid: false, error: 'authority, projection, SceneGraph, and provenance are required for meaning qualification' };
    }
    const embedded = parseTrackBSemanticManifestHtml(htmlBytes);
    if (canonicalizeJson(embedded.semanticManifest) !== canonicalizeJson(sceneGraph.semanticManifest)) {
      return { valid: false, error: 'HTML semantic manifest does not equal SceneGraph semanticManifest' };
    }
    const exactProjectionReceiptHash = projectionReceiptBytes !== undefined
      ? hashTrackBRawBytes(projectionReceiptBytes)
      : projectionReceiptHash;
    if (!isTrackBHash(exactProjectionReceiptHash)) {
      return { valid: false, error: 'projectionReceiptHash must be bound to exact projection receipt bytes' };
    }
    if (projectionReceipt.sourceSemanticManifestHash !== sceneGraph.semanticManifest.sourceSemanticManifestHash
      || projectionReceipt.semanticManifestHash !== sceneGraph.semanticManifest.semanticManifestHash) {
      return { valid: false, error: 'projection semantic manifest hashes do not equal SceneGraph' };
    }
    if (authority.sourceRefs?.projectionReceiptHash !== exactProjectionReceiptHash) {
      return { valid: false, error: 'authority projectionReceiptHash does not match exact projection receipt bytes' };
    }
    const entries = sceneGraph.semanticManifest.entries;
    const bySemanticId = new Map();
    for (const entry of entries) {
      if (bySemanticId.has(entry.semanticId)) return { valid: false, error: `duplicate semantic entry: ${entry.semanticId}` };
      bySemanticId.set(entry.semanticId, entry);
    }
    const qualifiedItems = [];
    const seenMeaningIds = new Set();
    const seenSemanticIds = new Set();
    for (const item of authority.meaningItems ?? []) {
      if (seenMeaningIds.has(item.meaningId)) return { valid: false, error: `duplicate authority meaningId: ${item.meaningId}` };
      seenMeaningIds.add(item.meaningId);
      const semanticId = item.semanticRef?.semanticId;
      const entry = bySemanticId.get(semanticId);
      if (!entry || seenSemanticIds.has(semanticId)) return { valid: false, error: `authority semanticRef is missing or ambiguous: ${semanticId}` };
      seenSemanticIds.add(semanticId);
      if (item.semanticRef.entryHash !== entry.entryHash || item.shotId !== entry.shotId || item.kind !== entry.kind || item.sourceText !== entry.sourceText) {
        return { valid: false, error: `authority and SceneGraph semantic occurrence differ: ${item.meaningId}` };
      }
      qualifiedItems.push({ meaningId: item.meaningId, entryHash: entry.entryHash });
    }
    if (qualifiedItems.length !== entries.length) return { valid: false, error: 'authority meaningItems and SceneGraph semantic entries are not a bijection' };
    const domain = {
      authorityHash: authority.authorityHash,
      projectionReceiptHash: exactProjectionReceiptHash,
      sceneGraphHash: canonicalHash(sceneGraph),
      sourceSemanticManifestHash: sceneGraph.semanticManifest.sourceSemanticManifestHash,
      semanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
      htmlHash: hashTrackBRawBytes(typeof htmlBytes === 'string' ? Buffer.from(htmlBytes, 'utf8') : Buffer.from(htmlBytes)),
      provenanceHash: provenance.provenanceHash,
      qualifiedItems
    };
    const meaningQualificationHash = hashTrackBMeaningQualification(domain);
    return { valid: true, domain, qualifiedItems, meaningQualificationHash, embedding: embedded };
  } catch (error) {
    return { valid: false, error: error.message };
  }
}

export function parseTrackBSemanticManifestHtml(htmlBytes) {
  if (!(typeof htmlBytes === 'string' || Buffer.isBuffer(htmlBytes) || htmlBytes instanceof Uint8Array)) throw new Error('HTML bytes are required');
  const bytes = typeof htmlBytes === 'string' ? Buffer.from(htmlBytes, 'utf8') : Buffer.from(htmlBytes);
  const html = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  assertSingleTrackBAnchorId(html);
  const pattern = new RegExp(
    `<div\\b([^>]*)\\bid=(['"])${TRACKB_SEMANTIC_MANIFEST_ANCHOR}\\2([^>]*)>([\\s\\S]*?)<\\/div\\s*>`,
    'gi'
  );
  const matches = [...html.matchAll(pattern)];
  if (matches.length === 0) throw new Error('semantic manifest marker is missing');
  if (matches.length !== 1) throw new Error('semantic manifest marker is duplicated');
  const [, before, quote, after, body] = matches[0];
  if (quote !== '"' && quote !== "'") throw new Error('semantic manifest marker quote is invalid');
  const attributes = parseHtmlAttributes(`${before}id="${TRACKB_SEMANTIC_MANIFEST_ANCHOR}"${after}`);
  const expectedAttributeKeys = ['aria-hidden', 'encoding', 'hidden', 'id', 'payloadHash', 'semanticManifestHash'];
  if (Object.keys(attributes).sort().join('\0') !== expectedAttributeKeys.join('\0')) throw new Error('semantic manifest marker attributes are not the exact non-executable contract');
  if (attributes.id !== TRACKB_SEMANTIC_MANIFEST_ANCHOR) throw new Error('semantic manifest anchor is invalid');
  if (attributes.hidden !== '' || attributes['aria-hidden'] !== 'true') throw new Error('semantic manifest marker must remain hidden and non-interactive');
  if (attributes.encoding !== TRACKB_SEMANTIC_MANIFEST_ENCODING) throw new Error('semantic manifest encoding is invalid');
  if (!isTrackBHash(attributes.semanticManifestHash) || !isTrackBHash(attributes.payloadHash)) throw new Error('semantic manifest hash attribute is invalid');
  const payload = body.trim();
  if (!payload || payload.length % 4 === 1 || !/^[A-Za-z0-9_-]+$/.test(payload)) throw new Error('semantic manifest payload is invalid base64url');
  const payloadBytes = Buffer.from(payload, 'base64url');
  if (payloadBytes.toString('base64url') !== payload) throw new Error('semantic manifest payload base64 encoding is not canonical');
  const payloadHash = hashTrackBRawBytes(payloadBytes);
  if (payloadHash !== attributes.payloadHash) throw new Error('semantic manifest payload hash drift');
  const payloadText = new TextDecoder('utf-8', { fatal: true }).decode(payloadBytes);
  const manifest = JSON.parse(payloadText);
  if (canonicalizeJson(manifest) !== payloadText) throw new Error('semantic manifest payload JSON is not canonical');
  if (manifest.semanticManifestHash !== attributes.semanticManifestHash) throw new Error('semantic manifest hash attribute drift');
  const integrity = validateTrackBSemanticManifestObject(manifest);
  if (!integrity.valid) throw new Error(integrity.error);
  return {
    anchor: TRACKB_SEMANTIC_MANIFEST_ANCHOR,
    encoding: TRACKB_SEMANTIC_MANIFEST_ENCODING,
    payload,
    payloadBytes,
    payloadHash,
    semanticManifestHash: attributes.semanticManifestHash,
    semanticManifest: manifest
  };
}

function assertSingleTrackBAnchorId(html) {
  const idPattern = /\bid\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))/gi;
  let count = 0;
  let match;
  while ((match = idPattern.exec(html)) !== null) {
    const value = decodeTrackBNumericHtmlEntities(match[1] ?? match[2] ?? match[3] ?? '');
    if (value === TRACKB_SEMANTIC_MANIFEST_ANCHOR) count += 1;
  }
  if (count !== 1) throw new Error(`semantic manifest anchor id must occur exactly once across all elements: found ${count}`);
}

function decodeTrackBNumericHtmlEntities(value) {
  return value.replace(/&#(?:x([0-9a-f]+)|([0-9]+));?/gi, (_match, hex, decimal) => {
    const codePoint = Number.parseInt(hex ?? decimal, hex === undefined ? 10 : 16);
    return Number.isSafeInteger(codePoint) && codePoint >= 0 && codePoint <= 0x10ffff
      ? String.fromCodePoint(codePoint)
      : '\uFFFD';
  });
}

function parseHtmlAttributes(source) {
  const attributes = {};
  const pattern = /([A-Za-z][A-Za-z0-9:_-]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'))?/g;
  let match;
  while ((match = pattern.exec(source)) !== null) {
    const [, key, doubleValue, singleValue] = match;
    if (Object.hasOwn(attributes, key)) throw new Error(`semantic manifest marker attribute is duplicated: ${key}`);
    attributes[key] = doubleValue ?? singleValue ?? '';
  }
  return attributes;
}

export function validateTrackBSemanticManifestObject(manifest) {
  if (!isPlainObject(manifest) || manifest.schemaVersion !== 1 || !Array.isArray(manifest.entries)) return { valid: false, error: 'semantic manifest object is invalid' };
  if (!isTrackBHash(manifest.sourceSemanticManifestHash) || !isTrackBHash(manifest.semanticManifestHash) || manifest.sourceSemanticManifestHash === manifest.semanticManifestHash) return { valid: false, error: 'semantic manifest hash pins are invalid' };
  const withoutSelf = { ...manifest }; delete withoutSelf.semanticManifestHash;
  if (canonicalHash(withoutSelf) !== manifest.semanticManifestHash) return { valid: false, error: 'semantic manifest hash does not match entries' };
  const semanticIds = new Set();
  const occurrenceKeys = new Set();
  for (const entry of manifest.entries) {
    if (!isPlainObject(entry) || !isTrackBHash(entry.entryHash) || typeof entry.semanticId !== 'string' || typeof entry.shotId !== 'string' || typeof entry.sourceSceneId !== 'string' || typeof entry.kind !== 'string' || typeof entry.sourceText !== 'string' || !isPlainObject(entry.sourceLocator)) return { valid: false, error: 'semantic manifest entry is invalid' };
    if (semanticIds.has(entry.semanticId)) return { valid: false, error: `duplicate semanticId: ${entry.semanticId}` };
    semanticIds.add(entry.semanticId);
    const entryWithoutSelf = { ...entry }; delete entryWithoutSelf.entryHash;
    if (canonicalHash(entryWithoutSelf) !== entry.entryHash) return { valid: false, error: `semantic entry hash drift: ${entry.semanticId}` };
    const locator = entry.sourceLocator;
    const occurrenceKey = `${locator.sceneId}|${locator.field}|${locator.occurrenceIndex}`;
    if (occurrenceKeys.has(occurrenceKey)) return { valid: false, error: `duplicate semantic occurrence: ${occurrenceKey}` };
    occurrenceKeys.add(occurrenceKey);
    if (locator.sceneId !== entry.sourceSceneId || locator.field !== entry.kind) return { valid: false, error: `semantic entry locator mismatch: ${entry.semanticId}` };
    if (entry.shotId.length === 0 || entry.sourceText.length === 0) return { valid: false, error: `semantic entry binding is invalid: ${entry.semanticId}` };
  }
  return { valid: true };
}
