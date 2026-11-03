import { randomBytes } from 'node:crypto';
import {
  materializeTrackBNavigationGuardSource,
  TRACK_B_NAVIGATION_GUARD_MECHANISM,
  TRACK_B_NAVIGATION_GUARD_SOURCE_HASH
} from '../../contracts/src/trackb-navigation-guard.js';

/** Apply one deterministic CDP setup and one shared evidence/log contract. */
export async function configureCdpSession(client, {
  rendererLock,
  viewport,
  logs,
  requireNavigationApi = true,
  navigationGuardMode = 'binding',
  timeoutMs = 10_000
}) {
  const executionContextIds = new Set();
  const removeExecutionContextListener = client.on('Runtime.executionContextCreated', (event) => {
    const id = event?.context?.id;
    if (Number.isSafeInteger(id)) executionContextIds.add(id);
  });
  attachRuntimeLogs(client, logs);
  await client.send('Page.enable');
  await client.send('Runtime.enable');
  await Promise.all([
    client.send('Network.enable'),
    client.send('Accessibility.enable')
  ]);
  const navigationApi = await runtimeValue(client, "typeof globalThis.navigation === 'object' && typeof globalThis.navigation.addEventListener === 'function'");
  if (requireNavigationApi && navigationApi !== true) throw new Error('Pinned Chromium Navigation API is unavailable; refusing to render without a navigation guard');
  let navigationGuard = null;
  if (navigationGuardMode === 'binding') {
    navigationGuard = await installTrustedNavigationGuard(client, { timeoutMs, knownExecutionContextIds: executionContextIds });
  } else {
    throw new Error('Only binding navigation guard mode is supported');
  }
  await client.send('Network.setBlockedURLs', { urls: ['http://*', 'https://*', 'ws://*', 'wss://*', 'file:*', 'file://*', 'ftp://*', 'ftp:*'] });
  await client.send('Emulation.setDeviceMetricsOverride', {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: viewport.deviceScaleFactor,
    mobile: false,
    screenWidth: viewport.width,
    screenHeight: viewport.height
  });
  await client.send('Emulation.setTimezoneOverride', { timezoneId: rendererLock.timezone });
  await client.send('Emulation.setLocaleOverride', { locale: rendererLock.locale });
  await client.send('Emulation.setEmulatedMedia', {
    features: [
      { name: 'prefers-color-scheme', value: rendererLock.colorScheme },
      { name: 'prefers-reduced-motion', value: rendererLock.reducedMotion }
    ]
  });
  try { removeExecutionContextListener?.(); } catch { /* context evidence listener is scoped to this CDP setup. */ }
  return { navigationApi, navigationGuard };
}

function createNavigationGuardCredentials() {
  return {
    bindingName: `__smcNavigationGuard_${randomBytes(16).toString('hex')}`,
    nonce: randomBytes(32).toString('hex')
  };
}

/**
 * Install the contract-owned navigation guard before the measured document
 * exists.  The binding name and nonce are intentionally generated per run;
 * the page can never learn the trusted closure that validates its reports.
 */
export async function installTrustedNavigationGuard(client, { timeoutMs = 10_000, knownExecutionContextIds = [] } = {}) {
  if (!client || typeof client.send !== 'function' || typeof client.on !== 'function') {
    throw new Error('CDP binding capability is unavailable');
  }
  const credentials = createNavigationGuardCredentials();
  const initialExecutionContextId = [...knownExecutionContextIds].at(-1);
  if (!Number.isSafeInteger(initialExecutionContextId)) {
    throw new Error('CDP execution-context attestation is unavailable');
  }
  const attestation = createTrustedNavigationGuardAttestation(client, { ...credentials, initialExecutionContextId });
  try {
    await client.send('Runtime.addBinding', { name: credentials.bindingName }, timeoutMs);
    const source = materializeTrackBNavigationGuardSource(credentials.bindingName, credentials.nonce);
    const registration = await client.send('Page.addScriptToEvaluateOnNewDocument', { source, runImmediately: true }, timeoutMs);
    if (typeof registration?.identifier !== 'string' || registration.identifier.length === 0) throw new Error('Pinned Chromium could not install the trusted navigation guard');
    return Object.freeze({
      mechanism: TRACK_B_NAVIGATION_GUARD_MECHANISM,
      sourceHash: TRACK_B_NAVIGATION_GUARD_SOURCE_HASH,
      bindingName: credentials.bindingName,
      initialExecutionContextId,
      runImmediately: true,
      waitForReady: attestation.waitForReady,
      waitForDocumentReady: attestation.waitForDocumentReady,
      assertHealthy: attestation.assertHealthy,
      drainAttempts: attestation.drainAttempts,
      snapshot: attestation.snapshot,
      dispose: attestation.dispose
    });
  } catch (error) {
    attestation.dispose();
    throw error;
  }
}

function createTrustedNavigationGuardAttestation(client, { bindingName, nonce, initialExecutionContextId }) {
  const readyMessages = [];
  const attempts = [];
  const waiters = new Set();
  const newExecutionContextIds = new Set();
  let failure = null;
  let documentReady = false;
  let expectedDocumentUrl;
  let drainedAttemptCount = 0;
  let disposed = false;

  const rejectWaiters = (error) => {
    for (const waiter of [...waiters]) {
      waiters.delete(waiter);
      clearTimeout(waiter.timer);
      waiter.reject(error);
    }
  };
  const fail = (error) => {
    if (failure) return;
    failure = error instanceof Error ? error : new Error(String(error));
    rejectWaiters(failure);
  };
  const resolveWaiters = () => {
    for (const waiter of [...waiters]) {
      try {
        const ready = findReadyMessage(readyMessages, waiter.afterExecutionContextId, waiter.expectedUrl, waiter.allowExistingContext && newExecutionContextIds.size === 0);
        if (!ready) continue;
        waiters.delete(waiter);
        clearTimeout(waiter.timer);
        documentReady = true;
        waiter.resolve(ready);
      } catch (error) {
        fail(error);
        return;
      }
    }
  };
  const unsubscribe = client.on('Runtime.bindingCalled', (event) => {
    if (disposed || event?.name !== bindingName) return;
    try {
      const message = validateNavigationGuardBindingEvent(event, { bindingName, nonce });
      if (!message) return;
      if (message.type === 'ready') readyMessages.push(message);
      else if (message.type === 'attempt') attempts.push(message);
      resolveWaiters();
    } catch (error) {
      fail(error);
    }
  });
  const unsubscribeExecutionContext = client.on('Runtime.executionContextCreated', (event) => {
    const id = event?.context?.id;
    if (Number.isSafeInteger(id) && id !== initialExecutionContextId) newExecutionContextIds.add(id);
    resolveWaiters();
  });

  return {
    waitForReady({ expectedUrl, timeoutMs = 10_000 } = {}) {
      if (failure) return Promise.reject(failure);
      try {
        const ready = findReadyMessage(readyMessages, undefined, expectedUrl, true);
        if (ready) {
          expectedDocumentUrl = expectedUrl;
          documentReady = true;
          return Promise.resolve(ready);
        }
      } catch (error) {
        fail(error);
        return Promise.reject(failure);
      }
      return new Promise((resolve, reject) => {
        const waiter = { afterExecutionContextId: undefined, expectedUrl, allowExistingContext: true, resolve, reject, timer: undefined };
        expectedDocumentUrl = expectedUrl;
        waiter.timer = setTimeout(() => {
          waiters.delete(waiter);
          reject(new Error('Trusted navigation guard ready attestation was not received for the measured document'));
        }, timeoutMs);
        waiters.add(waiter);
        resolveWaiters();
      });
    },
    waitForDocumentReady({ afterExecutionContextId, expectedUrl, allowExistingContext = false, timeoutMs = 10_000 } = {}) {
      if (!Number.isSafeInteger(afterExecutionContextId)) return Promise.reject(new Error('Document execution-context attestation is missing'));
      if (failure) return Promise.reject(failure);
      try {
        const ready = findReadyMessage(readyMessages, afterExecutionContextId, expectedUrl, allowExistingContext && newExecutionContextIds.size === 0);
        if (ready) {
          expectedDocumentUrl = expectedUrl;
          documentReady = true;
          return Promise.resolve(ready);
        }
      } catch (error) {
        fail(error);
        return Promise.reject(failure);
      }
      return new Promise((resolve, reject) => {
        const waiter = { afterExecutionContextId, expectedUrl, allowExistingContext, resolve, reject, timer: undefined };
        expectedDocumentUrl = expectedUrl;
        waiter.timer = setTimeout(() => {
          waiters.delete(waiter);
          reject(new Error('Trusted navigation guard ready attestation was not received for the measured document'));
        }, timeoutMs);
        waiters.add(waiter);
        resolveWaiters();
      });
    },
    assertHealthy() {
      if (failure) throw failure;
      if (!documentReady) throw new Error('Trusted navigation guard ready attestation is missing');
      for (const executionContextId of newExecutionContextIds) {
        const ready = readyMessages.find((message) => message.executionContextId === executionContextId);
        if (!ready) throw new Error(`Trusted navigation guard ready attestation is missing for execution context ${executionContextId}`);
        if (expectedDocumentUrl !== undefined && ready.url !== expectedDocumentUrl) throw new Error(`Trusted navigation guard ready URL mismatch: ${ready.url}`);
      }
    },
    drainAttempts() {
      const fresh = attempts.slice(drainedAttemptCount);
      drainedAttemptCount = attempts.length;
      return fresh.map((attempt) => ({ ...attempt }));
    },
    snapshot() {
      return {
        available: true,
        ready: documentReady,
        error: failure?.message ?? null,
        attempts: attempts.map((attempt) => ({ ...attempt })),
        readyMessages: readyMessages.map((message) => ({ ...message })),
        executionContextIds: [...newExecutionContextIds]
      };
    },
    dispose() {
      disposed = true;
      try { unsubscribe?.(); } catch { /* CDP cleanup is best effort before browser close. */ }
      try { unsubscribeExecutionContext?.(); } catch { /* CDP cleanup is best effort before browser close. */ }
      rejectWaiters(new Error('Trusted navigation guard attestation disposed'));
    }
  };
}

function findReadyMessage(messages, afterExecutionContextId, expectedUrl, allowExistingContext = false) {
  const candidates = afterExecutionContextId === undefined || allowExistingContext
    ? messages
    : messages.filter((message) => message.executionContextId !== afterExecutionContextId);
  const wrongUrl = expectedUrl === undefined ? null : candidates.find((message) => message.url !== expectedUrl);
  if (wrongUrl) throw new Error(`Trusted navigation guard ready URL mismatch: ${wrongUrl.url}`);
  return candidates.find((message) => expectedUrl === undefined || message.url === expectedUrl) ?? null;
}

/** Validate one CDP binding event without consulting page-owned state. */
export function validateNavigationGuardBindingEvent(event, { bindingName, nonce } = {}) {
  if (!event || event.name !== bindingName) return null;
  if (!Number.isSafeInteger(event.executionContextId)) throw new Error('Trusted navigation guard binding execution context is missing');
  if (typeof event.payload !== 'string' || event.payload.length === 0) throw new Error('Trusted navigation guard binding payload is missing');
  let message;
  try { message = JSON.parse(event.payload); } catch (error) { throw new Error(`Trusted navigation guard binding payload is invalid: ${error.message}`); }
  if (!message || typeof message !== 'object' || Array.isArray(message)) throw new Error('Trusted navigation guard binding payload must be an object');
  if (message.nonce !== nonce) throw new Error('Trusted navigation guard binding nonce mismatch');
  if (!['ready', 'attempt', 'error'].includes(message.type)) throw new Error('Trusted navigation guard binding message type is invalid');
  if (message.type === 'ready' && (typeof message.url !== 'string' || message.url.length === 0)) throw new Error('Trusted navigation guard ready attestation is invalid');
  if (message.type === 'attempt' && (typeof message.url !== 'string' || typeof message.kind !== 'string' || message.url.length === 0 || message.kind.length === 0)) throw new Error('Trusted navigation guard attempt attestation is invalid');
  if (message.type === 'error' && (typeof message.message !== 'string' || message.message.length === 0)) throw new Error(`Trusted navigation guard initialization failed: ${message.message ?? 'unknown error'}`);
  return { ...message, executionContextId: event.executionContextId };
}

export async function collectNavigationGuardLogs(client, logs, label, timeoutMs = 10_000, trustedNavigationGuard = null) {
  if (!trustedNavigationGuard) throw new Error(`${label} trusted navigation guard capability is unavailable`);
  trustedNavigationGuard.assertHealthy();
  for (const attempt of trustedNavigationGuard.drainAttempts()) {
    logs.navigation.push({ kind: 'guard', frameId: null, url: attempt.url, reason: attempt.kind });
  }
  return trustedNavigationGuard.snapshot();
}

export function createRuntimeLogs() {
  return { console: [], exceptions: [], network: [], blocked: [], navigation: [] };
}

function attachRuntimeLogs(client, logs) {
  client.on('Runtime.consoleAPICalled', (event) => logs.console.push(event));
  client.on('Runtime.exceptionThrown', (event) => logs.exceptions.push(event));
  client.on('Network.requestWillBeSent', (event) => logs.network.push({ url: event.request?.url, method: event.request?.method }));
  client.on('Network.loadingFailed', (event) => {
    if (event.blockedReason || event.errorText) logs.blocked.push({ requestId: event.requestId, blockedReason: event.blockedReason, errorText: event.errorText });
  });
  client.on('Page.frameRequestedNavigation', (event) => logs.navigation.push({ kind: 'requested', frameId: event.frameId, url: event.url, reason: event.reason, disposition: event.disposition }));
  client.on('Page.frameNavigated', (event) => logs.navigation.push({ kind: 'navigated', frameId: event.frame?.id, url: event.frame?.url }));
  client.on('Page.navigatedWithinDocument', (event) => logs.navigation.push({ kind: 'navigated-within-document', frameId: event.frameId, url: event.url }));
  client.on('Page.windowOpen', (event) => logs.navigation.push({ kind: 'window-open', frameId: null, url: event.url ?? 'forbidden-navigation', windowId: event.windowId ?? null, windowName: event.windowName ?? null }));
}

export function classifyRuntimeLogs(logs, { allowedOrigin = null } = {}) {
  const externalRequests = logs.network.filter((item) => isForbiddenNetworkUrl(item.url, allowedOrigin));
  const navigationMap = new Map();
  for (const item of logs.navigation) {
    if (item.kind === 'window-open') {
      const key = `window\u0000${item.windowId ?? ''}\u0000${item.url ?? ''}`;
      navigationMap.set(key, item);
      continue;
    }
    if (!isForbiddenNavigationUrl(item.url)) continue;
    const key = `${item.frameId ?? ''}\u0000${item.url}`;
    navigationMap.set(key, item);
  }
  const forbiddenNavigations = [...navigationMap.values()];
  const consoleErrors = logs.console.filter((item) => ['error', 'assert'].includes(item.type));
  return {
    externalRequests,
    forbiddenNavigations,
    consoleErrors,
    externalRequestCount: externalRequests.length + forbiddenNavigations.length
  };
}

function isForbiddenNetworkUrl(value, allowedOrigin) {
  if (!value || /^(?:about:blank|data:)/i.test(value)) return false;
  if (/^(?:file|ftp):/i.test(value)) return true;
  if (!/^(?:https?|wss?):/i.test(value)) return true;
  if (!allowedOrigin) return true;
  try { return new URL(value).origin !== allowedOrigin; } catch { return true; }
}

function isForbiddenNavigationUrl(value) {
  if (!value || /^(?:about:blank|about:srcdoc|data:)/i.test(value)) return false;
  return true;
}

async function runtimeValue(client, expression, timeoutMs) {
  const result = await client.send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }, timeoutMs);
  if (result.exceptionDetails) throw new Error(`Runtime evaluation failed: ${JSON.stringify(result.exceptionDetails)}`);
  return result.result?.value;
}
