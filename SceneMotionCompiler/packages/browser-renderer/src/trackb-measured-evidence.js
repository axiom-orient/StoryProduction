import { canonicalHash, canonicalizeJson, sha256Hex, sha256Prefixed } from '../../core/src/canonical.js';
import {
  TRACKB_SEMANTIC_MANIFEST_ANCHOR,
  TRACKB_SEMANTIC_MANIFEST_ENCODING,
  hashTrackBMeasuredEvidenceReceipt,
  isTrackBHash,
  isTrackBNarrativeId,
  isTrackBOpaqueId,
  parseTrackBSemanticManifestHtml,
  validateTrackBMeasuredEvidenceReceipt
} from '../../contracts/src/trackb-contract.js';
import {
  TRACK_B_NAVIGATION_GUARD_MECHANISM,
  TRACK_B_NAVIGATION_GUARD_SOURCE_HASH
} from '../../contracts/src/trackb-navigation-guard.js';
import { loadTrackBContractRegistry } from '../../contracts/src/trackb-registry.js';
import { CdpClient } from './cdp-client.js';
import { createPageEndpoint, createRendererLock, launchChromium, validateRendererLockForLaunch } from './chromium.js';
import { classifyRuntimeLogs, collectNavigationGuardLogs, configureCdpSession, createRuntimeLogs } from './render.js';

const TRACK_B_MEASURED_EVIDENCE_MAX_HTML_BYTES = 16 * 1024 * 1024;
const TRACK_B_MEASURED_EVIDENCE_MAX_SHOTS = 64;
const TRACK_B_MEASURED_EVIDENCE_DEFAULT_TIMEOUT_MS = 30_000;
const TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS = Object.freeze({ min: 1_000, max: 120_000 });

class TrackBMeasuredEvidenceError extends Error {
  constructor(code, message, options = {}) {
    super(message, options.cause ? { cause: options.cause } : undefined);
    this.name = 'TrackBMeasuredEvidenceError';
    this.code = code;
    this.details = options.details;
    this.retryable = options.retryable === true;
  }
}

/**
 * Execute the exact HTML bytes in the pinned Chromium/CDP renderer and return
 * a durable, self-hashed measured-evidence receipt.  The function deliberately
 * does not write a receipt: the application command owns the safe output path.
 */
export async function measureTrackBHtml(request = {}) {
  const normalized = normalizeRequest(request);
  const registry = request.registry ?? await loadTrackBContractRegistry(request.trackBSchemaDir);
  const htmlBytes = normalizeHtmlBytes(request.htmlBytes ?? request.html);
  let sourceManifest;
  try {
    sourceManifest = parseTrackBSemanticManifestHtml(htmlBytes);
  } catch (error) {
    throw evidenceError('trackb-evidence-incomplete', `HTML semantic manifest is invalid: ${error.message}`, { cause: error });
  }
  if (sourceManifest.semanticManifestHash !== normalized.semanticManifestHash) {
    throw evidenceError('trackb-evidence-incomplete', 'HTML semanticManifestHash does not match the expected semantic manifest hash');
  }

  const logs = createRuntimeLogs();
  const rendererLock = request.rendererLock ?? await createRendererLock(request.rendererProfile ?? {});
  await validateRendererLockForLaunch(rendererLock);
  const rendererLockHash = canonicalHash(rendererLock);
  const viewport = normalizeViewport(request.viewport);
  let browser;
  let client;
  let navigationGuard;
  let timeoutTimer;
  let timedOut = false;
  let operationFailure = null;
  const deadlineError = () => evidenceError('trackb-evidence-timeout', `Track B measured evidence exceeded the ${normalized.timeoutMs}ms deadline`, { retryable: true });

  try {
    browser = await launchChromium(rendererLock);
    const endpointTimeout = Math.max(100, Math.min(10_000, normalized.timeoutMs));
    const page = await createPageEndpoint(browser.port, 'about:blank', { timeoutMs: endpointTimeout });
    client = await CdpClient.connect(page.webSocketDebuggerUrl, endpointTimeout);
    timeoutTimer = setTimeout(() => {
      timedOut = true;
      try { client?.close(); } catch { /* cleanup is verified in finally */ }
    }, normalized.timeoutMs);

    const cdpSetup = await configureCdpSession(client, {
      rendererLock,
      viewport,
      logs,
      requireNavigationApi: true,
      navigationGuardMode: 'binding',
      timeoutMs: boundedOperationTimeout(normalized.timeoutMs)
    });
    navigationGuard = cdpSetup.navigationGuard;
    if (!navigationGuard) throw evidenceError('trackb-evidence-incomplete', 'trusted navigation guard capability is unavailable');
    if (navigationGuard.mechanism !== TRACK_B_NAVIGATION_GUARD_MECHANISM
      || navigationGuard.sourceHash !== TRACK_B_NAVIGATION_GUARD_SOURCE_HASH) {
      throw evidenceError('trackb-evidence-incomplete', 'trusted navigation guard binding result does not match the contract-owned source');
    }
    await client.send('DOM.enable');
    const { frameTree } = await client.send('Page.getFrameTree');
    const initialUrl = frameTree?.frame?.url;
    if (typeof initialUrl !== 'string' || initialUrl.length === 0) throw evidenceError('trackb-evidence-incomplete', 'initial document URL attestation is unavailable');
    await navigationGuard.waitForReady({ expectedUrl: initialUrl, timeoutMs: boundedOperationTimeout(normalized.timeoutMs) });
    const load = client.waitFor('Page.loadEventFired', boundedOperationTimeout(normalized.timeoutMs)).catch(() => null);
    await client.send('Page.setDocumentContent', { frameId: frameTree.frame.id, html: htmlBytes.toString('utf8') }, boundedOperationTimeout(normalized.timeoutMs));
    await navigationGuard.waitForDocumentReady({
      afterExecutionContextId: navigationGuard.initialExecutionContextId,
      expectedUrl: initialUrl,
      allowExistingContext: true,
      timeoutMs: boundedOperationTimeout(normalized.timeoutMs)
    });
    await load;
    await waitForReady(client, normalized.timeoutMs);
    await assertDocumentUrlInvariant(client, initialUrl, 'Track B measured evidence', boundedOperationTimeout(normalized.timeoutMs));
    await collectNavigationGuardLogs(client, logs, 'Track B measured evidence', boundedOperationTimeout(normalized.timeoutMs), navigationGuard);
    verifyRuntimeLogs(logs, 'Track B measured evidence');
    await verifyBrowserSemanticAnchor(client, normalized.semanticManifestHash, sourceManifest.semanticManifest);
    verifyShotIds(await readShotIds(client), normalized.shotIds);

    const shots = [];
    for (let ordinal = 0; ordinal < normalized.shotIds.length; ordinal += 1) {
      if (timedOut) throw deadlineError();
      const shotId = normalized.shotIds[ordinal];
      const runtimeBaseline = captureRuntimeCounts(logs);
      navigationGuard.assertHealthy();
      await assertDocumentUrlInvariant(client, initialUrl, `Track B measured evidence before shot ${shotId}`, boundedOperationTimeout(normalized.timeoutMs));
      await verifyBrowserSemanticAnchor(client, normalized.semanticManifestHash, sourceManifest.semanticManifest);
      const activated = await runtimeValue(client, `(() => {
        const id = ${JSON.stringify(shotId)};
        const elements = [...document.querySelectorAll('[data-shot-id]')].filter((element) => element.getAttribute('data-shot-id') === id);
        if (elements.length !== 1) return { found: elements.length };
        elements[0].click();
        return { found: 1 };
      })()`, normalized.timeoutMs);
      if (activated?.found !== 1) throw evidenceError('trackb-evidence-incomplete', `exactly one data-shot-id=${shotId} is required for activation`);
      await waitDoubleRaf(client, normalized.timeoutMs);
      const visual = await inspectShot(client, shotId, normalized.timeoutMs);
      if (visual.activeShotCount !== 1 || visual.visibleShotCount !== 1 || visual.activeShotId !== shotId || visual.visibleShotId !== shotId) {
        throw evidenceError('trackb-evidence-incomplete', `shot ${shotId} did not produce exactly one active and visible shot`);
      }
      if (!hasPositiveGeometry(visual.stageGeometry) || !hasPositiveGeometry(visual.shotGeometry)) {
        throw evidenceError('trackb-evidence-incomplete', `shot ${shotId} has zero stage or shot geometry`);
      }
      const screenshot = await captureScreenshot(client, normalized.timeoutMs);
      await assertDocumentUrlInvariant(client, initialUrl, `Track B measured evidence shot ${shotId}`, boundedOperationTimeout(normalized.timeoutMs));
      await collectNavigationGuardLogs(client, logs, `Track B measured evidence shot ${shotId}`, boundedOperationTimeout(normalized.timeoutMs), navigationGuard);
      verifyRuntimeLogs(logs, `Track B measured evidence shot ${shotId}`);
      shots.push({
        shotId,
        ordinal,
        screenshotHash: sha256Prefixed(screenshot),
        runtime: runtimeResult(logs, true, runtimeBaseline),
        visual: { ...visual, pass: true }
      });
    }

    verifyRuntimeLogs(logs, 'Track B measured evidence');
    navigationGuard.assertHealthy();
    await assertDocumentUrlInvariant(client, initialUrl, 'Track B measured evidence final', boundedOperationTimeout(normalized.timeoutMs));
    await collectNavigationGuardLogs(client, logs, 'Track B measured evidence final', boundedOperationTimeout(normalized.timeoutMs), navigationGuard);
    verifyRuntimeLogs(logs, 'Track B measured evidence final');
    const receipt = {
      schemaVersion: 2,
      receiptId: normalized.receiptId ?? `evidence.${sha256Hex(htmlBytes).slice(0, 32)}`,
      narrativeId: normalized.narrativeId,
      authorityHash: normalized.authorityHash,
      htmlHash: sha256Prefixed(htmlBytes),
      assetManifestHash: normalized.assetManifestHash,
      semanticManifestHash: normalized.semanticManifestHash,
      rendererLockHash,
      navigationGuard: {
        mechanism: navigationGuard.mechanism,
        guardSourceHash: navigationGuard.sourceHash
      },
      shotIds: [...normalized.shotIds],
      shots,
    };
    receipt.receiptHash = hashTrackBMeasuredEvidenceReceipt(receipt);
    const validation = validateTrackBMeasuredEvidenceReceipt(receipt, { registry });
    if (validation.valid !== true) {
      throw evidenceError('trackb-evidence-incomplete', validation.errors?.[0]?.message ?? 'measured evidence receipt is invalid');
    }
    const receiptBytes = Buffer.from(canonicalizeJson(receipt), 'utf8');
    return Object.freeze({
      receipt: Object.freeze(receipt),
      receiptBytes,
      receiptHash: receipt.receiptHash,
      htmlHash: receipt.htmlHash,
      rendererLockHash,
      shotIds: [...receipt.shotIds],
      shotCount: receipt.shots.length
    });
  } catch (error) {
    operationFailure = timedOut ? deadlineError() : mapRunnerError(error);
    throw operationFailure;
  } finally {
    if (timeoutTimer) clearTimeout(timeoutTimer);
    try { navigationGuard?.dispose(); } catch (error) { if (!operationFailure) operationFailure = evidenceError('trackb-evidence-cleanup', 'Navigation guard cleanup failed', { cause: error }); }
    try { client?.close(); } catch (error) { if (!operationFailure) operationFailure = evidenceError('trackb-evidence-cleanup', 'CDP cleanup failed', { cause: error }); }
    try { await browser?.close(); } catch (error) {
      const cleanupFailure = evidenceError('trackb-evidence-cleanup', 'Chromium/profile cleanup failed', { cause: error });
      if (!operationFailure) throw cleanupFailure;
      operationFailure.cleanupError = cleanupFailure;
    }
  }
}


function normalizeRequest(request) {
  if (!request || typeof request !== 'object' || Array.isArray(request)) throw evidenceError('trackb-input-invalid', 'measurement request must be an object');
  const timeoutMs = request.timeoutMs ?? TRACK_B_MEASURED_EVIDENCE_DEFAULT_TIMEOUT_MS;
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS.min || timeoutMs > TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS.max) {
    throw evidenceError('trackb-input-invalid', `timeoutMs must be an integer between ${TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS.min} and ${TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS.max}`);
  }
  if (!isTrackBNarrativeId(request.narrativeId)) throw evidenceError('trackb-input-invalid', 'narrativeId is invalid');
  for (const [name, value] of [['authorityHash', request.authorityHash], ['assetManifestHash', request.assetManifestHash], ['semanticManifestHash', request.semanticManifestHash]]) {
    if (!isTrackBHash(value)) throw evidenceError('trackb-input-invalid', `${name} must be a sha256 hash`);
  }
  if (!Array.isArray(request.shotIds) || request.shotIds.length < 1 || request.shotIds.length > TRACK_B_MEASURED_EVIDENCE_MAX_SHOTS) throw evidenceError('trackb-input-invalid', `shotIds must contain 1..${TRACK_B_MEASURED_EVIDENCE_MAX_SHOTS} entries`);
  const seen = new Set();
  for (const shotId of request.shotIds) {
    if (!isTrackBOpaqueId(shotId) || seen.has(shotId)) throw evidenceError('trackb-input-invalid', 'shotIds must be unique valid opaque IDs');
    seen.add(shotId);
  }
  if (request.receiptId !== undefined && !isTrackBOpaqueId(request.receiptId)) throw evidenceError('trackb-input-invalid', 'receiptId is invalid');
  return { timeoutMs, narrativeId: request.narrativeId, authorityHash: request.authorityHash, assetManifestHash: request.assetManifestHash, semanticManifestHash: request.semanticManifestHash, shotIds: request.shotIds, receiptId: request.receiptId };
}

function normalizeHtmlBytes(value) {
  const bytes = typeof value === 'string' ? Buffer.from(value, 'utf8') : Buffer.isBuffer(value) ? Buffer.from(value) : value instanceof Uint8Array ? Buffer.from(value) : null;
  if (!bytes || bytes.length === 0) throw evidenceError('trackb-input-invalid', 'exact self-contained HTML bytes are required');
  if (bytes.length > TRACK_B_MEASURED_EVIDENCE_MAX_HTML_BYTES) throw evidenceError('trackb-input-invalid', 'HTML exceeds the bounded measurement size');
  try {
    new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch (error) {
    throw evidenceError('trackb-input-invalid', 'HTML bytes must be valid UTF-8', { cause: error });
  }
  return bytes;
}

function normalizeViewport(viewport = {}) {
  const width = viewport.width ?? 1280;
  const height = viewport.height ?? 720;
  const deviceScaleFactor = viewport.deviceScaleFactor ?? 1;
  if (!Number.isSafeInteger(width) || width < 320 || width > 4096 || !Number.isSafeInteger(height) || height < 240 || height > 4096 || typeof deviceScaleFactor !== 'number' || !Number.isFinite(deviceScaleFactor) || deviceScaleFactor <= 0 || deviceScaleFactor > 4) {
    throw evidenceError('trackb-input-invalid', 'viewport dimensions are outside the bounded measurement range');
  }
  return { width, height, deviceScaleFactor };
}

async function verifyBrowserSemanticAnchor(client, expectedHash, expectedManifest) {
  const observed = await runtimeValue(client, `(() => {
    const elements = [...document.querySelectorAll('#${TRACKB_SEMANTIC_MANIFEST_ANCHOR}')];
    if (elements.length !== 1) return { count: elements.length };
    const element = elements[0];
    return {
      count: 1,
      attributes: Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value])),
      payload: element.textContent
    };
  })()`, 10_000);
  if (!observed || observed.count !== 1) throw evidenceError('trackb-evidence-incomplete', 'browser DOM must contain exactly one semantic manifest anchor');
  const attrs = observed.attributes;
  const expectedKeys = ['aria-hidden', 'encoding', 'hidden', 'id', 'payloadhash', 'semanticmanifesthash'];
  if (!attrs || Object.keys(attrs).sort().join('\0') !== expectedKeys.join('\0') || attrs.id !== TRACKB_SEMANTIC_MANIFEST_ANCHOR || attrs.hidden !== '' || attrs['aria-hidden'] !== 'true' || attrs.encoding !== TRACKB_SEMANTIC_MANIFEST_ENCODING) {
    throw evidenceError('trackb-evidence-incomplete', 'browser semantic manifest anchor attributes are invalid');
  }
  if (!isTrackBHash(attrs.semanticmanifesthash) || !isTrackBHash(attrs.payloadhash) || attrs.semanticmanifesthash !== expectedHash) {
    throw evidenceError('trackb-evidence-incomplete', 'browser semantic manifest hash attributes are invalid');
  }
  const payload = String(observed.payload ?? '').trim();
  if (!payload || !/^[A-Za-z0-9_-]+$/.test(payload) || payload.length % 4 === 1 || Buffer.from(payload, 'base64url').toString('base64url') !== payload) {
    throw evidenceError('trackb-evidence-incomplete', 'browser semantic manifest payload encoding is invalid');
  }
  const payloadBytes = Buffer.from(payload, 'base64url');
  if (sha256Prefixed(payloadBytes) !== attrs.payloadhash) throw evidenceError('trackb-evidence-incomplete', 'browser semantic manifest payload hash drifted');
  let manifest;
  try {
    manifest = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(payloadBytes));
  } catch (error) {
    throw evidenceError('trackb-evidence-incomplete', 'browser semantic manifest payload is not valid UTF-8 JSON', { cause: error });
  }
  if (canonicalizeJson(manifest) !== payloadBytes.toString('utf8') || manifest.semanticManifestHash !== expectedHash || canonicalizeJson(manifest) !== canonicalizeJson(expectedManifest)) {
    throw evidenceError('trackb-evidence-incomplete', 'browser DOM semantic manifest payload/hash drifted from exact HTML');
  }
  return { semanticManifestHash: expectedHash, semanticManifest: manifest };
}

async function readShotIds(client) {
  return runtimeValue(client, `([...document.querySelectorAll('[data-shot-id]')]).map((element) => element.getAttribute('data-shot-id'))`, 10_000);
}

function verifyShotIds(actual, expected) {
  if (!Array.isArray(actual) || actual.length !== expected.length || actual.some((id, index) => id !== expected[index]) || new Set(actual).size !== actual.length) {
    throw evidenceError('trackb-evidence-incomplete', 'browser data-shot-id occurrences must exactly match authority order');
  }
}

async function inspectShot(client, shotId, timeoutMs) {
  return runtimeValue(client, `(() => {
    const requested = ${JSON.stringify(shotId)};
    const shots = [...document.querySelectorAll('[data-shot-id]')];
    const active = shots.filter((element) => element.classList.contains('active'));
    const visible = shots.filter((element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== 'none' && style.visibility !== 'hidden' && rect.width > 0 && rect.height > 0 && element.classList.contains('active');
    });
    const rect = (element) => { const value = element?.getBoundingClientRect(); return value ? { x: value.x, y: value.y, width: value.width, height: value.height } : null; };
    const stage = document.querySelector('#stage');
    const selected = shots.find((element) => element.getAttribute('data-shot-id') === requested);
    return { activeShotCount: active.length, visibleShotCount: visible.length, activeShotId: active.length === 1 ? active[0].getAttribute('data-shot-id') : null, visibleShotId: visible.length === 1 ? visible[0].getAttribute('data-shot-id') : null, stageGeometry: rect(stage), shotGeometry: rect(selected) };
  })()`, timeoutMs);
}

async function waitDoubleRaf(client, timeoutMs) {
  await runtimeValue(client, 'new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))', timeoutMs);
}

async function waitForReady(client, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const state = await runtimeValue(client, 'document.readyState', Math.max(100, Math.min(2_000, deadline - Date.now())));
    if (state === 'interactive' || state === 'complete') return;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw evidenceError('trackb-evidence-timeout', 'measured HTML did not reach readyState before the deadline', { retryable: true });
}

async function captureScreenshot(client, timeoutMs) {
  const result = await client.send('Page.captureScreenshot', { format: 'png', fromSurface: true, captureBeyondViewport: false }, timeoutMs);
  if (typeof result.data !== 'string' || result.data.length === 0) throw evidenceError('trackb-evidence-incomplete', 'Chromium returned an empty screenshot');
  return Buffer.from(result.data, 'base64');
}

async function runtimeValue(client, expression, timeoutMs) {
  const result = await client.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, timeoutMs);
  if (result.exceptionDetails) throw evidenceError('trackb-evidence-incomplete', `browser runtime evaluation failed: ${JSON.stringify(result.exceptionDetails)}`);
  return result.result?.value;
}

async function assertDocumentUrlInvariant(client, expectedUrl, label, timeoutMs) {
  const frameTree = await client.send('Page.getFrameTree', {}, timeoutMs);
  const frameUrl = frameTree?.frameTree?.frame?.url;
  const runtimeUrl = await runtimeValue(client, 'String(globalThis.location.href)', timeoutMs);
  if (typeof frameUrl !== 'string' || typeof runtimeUrl !== 'string' || frameUrl.length === 0 || runtimeUrl.length === 0) {
    throw evidenceError('trackb-evidence-incomplete', `${label} final URL attestation is unavailable`);
  }
  if (frameUrl !== runtimeUrl || frameUrl !== expectedUrl) {
    throw evidenceError('trackb-evidence-incomplete', `${label} document URL changed: expected ${expectedUrl}, observed frame=${frameUrl}, runtime=${runtimeUrl}`);
  }
  return frameUrl;
}

function captureRuntimeCounts(logs) {
  return {
    network: logs.network.length,
    navigation: logs.navigation.length,
    exceptions: logs.exceptions.length,
    console: logs.console.length
  };
}

function runtimeResult(logs, semanticManifestVerified, baseline = { network: 0, navigation: 0, exceptions: 0, console: 0 }) {
  const shotLogs = {
    network: logs.network.slice(baseline.network),
    navigation: logs.navigation.slice(baseline.navigation),
    exceptions: logs.exceptions.slice(baseline.exceptions),
    console: logs.console.slice(baseline.console)
  };
  const classification = classifyRuntimeLogs(shotLogs);
  return {
    networkRequestCount: shotLogs.network.length,
    externalRequestCount: classification.externalRequestCount,
    navigationAttemptCount: shotLogs.navigation.length,
    exceptionCount: shotLogs.exceptions.length,
    semanticManifestVerified,
    pass: true
  };
}

function verifyRuntimeLogs(logs, label) {
  const classification = classifyRuntimeLogs(logs);
  if (classification.externalRequestCount > 0) throw evidenceError('trackb-evidence-incomplete', `${label} attempted forbidden network/navigation`);
  if (logs.exceptions.length > 0) throw evidenceError('trackb-evidence-incomplete', `${label} raised a runtime exception`);
  if (classification.consoleErrors.length > 0) throw evidenceError('trackb-evidence-incomplete', `${label} emitted a console error/assertion`);
}

function hasPositiveGeometry(value) {
  return value && Number.isFinite(value.x) && Number.isFinite(value.y) && Number.isFinite(value.width) && Number.isFinite(value.height) && value.width > 0 && value.height > 0;
}

function boundedOperationTimeout(timeoutMs) {
  return Math.max(100, Math.min(15_000, timeoutMs));
}

function evidenceError(code, message, options = {}) {
  return new TrackBMeasuredEvidenceError(code, message, options);
}

function mapRunnerError(error) {
  if (error instanceof TrackBMeasuredEvidenceError) return error;
  if (error?.code === 'E_CHROMIUM_ENDPOINT_TIMEOUT') return evidenceError('trackb-evidence-timeout', error.message, { cause: error, retryable: true });
  return evidenceError('trackb-evidence-incomplete', error?.message ?? 'Track B measured evidence failed', { cause: error });
}
