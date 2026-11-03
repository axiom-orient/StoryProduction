const REJECTED_REQUEST_CLOSE_GRACE_MS = 100;

export function readRequestBody(req, maxBytes, trackBError) {
  return new Promise((resolveBody, rejectBody) => {
    const chunks = [];
    let total = 0;
    let settled = false;
    req.on('data', (chunk) => {
      if (settled) return;
      total += chunk.length;
      if (total > maxBytes) {
        settled = true;
        req.resume();
        const error = new Error('Studio command body exceeds 8 MiB');
        error.status = 413;
        error.code = trackBError ? 'trackb-input-invalid' : 'request-too-large';
        rejectBody(error);
        return;
      }
      chunks.push(Buffer.from(chunk));
    });
    req.once('error', (error) => {
      if (settled) return;
      settled = true;
      rejectBody(error);
    });
    req.once('end', () => {
      if (settled) return;
      settled = true;
      resolveBody(Buffer.concat(chunks, total));
    });
  });
}

export function parseContentLength(value, maxBytes) {
  if (value === undefined) return null;
  if (typeof value !== 'string' || !/^\d+$/.test(value)) return { invalid: true };
  try {
    return { tooLarge: BigInt(value) > BigInt(maxBytes) };
  } catch {
    return { invalid: true };
  }
}

export function sendRejectedExecuteRequest(req, res, status, body) {
  // Keep the transport alive while the bounded discard finishes. The explicit
  // Connection header remains a contract signal; the socket is closed below
  // only after the response is handed off and the request is complete or the
  // short discard grace expires.
  res.shouldKeepAlive = true;
  const socket = req?.socket;
  let responseFinished = false;
  let closeTimer;
  const closeSocket = () => {
    if (!responseFinished) return;
    clearTimeout(closeTimer);
    if (socket && !socket.destroyed && !socket.writableEnded) socket.end();
  };
  req.once('end', closeSocket);
  res.once('finish', () => {
    responseFinished = true;
    if (req.complete) {
      closeSocket();
      return;
    }
    closeTimer = setTimeout(closeSocket, REJECTED_REQUEST_CLOSE_GRACE_MS);
  });
  req.resume();
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Cache-Control': 'no-store',
    Connection: 'close'
  });
  // Node treats an explicit Connection: close header as permission to end the
  // socket immediately. Keep the response transport open until the bounded
  // discard below completes so clients already sending a full body receive the
  // response without a reset; the explicit socket.end() then still closes it.
  res._last = false;
  return res.end(JSON.stringify(body));
}
