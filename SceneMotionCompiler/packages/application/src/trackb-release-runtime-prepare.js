import { isAbsolute } from 'node:path';
import { openTrackBCustodyProjectSession } from '../../store/src/index.js';
import {
  hashTrackBAuthority,
  hashTrackBRawBytes,
  isTrackBSafeRelativePath,
  validateTrackBCommandInput,
  validateTrackBCommandResult
} from '../../contracts/src/trackb-contract.js';
import { throwValidationIfInvalid } from './trackb-release-bindings.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';
import { buildTrackBReleaseCandidate } from './trackb-release-candidate.js';
import { loadTrackBReleaseInputsFromArtifacts } from './trackb-release-input-contract.js';
import { canonicalizeJson } from '../../core/src/canonical.js';
import { loadRegistries } from './trackb-release-runtime-registry.js';
import { resolveCurrentDeliveryBase } from './trackb-release-runtime-delivery.js';
import { parseJsonBytes, readProjectArtifact } from './trackb-release-runtime-artifacts.js';
import {
  assertBoundedWriteBytes,
  attachCloseFailure,
  verifyCloseResult,
  verifyImmutableWrite,
  verifyInitializeResult
} from './trackb-release-runtime-custody.js';
import { hashHex, mapRuntimeError, runtimeError } from './trackb-release-runtime-support.js';

/**
 * Prepare and stage one immutable Track B release candidate.
 *
 * This runtime deliberately owns no pathname reads. Project artifacts are
 * obtained from one project-ingress custody session, parsed from the exact
 * returned bytes, and staged only after the verified current-pointer CAS base
 * has been checked. It never publishes the stable pointer.
 */
export async function prepareTrackBRelease(request, dependencies = {}) {
  const normalized = normalizePrepareRequest(request);
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'Track B runtime dependencies must be an object');
  if (Object.hasOwn(dependencies, 'measuredEvidence')) {
    throw runtimeError(
      'trackb-evidence-incomplete',
      'in-memory measuredEvidence injection is not accepted; provide a durable measured evidence receipt path'
    );
  }

  const openProjectSession = dependencies.openProjectSession ?? openTrackBCustodyProjectSession;
  if (typeof openProjectSession !== 'function') throw runtimeError('trackb-input-invalid', 'openProjectSession must be a function');
  const workflowDependencies = isPlainObject(dependencies.workflowDeps)
    ? dependencies.workflowDeps
    : Object.fromEntries(Object.entries(dependencies).filter(([key]) => !['openProjectSession', 'workflowDeps'].includes(key)));
  const { registry, artifactRegistries } = await loadRegistries(workflowDependencies);

  let session;
  let operationFailure = null;
  try {
    session = await openProjectSession({ projectRoot: normalized.projectRoot });
    if (!session || typeof session.readProjectArtifact !== 'function'
      || typeof session.initializeNarrative !== 'function'
      || typeof session.snapshot !== 'function'
      || typeof session.writeImmutable !== 'function'
      || typeof session.close !== 'function') {
      throw runtimeError('trackb-storage-integrity', 'project-ingress custody session is incomplete');
    }

    const input = normalized.input;
    const authorityRead = await readProjectArtifact(session, 'narrativeAuthority', input.narrativeAuthorityPath);
    const projectionRead = await readProjectArtifact(session, 'projectionReceipt', input.projectionReceiptPath);
    const projection = parseJsonBytes(projectionRead, 'NarrativeSceneProjection', 'trackb-artifact-binding-invalid');
    if (!isPlainObject(projection) || !isTrackBSafeRelativePath(projection.sceneGraphPath)) {
      throw runtimeError('trackb-artifact-binding-invalid', 'projection receipt must provide a safe SceneGraph path');
    }
    const projectionReceiptHash = hashTrackBRawBytes(projectionRead);
    const handoffRead = await readProjectArtifact(session, 'assetHandoff', input.assetHandoffPath);
    const htmlRead = await readProjectArtifact(session, 'html', input.htmlPath);
    const provenanceRead = await readProjectArtifact(session, 'provenance', input.provenancePath);
    const evidenceReceiptRead = await readProjectArtifact(session, 'evidenceReceipt', input.evidenceReceiptPath);
    const sceneGraphRead = await readProjectArtifact(
      session,
      'sceneGraph',
      projection.sceneGraphPath,
      { projectionReceiptHash }
    );

    const loaded = loadTrackBReleaseInputsFromArtifacts({
      input,
      artifacts: {
        authorityBytes: authorityRead,
        projectionReceiptBytes: projectionRead,
        handoffBytes: handoffRead,
        htmlBytes: htmlRead,
        provenanceBytes: provenanceRead,
        evidenceReceiptBytes: evidenceReceiptRead,
        sceneGraphBytes: sceneGraphRead
      },
      registry,
      artifactRegistries
    });

    const initializeResult = await session.initializeNarrative({
      narrativeId: loaded.authority.narrativeId,
      readOnly: false
    });
    verifyInitializeResult(initializeResult, loaded.authority.narrativeId);
    const currentBase = await resolveCurrentDeliveryBase(session, loaded.authority.narrativeId, registry);
    if (normalized.input.expectedBaseDeliverySetHash !== currentBase.baseDeliverySetHash) {
      throw runtimeError(
        'trackb-cas-conflict',
        'expectedBaseDeliverySetHash does not match the verified current delivery pointer'
      );
    }

    const built = buildTrackBReleaseCandidate({
      ...loaded,
      candidateId: input.candidateId,
      expectedBaseDeliverySetHash: currentBase.baseDeliverySetHash
    });
    const authorityBytes = Buffer.from(canonicalizeJson(loaded.authority), 'utf8');
    assertBoundedWriteBytes(built.candidateBytes, 'candidate');
    assertBoundedWriteBytes(authorityBytes, 'authority');
    if (hashTrackBAuthority(loaded.authority) !== loaded.authority.authorityHash) {
      throw runtimeError('trackb-artifact-binding-invalid', 'authority canonical hash does not match authorityHash');
    }
    const authorityPath = `authority/${hashHex(loaded.authority.authorityHash)}/trackb-narrative-authority-v2.json`;
    const candidatePath = `candidates/${hashHex(built.candidateHash)}/trackb-release-candidate-v4.json`;
    const authorityWrite = await session.writeImmutable(authorityPath, authorityBytes);
    verifyImmutableWrite(authorityWrite, authorityPath, authorityBytes);
    const candidateWrite = await session.writeImmutable(candidatePath, built.candidateBytes);
    verifyImmutableWrite(candidateWrite, candidatePath, built.candidateBytes);

    const result = {
      candidatePath: `scenes/trackb/${loaded.authority.narrativeId}/${candidatePath}`,
      candidateHash: built.candidateHash,
      authorityHash: built.authorityHash,
      meaningQualificationHash: built.meaningQualificationHash,
      evidenceReceiptHash: built.evidenceReceiptHash,
      baseDeliverySetHash: built.baseDeliverySetHash,
      candidateVerdict: built.candidateVerdict,
      evidenceCellCount: built.evidenceCellCount
    };
    const resultValidation = validateTrackBCommandResult('trackb.release.prepare', result);
    throwValidationIfInvalid(resultValidation, 'trackb-input-invalid', 'Track B prepare result');
    return Object.freeze(result);
  } catch (error) {
    operationFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B release prepare failed');
    throw operationFailure;
  } finally {
    if (session) {
      try {
        const closeResult = await session.close();
        verifyCloseResult(closeResult);
      } catch (error) {
        const closeFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B custody session close failed');
        if (!operationFailure) {
          throw runtimeError('trackb-storage-integrity', 'Track B custody session close failed', {
            cause: closeFailure,
            details: { closeCode: closeFailure.code, closeMessage: boundedMessage(closeFailure.message) }
          });
        }
        attachCloseFailure(operationFailure, closeFailure);
      }
    }
  }
}

function normalizePrepareRequest(request) {
  if (!isPlainObject(request)) throw runtimeError('trackb-input-invalid', 'Track B prepare request must be an object');
  let projectRoot;
  let input;
  if (Object.hasOwn(request, 'input')) {
    const unsupported = Object.keys(request).filter((key) => !['projectRoot', 'input'].includes(key));
    if (unsupported.length > 0) throw runtimeError('trackb-input-invalid', `unsupported prepare request key: ${unsupported[0]}`);
    projectRoot = request.projectRoot;
    input = request.input;
  } else {
    projectRoot = request.projectRoot;
    input = { ...request };
    delete input.projectRoot;
  }
  if (typeof projectRoot !== 'string' || !isAbsolute(projectRoot)) {
    throw runtimeError('trackb-input-invalid', 'projectRoot must be an absolute path');
  }
  const validation = validateTrackBCommandInput('trackb.release.prepare', input);
  throwValidationIfInvalid(validation, 'trackb-input-invalid', 'Track B prepare input');
  return { projectRoot, input };
}
