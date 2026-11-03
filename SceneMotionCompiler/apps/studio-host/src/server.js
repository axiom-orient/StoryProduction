// SceneMotionCompiler Studio — Track B human review dashboard.
// loopback HTTP 서버로 Track B application commands에 직접 바인딩한다.

import { createServer } from 'node:http';
import { basename } from 'node:path';
import {
  APPLICATION_COMMAND_METADATA,
  STUDIO_COMMANDS,
  executeApplicationCommand,
  PRODUCT_VERSION
} from '../../../packages/application/src/index.js';
import { createStudioApprovalDispatcher } from '../../../packages/application/src/studio-dispatcher.js';
import { ApplicationCommandError } from '../../../packages/application/src/command-error.js';
import {
  TRACK_B_STUDIO_COOKIE,
  TRACK_B_STUDIO_MAX_BODY_BYTES,
  TRACK_B_STUDIO_ENDPOINT,
  TRACK_B_STUDIO_SESSION_ENDPOINT,
  isTrackBOpaqueId,
  validateTrackBApplicationEnvelope
} from '../../../packages/contracts/src/trackb-contract.js';
import {
  TRACK_B_STUDIO_ROUTE,
  validateTrackBStudioRequest
} from '../../../packages/application/src/trackb-release-contract.js';
import {
  createReviewSession,
  getLiveReviewSession,
  hashReviewSessionToken,
  parseReviewSessionCookie
} from './review-sessions.js';
import { hasSafeSceneOutput, renderDashboard, serveProjectArtifact } from './project-artifacts.js';
import { parseRawRequestHead } from './raw-request.js';
import {
  parseContentLength,
  readRequestBody,
  sendRejectedExecuteRequest
} from './request-body.js';
import {
  errorEnvelope,
  sendJson,
  sendNoStoreJson,
  sendTrackBJson,
  trackBFailureEnvelope,
  writeRawSocketResponse
} from './responses.js';

const MAX_EXECUTE_BODY_BYTES = TRACK_B_STUDIO_MAX_BODY_BYTES;
const TRACK_B_STUDIO_COMMAND = TRACK_B_STUDIO_ROUTE.command;

export function createStudioServer(config) {
  const normalized = normalizeStudioConfig(config);
  const reviewSessions = new Map();
  const approvalDispatcher = createStudioApprovalDispatcher(normalized);
  const server = createServer(async (req, res) => {
    const canonicalOrigin = getCanonicalOrigin(server);
    const url = new URL(req.url ?? '/', canonicalOrigin ?? `http://127.0.0.1:${normalized.port}`);
    const origin = headerValue(req.headers.origin);

    // Track B routes are admitted before the generic API/static dispatch so a
    // wrong method or origin cannot fall through to a plaintext 404 or file
    // handler. CORS is added only for an exact canonical origin.
    if (url.pathname === TRACK_B_STUDIO_SESSION_ENDPOINT) {
      if (req.method === 'OPTIONS') {
        if (origin === canonicalOrigin && isLoopbackRequest(req, server)) setCORS(res, origin, canonicalOrigin);
        return handlePreflight(req, res, url.pathname, origin, canonicalOrigin, server);
      }
      if (!admitReviewSessionRequest(req, canonicalOrigin, server)) {
        req.resume();
        return sendTrackBJson(res, 403, trackBFailureEnvelope(
          undefined,
          'trackb-approval-forbidden',
          'Track B review-session requires an exact GET, canonical loopback origin or trusted same-origin browser metadata, and loopback socket'
        ));
      }
      setCORS(res, origin, canonicalOrigin);
      try {
        return issueReviewSession(res, reviewSessions, normalized.localOperatorId, normalized.clock());
      } catch {
        return sendTrackBJson(res, 500, trackBFailureEnvelope(
          undefined,
          'trackb-storage-integrity',
          'Track B review session issuance failed'
        ));
      }
    }

    if (url.pathname === TRACK_B_STUDIO_ENDPOINT) {
      if (req.method === 'OPTIONS') {
        if (origin === canonicalOrigin && isLoopbackRequest(req, server)) setCORS(res, origin, canonicalOrigin);
        return handlePreflight(req, res, url.pathname, origin, canonicalOrigin, server);
      }
      if (req.method !== 'POST') {
        req.resume();
        return sendTrackBJson(res, 403, trackBFailureEnvelope(
          TRACK_B_STUDIO_COMMAND,
          'trackb-approval-forbidden',
          'Track B Studio execute requires POST'
        ));
      }
      if (origin !== undefined && origin !== canonicalOrigin) {
        req.resume();
        return sendTrackBJson(res, 403, trackBFailureEnvelope(
          TRACK_B_STUDIO_COMMAND,
          'trackb-approval-forbidden',
          'Studio origin is not the canonical loopback origin'
        ));
      }
      setCORS(res, origin, canonicalOrigin);
      return handleExecuteRequest(req, res, normalized, reviewSessions, canonicalOrigin, approvalDispatcher, server);
    }

    if (origin && origin !== canonicalOrigin) {
      return sendJson(res, 403, {
        ok: false,
        error: { code: 'origin-forbidden', message: 'Origin is not allowed' }
      });
    }
    setCORS(res, origin, canonicalOrigin);

    if (req.method === 'OPTIONS') {
      return handlePreflight(req, res, url.pathname, origin, canonicalOrigin, server);
    }

    // API: GET /api/metadata
    if (url.pathname === '/api/metadata') {
      return sendNoStoreJson(res, 200, {
        version: PRODUCT_VERSION,
        projectName: basename(normalized.projectRoot),
        previewAvailable: await hasSafeSceneOutput(normalized.projectRoot),
        commands: Object.fromEntries(
          STUDIO_COMMANDS
            .filter((command) => Object.hasOwn(APPLICATION_COMMAND_METADATA, command))
            .map((command) => [command, APPLICATION_COMMAND_METADATA[command]])
        )
      });
    }

    // Static: GET / → dashboard HTML
    if (url.pathname === '/') {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      return res.end(renderDashboard());
    }

    if (req.method === 'GET') {
      return serveProjectArtifact(req, res, url.pathname, config.projectRoot);
    }

    res.writeHead(404);
    res.end('Not Found');
  });
  server.on('clientError', (error, socket) => {
    handleClientError(error, socket, server, reviewSessions, normalized);
  });

  return server;
}

function normalizeStudioConfig(config) {
  if (!config || typeof config !== 'object' || Array.isArray(config)) {
    throw new ApplicationCommandError('configuration-invalid', 'Studio configuration is required');
  }
  if (!isTrackBOpaqueId(config.localOperatorId)) {
    throw new ApplicationCommandError('configuration-invalid', 'localOperatorId must be a valid opaque Track B operator ID');
  }
  const configuredPort = config.port ?? 3456;
  if (!Number.isInteger(configuredPort) || configuredPort < 0 || configuredPort > 65535) {
    throw new ApplicationCommandError('configuration-invalid', 'Studio port must be an integer from 0 through 65535');
  }
  if (config.clock !== undefined && typeof config.clock !== 'function') {
    throw new ApplicationCommandError('configuration-invalid', 'Studio clock must be a function when supplied');
  }
  return {
    ...config,
    port: configuredPort,
    clock: config.clock ?? (() => Date.now())
  };
}

function getCanonicalOrigin(server) {
  const address = server.address();
  if (!address || typeof address !== 'object') return null;
  const port = address.port;
  if (!Number.isInteger(port) || port < 1 || port > 65535) return null;
  if (isIpv4LoopbackAddress(address.address)) return `http://127.0.0.1:${port}`;
  return null;
}

function admitReviewSessionRequest(req, canonicalOrigin, server) {
  if (req.method !== 'GET' || !isLoopbackRequest(req, server)) return false;
  const origin = headerValue(req.headers.origin);
  if (origin !== undefined) return origin === canonicalOrigin;
  return isTrustedSameOriginBrowserGet(req, canonicalOrigin);
}

function isTrustedSameOriginBrowserGet(req, canonicalOrigin) {
  const referer = headerValue(req.headers.referer);
  const fetchSite = headerValue(req.headers['sec-fetch-site']);
  if (!referer || fetchSite !== 'same-origin') return false;
  let refererUrl;
  try {
    refererUrl = new URL(referer);
  } catch {
    return false;
  }
  if (refererUrl.origin !== canonicalOrigin || refererUrl.username || refererUrl.password) return false;

  // Chrome's same-origin fetch uses cors/empty here. Treat these headers as
  // additional authenticity signals when present, while keeping the required
  // Referer + Sec-Fetch-Site pair portable across browser versions.
  const fetchMode = headerValue(req.headers['sec-fetch-mode']);
  if (fetchMode !== undefined && fetchMode !== 'cors') return false;
  const fetchDest = headerValue(req.headers['sec-fetch-dest']);
  if (fetchDest !== undefined && fetchDest !== 'empty') return false;
  return true;
}

function admitTrackBRequest(req, config, reviewSessions, canonicalOrigin, server) {
  const forbidden = (message) => ({
    valid: false,
    status: 403,
    body: trackBFailureEnvelope(TRACK_B_STUDIO_COMMAND, 'trackb-approval-forbidden', message)
  });
  if (!isLoopbackRequest(req, server)) {
    return forbidden('Track B Studio requests require a loopback local and remote socket address');
  }
  const origin = headerValue(req.headers.origin);
  if (origin !== canonicalOrigin) {
    return forbidden('Studio origin is not the canonical loopback origin');
  }
  const token = parseReviewSessionCookie(req.headers.cookie);
  if (!token) {
    return forbidden('a canonical, unexpired, single-use Studio review session is required');
  }
  let now;
  try {
    now = config.clock();
  } catch {
    return {
      valid: false,
      status: 500,
      body: trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-storage-integrity',
        'Track B review session validation failed'
      )
    };
  }
  const sessionHash = hashReviewSessionToken(token);
  const session = getLiveReviewSession(reviewSessions, sessionHash, now);
  if (!session) {
    return forbidden('a canonical, unexpired, single-use Studio review session is required');
  }
  return { valid: true, now, origin, sessionHash, session };
}

async function handleExecuteRequest(req, res, config, reviewSessions, canonicalOrigin, approvalDispatcher, server) {
  const potentialTrackB = headerValue(req.headers.origin) === canonicalOrigin
    || String(req.headers.cookie ?? '').includes(`${TRACK_B_STUDIO_COOKIE.name}=`);
  const trackBAdmission = potentialTrackB
    ? admitTrackBRequest(req, config, reviewSessions, canonicalOrigin, server)
    : null;
  if (potentialTrackB && !trackBAdmission.valid) {
    return sendRejectedExecuteRequest(req, res, trackBAdmission.status, trackBAdmission.body);
  }
  const contentLength = parseContentLength(req.headers['content-length'], MAX_EXECUTE_BODY_BYTES);
  if (contentLength?.invalid === true) {
    const body = potentialTrackB
      ? trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-input-invalid',
        'Track B Studio content-length must be a non-negative decimal integer'
      )
      : errorEnvelope('bad-request', 'Studio content-length must be a non-negative decimal integer');
    return sendRejectedExecuteRequest(req, res, 400, body);
  }
  if (contentLength?.tooLarge === true) {
    const body = potentialTrackB
      ? trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-input-invalid',
        'Track B Studio command body exceeds 8 MiB'
      )
      : errorEnvelope('request-too-large', 'Studio command body exceeds 8 MiB');
    return sendRejectedExecuteRequest(req, res, 413, body);
  }
  if (potentialTrackB && headerValue(req.headers['content-type']) !== TRACK_B_STUDIO_ROUTE.contentType) {
    req.resume();
    return sendTrackBJson(res, 400, trackBFailureEnvelope(
      TRACK_B_STUDIO_COMMAND,
      'trackb-input-invalid',
      `Track B Studio content-type must be ${TRACK_B_STUDIO_ROUTE.contentType}`
    ));
  }
  let bodyBytes;
  try {
    bodyBytes = await readRequestBody(req, MAX_EXECUTE_BODY_BYTES, potentialTrackB);
  } catch (error) {
    if (potentialTrackB) {
      const status = error.status ?? 400;
      const body = trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        error.code ?? 'trackb-input-invalid',
        error.message ?? 'Studio command body could not be read'
      );
      return status === 413
        ? sendRejectedExecuteRequest(req, res, status, body)
        : sendTrackBJson(res, status, body);
    }
    const status = error.status ?? 400;
    const body = errorEnvelope(
      error.code ?? 'bad-request',
      error.message ?? 'Studio request body could not be read'
    );
    return status === 413
      ? sendRejectedExecuteRequest(req, res, status, body)
      : sendJson(res, status, body);
  }

  let body;
  try {
    body = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bodyBytes));
  } catch (error) {
    const body = potentialTrackB
      ? trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-input-invalid',
        'Studio request body must be valid UTF-8 JSON'
      )
      : errorEnvelope('trackb-input-invalid', 'Studio request body must be valid UTF-8 JSON');
    return (potentialTrackB ? sendTrackBJson : sendJson)(
      res,
      400,
      body
    );
  }

  if (body?.command === TRACK_B_STUDIO_COMMAND) {
    if (!trackBAdmission?.valid) {
      return sendTrackBJson(res, 403, trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-approval-forbidden',
        'a canonical, unexpired, single-use Studio review session is required'
      ));
    }
    try {
      return await handleTrackBApprovalRequest(
        req,
        res,
        config,
        reviewSessions,
        canonicalOrigin,
        body,
        bodyBytes.length,
        approvalDispatcher,
        trackBAdmission
      );
    } catch {
      return sendTrackBJson(res, 500, trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-storage-integrity',
        'Track B approval execution failed'
      ));
    }
  }

  // Canonical-origin browser callers are admitted through the same review
  // session gate so the body cannot select a privileged Track B path.
  if (!STUDIO_COMMANDS.includes(body?.command)) {
    return sendJson(res, 403, errorEnvelope(
      'studio-command-forbidden',
      `Command is not exposed by Studio: ${String(body?.command)}`
    ));
  }

  try {
    const result = await executeApplicationCommand(config, body.command, body.input ?? {});
    return sendJson(res, 200, result);
  } catch (error) {
    return sendJson(res, 400, errorEnvelope('bad-request', error.message));
  }
}

async function handleTrackBApprovalRequest(req, res, _config, reviewSessions, canonicalOrigin, body, bodyBytes, approvalDispatcher, trackBAdmission) {
  const { now, origin, sessionHash, session } = trackBAdmission;

  const frame = {
    method: req.method,
    path: TRACK_B_STUDIO_ENDPOINT,
    contentType: headerValue(req.headers['content-type']),
    bodyBytes,
    origin,
    allowedOrigin: canonicalOrigin,
    sessionValid: true
  };
  const validation = validateTrackBStudioRequest(body, frame, { now });
  if (!validation.valid) {
    const first = validation.errors?.[0];
    const forbidden = validation.errors?.some((entry) => entry.code === 'trackb-approval-forbidden');
    return sendTrackBJson(res, forbidden ? 403 : 400, trackBFailureEnvelope(
      TRACK_B_STUDIO_COMMAND,
      forbidden ? 'trackb-approval-forbidden' : (first?.code ?? 'trackb-input-invalid'),
      first?.message ?? 'Studio Track B approval request is invalid'
    ));
  }

  // Consume synchronously before entering the async application call. A valid
  // token therefore cannot be replayed even if the application operation fails.
  if (!reviewSessions.delete(sessionHash)) {
    return sendTrackBJson(res, 403, trackBFailureEnvelope(
      TRACK_B_STUDIO_COMMAND,
      'trackb-approval-forbidden',
      'a canonical, unexpired, single-use Studio review session is required'
    ));
  }
  const executionContext = {
    actorId: session.actorId,
    origin: 'studio',
    reviewSessionId: sessionHash
  };
  const result = await approvalDispatcher.executeApproval(body.input, executionContext);
  const envelopeValidation = validateTrackBApplicationEnvelope(result, {
    command: TRACK_B_STUDIO_COMMAND,
    input: body.input
  });
  if (!envelopeValidation.valid) {
    return sendTrackBJson(res, 500, trackBFailureEnvelope(
      TRACK_B_STUDIO_COMMAND,
      'trackb-storage-integrity',
      'application returned an invalid Track B envelope'
    ));
  }
  return sendTrackBJson(res, 200, result);
}

function handleClientError(error, socket, server, reviewSessions, config) {
  if (!socket || socket.destroyed || socket.writableEnded) return;
  const request = parseRawRequestHead(error?.rawPacket);
  if (!request) {
    socket.end('HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n');
    return;
  }
  const canonicalOrigin = getCanonicalOrigin(server);
  const potentialTrackB = request.originValues.includes(canonicalOrigin)
    || request.cookieMarkerPresent;
  if (potentialTrackB) {
    const admission = admitRawTrackBRequest(request, socket, config, reviewSessions, canonicalOrigin, server);
    if (!admission.valid) {
      return writeRawSocketResponse(socket, admission.status, admission.body);
    }
  }
  const message = error?.code === 'HPE_INVALID_CONTENT_LENGTH'
    ? 'Studio content-length must be a non-negative decimal integer'
    : 'Studio request framing is invalid';
  const body = potentialTrackB
    ? trackBFailureEnvelope(
      TRACK_B_STUDIO_COMMAND,
      'trackb-input-invalid',
      message
    )
    : errorEnvelope('bad-request', message);
  return writeRawSocketResponse(socket, 400, body);
}

function admitRawTrackBRequest(request, socket, config, reviewSessions, canonicalOrigin, server) {
  const forbidden = (message) => ({
    valid: false,
    status: 403,
    body: trackBFailureEnvelope(TRACK_B_STUDIO_COMMAND, 'trackb-approval-forbidden', message)
  });
  // Duplicate or otherwise malformed marker headers are not safe to interpret
  // as an authenticated request. They remain classified as Track B only so the
  // caller receives the locked admission error instead of a generic framing
  // response.
  if (!request.markersTrusted) {
    return forbidden('Track B request headers are ambiguous or malformed');
  }
  if (!isLoopbackRequest({ socket }, server)) {
    return forbidden('Track B Studio requests require a loopback local and remote socket address');
  }
  if (request.headers.origin !== canonicalOrigin) {
    return forbidden('Studio origin is not the canonical loopback origin');
  }
  const token = parseReviewSessionCookie(request.headers.cookie);
  if (!token) {
    return forbidden('a canonical, unexpired, single-use Studio review session is required');
  }
  let now;
  try {
    now = config.clock();
  } catch {
    return {
      valid: false,
      status: 500,
      body: trackBFailureEnvelope(
        TRACK_B_STUDIO_COMMAND,
        'trackb-storage-integrity',
        'Track B review session validation failed'
      )
    };
  }
  const sessionHash = hashReviewSessionToken(token);
  if (!getLiveReviewSession(reviewSessions, sessionHash, now)) {
    return forbidden('a canonical, unexpired, single-use Studio review session is required');
  }
  return { valid: true };
}

function issueReviewSession(res, reviewSessions, actorId, now) {
  const issued = createReviewSession(reviewSessions, actorId, now);
  return sendTrackBJson(res, 200, { ok: true, expiresAt: issued.expiresAt }, {
    'Cache-Control': 'no-store',
    'Set-Cookie': issued.cookie
  });
}

function handlePreflight(req, res, pathname, origin, canonicalOrigin, server) {
  const trackB = isTrackBPath(pathname);
  const send = trackB ? sendTrackBJson : sendJson;
  if (trackB && !isLoopbackRequest(req, server)) {
    return send(res, 403, trackBFailureEnvelope(
      pathname === TRACK_B_STUDIO_ENDPOINT ? TRACK_B_STUDIO_COMMAND : undefined,
      'trackb-approval-forbidden',
      'Track B Studio requests require a loopback local and remote socket address'
    ));
  }
  const expectedMethod = pathname === TRACK_B_STUDIO_SESSION_ENDPOINT ? 'GET' : 'POST';
  if (origin !== canonicalOrigin) {
    return send(res, 403, trackBFailureEnvelope(
      pathname === TRACK_B_STUDIO_ENDPOINT ? TRACK_B_STUDIO_COMMAND : undefined,
      'trackb-approval-forbidden',
      'preflight origin must equal the canonical loopback origin'
    ));
  }
  const requestedMethod = headerValue(req.headers['access-control-request-method']);
  if (requestedMethod && requestedMethod !== expectedMethod) {
    return send(res, 403, trackBFailureEnvelope(
      pathname === TRACK_B_STUDIO_ENDPOINT ? TRACK_B_STUDIO_COMMAND : undefined,
      'trackb-approval-forbidden',
      'preflight method is not allowed'
    ));
  }
  const requestedHeaders = headerValue(req.headers['access-control-request-headers']);
  if (requestedHeaders) {
    const headers = requestedHeaders.split(',').map((value) => value.trim().toLowerCase()).filter(Boolean);
    if (headers.some((value) => value !== 'content-type')) {
      return send(res, 403, trackBFailureEnvelope(
        pathname === TRACK_B_STUDIO_ENDPOINT ? TRACK_B_STUDIO_COMMAND : undefined,
        'trackb-approval-forbidden',
        'preflight requested headers are not allowed'
      ));
    }
  }
  if (trackB) res.setHeader('Cache-Control', 'no-store');
  res.writeHead(204);
  return res.end();
}

function setCORS(res, origin, canonicalOrigin) {
  res.setHeader('Vary', 'Origin');
  if (origin && origin === canonicalOrigin) {
    res.setHeader('Access-Control-Allow-Origin', canonicalOrigin);
    res.setHeader('Access-Control-Allow-Credentials', 'true');
    res.setHeader('Access-Control-Allow-Methods', 'GET,POST,OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  }
}

function headerValue(value) {
  return typeof value === 'string' ? value : undefined;
}

function isTrackBPath(pathname) {
  return pathname === TRACK_B_STUDIO_ENDPOINT || pathname === TRACK_B_STUDIO_SESSION_ENDPOINT;
}

function isLoopbackRequest(req, server) {
  const binding = server?.address();
  const bindingAddress = binding && typeof binding === 'object' ? binding.address : undefined;
  return isIpv4LoopbackAddress(req?.socket?.localAddress)
    && isIpv4LoopbackAddress(req?.socket?.remoteAddress)
    && isIpv4LoopbackAddress(bindingAddress);
}

function isIpv4LoopbackAddress(address) {
  if (typeof address !== 'string' || address.length === 0) return false;
  const normalized = address.toLowerCase();
  const ipv4 = normalized.startsWith('::ffff:') ? normalized.slice('::ffff:'.length) : normalized;
  const octets = ipv4.split('.');
  return octets.length === 4
    && octets.every((octet) => /^(?:0|[1-9]\d{0,2})$/.test(octet) && Number(octet) <= 255)
    && Number(octets[0]) === 127;
}
