import {
  assertResult,
  contractError,
  hashTrackBEvidenceCell,
  hashTrackBReleaseCandidate,
  resultWithErrors
} from '../trackb-contract-primitives.js';
import { validateTrackBArtifactBindings } from './bindings.js';
import { validateSchemaValue } from './schema.js';

export function validateTrackBReleaseCandidate(value, { registry, authority, artifactBindings } = {}) {
  const result = validateSchemaValue('trackBReleaseCandidate', value, registry);
  if (!result.valid) return result;
  const errors = [];
  if (hashTrackBReleaseCandidate(value) !== value.candidateHash) errors.push(contractError('/candidateHash', 'candidateHash does not match its canonical self-hash', 'trackb-candidate-invalid'));
  for (let index = 0; index < value.evidenceCells.length; index += 1) {
    const cell = value.evidenceCells[index];
    if (hashTrackBEvidenceCell(cell) !== cell.evidenceCellHash) errors.push(contractError(`/evidenceCells/${index}/evidenceCellHash`, 'evidenceCellHash does not match its canonical self-hash', 'trackb-candidate-invalid'));
  }
  if (authority) validateCandidateRelations(value, authority, errors);
  if (artifactBindings) {
    const binding = validateTrackBArtifactBindings(value, { ...artifactBindings, registry });
    errors.push(...binding.errors);
  }
  const expectedVerdict = deriveCandidateVerdict(value);
  if (expectedVerdict !== value.candidateVerdict) errors.push(contractError('/candidateVerdict', `candidateVerdict must be ${expectedVerdict} for the supplied evidence`, 'trackb-candidate-invalid'));
  return resultWithErrors(errors);
}

export function assertTrackBReleaseCandidate(value, options = {}) {
  assertResult(validateTrackBReleaseCandidate(value, options));
  return value;
}

function validateCandidateRelations(candidate, authority, errors) {
  if (candidate.narrativeId !== authority.narrativeId || candidate.authorityHash !== authority.authorityHash) errors.push(contractError('/', 'candidate is not bound to the supplied authority', 'trackb-narrative-mismatch'));
  const authorityShotIds = authority.shots.map((shot) => shot.shotId);
  const candidateShotIds = candidate.evidenceCells.map((cell) => cell.shotId);
  if (candidateShotIds.length !== authorityShotIds.length || candidateShotIds.some((shotId, index) => shotId !== authorityShotIds[index]) || new Set(candidateShotIds).size !== candidateShotIds.length) errors.push(contractError('/evidenceCells', 'evidenceCells must be a shot-order bijection', 'trackb-evidence-incomplete'));
  const expectedMeanings = new Map(authority.meaningItems.map((item) => [item.meaningId, item]));
  const seen = new Set();
  for (let cellIndex = 0; cellIndex < candidate.evidenceCells.length; cellIndex += 1) {
    const cell = candidate.evidenceCells[cellIndex];
    for (let itemIndex = 0; itemIndex < cell.meaningCoverage.length; itemIndex += 1) {
      const item = cell.meaningCoverage[itemIndex];
      const expected = expectedMeanings.get(item.meaningId);
      if (!expected || expected.shotId !== cell.shotId || expected.semanticRef?.entryHash !== item.entryHash || item.status !== 'SUPPORTED' || item.representation !== 'METADATA' || seen.has(item.meaningId)) errors.push(contractError(`/evidenceCells/${cellIndex}/meaningCoverage/${itemIndex}`, 'meaning coverage must be a supported METADATA bijection to authority meaningItems', 'trackb-evidence-incomplete'));
      seen.add(item.meaningId);
    }
  }
  if (seen.size !== expectedMeanings.size) errors.push(contractError('/evidenceCells', 'meaning coverage is missing authority meaning items', 'trackb-evidence-incomplete'));
}

export function deriveCandidateVerdict(candidate) {
  const hardFail = candidate.evidenceCells.some((cell) => cell.gateVerdict === 'FAIL' || cell.runtimeEvidence.pass !== true || cell.assetEvidence.pass !== true || cell.visualEvidence.pass !== true);
  if (hardFail) return 'FAIL';
  const iterate = candidate.evidenceCells.some((cell) => cell.gateVerdict === 'ITERATE');
  return iterate ? 'ITERATE' : 'PROMOTE';
}
