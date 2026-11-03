import { canonicalHash } from '../../core/src/canonical.js';
import { readJson } from './io.js';
import { artifact, assertProductionContract, check, exact, integer, sealed } from './production-contract.js';
import { visualContext } from './visual-context.js';

export async function prepareMotionRequest(root, draft) {
  assertProductionContract(draft);
  exact(draft, 'kind sources shotId width height providerTransferAllowed', 'MotionRequestDraft');
  check(draft.kind === 'MotionRequestDraft', 'DRAFT_KIND');
  integer(draft.width, 1, 16384, 'width');
  integer(draft.height, 1, 16384, 'height');
  check(typeof draft.providerTransferAllowed === 'boolean', 'TRANSFER_PERMISSION_REQUIRED');

  const context = await visualContext(root, draft.sources);
  const shot = context.graph.shots.find((candidate) => candidate.id === draft.shotId);
  check(shot, 'SHOT_NOT_FOUND');
  const entityIds = [shot.environmentId, ...shot.characterIds, ...shot.propIds];
  return artifact({
    schemaVersion: 1,
    kind: 'ShotMotionRequest',
    sources: draft.sources,
    shotId: shot.id,
    sceneGraphHash: canonicalHash(context.graph),
    sourceNarrativeSha256: context.graph.sourceNarrativeSha256,
    directorPlanSha256: context.graph.directorPlanSha256,
    continuityBibleSha256: context.graph.continuityBibleSha256,
    visualReferencePackHash: context.pack.contentHash,
    visualReferenceApprovalHash: context.approval.contentHash,
    durationFrames: shot.durationFrames,
    fps: context.graph.timeline.fps,
    width: draft.width,
    height: draft.height,
    camera: shot.camera,
    composition: shot.composition,
    subjectAction: shot.action,
    performanceBeat: shot.performanceBeat,
    semanticRefs: shot.semanticRefs,
    startFrame: context.frames.get(shot.id)?.artifact ?? null,
    entityReferences: entityIds.filter((id) => context.refs.has(id)).map((id) => context.refs.get(id)),
    providerTransferAllowed: draft.providerTransferAllowed
  });
}

/** Internal revalidation used when a clip candidate is consumed downstream. */
export async function currentRequest(root, path) {
  const { value } = await readJson(root, path);
  sealed(value, 'ShotMotionRequest');
  const expected = await prepareMotionRequest(root, {
    kind: 'MotionRequestDraft',
    sources: value.sources,
    shotId: value.shotId,
    width: value.width,
    height: value.height,
    providerTransferAllowed: value.providerTransferAllowed
  });
  check(canonicalHash(value) === canonicalHash(expected), 'MOTION_REQUEST_STALE_OR_CHANGED');
  return value;
}
