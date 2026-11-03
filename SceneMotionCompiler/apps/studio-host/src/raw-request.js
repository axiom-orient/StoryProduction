import { TRACK_B_STUDIO_COOKIE } from '../../../packages/contracts/src/trackb-contract.js';

/**
 * Recover only the small set of header markers needed to classify a request
 * that Node's HTTP parser already rejected. This does not admit or authorize
 * the request; the server still applies the loopback and review-session rules.
 */
export function parseRawRequestHead(rawPacket) {
  if (!Buffer.isBuffer(rawPacket)) return null;
  const requestLineEnd = rawPacket.indexOf('\r\n');
  if (requestLineEnd < 0) return null;
  const headerEnd = rawPacket.indexOf('\r\n\r\n', requestLineEnd + 2);
  const headEnd = headerEnd >= 0 ? headerEnd : rawPacket.length;
  const lines = rawPacket.subarray(0, headEnd).toString('latin1').split('\r\n');
  const requestLine = lines.shift() ?? '';
  const requestParts = requestLine.split(' ');
  if (requestParts.length !== 3) return null;
  const [method, target, version] = requestParts;
  if (method !== 'POST'
    || !/^\/api\/execute(?:\?[^\s\x00-\x1f\x7f#]*)?$/.test(target ?? '')
    || !/^HTTP\/1\.[01]$/.test(version ?? '')) {
    return null;
  }
  const headers = Object.create(null);
  let markersTrusted = headerEnd >= 0;
  const originValues = [];
  let cookieMarkerPresent = false;
  for (const line of lines) {
    if (!line || /^[ \t]/.test(line)) {
      markersTrusted = false;
      continue;
    }
    const separator = line.indexOf(':');
    if (separator <= 0) {
      markersTrusted = false;
      continue;
    }
    const name = line.slice(0, separator).toLowerCase();
    const value = line.slice(separator + 1);
    if (!/^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$/.test(name) || hasInvalidHeaderValue(value)) {
      markersTrusted = false;
      continue;
    }
    const normalizedValue = value.trim();
    if (name === 'origin') originValues.push(normalizedValue);
    if (name === 'cookie' && hasReviewSessionCookieMarker(normalizedValue)) cookieMarkerPresent = true;
    if (headers[name] !== undefined) {
      if (name === 'origin' || name === 'cookie') markersTrusted = false;
      continue;
    }
    headers[name] = normalizedValue;
  }
  return { headers, markersTrusted, originValues, cookieMarkerPresent };
}

function hasReviewSessionCookieMarker(value) {
  const cookieName = `${TRACK_B_STUDIO_COOKIE.name}=`;
  return typeof value === 'string'
    && value.split(';').some((part) => part.trim().startsWith(cookieName));
}

function hasInvalidHeaderValue(value) {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if ((code < 0x20 && code !== 0x09) || code === 0x7f) return true;
  }
  return false;
}
