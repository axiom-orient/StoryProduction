import { isAbsolute } from 'node:path';
import {
  openMeasurementSession as openDefaultMeasurementSession,
  verifyMeasurementPublishResult
} from '../../store/src/index.js';
import {
  validateTrackBCommandInput,
  validateTrackBCommandResult
} from '../../contracts/src/trackb-contract.js';
import { loadTrackBContractRegistry } from '../../contracts/src/trackb-registry.js';
import { measureTrackBHtml as defaultMeasureTrackBHtml } from '../../browser-renderer/src/index.js';
import {
  MEASUREMENT_ARTIFACTS,
  parseJsonArtifact,
  readMeasurementArtifact,
  validateAuthority,
  validateMeasurementProvenance,
  verifyMeasuredResult
} from './trackb-measure-evidence.js';
import {
  attachCloseFailure,
  mapRuntimeError,
  runtimeError,
  throwValidationIfInvalid
} from './trackb-measure-support.js';
import { boundedMessage, isPlainObject } from './trackb-application-primitives.js';

/**
 * Read, prove, measure, and publish one Track B measured-evidence receipt.
 * Artifact bytes are supplied only by the measurement-purpose custody
 * session. This runtime intentionally has no pathname filesystem access.
 */
export async function measureTrackBEvidence(request, dependencies = {}) {
  const normalized = normalizeMeasureRequest(request);
  if (!isPlainObject(dependencies)) throw runtimeError('trackb-input-invalid', 'Track B measurement dependencies must be an object');

  const openMeasurementSession = dependencies.openMeasurementSession
    ?? dependencies.openSession
    ?? openDefaultMeasurementSession;
  if (typeof openMeasurementSession !== 'function') {
    throw runtimeError('trackb-input-invalid', 'openMeasurementSession must be a function');
  }
  const measureTrackBHtml = dependencies.measureTrackBHtml ?? defaultMeasureTrackBHtml;
  if (typeof measureTrackBHtml !== 'function') {
    throw runtimeError('trackb-input-invalid', 'measureTrackBHtml must be a function');
  }

  const registry = await loadRegistry(dependencies);
  let session;
  let operationFailure = null;
  try {
    session = await openMeasurementSession({ projectRoot: normalized.projectRoot });
    assertMeasurementSession(session);

    const artifacts = {};
    for (const [kind, inputKey, maximumBytes] of MEASUREMENT_ARTIFACTS) {
      const response = await readMeasurementArtifact(session, kind, normalized.input[inputKey], maximumBytes);
      artifacts[kind] = response.bytes;
    }

    const authority = parseJsonArtifact(artifacts.narrativeAuthority, 'NarrativeAuthority', 'trackb-authority-missing');
    validateAuthority(authority, registry);
    const provenance = parseJsonArtifact(artifacts.provenance, 'Track B provenance', 'trackb-evidence-incomplete');
    validateMeasurementProvenance(provenance, authority, artifacts.html);

    let measured;
    try {
      measured = await measureTrackBHtml({
        htmlBytes: artifacts.html,
        narrativeId: authority.narrativeId,
        authorityHash: authority.authorityHash,
        assetManifestHash: provenance.assetManifestHash,
        semanticManifestHash: provenance.semanticManifestHash,
        shotIds: authority.shots.map((shot) => shot.shotId),
        registry
      });
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-evidence-incomplete', 'Track B measured renderer failed');
    }
    verifyMeasuredResult(measured, {
      authority,
      provenance,
      htmlBytes: artifacts.html,
      registry
    });

    let published;
    try {
      published = await session.publishMeasurementReceipt(normalized.input.outputPath, measured.receiptBytes);
      verifyMeasurementPublishResult(published, {
        path: normalized.input.outputPath,
        bytes: measured.receiptBytes
      });
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-storage-integrity', 'Track B measured receipt publish failed');
    }

    const result = {
      receiptPath: normalized.input.outputPath,
      receiptHash: measured.receiptHash,
      narrativeId: authority.narrativeId,
      authorityHash: authority.authorityHash,
      htmlHash: measured.htmlHash,
      assetManifestHash: provenance.assetManifestHash,
      semanticManifestHash: provenance.semanticManifestHash,
      rendererLockHash: measured.rendererLockHash,
      shotIds: [...measured.shotIds],
      shotCount: measured.shotCount
    };
    const resultValidation = validateTrackBCommandResult('trackb.evidence.measure', result, {
      input: normalized.input
    });
    throwValidationIfInvalid(resultValidation, 'trackb-input-invalid', 'Track B measurement result');
    return Object.freeze(result);
  } catch (error) {
    operationFailure = mapRuntimeError(error, 'trackb-evidence-incomplete', 'Track B measurement failed');
    throw operationFailure;
  } finally {
    if (session) {
      try {
        const closeResult = await session.close();
        if (!closeResult || closeResult.closed !== true || Object.keys(closeResult).sort().join(',') !== 'closed') {
          throw runtimeError('trackb-storage-integrity', 'Track B measurement custody close response is invalid');
        }
      } catch (error) {
        const closeFailure = mapRuntimeError(error, 'trackb-storage-integrity', 'Track B measurement custody session close failed');
        if (!operationFailure) {
          throw runtimeError('trackb-storage-integrity', 'Track B measurement custody session close failed', {
            cause: closeFailure,
            details: {
              closeCode: closeFailure.code,
              closeMessage: boundedMessage(closeFailure.message, 'Track B measurement failed')
            }
          });
        }
        attachCloseFailure(operationFailure, closeFailure);
      }
    }
  }
}


async function loadRegistry(dependencies) {
  let registry = dependencies.registry;
  if (registry === undefined) {
    try {
      registry = await loadTrackBContractRegistry(dependencies.trackBSchemaDir);
    } catch (error) {
      throw mapRuntimeError(error, 'trackb-storage-integrity', 'Track B schema registry could not be loaded');
    }
  }
  if (!registry || typeof registry.validate !== 'function' || typeof registry.definition !== 'function' || !Array.isArray(registry.definitions) || registry.definitions.length !== 6) {
    throw runtimeError('trackb-storage-integrity', 'a loaded six-schema Track B registry is required');
  }
  return registry;
}

function normalizeMeasureRequest(request) {
  if (!isPlainObject(request)) throw runtimeError('trackb-input-invalid', 'Track B measurement request must be an object');
  let projectRoot;
  let input;
  if (Object.hasOwn(request, 'input')) {
    const unsupported = Object.keys(request).filter((key) => !['projectRoot', 'input'].includes(key));
    if (unsupported.length > 0) throw runtimeError('trackb-input-invalid', `unsupported measurement request key: ${unsupported[0]}`);
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
  const validation = validateTrackBCommandInput('trackb.evidence.measure', input);
  throwValidationIfInvalid(validation, 'trackb-input-invalid', 'Track B measurement input');
  return { projectRoot, input };
}

function assertMeasurementSession(session) {
  if (!session || typeof session.readMeasurementArtifact !== 'function' || typeof session.publishMeasurementReceipt !== 'function' || typeof session.close !== 'function') {
    throw runtimeError('trackb-storage-integrity', 'measurement custody session is incomplete');
  }
}
