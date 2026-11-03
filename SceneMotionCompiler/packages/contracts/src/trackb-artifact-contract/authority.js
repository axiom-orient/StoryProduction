import {
  assertResult,
  contractError,
  hashTrackBAuthority,
  hashTrackBMeasuredEvidenceReceipt,
  isTrackBNonEmptyText,
  resultWithErrors
} from '../trackb-contract-primitives.js';
import {
  TRACK_B_NAVIGATION_GUARD_MECHANISM,
  TRACK_B_NAVIGATION_GUARD_SOURCE_HASH
} from '../trackb-navigation-guard.js';
import { validateSchemaValue } from './schema.js';

export function validateTrackBNarrativeAuthority(value, { registry } = {}) {
  const result = validateSchemaValue('trackBNarrativeAuthority', value, registry);
  if (!result.valid) return result;
  const errors = [];
  validateAuthorityTextFields(value, errors);
  if (hashTrackBAuthority(value) !== value.authorityHash) errors.push(contractError('/authorityHash', 'authorityHash does not match its canonical self-hash', 'trackb-authority-invalid'));
  validateAuthorityRelations(value, errors);
  return resultWithErrors(errors);
}

export function assertTrackBNarrativeAuthority(value, options = {}) {
  assertResult(validateTrackBNarrativeAuthority(value, options));
  return value;
}

export function validateTrackBMeasuredEvidenceReceipt(value, { registry } = {}) {
  const result = validateSchemaValue('trackBMeasuredEvidenceReceipt', value, registry);
  if (!result.valid) return result;
  const errors = [];
  if (value.navigationGuard.mechanism !== TRACK_B_NAVIGATION_GUARD_MECHANISM) {
    errors.push(contractError('/navigationGuard/mechanism', 'navigationGuard mechanism does not match the trusted CDP binding mechanism', 'trackb-evidence-incomplete'));
  }
  if (value.navigationGuard.guardSourceHash !== TRACK_B_NAVIGATION_GUARD_SOURCE_HASH) {
    errors.push(contractError('/navigationGuard/guardSourceHash', 'navigationGuard guardSourceHash does not match the trusted deterministic source', 'trackb-evidence-incomplete'));
  }
  if (hashTrackBMeasuredEvidenceReceipt(value) !== value.receiptHash) {
    errors.push(contractError('/receiptHash', 'receiptHash does not match its canonical self-hash', 'trackb-evidence-incomplete'));
  }
  if (value.shotIds.length !== value.shots.length) {
    errors.push(contractError('/shotIds', 'shotIds and shots must have the same length', 'trackb-evidence-incomplete'));
  }
  const seenShotIds = new Set();
  for (let index = 0; index < value.shots.length; index += 1) {
    const shot = value.shots[index];
    if (value.shotIds[index] !== shot.shotId || shot.ordinal !== index) {
      errors.push(contractError(`/shots/${index}`, 'measured evidence shots must be in shotIds order with zero-based ordinals', 'trackb-evidence-incomplete'));
    }
    if (seenShotIds.has(shot.shotId)) errors.push(contractError(`/shots/${index}/shotId`, 'measured evidence shotId must be unique', 'trackb-evidence-incomplete'));
    seenShotIds.add(shot.shotId);
    if (shot.visual.activeShotId !== shot.shotId || shot.visual.visibleShotId !== shot.shotId) {
      errors.push(contractError(`/shots/${index}/visual`, 'measured visual result must identify the requested shot', 'trackb-evidence-incomplete'));
    }
    if (shot.runtime.networkRequestCount !== 0
      || shot.runtime.externalRequestCount !== 0
      || shot.runtime.navigationAttemptCount !== 0
      || shot.runtime.exceptionCount !== 0
      || shot.runtime.semanticManifestVerified !== true
      || shot.runtime.pass !== true) {
      errors.push(contractError(`/shots/${index}/runtime`, 'measured runtime must pass with zero network, navigation, and exception counts', 'trackb-evidence-incomplete'));
    }
    if (shot.visual.activeShotCount !== 1 || shot.visual.visibleShotCount !== 1 || shot.visual.pass !== true) {
      errors.push(contractError(`/shots/${index}/visual`, 'measured visual result must pass with exactly one active and visible shot', 'trackb-evidence-incomplete'));
    }
    if (shot.visual.stageGeometry.width <= 0 || shot.visual.stageGeometry.height <= 0 || shot.visual.shotGeometry.width <= 0 || shot.visual.shotGeometry.height <= 0) {
      errors.push(contractError(`/shots/${index}/visual`, 'measured stage and shot geometry must be nonzero', 'trackb-evidence-incomplete'));
    }
  }
  return resultWithErrors(errors);
}

export function assertTrackBMeasuredEvidenceReceipt(value, options = {}) {
  assertResult(validateTrackBMeasuredEvidenceReceipt(value, options));
  return value;
}

function validateAuthorityRelations(value, errors) {
  const characterIds = uniqueField(value.characters, 'characterId', '/characters', errors);
  const environmentIds = uniqueField(value.environments, 'environmentId', '/environments', errors);
  const shotIds = uniqueField(value.shots, 'shotId', '/shots', errors);
  if (value.shots.some((shot, index) => shot.ordinal !== index)) errors.push(contractError('/shots', 'shot ordinal must equal its zero-based authority array position', 'trackb-candidate-invalid'));
  const meaningIds = uniqueField(value.meaningItems, 'meaningId', '/meaningItems', errors);
  const shotSet = new Set(shotIds);
  const semanticIds = new Set();
  for (let index = 0; index < value.meaningItems.length; index += 1) {
    const item = value.meaningItems[index];
    if (!shotSet.has(item.shotId)) errors.push(contractError(`/meaningItems/${index}/shotId`, 'meaning item references an unknown shot', 'trackb-evidence-incomplete'));
    if (semanticIds.has(item.semanticRef?.semanticId)) errors.push(contractError(`/meaningItems/${index}/semanticRef/semanticId`, 'meaning item semanticRef must be unique', 'trackb-evidence-incomplete'));
    semanticIds.add(item.semanticRef?.semanticId);
  }
  if (value.sourceRefs?.sourceSemanticManifestHash === value.sourceRefs?.semanticManifestHash) {
    errors.push(contractError('/sourceRefs', 'source and normalized semantic manifest hashes must differ', 'trackb-artifact-binding-invalid'));
  }
  // Keep these sets material to the relation pass: duplicate entity ids are an authority defect even when not referenced yet.
  void characterIds; void environmentIds; void meaningIds;
}

function uniqueField(items, field, path, errors) {
  const seen = new Set();
  for (let index = 0; index < items.length; index += 1) {
    const value = items[index][field];
    if (seen.has(value)) errors.push(contractError(`${path}/${index}/${field}`, `${field} must be unique`, 'trackb-input-invalid'));
    seen.add(value);
  }
  return [...seen];
}

function validateAuthorityTextFields(value, errors) {
  for (const [path, text] of [
    ['/story/premise', value.story.premise],
    ['/story/theme', value.story.theme],
    ['/story/setting', value.story.setting]
  ]) if (!isTrackBNonEmptyText(text, 2048)) errors.push(contractError(path, 'text must be non-empty after trimming and at most 2048 characters', 'trackb-input-invalid'));
  for (let index = 0; index < value.characters.length; index += 1) {
    if (!isTrackBNonEmptyText(value.characters[index].name, 2048)) errors.push(contractError(`/characters/${index}/name`, 'name must be non-empty after trimming and at most 2048 characters', 'trackb-input-invalid'));
  }
  for (let index = 0; index < value.environments.length; index += 1) {
    if (!isTrackBNonEmptyText(value.environments[index].name, 2048)) errors.push(contractError(`/environments/${index}/name`, 'name must be non-empty after trimming and at most 2048 characters', 'trackb-input-invalid'));
  }
  for (let index = 0; index < value.meaningItems.length; index += 1) {
    if (!isTrackBNonEmptyText(value.meaningItems[index].sourceText, 16384)) errors.push(contractError(`/meaningItems/${index}/sourceText`, 'sourceText must be non-empty after trimming and at most 16384 characters', 'trackb-input-invalid'));
  }
}
