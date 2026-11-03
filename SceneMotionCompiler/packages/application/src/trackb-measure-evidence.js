import {
  TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_RECEIPT_BYTES,
  verifyMeasurementArtifactResult
} from '../../store/src/index.js';
import {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  hashTrackBAuthority,
  hashTrackBMeasuredEvidenceReceipt,
  hashTrackBRawBytes,
  isTrackBHash,
  isTrackBSafeRelativePath,
  TRACK_B_PROVENANCE_REQUIRED_KEYS,
  validateTrackBMeasuredEvidenceReceipt,
  validateTrackBNarrativeAuthority
} from '../../contracts/src/trackb-contract.js';
import { canonicalHash, canonicalizeJson } from '../../core/src/canonical.js';
import { mapRuntimeError, runtimeError } from './trackb-measure-support.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';

export const MEASUREMENT_ARTIFACTS = Object.freeze([
  ['narrativeAuthority', 'narrativeAuthorityPath', TRACKB_CUSTODY_MAX_MEASUREMENT_AUTHORITY_BYTES],
  ['html', 'htmlPath', TRACKB_CUSTODY_MAX_MEASUREMENT_HTML_BYTES],
  ['provenance', 'provenancePath', TRACKB_CUSTODY_MAX_MEASUREMENT_PROVENANCE_BYTES]
]);

export async function readMeasurementArtifact(session, kind, path, maximumBytes) {
  if (!isTrackBSafeRelativePath(path)) {
    throw runtimeError('trackb-input-invalid', `${kind} path must be a safe project-relative path`);
  }
  let response;
  try {
    response = await session.readMeasurementArtifact({ kind, path });
    verifyMeasurementArtifactResult(response, { kind, path });
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-storage-integrity', `custody could not read ${kind}`);
  }
  if (response.exists !== true) {
    throw runtimeError('trackb-evidence-incomplete', `${kind} artifact is absent`);
  }
  const bytes = decodeArtifactBytes(response, kind, maximumBytes);
  if (hashTrackBRawBytes(bytes) !== response.sha256) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response hash does not match its bytes`);
  }
  return { response, bytes };
}

function decodeArtifactBytes(response, kind, maximumBytes) {
  if (typeof response.bytesBase64 !== 'string' || !/^[A-Za-z0-9_-]*$/.test(response.bytesBase64)) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response bytesBase64 is invalid`);
  }
  let bytes;
  try {
    bytes = Buffer.from(response.bytesBase64, 'base64url');
  } catch (error) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response bytesBase64 could not be decoded`, { cause: error });
  }
  if (bytes.toString('base64url') !== response.bytesBase64
    || !Number.isSafeInteger(response.byteLength)
    || response.byteLength !== bytes.length
    || bytes.length > maximumBytes) {
    throw runtimeError('trackb-storage-integrity', `${kind} custody response byte length or encoding is invalid`);
  }
  return bytes;
}

export function parseJsonArtifact(bytes, label, code) {
  try {
    return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch (error) {
    throw runtimeError(code, `${label} is not valid UTF-8 JSON`, { cause: error });
  }
}

export function validateAuthority(authority, registry) {
  let validation;
  try {
    validation = validateTrackBNarrativeAuthority(authority, { registry });
  } catch (error) {
    throw mapRuntimeError(error, 'trackb-authority-missing', 'NarrativeAuthority validation failed');
  }
  if (validation?.valid !== true) {
    const first = validation?.errors?.[0];
    throw runtimeError('trackb-authority-missing', `NarrativeAuthority validation failed: ${boundedMessage(first?.message ?? 'invalid authority')}`);
  }
  if (hashTrackBAuthority(authority) !== authority.authorityHash) {
    throw runtimeError('trackb-authority-missing', 'NarrativeAuthority authorityHash is invalid');
  }
}

export function validateMeasurementProvenance(provenance, authority, htmlBytes) {
  if (!isPlainObject(provenance)) throw runtimeError('trackb-evidence-incomplete', 'Track B provenance must be an object');
  const expectedKeys = [...TRACK_B_PROVENANCE_REQUIRED_KEYS].sort();
  const actualKeys = Object.keys(provenance).sort();
  if (actualKeys.length !== expectedKeys.length || actualKeys.some((key, index) => key !== expectedKeys[index]) || Object.hasOwn(provenance, 'authorityHash')) {
    throw runtimeError('trackb-evidence-incomplete', 'Track B provenance v2 fields are invalid for measurement');
  }
  const hashFields = [
    'projectionReceiptHash',
    'sceneGraphHash',
    'sourceSemanticManifestHash',
    'semanticManifestHash',
    'htmlHash',
    'handoffHash',
    'assetManifestHash',
    'provenanceHash'
  ];
  if (hashFields.some((field) => !isTrackBHash(provenance[field]))) {
    throw runtimeError('trackb-evidence-incomplete', 'Track B provenance contains an invalid artifact hash');
  }
  if (provenance.schemaVersion !== 2
    || provenance.narrativeId !== authority.narrativeId
    || provenance.htmlHash !== hashTrackBRawBytes(htmlBytes)
    || provenance.projectionReceiptHash !== authority.sourceRefs?.projectionReceiptHash
    || provenance.sceneGraphHash !== authority.sourceRefs?.sceneGraphHash
    || provenance.semanticManifestHash !== authority.sourceRefs?.semanticManifestHash
    || provenance.sourceSemanticManifestHash !== authority.sourceRefs?.sourceSemanticManifestHash
    || provenance.sourceSemanticManifestHash === provenance.semanticManifestHash) {
    throw runtimeError('trackb-evidence-incomplete', 'Track B provenance is stale or not bound to authority/HTML');
  }
  if (!isPlainObject(provenance.embedding)
    || Object.keys(provenance.embedding).sort().join(',') !== 'anchor,encoding,payloadHash'
    || provenance.embedding.anchor !== TRACKB_SEMANTIC_MANIFEST_ANCHOR
    || provenance.embedding.encoding !== TRACKB_SEMANTIC_MANIFEST_ENCODING
    || !isTrackBHash(provenance.embedding.payloadHash)) {
    throw runtimeError('trackb-evidence-incomplete', 'Track B provenance embedding is invalid');
  }
  const withoutSelf = { ...provenance };
  delete withoutSelf.provenanceHash;
  if (!isTrackBHash(provenance.provenanceHash) || provenance.provenanceHash !== canonicalHash(withoutSelf)) {
    throw runtimeError('trackb-evidence-incomplete', 'Track B provenance self-hash is invalid');
  }
}

export function verifyMeasuredResult(measured, { authority, provenance, htmlBytes, registry }) {
  if (!isPlainObject(measured)) throw runtimeError('trackb-evidence-incomplete', 'measured renderer result must be an object');
  const expectedKeys = ['receipt', 'receiptBytes', 'receiptHash', 'htmlHash', 'rendererLockHash', 'shotIds', 'shotCount'];
  const actualKeys = Object.keys(measured).sort();
  if (actualKeys.length !== expectedKeys.length || actualKeys.some((key, index) => key !== [...expectedKeys].sort()[index])) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer result fields are invalid');
  }
  if (!(Buffer.isBuffer(measured.receiptBytes) || measured.receiptBytes instanceof Uint8Array)) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer receiptBytes must be bytes');
  }
  const receiptBytes = Buffer.from(measured.receiptBytes);
  if (receiptBytes.length > TRACKB_CUSTODY_MAX_MEASUREMENT_RECEIPT_BYTES) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer receipt exceeds the custody receipt bound');
  }
  if (!isPlainObject(measured.receipt)
    || canonicalizeJson(measured.receipt) !== receiptBytes.toString('utf8')
    || !isTrackBHash(measured.receiptHash)
    || measured.receiptHash !== hashTrackBMeasuredEvidenceReceipt(measured.receipt)
    || measured.receipt.receiptHash !== measured.receiptHash) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer receipt bytes or self-hash is invalid');
  }
  const validation = validateTrackBMeasuredEvidenceReceipt(measured.receipt, { registry });
  if (validation?.valid !== true) {
    throw runtimeError('trackb-evidence-incomplete', `measured renderer receipt validation failed: ${boundedMessage(validation?.errors?.[0]?.message ?? 'invalid receipt')}`);
  }
  const expectedShotIds = authority.shots.map((shot) => shot.shotId);
  if (measured.receipt.narrativeId !== authority.narrativeId
    || measured.receipt.authorityHash !== authority.authorityHash
    || measured.receipt.htmlHash !== hashTrackBRawBytes(htmlBytes)
    || measured.receipt.assetManifestHash !== provenance.assetManifestHash
    || measured.receipt.semanticManifestHash !== provenance.semanticManifestHash
    || canonicalizeJson(measured.receipt.shotIds) !== canonicalizeJson(expectedShotIds)) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer receipt is not bound to the custody artifacts');
  }
  if (measured.htmlHash !== measured.receipt.htmlHash
    || measured.rendererLockHash !== measured.receipt.rendererLockHash
    || !isTrackBHash(measured.rendererLockHash)
    || !Array.isArray(measured.shotIds)
    || canonicalizeJson(measured.shotIds) !== canonicalizeJson(expectedShotIds)
    || !Number.isSafeInteger(measured.shotCount)
    || measured.shotCount !== expectedShotIds.length
    || measured.shotCount !== measured.receipt.shots.length) {
    throw runtimeError('trackb-evidence-incomplete', 'measured renderer result is not bound to authority order');
  }
}
