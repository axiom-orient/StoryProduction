import { canonicalHash } from '../../core/src/canonical.js';
import { probeClip, readJson, verifyMedia } from './io.js';
import { artifact, assertProductionContract, check, exact, sealed } from './production-contract.js';
import { currentRequest } from './motion-request.js';

const STATES = new Set(['SUCCEEDED', 'FAILED', 'CANCELLED', 'OUTCOME_UNKNOWN']);
const RATINGS = new Set(['PASS', 'FAIL', 'NOT_RUN']);
const REVIEW_CATEGORIES = ['visualContinuity', 'motion', 'narrative'];

function validateProviderReceipt(receipt, request) {
  assertProductionContract(receipt);
  exact(receipt, 'schemaVersion kind requestHash providerId providerJobId state transferred mediaSha256', 'MotionProviderReceipt');
  check(receipt.schemaVersion === 1 && receipt.kind === 'MotionProviderReceipt'
    && receipt.requestHash === request.contentHash, 'PROVIDER_REQUEST_BINDING');
  check(typeof receipt.providerId === 'string' && receipt.providerId.trim()
    && typeof receipt.providerJobId === 'string' && receipt.providerJobId.trim(), 'PROVIDER_ID');
  check(STATES.has(receipt.state) && typeof receipt.transferred === 'boolean', 'PROVIDER_STATE');
  check(!receipt.transferred || request.providerTransferAllowed, 'TRANSFER_NOT_ALLOWED');
}

export async function ingestClip(root, draft, options = {}) {
  assertProductionContract(draft);
  exact(draft, 'kind requestPath providerReceiptPath media', 'ClipIngestDraft');
  check(draft.kind === 'ClipIngestDraft', 'DRAFT_KIND');
  const request = await currentRequest(root, draft.requestPath);
  const receipt = await readJson(root, draft.providerReceiptPath);
  const providerReceipt = receipt.value;
  validateProviderReceipt(providerReceipt, request);

  let observation = null;
  if (providerReceipt.state === 'SUCCEEDED') {
    check(draft.media && draft.media.sha256 === providerReceipt.mediaSha256, 'PROVIDER_MEDIA_BINDING');
    observation = await probeClip(root, draft.media, options);
  } else {
    check(draft.media === null && providerReceipt.mediaSha256 === null, 'UNCONFIRMED_MEDIA_MUST_NOT_BE_ADOPTED');
  }

  return artifact({
    schemaVersion: 1,
    kind: 'GeneratedClipCandidate',
    requestPath: draft.requestPath,
    requestHash: request.contentHash,
    shotId: request.shotId,
    providerReceiptPath: draft.providerReceiptPath,
    providerReceiptSha256: receipt.sha256,
    state: providerReceipt.state,
    media: draft.media,
    observation
  });
}

/** Internal candidate revalidation shared by review and editorial assembly. */
export async function currentCandidate(root, path, { reprobe = false, signal } = {}) {
  const { value } = await readJson(root, path);
  exact(value, 'schemaVersion kind requestPath requestHash shotId providerReceiptPath providerReceiptSha256 state media observation contentHash', 'candidate');
  sealed(value, 'GeneratedClipCandidate');
  const request = await currentRequest(root, value.requestPath);
  check(value.requestHash === request.contentHash && value.shotId === request.shotId, 'CANDIDATE_STALE');

  const receipt = await readJson(root, value.providerReceiptPath);
  validateProviderReceipt(receipt.value, request);
  check(receipt.sha256 === value.providerReceiptSha256
    && receipt.value.requestHash === request.contentHash
    && receipt.value.state === value.state, 'PROVIDER_RECEIPT_CHANGED');
  check(STATES.has(value.state), 'CANDIDATE_STATE');

  if (value.state === 'SUCCEEDED') {
    check(value.media && value.observation && receipt.value.mediaSha256 === value.media.sha256, 'CANDIDATE_MEDIA');
    await verifyMedia(root, value.media);
    if (reprobe) {
      check(canonicalHash(await probeClip(root, value.media, { signal })) === canonicalHash(value.observation), 'OBSERVATION_CHANGED');
    }
  } else {
    check(value.media === null && value.observation === null && receipt.value.mediaSha256 === null, 'INVALID_CANDIDATE_STATE');
  }
  return { candidate: value, request };
}

export async function reviewClip(root, draft, options = {}) {
  assertProductionContract(draft);
  exact(draft, 'kind candidatePath reviewEvidencePath', 'ClipReviewDraft');
  check(draft.kind === 'ClipReviewDraft', 'DRAFT_KIND');
  const { candidate, request } = await currentCandidate(root, draft.candidatePath, { reprobe: true, ...options });
  const observation = candidate.observation;
  const checks = {
    providerCompleted: candidate.state === 'SUCCEEDED',
    frameRate: !!observation && BigInt(observation.fpsNumerator) === BigInt(request.fps) * BigInt(observation.fpsDenominator),
    duration: !!observation && observation.frameCount >= request.durationFrames,
    resolution: !!observation && observation.width === request.width && observation.height === request.height
  };
  const observed = Object.fromEntries(REVIEW_CATEGORIES.map((category) => [category, 'NOT_RUN']));
  let reviewEvidenceSha256 = null;

  if (draft.reviewEvidencePath !== null) {
    const evidence = await readJson(root, draft.reviewEvidencePath);
    const value = evidence.value;
    assertProductionContract(value);
    exact(value, 'schemaVersion kind candidateHash reviewer basis observed', 'ShotReviewEvidence');
    check(value.schemaVersion === 1 && value.kind === 'ShotReviewEvidence'
      && value.candidateHash === candidate.contentHash, 'REVIEW_CANDIDATE_BINDING');
    check(['OPERATOR_REVIEW', 'MODEL_REVIEW'].includes(value.basis)
      && typeof value.reviewer === 'string' && value.reviewer.trim(), 'REVIEW_SOURCE');
    exact(value.observed, REVIEW_CATEGORIES.join(' '), 'observed');
    for (const category of REVIEW_CATEGORIES) {
      check(RATINGS.has(value.observed[category]), 'REVIEW_RATING');
      observed[category] = value.observed[category];
    }
    reviewEvidenceSha256 = evidence.sha256;
  }

  const contractPassed = Object.values(checks).every(Boolean);
  const repairs = [];
  if (!contractPassed) {
    repairs.push({
      owner: 'SceneMotionCompiler',
      scope: 'SHOT',
      shotId: request.shotId,
      decision: ['OUTCOME_UNKNOWN', 'CANCELLED'].includes(candidate.state) ? 'BLOCKED' : 'REGENERATE',
      reason: candidate.state === 'OUTCOME_UNKNOWN'
        ? 'Reconcile provider job; do not blindly retry'
        : candidate.state === 'CANCELLED'
          ? 'Cancellation is terminal; a new request needs fresh operator authorization'
          : 'Media contract failed'
    });
  }
  for (const category of REVIEW_CATEGORIES) {
    if (observed[category] === 'PASS') continue;
    repairs.push({
      owner: observed[category] === 'NOT_RUN'
        ? 'Human'
        : category === 'narrative'
          ? 'NarrativeCourier'
          : category === 'visualContinuity'
            ? 'SceneAssetCompiler'
            : 'SceneMotionCompiler',
      scope: 'SHOT',
      shotId: request.shotId,
      decision: observed[category] === 'NOT_RUN' || category === 'narrative'
        ? 'BLOCKED'
        : category === 'visualContinuity'
          ? 'REPAIRABLE'
          : 'REGENERATE',
      reason: `${category}:${observed[category]}`
    });
  }

  return artifact({
    schemaVersion: 1,
    kind: 'ShotQCReport',
    candidatePath: draft.candidatePath,
    candidateHash: candidate.contentHash,
    requestHash: request.contentHash,
    shotId: request.shotId,
    contractChecks: checks,
    observed,
    reviewEvidencePath: draft.reviewEvidencePath,
    reviewEvidenceSha256,
    verdict: contractPassed && REVIEW_CATEGORIES.every((category) => observed[category] === 'PASS') ? 'PASS' : 'BLOCKED',
    repairs
  });
}

/** Internal QC recomputation used before timeline assembly. */
export async function currentQC(root, path, candidate) {
  const { value } = await readJson(root, path);
  exact(value, 'schemaVersion kind candidatePath candidateHash requestHash shotId contractChecks observed reviewEvidencePath reviewEvidenceSha256 verdict repairs contentHash', 'QC');
  sealed(value, 'ShotQCReport');
  check(value.candidateHash === candidate.contentHash && value.requestHash === candidate.requestHash
    && value.shotId === candidate.shotId, 'QC_STALE');
  const expected = await reviewClip(root, {
    kind: 'ClipReviewDraft',
    candidatePath: value.candidatePath,
    reviewEvidencePath: value.reviewEvidencePath
  });
  check(canonicalHash(value) === canonicalHash(expected), 'QC_OBSERVATION_OR_EVIDENCE_CHANGED');
  return value;
}
