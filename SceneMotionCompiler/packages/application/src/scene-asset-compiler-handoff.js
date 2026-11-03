import { canonicalHash } from '../../core/src/canonical.js';
import { loadNarrativeSceneProjection } from '../../adapters/src/narrative-courier-release.js';
import { ApplicationCommandError } from './command-error.js';
import {
  assertRawHash,
  expectedRequirementMap,
  HANDOFF_PATH,
  PROJECTION_RECEIPT_PATH,
  sameValue,
  SOURCE_KIND,
  stripSha256,
  validateAssetMetadata,
  validateAudit,
  validateCoverage
} from './scene-asset-compiler-handoff-contract.js';
import {
  readBoundArtifact,
  readJson,
  resolveProjectRoot,
  safeRelativePath,
  validateGeneratedAssetManifest
} from './scene-asset-compiler-handoff-files.js';

/**
 * Load and verify one producer-published SceneMotionAssetHandoff.
 *
 * The receipt is the only input pointer. Projection, graph, pack, audit,
 * binding and approval bytes are independently hash-checked before any image
 * is exposed to the HTML composer. This adapter never reparses screenplay or
 * synopsis Markdown and never invents rights or source regions. The returned
 * handoff object, raw bytes, and `handoffHash` are the one verified tuple that
 * downstream producers must consume.
 */
export async function loadVerifiedSceneAssetCompilerHandoff({ projectRoot, handoffPath }) {
  const root = await resolveProjectRoot(projectRoot);
  const handoffRelative = safeRelativePath(handoffPath, 'sceneMotionAssetHandoffPath');
  if (handoffRelative !== HANDOFF_PATH) {
    throw new ApplicationCommandError('scene-asset-compiler-handoff-path-invalid', `sceneMotionAssetHandoffPath must be ${HANDOFF_PATH}`);
  }

  const handoffArtifact = await readJson(root, handoffRelative, 'scene-motion-asset-handoff-v2.schema.json');
  const handoff = handoffArtifact.value;
  if (handoff.sourceKind !== SOURCE_KIND) {
    throw new ApplicationCommandError('scene-asset-compiler-source-kind-invalid', `unsupported SceneAssetCompiler sourceKind: ${handoff.sourceKind}`);
  }
  const handoffIdentity = { ...handoff };
  delete handoffIdentity.handoffId;
  const expectedHandoffId = 'SAC-SMC-' + canonicalHash(handoffIdentity).slice('sha256:'.length);
  if (handoff.handoffId !== expectedHandoffId) {
    throw new ApplicationCommandError('scene-asset-compiler-handoff-id-mismatch', 'handoffId does not hash the complete handoff body', {
      details: { expected: expectedHandoffId, actual: handoff.handoffId }
    });
  }

  const projection = await loadNarrativeSceneProjection({
    projectRoot: root,
    projectionReceiptPath: PROJECTION_RECEIPT_PATH
  });
  const projectionReceiptSha256 = stripSha256(projection.projectionReceiptHash);
  const projectionSemanticManifest = projection.sceneGraph.semanticManifest;
  if (projectionReceiptSha256 !== handoff.projectionReceiptSha256 ||
      projection.sceneGraphHash !== handoff.sceneGraphHash ||
      projectionSemanticManifest.sourceSemanticManifestHash !== handoff.sourceSemanticManifestHash ||
      projectionSemanticManifest.semanticManifestHash !== handoff.semanticManifestHash ||
      projection.projectionReceipt.sourceSemanticManifestHash !== handoff.sourceSemanticManifestHash ||
      projection.projectionReceipt.semanticManifestHash !== handoff.semanticManifestHash) {
    throw new ApplicationCommandError('scene-asset-compiler-projection-stale', 'SceneAssetCompiler handoff does not reference the current durable NVC projection', {
      details: {
        expectedReceiptSha256: projectionReceiptSha256,
        receivedReceiptSha256: handoff.projectionReceiptSha256,
        expectedSceneGraphHash: projection.sceneGraphHash,
        receivedSceneGraphHash: handoff.sceneGraphHash,
        expectedSourceSemanticManifestHash: projectionSemanticManifest.sourceSemanticManifestHash,
        receivedSourceSemanticManifestHash: handoff.sourceSemanticManifestHash,
        expectedSemanticManifestHash: projectionSemanticManifest.semanticManifestHash,
        receivedSemanticManifestHash: handoff.semanticManifestHash
      }
    });
  }

  const packArtifact = await readJson(root, handoff.assetPackPath, 'scene-asset-pack-v2.schema.json');
  const auditArtifact = await readJson(root, handoff.assetPackAuditPath, 'scene-asset-pack-audit-v1.schema.json');
  const bindingArtifact = await readJson(root, handoff.bindingManifestPath, 'binding-manifest-v1.schema.json');
  const approvalArtifact = await readJson(root, handoff.approvalPath, 'scene-asset-release-approval-v1.schema.json');
  assertRawHash(handoff.assetPackSha256, packArtifact.sha256, 'assetPack');
  assertRawHash(handoff.assetPackAuditSha256, auditArtifact.sha256, 'assetPackAudit');
  assertRawHash(handoff.bindingManifestSha256, bindingArtifact.sha256, 'bindingManifest');
  assertRawHash(handoff.approvalSha256, approvalArtifact.sha256, 'approval');

  const assetPack = packArtifact.value;
  const audit = auditArtifact.value;
  const binding = bindingArtifact.value;
  const approval = approvalArtifact.value;
  const projectionRef = {
    receiptSha256: projectionReceiptSha256,
    sceneGraphHash: projection.sceneGraphHash,
    sceneCount: projection.projectionReceipt.sceneCount,
    sceneIds: projection.projectionReceipt.sceneIds
  };

  for (const [label, value] of [['assetPack', assetPack], ['audit', audit], ['binding', binding]]) {
    if (!sameValue(value.projection, projectionRef)) {
      throw new ApplicationCommandError('scene-asset-compiler-projection-binding-mismatch', `${label} is not bound to the current NVC projection`);
    }
  }
  if (canonicalHash(assetPack) !== handoff.assetPackHash || canonicalHash(assetPack) !== audit.assetPackHash) {
    throw new ApplicationCommandError('scene-asset-compiler-asset-pack-stale', 'SceneAssetPack hash does not match the handoff and audit');
  }
  if (canonicalHash(binding) !== handoff.bindingManifestHash || canonicalHash(binding) !== audit.bindingManifestHash) {
    throw new ApplicationCommandError('scene-asset-compiler-binding-stale', 'binding manifest hash does not match the handoff and audit');
  }

  const subject = {
    sceneGraphHash: handoff.sceneGraphHash,
    assetPackHash: handoff.assetPackHash,
    assetPackAuditHash: handoff.assetPackAuditHash,
    bindingManifestHash: handoff.bindingManifestHash
  };
  if (!sameValue(approval.subject, subject) || approval.subjectHash !== canonicalHash(subject) || handoff.approvalSubjectHash !== approval.subjectHash) {
    throw new ApplicationCommandError('scene-asset-compiler-approval-stale', 'human approval subject is not exactly bound to the published artifacts');
  }
  if (approval.status !== 'APPROVED' || typeof approval.reviewer !== 'string' || approval.reviewer.trim().length === 0) {
    throw new ApplicationCommandError('scene-asset-compiler-approval-invalid', 'SceneAssetReleaseApproval must be APPROVED with a reviewer');
  }

  const expectedRequirements = expectedRequirementMap(projection.sceneGraph);
  const requirements = validateCoverage(assetPack, binding, expectedRequirements);
  const assetsById = new Map();
  for (const asset of assetPack.assets) {
    if (assetsById.has(asset.assetId)) throw new ApplicationCommandError('scene-asset-compiler-asset-duplicate', `duplicate assetId: ${asset.assetId}`);
    validateAssetMetadata(asset, `asset ${asset.assetId}`);
    const source = await readBoundArtifact(root, asset.sourceArtifact, `asset ${asset.assetId}.sourceArtifact`, { runtime: false });
    const runtime = await readBoundArtifact(root, asset.runtimeDerivative, `asset ${asset.assetId}.runtimeDerivative`, { runtime: true });
    if (source.sha256 === runtime.sha256 || source.path === runtime.path) {
      throw new ApplicationCommandError('scene-asset-compiler-source-runtime-alias', `source and runtime derivative must be distinct: ${asset.assetId}`);
    }
    assetsById.set(asset.assetId, { asset, source, runtime });
  }

  validateAudit(audit, assetPack, requirements, assetsById);

  const assets = {};
  const manifestAssets = [];
  for (const requirement of requirements) {
    const verified = assetsById.get(requirement.assetId);
    if (!verified) throw new ApplicationCommandError('scene-asset-compiler-asset-missing', `requirement references unknown asset: ${requirement.assetId}`);
    const { asset, runtime } = verified;
    const manifestAsset = {
      assetId: requirement.sceneEntityId,
      sourceRegionId: requirement.sourceRegionId,
      objectHash: `sha256:${runtime.sha256}`,
      kind: requirement.kind,
      rights: requirement.rights,
      allowedUse: requirement.allowedUse,
      crop: requirement.crop
    };
    manifestAssets.push(manifestAsset);
    if (requirement.requirementType !== 'SHOT_KEYFRAME') {
      assets[requirement.sceneEntityId] = `data:${runtime.mime};base64,${runtime.bytes.toString('base64')}`;
    }
    // SMC cannot infer rights, source region, or semantic kind from a path or extension.
    if (asset.kind !== requirement.kind || !sameValue(asset.rights, requirement.rights) || asset.sourceRegionId !== requirement.sourceRegionId) {
      throw new ApplicationCommandError('scene-asset-compiler-asset-metadata-mismatch', `asset metadata differs from binding: ${requirement.requirementId}`);
    }
  }

  const assetManifest = {
    schemaVersion: 1,
    assetManifestId: handoff.handoffId,
    approvalSubjectHash: handoff.approvalSubjectHash,
    assets: manifestAssets
  };
  await validateGeneratedAssetManifest(assetManifest);

  return {
    handoff,
    handoffBytes: handoffArtifact.bytes,
    handoffHash: `sha256:${handoffArtifact.sha256}`,
    sceneGraph: projection.sceneGraph,
    projection,
    assets,
    assetManifest,
    // The verified read set is also the write-protection boundary for composers.
    inputPaths: [
      handoffRelative, projection.projectionReceiptPath, projection.projectionReceipt.sceneGraphPath,
      handoff.assetPackPath, handoff.assetPackAuditPath, handoff.bindingManifestPath, handoff.approvalPath,
      ...[...assetsById.values()].flatMap(({ source, runtime }) => [source.path, runtime.path])
    ],
    provenance: {
      handoffId: handoff.handoffId,
      sourceKind: handoff.sourceKind,
      sceneGraphHash: handoff.sceneGraphHash,
      sourceSemanticManifestHash: handoff.sourceSemanticManifestHash,
      semanticManifestHash: handoff.semanticManifestHash,
      projectionReceiptSha256: handoff.projectionReceiptSha256,
      assetPackId: assetPack.packId,
      assetPackHash: handoff.assetPackHash,
      assetPackAuditHash: handoff.assetPackAuditHash,
      bindingManifestHash: handoff.bindingManifestHash,
      approvalSubjectHash: handoff.approvalSubjectHash,
      assetManifestHash: canonicalHash(assetManifest),
      reviewer: approval.reviewer,
      rights: manifestAssets.map(({ assetId, rights }) => ({ assetId, rights }))
    }
  };
}
