import { APPLICATION_COMMAND_METADATA } from '../../../packages/application/src/index.js';

export function writeRawSocketResponse(socket, status, body) {
  if (!socket || socket.destroyed || socket.writableEnded) return;
  const statusText = status === 403
    ? 'Forbidden'
    : status === 413
      ? 'Payload Too Large'
      : status >= 500
        ? 'Internal Server Error'
        : 'Bad Request';
  const bodyBytes = Buffer.from(JSON.stringify(body));
  const responseHead = Buffer.from(
    `HTTP/1.1 ${status} ${statusText}\r\nContent-Type: application/json; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: ${bodyBytes.length}\r\n\r\n`,
    'latin1'
  );
  socket.end(Buffer.concat([responseHead, bodyBytes]));
}

export function sendJson(res, status, body, headers = {}) {
  res.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8', ...headers });
  return res.end(JSON.stringify(body));
}

export function sendNoStoreJson(res, status, body, headers = {}) {
  return sendJson(res, status, body, { ...headers, 'Cache-Control': 'no-store' });
}

export function sendTrackBJson(res, status, body, headers = {}) {
  return sendNoStoreJson(res, status, body, headers);
}

export function errorEnvelope(code, message) {
  return { ok: false, error: { code, message } };
}

export function trackBFailureEnvelope(command, code, message) {
  if (!command) return errorEnvelope(code, message);
  return {
    schemaVersion: 1,
    ok: false,
    command,
    metadata: APPLICATION_COMMAND_METADATA[command] ?? {},
    error: { code, message, retryable: false }
  };
}
