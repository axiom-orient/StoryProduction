import { pathToFileURL } from 'node:url';
import { canonicalHash } from '../../core/src/canonical.js';
import { loadNarrativeSceneProjection } from '../../adapters/src/narrative-courier-release.js';
import { readJson, safePath, verifyMedia } from './io.js';
import { artifact, assertProductionContract, check, exact, integer, sealed } from './production-contract.js';
import { currentCandidate, currentQC } from './clip-lifecycle.js';

export async function buildEditorialTimeline(root, draft) {
  assertProductionContract(draft);
  exact(draft, 'kind projectionReceiptPath timelineId clips audioPlan captions', 'EditorialDraft');
  check(draft.kind === 'EditorialDraft', 'DRAFT_KIND');
  check(typeof draft.timelineId === 'string' && draft.timelineId.trim(), 'TIMELINE_ID');
  check(Array.isArray(draft.clips) && Array.isArray(draft.audioPlan) && Array.isArray(draft.captions), 'TIMELINE_ARRAYS');

  const projection = await loadNarrativeSceneProjection({
    projectRoot: root,
    projectionReceiptPath: draft.projectionReceiptPath
  });
  const graph = projection.sceneGraph;
  check(draft.clips.length === graph.shots.length, 'EDITORIAL_SHOT_COUNT');
  const clips = [];
  let cursor = 0;

  for (let index = 0; index < graph.shots.length; index += 1) {
    const item = draft.clips[index];
    const shot = graph.shots[index];
    exact(item, 'shotId candidatePath qcPath sourceStartFrame', 'editorial.clip');
    check(item.shotId === shot.id, 'EDITORIAL_SHOT_ORDER');
    integer(item.sourceStartFrame, 0, Number.MAX_SAFE_INTEGER, 'sourceStartFrame');

    const { candidate, request } = await currentCandidate(root, item.candidatePath);
    check(request.sceneGraphHash === canonicalHash(graph) && request.shotId === shot.id, 'EDITORIAL_SOURCE_STALE');
    const qc = await currentQC(root, item.qcPath, candidate);
    check(qc.verdict === 'PASS', 'EDITORIAL_QC_NOT_PASSED');
    check(Number.isSafeInteger(item.sourceStartFrame + shot.durationFrames)
      && item.sourceStartFrame + shot.durationFrames <= candidate.observation.frameCount, 'EDITORIAL_TRIM_OUT_OF_RANGE');

    clips.push({
      ...item,
      candidateHash: candidate.contentHash,
      qcHash: qc.contentHash,
      media: candidate.media,
      availableFrames: candidate.observation.frameCount,
      timelineStartFrame: cursor,
      durationFrames: shot.durationFrames
    });
    cursor += shot.durationFrames;
  }

  for (const transition of graph.timeline.transitions) {
    if (transition.type === 'cut') continue;
    const index = graph.shots.findIndex((shot) => shot.id === transition.fromShot);
    const first = clips[index];
    const second = clips[index + 1];
    const pre = Math.floor(transition.durationFrames / 2);
    const post = transition.durationFrames - pre;
    check(first && second && second.sourceStartFrame >= pre
      && first.sourceStartFrame + first.durationFrames + post <= first.availableFrames, 'TRANSITION_HANDLES_REQUIRED');
  }

  for (const audio of draft.audioPlan) {
    exact(audio, 'role media sourceStartFrame timelineStartFrame durationFrames gainMilliDb', 'audioCue');
    check(['dialogue', 'ambience', 'sfx', 'music'].includes(audio.role), 'AUDIO_ROLE');
    await verifyMedia(root, audio.media);
    integer(audio.sourceStartFrame, 0, Number.MAX_SAFE_INTEGER, 'audio.source');
    integer(audio.timelineStartFrame, 0, cursor, 'audio.start');
    integer(audio.durationFrames, 1, cursor, 'audio.duration');
    integer(audio.gainMilliDb, -96000, 24000, 'audio.gain');
    check(audio.timelineStartFrame + audio.durationFrames <= cursor, 'AUDIO_RANGE');
  }

  for (const caption of draft.captions) {
    exact(caption, 'text startFrame durationFrames', 'caption');
    check(typeof caption.text === 'string' && caption.text.trim(), 'CAPTION_TEXT');
    integer(caption.startFrame, 0, cursor, 'caption.start');
    integer(caption.durationFrames, 1, cursor, 'caption.duration');
    check(caption.startFrame + caption.durationFrames <= cursor, 'CAPTION_RANGE');
  }

  return artifact({
    schemaVersion: 1,
    kind: 'StoryEditorialTimeline',
    state: 'CANDIDATE',
    timelineId: draft.timelineId,
    projectionReceiptPath: draft.projectionReceiptPath,
    sceneGraphHash: canonicalHash(graph),
    directorPlanSha256: graph.directorPlanSha256,
    continuityBibleSha256: graph.continuityBibleSha256,
    fps: graph.timeline.fps,
    totalDurationFrames: cursor,
    clips,
    transitions: graph.timeline.transitions,
    audioPlan: draft.audioPlan,
    audioQualification: draft.audioPlan.length ? 'NOT_RUN' : 'N/A',
    captions: draft.captions
  });
}

export async function exportEditorialOTIO(root, draft) {
  assertProductionContract(draft);
  exact(draft, 'kind timelinePath', 'OTIOExportDraft');
  check(draft.kind === 'OTIOExportDraft', 'DRAFT_KIND');
  const { value: timeline } = await readJson(root, draft.timelinePath);
  sealed(timeline, 'StoryEditorialTimeline');
  const expected = await buildEditorialTimeline(root, {
    kind: 'EditorialDraft',
    projectionReceiptPath: timeline.projectionReceiptPath,
    timelineId: timeline.timelineId,
    clips: timeline.clips.map(({ shotId, candidatePath, qcPath, sourceStartFrame }) => ({
      shotId, candidatePath, qcPath, sourceStartFrame
    })),
    audioPlan: timeline.audioPlan,
    captions: timeline.captions
  });
  check(canonicalHash(expected) === canonicalHash(timeline), 'TIMELINE_STALE');
  check(timeline.audioPlan.length === 0, 'OTIO_AUDIO_EXPORT_UNQUALIFIED');
  check(timeline.transitions.every((transition) => ['cut', 'cross-dissolve'].includes(transition.type)), 'OTIO_TRANSITION_UNSUPPORTED');

  const time = (value) => ({ OTIO_SCHEMA: 'RationalTime.1', rate: timeline.fps, value });
  const range = (start, length) => ({
    OTIO_SCHEMA: 'TimeRange.1',
    start_time: time(start),
    duration: time(length)
  });
  const children = [];
  for (let index = 0; index < timeline.clips.length; index += 1) {
    const clip = timeline.clips[index];
    children.push({
      OTIO_SCHEMA: 'Clip.1',
      name: clip.shotId,
      enabled: true,
      metadata: {
        StoryProduction: {
          candidateHash: clip.candidateHash,
          qcHash: clip.qcHash,
          mediaSha256: clip.media.sha256
        }
      },
      effects: [],
      markers: [],
      source_range: range(clip.sourceStartFrame, clip.durationFrames),
      media_reference: {
        OTIO_SCHEMA: 'ExternalReference.1',
        target_url: pathToFileURL(await safePath(root, clip.media.path)).href,
        available_range: range(0, clip.availableFrames),
        metadata: {},
        name: clip.shotId
      }
    });
    const transition = timeline.transitions[index];
    if (transition?.type === 'cross-dissolve') {
      const pre = Math.floor(transition.durationFrames / 2);
      children.push({
        OTIO_SCHEMA: 'Transition.1',
        name: 'Cross dissolve',
        metadata: {},
        transition_type: 'SMPTE_Dissolve',
        parameters: {},
        in_offset: time(pre),
        out_offset: time(transition.durationFrames - pre)
      });
    }
  }
  return {
    OTIO_SCHEMA: 'Timeline.1',
    name: timeline.timelineId,
    metadata: {
      StoryProduction: {
        timelineHash: timeline.contentHash,
        state: 'CANDIDATE',
        captions: timeline.captions,
        mediaReferenceMode: 'validated-local-file-URI',
        finalApproval: false
      }
    },
    tracks: {
      OTIO_SCHEMA: 'Stack.1',
      name: 'tracks',
      source_range: null,
      enabled: true,
      metadata: {},
      effects: [],
      markers: [],
      children: [{
        OTIO_SCHEMA: 'Track.1',
        kind: 'Video',
        name: 'Video',
        source_range: null,
        enabled: true,
        metadata: {},
        effects: [],
        markers: [],
        children
      }]
    }
  };
}
