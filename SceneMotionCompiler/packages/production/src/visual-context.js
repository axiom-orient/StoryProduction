import { canonicalHash } from '../../core/src/canonical.js';
import { loadNarrativeSceneProjection } from '../../adapters/src/narrative-courier-release.js';
import { readJson, verifyMedia } from './io.js';
import {
  assertVisualReferenceApproval,
  assertVisualReferencePack,
  check,
  exact,
  sealed
} from './production-contract.js';

export async function visualContext(root, sources) {
  exact(sources, 'projectionReceiptPath visualReferencePackPath visualReferenceApprovalPath', 'sources');
  const loaded = await loadNarrativeSceneProjection({
    projectRoot: root,
    projectionReceiptPath: sources.projectionReceiptPath
  });
  const graph = loaded.sceneGraph;
  const { value: pack } = await readJson(root, sources.visualReferencePackPath);
  const { value: approval } = await readJson(root, sources.visualReferenceApprovalPath);

  assertVisualReferencePack(pack);
  assertVisualReferenceApproval(approval);
  sealed(pack, 'VisualReferencePack');
  sealed(approval, 'VisualReferenceApproval');
  check(approval.packHash === pack.contentHash && approval.decision === 'APPROVED', 'VISUAL_APPROVAL_STALE');

  for (const [key, expected] of [
    ['sceneGraphHash', canonicalHash(graph)],
    ['directorPlanSha256', graph.directorPlanSha256],
    ['continuityBibleSha256', graph.continuityBibleSha256],
    ['sourceNarrativeSha256', graph.sourceNarrativeSha256]
  ]) {
    check(pack[key] === expected, `VISUAL_SOURCE_STALE:${key}`);
  }

  const known = new Map([
    ['characters', 'CHARACTER'],
    ['environments', 'ENVIRONMENT'],
    ['props', 'PROP']
  ].flatMap(([group, role]) => graph[group].map((entity) => [entity.id, role])));
  const refs = new Map();
  const frames = new Map();

  for (const ref of pack.entityReferences) {
    check(known.get(ref.entityId) === ref.role && !refs.has(ref.entityId), 'VISUAL_ENTITY');
    check(new Set(ref.images.map((image) => image.view)).size === ref.images.length, 'VISUAL_VIEW_DUPLICATE');
    for (const image of ref.images) {
      await verifyMedia(root, { path: image.artifact.path, sha256: image.artifact.sha256 });
    }
    refs.set(ref.entityId, ref);
  }

  for (const frame of pack.storyboardFrames) {
    const shot = graph.shots.find((candidate) => candidate.id === frame.shotId);
    check(shot && !frames.has(frame.shotId), 'KEYFRAME_SHOT');
    const allowed = new Set([shot.environmentId, ...shot.characterIds, ...shot.propIds]);
    check(frame.referenceEntityIds.every((id) => refs.has(id) && allowed.has(id)), 'KEYFRAME_REFERENCE');
    await verifyMedia(root, { path: frame.artifact.path, sha256: frame.artifact.sha256 });
    frames.set(frame.shotId, frame);
  }

  for (const shot of graph.shots) {
    const policy = shot.referencePolicy;
    const required = [
      ...shot.propIds,
      ...(policy.requiresCharacterReference ? shot.characterIds : []),
      ...(policy.requiresEnvironmentReference ? [shot.environmentId] : [])
    ];
    check(required.every((id) => refs.has(id)), 'REQUIRED_REFERENCE');
    check(!policy.requiresKeyframe || frames.has(shot.id), 'REQUIRED_KEYFRAME');
    if (frames.has(shot.id)) {
      check(required.every((id) => frames.get(shot.id).referenceEntityIds.includes(id)), 'KEYFRAME_CONTINUITY');
    }
  }

  return { loaded, graph, pack, approval, refs, frames };
}
