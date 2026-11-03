import { canonicalHash } from '../../core/src/canonical.js';
import { ApplicationCommandError } from './command-error.js';

export const SOURCE_KIND = 'SMC_NARRATIVE_SCENE_PROJECTION';
export const PROJECTION_RECEIPT_PATH = 'projection/narrative-scene-projection.v3.json';
export const HANDOFF_PATH = 'handoff/scene-motion-asset-handoff.v2.json';

const REQUIREMENT_TYPES = new Set(['CHARACTER_SPRITE', 'ENVIRONMENT_PLATE', 'PROP_CUTOUT', 'SHOT_KEYFRAME']);
const ASSET_KINDS = new Set(['illustration', 'photo', 'texture', 'product', 'icon', 'vector-decoration']);
const RIGHTS_STATUSES = new Set(['USER_OWNED', 'LICENSED', 'PUBLIC_DOMAIN', 'CC0']);

export function expectedRequirementMap(sceneGraph) {
  const result = [];
  for (const [key, requirementType] of [
    ['characters', 'CHARACTER_SPRITE'],
    ['environments', 'ENVIRONMENT_PLATE'],
    ['props', 'PROP_CUTOUT'],
    ['shots', 'SHOT_KEYFRAME']
  ]) {
    for (const entity of sceneGraph[key] ?? []) {
      result.push({
        requirementId: `${requirementType}:${entity.id}`,
        requirementType,
        sceneEntityId: entity.id
      });
    }
  }
  return new Map(result.map((item) => [item.requirementId, item]));
}

export function validateCoverage(assetPack, binding, expected) {
  const packRequirements = uniqueRequirements(assetPack.requirements, 'assetPack.requirements');
  const bindingRequirements = uniqueRequirements(binding.requirements, 'bindingManifest.requirements');
  if (packRequirements.length !== expected.size || bindingRequirements.length !== expected.size) {
    throw new ApplicationCommandError('scene-asset-compiler-requirements-incomplete', 'all four requirement types must be covered exactly once');
  }
  const byId = new Map(packRequirements.map((item) => [item.requirementId, item]));
  const bindingById = new Map(bindingRequirements.map((item) => [item.requirementId, item]));
  for (const [id, expectedItem] of expected) {
    const item = byId.get(id);
    const bound = bindingById.get(id);
    if (!item || !bound || item.requirementType !== expectedItem.requirementType || item.sceneEntityId !== expectedItem.sceneEntityId || !sameValue(item, bound)) {
      throw new ApplicationCommandError('scene-asset-compiler-requirements-incomplete', `missing or mismatched requirement: ${id}`);
    }
    validateRequirementMetadata(item, id);
  }
  const assetIds = new Set(assetPack.assets.map((asset) => asset.assetId));
  for (const item of packRequirements) if (!assetIds.has(item.assetId)) throw new ApplicationCommandError('scene-asset-compiler-asset-missing', `requirement references unknown asset: ${item.requirementId}`);
  return [...byId.values()].sort((left, right) => left.requirementId.localeCompare(right.requirementId));
}

export function validateAudit(audit, assetPack, requirements, assetsById) {
  const assetIds = assetPack.assets.map((asset) => asset.assetId);
  const requirementIds = requirements.map((requirement) => requirement.requirementId);
  if (!sameSet(audit.assetIds, assetIds) || !sameSet(audit.requirementIds, requirementIds)) {
    throw new ApplicationCommandError('scene-asset-compiler-audit-coverage-invalid', 'asset pack audit coverage does not match verified assets and requirements');
  }

  const expected = new Map();
  for (const assetId of assetIds) {
    const verified = assetsById.get(assetId);
    expected.set(`source:${assetId}`, {
      targetKind: 'SOURCE_ARTIFACT',
      targetId: assetId,
      evidence: [`sha256:${verified.source.sha256}`]
    });
    expected.set(`runtime:${assetId}`, {
      targetKind: 'RUNTIME_DERIVATIVE',
      targetId: assetId,
      evidence: [`sha256:${verified.runtime.sha256}`, verified.runtime.mime]
    });
  }
  for (const requirement of requirements) {
    expected.set(`requirement:${requirement.requirementId}`, {
      targetKind: 'REQUIREMENT',
      targetId: requirement.requirementId,
      evidence: [`asset:${requirement.assetId}`]
    });
  }

  if (!Array.isArray(audit.checks) || audit.checks.length !== expected.size) {
    throw new ApplicationCommandError('scene-asset-compiler-audit-checks-invalid', 'asset pack audit checks are incomplete');
  }
  const checks = new Map();
  for (const check of audit.checks) {
    if (checks.has(check.checkId)) throw new ApplicationCommandError('scene-asset-compiler-audit-checks-invalid', `duplicate audit checkId: ${check.checkId}`);
    checks.set(check.checkId, check);
  }
  for (const [checkId, expectedCheck] of expected) {
    const check = checks.get(checkId);
    if (!check || check.status !== 'PASS' || check.targetKind !== expectedCheck.targetKind || check.targetId !== expectedCheck.targetId || !Array.isArray(check.evidence) || !expectedCheck.evidence.every((evidence) => check.evidence.includes(evidence))) {
      throw new ApplicationCommandError('scene-asset-compiler-audit-check-invalid', `asset pack audit check is not bound to verified bytes: ${checkId}`);
    }
  }
}

export function validateAssetMetadata(asset, label) {
  if (!ASSET_KINDS.has(asset.kind)) throw new ApplicationCommandError('scene-asset-compiler-asset-kind-invalid', `${label}.kind is not a producer-declared kind`);
  if (!asset.sourceRegionId || typeof asset.sourceRegionId !== 'string') throw new ApplicationCommandError('scene-asset-compiler-source-region-invalid', `${label}.sourceRegionId is required`);
  validateRights(asset.rights, `${label}.rights`);
}

export function assertRawHash(expected, actual, label) {
  if (expected !== actual) throw new ApplicationCommandError('scene-asset-compiler-handoff-integrity-failed', `${label} raw SHA-256 does not match handoff`, { details: { expected, actual } });
}

export function stripSha256(value) {
  return typeof value === 'string' && value.startsWith('sha256:') ? value.slice('sha256:'.length) : value;
}

export function sameValue(left, right) {
  return canonicalHash(left) === canonicalHash(right);
}

function validateRequirementMetadata(item, label) {
  if (!REQUIREMENT_TYPES.has(item.requirementType)) throw new ApplicationCommandError('scene-asset-compiler-requirement-type-invalid', `${label}.requirementType is unsupported`);
  validateAssetMetadata(item, label);
  if (typeof item.allowedUse !== 'string' || item.allowedUse.trim().length === 0) throw new ApplicationCommandError('scene-asset-compiler-allowed-use-invalid', `${label}.allowedUse is required`);
  if (!item.crop || typeof item.crop !== 'object' || item.crop.width <= 0 || item.crop.height <= 0) throw new ApplicationCommandError('scene-asset-compiler-crop-invalid', `${label}.crop is invalid`);
}

function validateRights(rights, label) {
  if (!rights || typeof rights !== 'object' || Array.isArray(rights) || !RIGHTS_STATUSES.has(rights.status) || typeof rights.holder !== 'string' || !rights.holder.trim() || typeof rights.source !== 'string' || !rights.source.trim()) {
    throw new ApplicationCommandError('scene-asset-compiler-rights-invalid', `${label} must contain a declared status, holder, and source`);
  }
}

function uniqueRequirements(items, label) {
  if (!Array.isArray(items) || items.length === 0) throw new ApplicationCommandError('scene-asset-compiler-requirements-incomplete', `${label} must be non-empty`);
  const seen = new Set();
  for (const item of items) {
    if (!item || typeof item !== 'object' || typeof item.requirementId !== 'string') throw new ApplicationCommandError('scene-asset-compiler-requirement-invalid', `${label} contains an invalid requirement`);
    if (seen.has(item.requirementId)) throw new ApplicationCommandError('scene-asset-compiler-requirement-duplicate', `duplicate requirementId: ${item.requirementId}`);
    seen.add(item.requirementId);
  }
  return items;
}

function sameSet(left, right) {
  return Array.isArray(left) && Array.isArray(right) && left.length === right.length && new Set(left).size === left.length && new Set(right).size === right.length && left.every((value) => new Set(right).has(value));
}
