// Public stage producers remain callable directly; production.prepare is the
// single file-backed dispatch and immutable-publication entrypoint.
import { publish, readJson } from './io.js';
import { check } from './production-contract.js';
import { prepareMotionRequest } from './motion-request.js';
import { ingestClip, reviewClip } from './clip-lifecycle.js';
import { buildEditorialTimeline, exportEditorialOTIO } from './editorial.js';

export {
  prepareMotionRequest,
  ingestClip,
  reviewClip,
  buildEditorialTimeline,
  exportEditorialOTIO
};

const PRODUCTION_STAGES = Object.freeze({
  MotionRequestDraft: prepareMotionRequest,
  ClipIngestDraft: ingestClip,
  ClipReviewDraft: reviewClip,
  EditorialDraft: buildEditorialTimeline,
  OTIOExportDraft: exportEditorialOTIO
});

export async function prepareProductionArtifact({ projectRoot, requestPath, outputPath }, options = {}) {
  options.signal?.throwIfAborted();
  const request = await readJson(projectRoot, requestPath);
  const handler = PRODUCTION_STAGES[request.value.kind];
  check(handler, 'PRODUCTION_STAGE_UNSUPPORTED');
  const result = await handler(projectRoot, request.value, options);
  options.signal?.throwIfAborted();
  check((await readJson(projectRoot, requestPath)).sha256 === request.sha256, 'REQUEST_CHANGED_DURING_EXECUTION');
  const sha256 = await publish(projectRoot, outputPath, result);
  return {
    path: outputPath,
    sha256,
    artifactKind: result.kind ?? result.OTIO_SCHEMA,
    contentHash: result.contentHash ?? null,
    artifactState: result.verdict ?? result.state ?? 'CANDIDATE',
    finalApproval: false
  };
}
