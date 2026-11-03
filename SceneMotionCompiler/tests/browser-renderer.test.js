import assert from 'node:assert/strict';
import { createServer } from 'node:net';
import { access, lstat } from 'node:fs/promises';
import test from 'node:test';
import {
  ChromiumEndpointError,
  DEFAULT_PAGE_ENDPOINT_TIMEOUT_MS,
  createPageEndpoint,
  createRendererLock,
  launchChromium,
} from '../packages/browser-renderer/src/chromium.js';

async function localEndpoint(onConnection = () => {}) {
  const sockets = new Set();
  const server = createServer((socket) => {
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    onConnection(socket);
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  assert.ok(address && typeof address === 'object');
  return {
    port: address.port,
    get activeSocketCount() { return sockets.size; },
    get listening() { return server.listening; },
    async close() {
      const socketClosures = [...sockets].map((socket) => new Promise((resolveClose) => {
        if (socket.destroyed) {
          resolveClose();
          return;
        }
        socket.once('close', resolveClose);
        socket.destroy();
      }));
      await Promise.all(socketClosures);
      await new Promise((resolveClose) => server.close(() => resolveClose()));
    }
  };
}

async function nonResponsiveEndpoint() {
  return localEndpoint(() => {
    // Deliberately keep the accepted TCP connection open without sending a
    // response. This is a local deterministic endpoint, not external I/O.
  });
}

async function partialBodyEndpoint() {
  return localEndpoint((socket) => {
    // Send valid HTTP headers and one complete chunk, but omit the terminating
    // zero-length chunk and leave the JSON document incomplete. This exercises
    // the response-body deadline after fetch has already received headers.
    const partialJson = '{"webSocketDebuggerUrl":"ws://127.0.0.1:9222/devtools/page/partial"';
    socket.write(
      'HTTP/1.1 200 OK\r\n'
      + 'Content-Type: application/json\r\n'
      + 'Transfer-Encoding: chunked\r\n'
      + 'Connection: keep-alive\r\n'
      + '\r\n'
      + `${Buffer.byteLength(partialJson)}\r\n${partialJson}\r\n`,
    );
  });
}

test('createPageEndpoint fails a nonresponsive local endpoint within its bound with a typed error', async () => {
  const endpoint = await nonResponsiveEndpoint();
  const startedAt = performance.now();
  try {
    await assert.rejects(
      () => createPageEndpoint(endpoint.port, 'about:blank', { timeoutMs: 150 }),
      (error) => {
        assert.ok(error instanceof ChromiumEndpointError);
        assert.equal(error.name, 'ChromiumEndpointError');
        assert.equal(error.code, 'E_CHROMIUM_ENDPOINT_TIMEOUT');
        assert.equal(error.details.port, endpoint.port);
        assert.equal(error.details.timeoutMs, 150);
        return true;
      },
    );
  } finally {
    await endpoint.close();
  }
  const elapsedMs = performance.now() - startedAt;
  assert.equal(endpoint.activeSocketCount, 0);
  assert.equal(endpoint.listening, false);
  assert.ok(elapsedMs >= 100, `endpoint failed before its minimum bound: ${elapsedMs}ms`);
  assert.ok(elapsedMs < 2_000, `endpoint deadline was not bounded: ${elapsedMs}ms`);
});

test('createPageEndpoint bounds a local endpoint that stalls after headers and a partial chunked body', async () => {
  const endpoint = await partialBodyEndpoint();
  const startedAt = performance.now();
  try {
    await assert.rejects(
      () => createPageEndpoint(endpoint.port, 'about:blank', { timeoutMs: 150 }),
      (error) => {
        assert.ok(error instanceof ChromiumEndpointError);
        assert.equal(error.name, 'ChromiumEndpointError');
        assert.equal(error.code, 'E_CHROMIUM_ENDPOINT_TIMEOUT');
        assert.equal(error.details.port, endpoint.port);
        assert.equal(error.details.timeoutMs, 150);
        return true;
      },
    );
  } finally {
    await endpoint.close();
  }
  const elapsedMs = performance.now() - startedAt;
  assert.equal(endpoint.activeSocketCount, 0);
  assert.equal(endpoint.listening, false);
  assert.ok(elapsedMs >= 100, `endpoint failed before its minimum bound: ${elapsedMs}ms`);
  assert.ok(elapsedMs < 2_000, `partial response deadline was not bounded: ${elapsedMs}ms`);
});

test('createPageEndpoint validates a bounded deadline before network I/O', async () => {
  assert.equal(DEFAULT_PAGE_ENDPOINT_TIMEOUT_MS, 10_000);
  await assert.rejects(
    () => createPageEndpoint(1, 'about:blank', { timeoutMs: 99 }),
    (error) => error?.code === 'E_CHROMIUM_ENDPOINT_CONFIG',
  );
  await assert.rejects(
    () => createPageEndpoint(1, 'about:blank', { timeoutMs: 60_001 }),
    (error) => error?.code === 'E_CHROMIUM_ENDPOINT_CONFIG',
  );
});

test('endpoint failure leaves the launch caller able to clean Chromium and its profile', async () => {
  const endpoint = await nonResponsiveEndpoint();
  let browser;
  let profilePath;
  let processId;
  try {
    const rendererLock = await createRendererLock();
    browser = await launchChromium(rendererLock);
    profilePath = browser.userDataDir;
    processId = browser.processHandle.pid;
    await assert.rejects(
      () => createPageEndpoint(endpoint.port, 'about:blank', { timeoutMs: 150 }),
      (error) => error?.code === 'E_CHROMIUM_ENDPOINT_TIMEOUT',
    );
  } finally {
    try { await browser?.close(); } finally { await endpoint.close(); }
  }
  assert.equal(endpoint.activeSocketCount, 0);
  assert.equal(endpoint.listening, false);
  assert.ok(processId);
  assert.notEqual(browser.processHandle.exitCode, null);
  await assert.rejects(() => access(profilePath), (error) => error?.code === 'ENOENT');
  assert.throws(() => process.kill(processId, 0), (error) => error?.code === 'ESRCH');
  await assert.rejects(() => lstat(profilePath), (error) => error?.code === 'ENOENT');
});
