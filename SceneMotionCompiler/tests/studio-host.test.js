import assert from 'node:assert/strict';
import { request as httpRequest } from 'node:http';
import { createConnection } from 'node:net';
import { mkdtemp, writeFile, readFile, realpath, rm, symlink } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { createStudioServer } from '../apps/studio-host/src/server.js';
import { canonicalizeJson } from '../packages/core/src/canonical.js';
import { hashTrackBRawBytes } from '../packages/contracts/src/trackb-contract.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import { validateTrackBApplicationEnvelope } from '../packages/contracts/src/trackb-contract.js';
import { CLI_COMMAND_REGISTRY, MCP_COMMAND_REGISTRY } from '../packages/application/src/command-registry.js';

const FIXED_NOW = Date.parse('2026-08-28T00:00:01Z');
const APPROVAL_INPUT = Object.freeze({
  narrativeId: 'narrative.demo',
  candidateHash: 'sha256:31b0422f01d8b0547ca679d57f057a7c07397225832b9cee12d034549b998fd7',
  baseDeliverySetHash: 'NONE',
  decision: 'REVISE',
  reason: 'reviewed by Studio',
  decidedAt: '2026-08-28T00:00:00Z'
});
const EXPECTED_SCENE_OUTPUT_CSP = "sandbox allow-scripts; default-src 'none'; connect-src 'none'; form-action 'none'; object-src 'none'; frame-src 'none'; child-src 'none'; worker-src 'none'; media-src 'none'; font-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'";

function assertNoStore(response) {
  assert.equal(response.headers.get('cache-control'), 'no-store');
}

function rawHttpRequest(url, { method = 'POST', headers = {}, body = '' } = {}) {
  return new Promise((resolve, reject) => {
    const request = httpRequest(url, { method, headers }, (response) => {
      const chunks = [];
      response.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
      response.on('end', () => resolve({
        status: response.statusCode,
        headers: response.headers,
        body: Buffer.concat(chunks).toString('utf8')
      }));
    });
    request.once('error', reject);
    request.end(body);
  });
}

function rawIncompleteHttpRequest(url, {
  headers = {},
  headerLines = null,
  bodyPrefix = '{',
  timeoutMs = 1000,
  waitForClose = false,
  onResponseHeaders = null
} = {}) {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const socket = createConnection({ host: parsed.hostname, port: Number(parsed.port) });
    let raw = '';
    let settled = false;
    let responseHeadSeen = false;
    const finish = (value, error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error || !waitForClose) socket.destroy();
      if (error) reject(error);
      else resolve(value);
    };
    const timer = setTimeout(() => {
      finish(undefined, new Error(`timed out waiting for an early response after ${timeoutMs}ms`));
    }, timeoutMs);
    socket.once('error', (error) => finish(undefined, error));
    socket.on('data', (chunk) => {
      raw += Buffer.from(chunk).toString('latin1');
      if (!responseHeadSeen && raw.includes('\r\n\r\n')) {
        responseHeadSeen = true;
        onResponseHeaders?.(raw);
        if (!waitForClose) finish(raw);
      }
    });
    socket.once('close', () => {
      if (waitForClose) {
        finish(responseHeadSeen ? raw : undefined, responseHeadSeen ? undefined : new Error('socket closed before response'));
      }
    });
    socket.once('connect', () => {
      const requestLines = [
        `POST ${parsed.pathname}${parsed.search} HTTP/1.1`,
        `Host: ${parsed.host}`,
        ...(headerLines ?? Object.entries(headers).map(([key, value]) => `${key}: ${value}`)),
        '',
        ''
      ];
      socket.write(requestLines.join('\r\n'));
      socket.write(bodyPrefix);
    });
  });
}

async function withServer(run) {
  const server = createStudioServer({
    port: 0,
    projectRoot: process.cwd(),
    repoRoot: process.cwd(),
    localOperatorId: 'studio.test'
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const { port } = server.address();
  try {
    return await run(`http://127.0.0.1:${port}`);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  }
}

test('Studio rejects untrusted origins without wildcard CORS', async () => {
  await withServer(async (origin) => {
    const response = await fetch(`${origin}/api/metadata`, { headers: { Origin: 'https://evil.example' } });
    assert.equal(response.status, 403);
    assert.equal((await response.json()).error.code, 'origin-forbidden');
    assert.equal(response.headers.get('access-control-allow-origin'), null);

    const metadataResponse = await fetch(`${origin}/api/metadata`);
    const metadata = await metadataResponse.json();
    assert.equal(metadataResponse.status, 200);
    assert.equal(metadataResponse.headers.get('cache-control'), 'no-store');
    assert.equal(metadata.previewAvailable, false);
    assert.ok(metadata.commands['trackb.release.approve']);
    assert.deepEqual(Object.keys(metadata.commands).sort(), ['scene.compileHtml', 'trackb.release.approve', 'trackb.release.status']);
  });
});

async function withApprovalServer(run, overrides = {}) {
  const projectRoot = await mkdtemp(join(tmpdir(), 'smc-studio-trackb-'));
  const fixture = await createApprovalFixture(projectRoot);
  let now = FIXED_NOW;
  const server = createStudioServer({
    port: 0,
    projectRoot,
    repoRoot: projectRoot,
    localOperatorId: 'operator.studio',
    clock: () => now,
    trackBRuntimeDependencies: fixture.runtimeDependencies,
    ...overrides
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  const { port } = server.address();
  const origin = `http://127.0.0.1:${port}`;
  try {
    return await run({ origin, fixture, setNow: (value) => { now = value; } });
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
    await rm(projectRoot, { recursive: true, force: true });
  }
}

test('Studio exposes only its canonical command allowlist', async () => {
  await withServer(async (origin) => {
    const response = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ command: 'workspace.chat', input: {} })
    });
    assert.equal(response.status, 403);
    assert.equal((await response.json()).error.code, 'studio-command-forbidden');
  });
});

test('Studio serves generated media artifacts without exposing source files', async () => {
  const projectRoot = await mkdtemp(join(tmpdir(), 'smc-studio-artifacts-'));
  try {
    await writeFile(join(projectRoot, 'scene-output.html'), '<!doctype html><title>scene</title>');
    await writeFile(join(projectRoot, 'evil.html'), '<!doctype html><script>fetch("/api/review-session")</script>');
    await writeFile(join(projectRoot, 'preview.png'), 'not an image');
    await writeFile(join(projectRoot, 'preview.mp4'), 'not a video');
    await writeFile(join(projectRoot, 'preview.webm'), 'not a video');
    await writeFile(join(projectRoot, 'secret.txt'), 'not public');
    const server = createStudioServer({ port: 0, projectRoot, repoRoot: projectRoot, localOperatorId: 'studio.test' });
    await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
    const { port } = server.address();
    try {
      const artifact = await fetch(`http://127.0.0.1:${port}/scene-output.html`);
      assert.equal(artifact.status, 200);
      assert.equal(artifact.headers.get('content-type'), 'text/html; charset=utf-8');
      assert.equal(artifact.headers.get('cache-control'), 'no-store');
      assert.equal(artifact.headers.get('content-security-policy'), EXPECTED_SCENE_OUTPUT_CSP);
      assert.equal(artifact.headers.get('referrer-policy'), 'no-referrer');
      assert.equal(artifact.headers.get('x-content-type-options'), 'nosniff');
      assert.match(await artifact.text(), /<title>scene<\/title>/);
      const availableMetadata = await fetch(`http://127.0.0.1:${port}/api/metadata`).then((response) => response.json());
      assert.equal(availableMetadata.previewAvailable, true);
      for (const path of ['evil.html', 'preview.png', 'preview.mp4', 'preview.webm']) {
        assert.equal((await fetch(`http://127.0.0.1:${port}/${path}`)).status, 404, path);
      }
      assert.equal((await fetch(`http://127.0.0.1:${port}/secret.txt`)).status, 404);
      assert.equal((await fetch(`http://127.0.0.1:${port}/%2e%2e%2fsecret.txt`)).status, 404);

      await rm(join(projectRoot, 'scene-output.html'));
      await writeFile(join(projectRoot, 'scene-output.html'), '<!doctype html><form action="/api/review-session"><input autofocus onfocus="fetch(\'/api/review-session\')"></form>');
      const malicious = await fetch(`http://127.0.0.1:${port}/scene-output.html`);
      assert.equal(malicious.status, 200);
      assert.equal(malicious.headers.get('content-security-policy'), EXPECTED_SCENE_OUTPUT_CSP);
      assert.equal(malicious.headers.get('referrer-policy'), 'no-referrer');
      assert.equal(malicious.headers.get('x-content-type-options'), 'nosniff');
      assert.match(await malicious.text(), /fetch\('\/api\/review-session'\)/);

      await rm(join(projectRoot, 'scene-output.html'));
      await symlink(join(projectRoot, 'evil.html'), join(projectRoot, 'scene-output.html'));
      assert.equal((await fetch(`http://127.0.0.1:${port}/scene-output.html`)).status, 403);
      const symlinkMetadata = await fetch(`http://127.0.0.1:${port}/api/metadata`).then((response) => response.json());
      assert.equal(symlinkMetadata.previewAvailable, false);
    } finally {
      await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
    }
  } finally {
    await rm(projectRoot, { recursive: true, force: true });
  }
});

test('Studio rejects oversized command bodies before JSON parsing', async () => {
  await withServer(async (origin) => {
    const response = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: Buffer.alloc(8 * 1024 * 1024 + 1, 0x78)
    });
    assert.equal(response.status, 413);
    assertNoStore(response);
    assert.equal((await response.json()).error.code, 'request-too-large');
  });
});

test('Studio review session issues a bounded HttpOnly cookie and dispatches only a valid Track B approval', async () => {
  await withApprovalServer(async ({ origin, fixture }) => {
    const sessionResponse = await fetch(`${origin}/api/review-session`, {
      headers: { Origin: origin }
    });
    assert.equal(sessionResponse.status, 200);
    assert.equal(sessionResponse.headers.get('cache-control'), 'no-store');
    assert.equal(sessionResponse.headers.get('access-control-allow-origin'), origin);
    assert.equal(sessionResponse.headers.get('access-control-allow-credentials'), 'true');
    const sessionBody = await sessionResponse.json();
    assert.equal(sessionBody.ok, true);
    assert.equal(sessionBody.expiresAt, new Date(FIXED_NOW + 900_000).toISOString());
    const setCookie = sessionResponse.headers.get('set-cookie');
    assert.match(setCookie ?? '', /^smc_trackb_review_session=[A-Za-z0-9_-]{43}; HttpOnly; SameSite=Strict; Path=\/api\/execute; Max-Age=900$/);
    const token = setCookie.split(';', 1)[0].split('=', 2)[1];
    assert.equal(JSON.stringify(sessionBody).includes(token), false);

    const response = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: {
        Origin: origin,
        'Content-Type': 'application/json',
        Cookie: setCookie.split(';', 1)[0]
      },
      body: JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT })
    });
    assert.equal(response.status, 200);
    assertNoStore(response);
    assert.equal(response.headers.get('access-control-allow-origin'), origin);
    assert.equal(response.headers.get('access-control-allow-credentials'), 'true');
    const envelope = await response.json();
    assert.equal(envelope.ok, true);
    assert.equal(envelope.command, 'trackb.release.approve');
    assert.equal(envelope.result.decision, 'REVISE');
    assert.equal(envelope.result.subject.candidateHash, fixture.candidate.candidateHash);
    assert.equal(JSON.stringify(envelope).includes(token), false);
    assert.equal(JSON.stringify(fixture.events).includes(token), false);
    assert.equal(fixture.writes.length, 1);
    const approvalReceipt = JSON.parse(fixture.writes[0].bytes.toString('utf8'));
    assert.deepEqual(approvalReceipt.actor, { actorId: 'operator.studio', actorKind: 'human' });
    assert.equal(approvalReceipt.origin, 'studio');

    const replay = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: {
        Origin: origin,
        'Content-Type': 'application/json',
        Cookie: setCookie.split(';', 1)[0]
      },
      body: JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT })
    });
    assert.equal(replay.status, 403);
    assertNoStore(replay);
    assert.equal((await replay.json()).error.code, 'trackb-approval-forbidden');
    assert.equal(fixture.writes.length, 1);
  });
});

test('Studio accepts a browser-shaped same-origin GET without Origin', async () => {
  await withServer(async (origin) => {
    const response = await fetch(`${origin}/api/review-session`, {
      headers: {
        Referer: `${origin}/studio`,
        'Sec-Fetch-Site': 'same-origin',
        'Sec-Fetch-Mode': 'cors',
        'Sec-Fetch-Dest': 'empty'
      }
    });
    assert.equal(response.status, 200);
    assertNoStore(response);
    assert.equal(response.headers.get('access-control-allow-origin'), null);
    assert.match(response.headers.get('set-cookie') ?? '', /^smc_trackb_review_session=/);
    assert.equal((await response.json()).ok, true);
  });
});

test('Studio rejects incomplete same-origin browser metadata without Origin', async () => {
  await withServer(async (origin) => {
    const cases = [
      { Referer: `${origin}/studio`, 'Sec-Fetch-Site': 'cross-site' },
      { Referer: `${origin}/studio` },
      { Referer: 'https://evil.example/studio', 'Sec-Fetch-Site': 'same-origin' },
      { Referer: 'not-a-url', 'Sec-Fetch-Site': 'same-origin' }
    ];
    for (const headers of cases) {
      const response = await fetch(`${origin}/api/review-session`, { headers });
      assert.equal(response.status, 403, JSON.stringify(headers));
      assertNoStore(response);
      assert.equal((await response.json()).error.code, 'trackb-approval-forbidden');
    }
  });
});

test('Studio dashboard bootstraps a same-origin review session before every browser execute', async () => {
  const source = await readFile(join(process.cwd(), 'apps/studio-host/src/index.html'), 'utf8');
  assert.match(source, /fetch\('\/api\/review-session'/);
  assert.match(source, /method:'POST'/);
  assert.match(source, /credentials:'same-origin'/);
  for (const field of ['narrative', 'candidate', 'base', 'decision', 'reason']) assert.match(source, new RegExp(`id="${field}"`));
  assert.match(source, /exec\('trackb\.release\.approve'/);
  for (const retired of ['scene.approveNarrative', 'scene.approvalStatus', 'approveNarrative', 'approvalStatus']) {
    assert.equal(source.includes(retired), false, `Studio dashboard must not expose retired ${retired}`);
  }
});

test('Studio compileHtml defaults to the v2 SceneMotionAssetHandoff path', async () => {
  const source = await readFile(join(process.cwd(), 'apps/studio-host/src/index.html'), 'utf8');
  assert.match(source, /id="handoff"\s+value="handoff\/scene-motion-asset-handoff\.v2\.json"/);
  assert.doesNotMatch(source, /scene-motion-asset-handoff\.v1\.json/);
  assert.match(source, /exec\('scene\.compileHtml'/);
});

test('Studio dashboard makes the novice preview flow primary and keeps diagnostics advanced', async () => {
  const source = await readFile(join(process.cwd(), 'apps/studio-host/src/index.html'), 'utf8');
  assert.match(source, /<html lang="ko">/);
  assert.match(source, /<button id="compile"[^>]*>장면 미리보기 만들기<\/button>/);
  assert.match(source, /id="compile-status"[^>]*role="status"[^>]*aria-live="polite"/);
  assert.match(source, /id="compile-success"[^>]*hidden/);
  assert.match(source, /id="compile-error"[^>]*role="alert"[^>]*aria-live="assertive"[^>]*hidden/);
  assert.match(source, /id="output-link"[^>]*href="\/scene-output\.html"[^>]*target="_blank"/);
  assert.match(source, /function renderExistingPreview\(\)/);
  assert.match(source, /\[hidden\]\s*\{\s*display:\s*none\s*!important;\s*\}/);
  assert.match(source, /metadata\?\.previewAvailable === true/);
  assert.match(source, /기존 미리보기를 바로 열거나 새로 만들 수 있습니다/);
  assert.match(source, /id="advanced-details"[^>]*>[\s\S]*<summary>고급 도구와 진단<\/summary>/);
  assert.match(source, /id="handoff"[^>]*readonly/);
  assert.match(source, /sceneMotionAssetHandoffPath: CANONICAL_HANDOFF/);
  assert.match(source, /outputPath: 'scene-output\.html'/);
  assert.match(source, /compileButton\.disabled = true/);
  assert.match(source, /미리보기를 만드는 중/);
  assert.match(source, /renderCompileSuccess/);
  assert.match(source, /renderCompileFailure/);
  assert.match(source, /shotCount/);
  assert.match(source, /characterCount/);
  assert.match(source, /environmentCount/);
  assert.match(source, /SceneAssetCompiler에서 장면 이미지와 필요한 자산을 검토한 뒤 승인·게시/);
  const advancedStart = source.indexOf('<details id="advanced-details"');
  const commandMetadataStart = source.indexOf('<pre id="commands">');
  assert.ok(advancedStart >= 0 && commandMetadataStart > advancedStart, 'raw command metadata must be inside advanced details');
});

test('Studio rejects spoofed context, malformed body, content type, origin, and cookie shapes', async () => {
  await withApprovalServer(async ({ origin, fixture }) => {
    const sessionResponse = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    const setCookie = sessionResponse.headers.get('set-cookie');
    const cookie = setCookie.split(';', 1)[0];
    const post = (body, headers = {}) => fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: cookie, ...headers },
      body
    });

    const missingOriginSession = await fetch(`${origin}/api/review-session`);
    assert.equal(missingOriginSession.status, 403);
    assertNoStore(missingOriginSession);
    assert.equal((await missingOriginSession.json()).error.code, 'trackb-approval-forbidden');

    const missingSession = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: '{'
    });
    assert.equal(missingSession.status, 403);
    assertNoStore(missingSession);
    assert.equal((await missingSession.json()).error.code, 'trackb-approval-forbidden');

    const spoofed = await post(JSON.stringify({
      command: 'trackb.release.approve',
      input: { ...APPROVAL_INPUT, actorId: 'attacker' }
    }));
    assert.equal(spoofed.status, 403);
    assertNoStore(spoofed);
    assert.equal((await spoofed.json()).error.code, 'trackb-approval-forbidden');

    const malformed = await post('{');
    assert.equal(malformed.status, 400);
    assertNoStore(malformed);
    const malformedBody = await malformed.json();
    assert.equal(validateTrackBApplicationEnvelope(malformedBody, { command: 'trackb.release.approve' }).valid, true);
    assert.equal(malformedBody.ok, false);
    assert.equal(malformedBody.error.code, 'trackb-input-invalid');
    assert.equal(malformedBody.error.retryable, false);

    const invalidUtf8 = await post(Buffer.from([0xff]));
    assert.equal(invalidUtf8.status, 400);
    assertNoStore(invalidUtf8);
    const invalidUtf8Body = await invalidUtf8.json();
    assert.equal(validateTrackBApplicationEnvelope(invalidUtf8Body, { command: 'trackb.release.approve' }).valid, true);
    assert.equal(invalidUtf8Body.ok, false);
    assert.equal(invalidUtf8Body.error.code, 'trackb-input-invalid');
    assert.equal(invalidUtf8Body.error.retryable, false);

    const wrongContentType = await post(JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT }), {
      'Content-Type': 'application/json; charset=utf-8'
    });
    assert.equal(wrongContentType.status, 400);
    assertNoStore(wrongContentType);
    assert.equal((await wrongContentType.json()).error.code, 'trackb-input-invalid');

    const invalidJsonWrongContentType = await post('{', { 'Content-Type': 'text/plain' });
    assert.equal(invalidJsonWrongContentType.status, 400);
    assertNoStore(invalidJsonWrongContentType);
    const invalidJsonWrongContentTypeBody = await invalidJsonWrongContentType.json();
    assert.equal(invalidJsonWrongContentTypeBody.error.code, 'trackb-input-invalid');
    assert.match(invalidJsonWrongContentTypeBody.error.message, /content-type/);

    const wrongOrigin = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: 'http://127.0.0.1:1', 'Content-Type': 'application/json', Cookie: cookie },
      body: '{'
    });
    assert.equal(wrongOrigin.status, 403);
    assertNoStore(wrongOrigin);
    assert.equal((await wrongOrigin.json()).error.code, 'trackb-approval-forbidden');

    const duplicateCookie = await post('{', {
      Cookie: `${cookie}; ${cookie}`
    });
    assert.equal(duplicateCookie.status, 403);
    assertNoStore(duplicateCookie);
    assert.equal((await duplicateCookie.json()).error.code, 'trackb-approval-forbidden');
    assert.equal(fixture.writes.length, 0);
  });
});

test('Studio rejects Track B candidates before parsing when session admission fails', async () => {
  await withApprovalServer(async ({ origin }) => {
    const noCookieMalformed = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: '{'
    });
    assert.equal(noCookieMalformed.status, 403);
    assertNoStore(noCookieMalformed);
    assert.equal((await noCookieMalformed.json()).error.code, 'trackb-approval-forbidden');

    const noCookieInvalidUtf8 = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: Buffer.from([0xff])
    });
    assert.equal(noCookieInvalidUtf8.status, 403);
    assertNoStore(noCookieInvalidUtf8);
    assert.equal((await noCookieInvalidUtf8.json()).error.code, 'trackb-approval-forbidden');

    const noCookieOversized = await rawIncompleteHttpRequest(`${origin}/api/execute`, {
      headers: {
        Origin: origin,
        'Content-Type': 'application/json',
        'Content-Length': String(8 * 1024 * 1024 + 1),
        Connection: 'close'
      },
      bodyPrefix: '{',
      timeoutMs: 1000
    });
    assert.match(noCookieOversized, /^HTTP\/1\.1 403 /);
    assert.match(noCookieOversized, /\r\ncache-control: no-store\r\n/i);
    assert.match(noCookieOversized, /"code":"trackb-approval-forbidden"/);

    const malformedCookie = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: 'smc_trackb_review_session=not-a-token'
      },
      body: '{'
    });
    assert.equal(malformedCookie.status, 403);
    assertNoStore(malformedCookie);
    assert.equal((await malformedCookie.json()).error.code, 'trackb-approval-forbidden');
  });
});

test('Studio handles review-session methods as an explicit Track B route', async () => {
  await withServer(async (origin) => {
    for (const method of ['POST', 'PUT', 'DELETE']) {
      const response = await fetch(`${origin}/api/review-session`, {
        method,
        headers: { Origin: origin },
        ...(method === 'POST' ? { body: '{' } : {})
      });
      assert.equal(response.status, 403);
      assertNoStore(response);
      assert.equal((await response.json()).error.code, 'trackb-approval-forbidden');
    }

    const options = await fetch(`${origin}/api/review-session`, {
      method: 'OPTIONS',
      headers: {
        Origin: origin,
        'Access-Control-Request-Method': 'GET'
      }
    });
    assert.equal(options.status, 204);
    assertNoStore(options);
    assert.equal(options.headers.get('access-control-allow-origin'), origin);

    const invalidOptions = await fetch(`${origin}/api/review-session`, {
      method: 'OPTIONS',
      headers: { 'Access-Control-Request-Method': 'GET' }
    });
    assert.equal(invalidOptions.status, 403);
    assertNoStore(invalidOptions);
    assert.equal((await invalidOptions.json()).error.code, 'trackb-approval-forbidden');
  });
});

test('Studio applies the Track B 8 MiB bound before parsing and returns the locked input error', async () => {
  await withApprovalServer(async ({ origin }) => {
    const sessionResponse = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    const cookie = sessionResponse.headers.get('set-cookie').split(';', 1)[0];
    const response = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: cookie },
      body: Buffer.alloc(8 * 1024 * 1024 + 1, 0x78)
    });
    assert.equal(response.status, 413);
    assertNoStore(response);
    assert.equal((await response.json()).error.code, 'trackb-input-invalid');
  });
});

test('Studio rejects a known oversized Content-Length before reading or parsing the body', async () => {
  await withApprovalServer(async ({ origin }) => {
    const response = await rawHttpRequest(`${origin}/api/execute`, {
      headers: {
        Origin: origin,
        'Content-Type': 'application/json',
        'Content-Length': String(8 * 1024 * 1024 + 1),
        Connection: 'close'
      },
      body: '{'
    });
    assert.equal(response.status, 403);
    assert.equal(response.headers['cache-control'], 'no-store');
    const body = JSON.parse(response.body);
    assert.equal(body.error.code, 'trackb-approval-forbidden');
    assert.doesNotMatch(body.error.message, /exceeds 8 MiB/);
  });
});

test('Studio rejects an unclassified incomplete oversized request before body completion', async () => {
  await withServer(async (origin) => {
    const raw = await rawIncompleteHttpRequest(`${origin}/api/execute`, {
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': String(8 * 1024 * 1024 + 1),
        Connection: 'close'
      },
      bodyPrefix: '{',
      timeoutMs: 1000
    });
    assert.match(raw, /^HTTP\/1\.1 413 /);
    assert.match(raw, /\r\ncache-control: no-store\r\n/i);
    assert.match(raw, /request-too-large/);
  });
});

test('Studio closes an incomplete oversized request without requiring Connection close', async () => {
  const server = createStudioServer({
    port: 0,
    projectRoot: process.cwd(),
    repoRoot: process.cwd(),
    localOperatorId: 'studio.test'
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  let closeStarted = false;
  let closeResult;
  let resolveClose;
  const closePromise = new Promise((resolve) => { resolveClose = resolve; });
  let closeTimer;
  try {
    const rawPromise = rawIncompleteHttpRequest(`${origin}/api/execute`, {
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': String(8 * 1024 * 1024 + 1)
      },
      bodyPrefix: '{',
      timeoutMs: 1000,
      waitForClose: true,
      onResponseHeaders: () => {
        if (closeStarted) return;
        closeStarted = true;
        const startedAt = Date.now();
        server.close((error) => {
          closeResult = { error, elapsedMs: Date.now() - startedAt };
          resolveClose();
        });
      }
    });
    const raw = await rawPromise;
    await Promise.race([
      closePromise,
      new Promise((_, reject) => {
        closeTimer = setTimeout(() => reject(new Error('server.close waited for the rejected request')), 500);
      })
    ]);
    assert.equal(closeResult?.error, undefined);
    assert.ok((closeResult?.elapsedMs ?? Infinity) < 500);
    assert.match(raw, /^HTTP\/1\.1 413 /);
    assert.match(raw, /\r\ncache-control: no-store\r\n/i);
    assert.match(raw, /\r\nconnection: close\r\n/i);
    assert.match(raw, /request-too-large/);
  } finally {
    clearTimeout(closeTimer);
    if (!closeStarted) {
      await new Promise((resolve) => server.close(() => resolve()));
    }
  }
});

test('Studio classifies malformed Content-Length as a bounded no-store error', async () => {
  await withServer(async (origin) => {
    const raw = await rawIncompleteHttpRequest(`${origin}/api/execute`, {
      headers: {
        'Content-Type': 'application/json',
        'Content-Length': '1.5',
        Connection: 'close'
      },
      bodyPrefix: '{',
      timeoutMs: 1000
    });
    assert.match(raw, /^HTTP\/1\.1 400 /);
    assert.match(raw, /\r\ncache-control: no-store\r\n/i);
    assert.match(raw, /bad-request/);
  });
});

test('Studio admits raw Track B framing errors only for a live session and never consumes it', async () => {
  await withApprovalServer(async ({ origin }) => {
    const unauthorizedCases = [
      [
        `Origin: ${origin}`,
        'Content-Type: application/json',
        'Content-Length: 1',
        'Content-Length: 1',
        'Connection: close'
      ],
      [
        `Origin: ${origin}`,
        'Cookie: smc_trackb_review_session=marker',
        'Content-Type: application/json',
        'Content-Length: 1',
        'Transfer-Encoding: chunked',
        'Connection: close'
      ],
      [
        `Origin: ${origin}`,
        'Cookie: smc_trackb_review_session=marker',
        'Cookie: smc_trackb_review_session=marker',
        'Content-Type: application/json',
        'Content-Length: 1',
        'Content-Length: 1',
        'Connection: close'
      ]
    ];
    for (const headerLines of unauthorizedCases) {
      const raw = await rawIncompleteHttpRequest(`${origin}/api/execute`, {
        headerLines,
        bodyPrefix: '{',
        timeoutMs: 1000
      });
      assert.match(raw, /^HTTP\/1\.1 403 /);
      assert.match(raw, /\r\ncache-control: no-store\r\n/i);
      const body = raw.slice(raw.indexOf('\r\n\r\n') + 4);
      const envelope = JSON.parse(body);
      assert.equal(envelope.command, 'trackb.release.approve');
      assert.equal(envelope.error.code, 'trackb-approval-forbidden');
      assert.equal(envelope.error.retryable, false);
    }

    const sessionResponse = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    const cookie = sessionResponse.headers.get('set-cookie').split(';', 1)[0];
    const framingCases = [
      [
        `Origin: ${origin}`,
        `Cookie: ${cookie}`,
        'Content-Type: application/json',
        'Content-Length: 1',
        'Content-Length: 1',
        'Connection: close'
      ],
      [
        `Origin: ${origin}`,
        `Cookie: ${cookie}`,
        'Content-Type: application/json',
        'Content-Length: 1',
        'Transfer-Encoding: chunked',
        'Connection: close'
      ]
    ];
    for (const headerLines of framingCases) {
      const raw = await rawIncompleteHttpRequest(`${origin}/api/execute`, {
        headerLines,
        bodyPrefix: '{',
        timeoutMs: 1000
      });
      assert.match(raw, /^HTTP\/1\.1 400 /);
      assert.match(raw, /\r\ncache-control: no-store\r\n/i);
      const body = raw.slice(raw.indexOf('\r\n\r\n') + 4);
      const envelope = JSON.parse(body);
      assert.equal(envelope.command, 'trackb.release.approve');
      assert.equal(envelope.error.code, 'trackb-input-invalid');
      assert.equal(envelope.error.retryable, false);
    }

    const approval = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT })
    });
    assert.equal(approval.status, 200);
    assertNoStore(approval);
    assert.equal((await approval.json()).ok, true);
  });
});

test('Studio keeps an unclassifiable parser head as a bare close', async () => {
  await withServer(async (origin) => {
    const raw = await rawIncompleteHttpRequest(`${origin}/not-execute`, {
      headerLines: [
        `Origin: ${origin}`,
        'Cookie: smc_trackb_review_session=marker',
        'Content-Length: 1',
        'Content-Length: 1',
        'Connection: close'
      ],
      bodyPrefix: '{',
      timeoutMs: 1000
    });
    assert.match(raw, /^HTTP\/1\.1 400 /);
    assert.doesNotMatch(raw, /cache-control/i);
    assert.doesNotMatch(raw, /trackb-input-invalid/);
  });
});

test('Studio requires an explicitly configured opaque local operator ID', () => {
  assert.throws(
    () => createStudioServer({ port: 0, projectRoot: process.cwd(), repoRoot: process.cwd() }),
    (error) => error.code === 'configuration-invalid'
  );
  assert.throws(
    () => createStudioServer({ port: 0, projectRoot: process.cwd(), repoRoot: process.cwd(), localOperatorId: 'not valid' }),
    (error) => error.code === 'configuration-invalid'
  );
});

test('Studio expires and isolates review sessions across server lifetimes', async () => {
  const projectRoot = await mkdtemp(join(tmpdir(), 'smc-studio-restart-'));
  const fixture = await createApprovalFixture(projectRoot);
  let now = FIXED_NOW;
  const config = {
    port: 0,
    projectRoot,
    repoRoot: projectRoot,
    localOperatorId: 'operator.studio',
    clock: () => now,
    trackBRuntimeDependencies: fixture.runtimeDependencies
  };
  let firstServer;
  let cookie;
  try {
    firstServer = createStudioServer(config);
    await new Promise((resolve, reject) => { firstServer.once('error', reject); firstServer.listen(0, '127.0.0.1', resolve); });
    const firstOrigin = `http://127.0.0.1:${firstServer.address().port}`;
    const sessionResponse = await fetch(`${firstOrigin}/api/review-session`, { headers: { Origin: firstOrigin } });
    cookie = sessionResponse.headers.get('set-cookie').split(';', 1)[0];
    now += 901_000;
    const expired = await fetch(`${firstOrigin}/api/execute`, {
      method: 'POST',
      headers: { Origin: firstOrigin, 'Content-Type': 'application/json', Cookie: cookie },
      body: '{'
    });
    assert.equal(expired.status, 403);
    assertNoStore(expired);
    assert.equal((await expired.json()).error.code, 'trackb-approval-forbidden');
    await new Promise((resolve, reject) => firstServer.close((error) => error ? reject(error) : resolve()));
    firstServer = null;

    const secondServer = createStudioServer(config);
    await new Promise((resolve, reject) => { secondServer.once('error', reject); secondServer.listen(0, '127.0.0.1', resolve); });
    const secondOrigin = `http://127.0.0.1:${secondServer.address().port}`;
    try {
    const restarted = await fetch(`${secondOrigin}/api/execute`, {
      method: 'POST',
      headers: { Origin: secondOrigin, 'Content-Type': 'application/json', Cookie: cookie },
      body: Buffer.from([0xff])
    });
      assert.equal(restarted.status, 403);
      assertNoStore(restarted);
      assert.equal((await restarted.json()).error.code, 'trackb-approval-forbidden');
    } finally {
      await new Promise((resolve, reject) => secondServer.close((error) => error ? reject(error) : resolve()));
    }
  } finally {
    if (firstServer) await new Promise((resolve, reject) => firstServer.close((error) => error ? reject(error) : resolve()));
    await rm(projectRoot, { recursive: true, force: true });
  }
});

test('Studio uses exact non-wildcard preflight and keeps approval off CLI/MCP surfaces', async () => {
  assert.equal(CLI_COMMAND_REGISTRY.some(({ name }) => name === 'trackb-release-approve'), false);
  assert.equal(MCP_COMMAND_REGISTRY.some(({ name }) => name === 'trackb_release_approve'), false);
  await withServer(async (origin) => {
    const preflight = await fetch(`${origin}/api/execute`, {
      method: 'OPTIONS',
      headers: {
        Origin: origin,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': 'Content-Type'
      }
    });
    assert.equal(preflight.status, 204);
    assertNoStore(preflight);
    assert.equal(preflight.headers.get('access-control-allow-origin'), origin);
    assert.equal(preflight.headers.get('access-control-allow-credentials'), 'true');
    assert.notEqual(preflight.headers.get('access-control-allow-origin'), '*');

    const permissive = await fetch(`${origin}/api/execute`, {
      method: 'OPTIONS',
      headers: {
        Origin: origin,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': 'X-Other'
      }
    });
    assert.equal(permissive.status, 403);
    assertNoStore(permissive);
  });
});

test('Studio rejects Track B on wildcard bindings even when the Origin is forged as canonical', async () => {
  const server = createStudioServer({
    port: 0,
    projectRoot: process.cwd(),
    repoRoot: process.cwd(),
    localOperatorId: 'studio.test'
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '0.0.0.0', resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  try {
    const session = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    assert.equal(session.status, 403);
    assertNoStore(session);
    assert.equal((await session.json()).error.code, 'trackb-approval-forbidden');

    const execute = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json' },
      body: JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT })
    });
    assert.equal(execute.status, 403);
    assertNoStore(execute);
    assert.equal((await execute.json()).error.code, 'trackb-approval-forbidden');
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  }
});

test('Studio fails closed for an IPv6 loopback binding even with an exact IPv6 Origin', async (t) => {
  const server = createStudioServer({
    port: 0,
    projectRoot: process.cwd(),
    repoRoot: process.cwd(),
    localOperatorId: 'studio.test'
  });
  try {
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '::1', resolve);
    });
  } catch (error) {
    if (['EADDRNOTAVAIL', 'EAFNOSUPPORT', 'EPERM', 'EACCES'].includes(error?.code)) {
      t.skip(`IPv6 loopback unavailable: ${error.code}`);
      return;
    }
    throw error;
  }

  const port = server.address().port;
  const origin = `http://[::1]:${port}`;
  try {
    const missingOrigin = await fetch(`${origin}/api/review-session`);
    assert.equal(missingOrigin.status, 403);
    assertNoStore(missingOrigin);
    assert.equal((await missingOrigin.json()).error.code, 'trackb-approval-forbidden');

    const wrongOrigin = await fetch(`${origin}/api/review-session`, {
      headers: { Origin: `http://127.0.0.1:${port}` }
    });
    assert.equal(wrongOrigin.status, 403);
    assertNoStore(wrongOrigin);
    assert.equal((await wrongOrigin.json()).error.code, 'trackb-approval-forbidden');

    const session = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    assert.equal(session.status, 403);
    assertNoStore(session);
    assert.equal((await session.json()).error.code, 'trackb-approval-forbidden');
    assert.equal(session.headers.get('set-cookie'), null);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  }
});

test('Studio marks unexpected Track B failures as no-store', async () => {
  let clockCalls = 0;
  await withApprovalServer(async ({ origin }) => {
    const sessionResponse = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    const cookie = sessionResponse.headers.get('set-cookie').split(';', 1)[0];
    const response = await fetch(`${origin}/api/execute`, {
      method: 'POST',
      headers: { Origin: origin, 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ command: 'trackb.release.approve', input: APPROVAL_INPUT })
    });
    assert.equal(response.status, 500);
    assertNoStore(response);
    assert.equal((await response.json()).error.code, 'trackb-storage-integrity');
  }, {
    clock: () => {
      clockCalls += 1;
      if (clockCalls === 1) return FIXED_NOW;
      throw new Error('clock failure for no-store regression');
    }
  });
});

test('Studio marks review-session issuance failures as no-store', async () => {
  const server = createStudioServer({
    port: 0,
    projectRoot: process.cwd(),
    repoRoot: process.cwd(),
    localOperatorId: 'studio.test',
    clock: () => { throw new Error('session clock failure for no-store regression'); }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const origin = `http://127.0.0.1:${server.address().port}`;
  try {
    const response = await fetch(`${origin}/api/review-session`, { headers: { Origin: origin } });
    assert.equal(response.status, 500);
    assertNoStore(response);
    assert.equal((await response.json()).error.code, 'trackb-storage-integrity');
  } finally {
    await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  }
});

async function createApprovalFixture(projectRoot) {
  const canonicalProjectRoot = await realpath(projectRoot);
  const authority = JSON.parse(await readFile(join('contracts', 'examples', 'trackb', 'trackb-narrative-authority.example.json'), 'utf8'));
  const candidate = JSON.parse(await readFile(join('contracts', 'examples', 'trackb', 'trackb-release-candidate.example.json'), 'utf8'));
  const authorityBytes = Buffer.from(canonicalizeJson(authority), 'utf8');
  const candidateBytes = Buffer.from(canonicalizeJson(candidate), 'utf8');
  const candidatePath = `candidates/${candidate.candidateHash.slice('sha256:'.length)}/trackb-release-candidate-v4.json`;
  const authorityPath = `authority/${candidate.authorityHash.slice('sha256:'.length)}/trackb-narrative-authority-v2.json`;
  const events = [];
  const writes = [];
  let closed = false;
  const session = {
    async read(path) {
      assert.equal(closed, false);
      events.push(`read:${path}`);
      if (path === candidatePath) return snapshot(path, candidateBytes, 'candidate', 1);
      if (path === authorityPath) return snapshot(path, authorityBytes, 'authority', 2);
      assert.fail(`unexpected custody read: ${path}`);
    },
    async snapshot(path) {
      assert.equal(closed, false);
      events.push(`snapshot:${path}`);
      if (path === 'delivery-pointer-v2.json') return { path, exists: false, snapshotToken: 'NONE' };
      return { path, exists: false, snapshotToken: 'NONE' };
    },
    async writeImmutable(path, bytes) {
      assert.equal(closed, false);
      const payload = Buffer.from(bytes);
      writes.push({ path, bytes: payload });
      return {
        path,
        reused: false,
        snapshot: snapshotWithoutPath(payload, 'written', 3)
      };
    },
    async close() {
      events.push('close');
      closed = true;
      return { closed: true };
    }
  };
  const registry = await loadTrackBContractRegistry();
  return {
    authority,
    candidate,
    events,
    writes,
    runtimeDependencies: {
      openNarrativeSession: async ({ projectRoot: actualRoot, narrativeId, readOnly }) => {
        assert.equal(actualRoot, canonicalProjectRoot);
        assert.equal(narrativeId, candidate.narrativeId);
        assert.equal(readOnly, false);
        events.push('open');
        return session;
      },
      workflowDeps: { registry },
      now: FIXED_NOW
    }
  };
}

function snapshot(path, bytes, token, inode) {
  const payload = Buffer.from(bytes);
  return {
    path,
    exists: true,
    snapshotToken: Buffer.from(token).toString('base64url'),
    bytesBase64: payload.toString('base64url'),
    byteLength: payload.length,
    sha256: hashTrackBRawBytes(payload),
    device: 1,
    inode
  };
}

function snapshotWithoutPath(bytes, token, inode) {
  const { path: _path, ...result } = snapshot('unused', bytes, token, inode);
  return result;
}
