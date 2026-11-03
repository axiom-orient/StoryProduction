import { randomBytes } from 'node:crypto';
import { sha256Prefixed } from '../../../packages/core/src/canonical.js';
import {
  TRACK_B_STUDIO_COOKIE,
  isTrackBHash,
  isTrackBOpaqueId
} from '../../../packages/contracts/src/trackb-contract.js';

const MAX_REVIEW_SESSIONS = 128;
const REVIEW_SESSION_TTL_MS = TRACK_B_STUDIO_COOKIE.maxAgeSeconds * 1000;

export function createReviewSession(reviewSessions, actorId, now) {
  if (!isTrackBOpaqueId(actorId) || !Number.isSafeInteger(now)) {
    throw new Error('invalid Track B review session issuance context');
  }
  pruneReviewSessions(reviewSessions, now);
  while (reviewSessions.size >= MAX_REVIEW_SESSIONS) {
    const oldest = reviewSessions.keys().next().value;
    if (oldest === undefined) break;
    reviewSessions.delete(oldest);
  }

  const token = randomBytes(TRACK_B_STUDIO_COOKIE.tokenBytes).toString('base64url');
  const sessionHash = hashReviewSessionToken(token);
  const expiresAt = now + REVIEW_SESSION_TTL_MS;
  const session = Object.freeze({ actorId, issuedAt: now, expiresAt });
  if (!isValidReviewSessionRecord(session, now)) {
    throw new Error('invalid Track B review session record');
  }
  // Retain only the token hash; the bearer exists only in the response cookie.
  reviewSessions.set(sessionHash, session);

  return {
    expiresAt: new Date(expiresAt).toISOString(),
    cookie: `${TRACK_B_STUDIO_COOKIE.name}=${token}; HttpOnly; SameSite=${TRACK_B_STUDIO_COOKIE.sameSite}; Path=${TRACK_B_STUDIO_COOKIE.path}; Max-Age=${TRACK_B_STUDIO_COOKIE.maxAgeSeconds}`
  };
}

export function getLiveReviewSession(reviewSessions, sessionHash, now) {
  pruneReviewSessions(reviewSessions, now);
  const session = reviewSessions.get(sessionHash);
  return isValidReviewSessionRecord(session, now) ? session : null;
}

export function parseReviewSessionCookie(header) {
  if (typeof header !== 'string' || header.trim() === '') return null;
  const parts = header.split(';').map((part) => part.trim());
  if (parts.length !== 1 || parts[0].length === 0) return null;
  const separator = parts[0].indexOf('=');
  if (separator <= 0 || separator !== parts[0].lastIndexOf('=')) return null;
  const name = parts[0].slice(0, separator);
  const value = parts[0].slice(separator + 1);
  if (name !== TRACK_B_STUDIO_COOKIE.name || !/^[A-Za-z0-9_-]{43}$/.test(value)) return null;
  try {
    const bytes = Buffer.from(value, 'base64url');
    if (bytes.length !== TRACK_B_STUDIO_COOKIE.tokenBytes || bytes.toString('base64url') !== value) return null;
  } catch {
    return null;
  }
  return value;
}

export function hashReviewSessionToken(token) {
  return sha256Prefixed(token);
}

function pruneReviewSessions(reviewSessions, now) {
  for (const [hash, session] of reviewSessions) {
    if (!isTrackBHash(hash) || !isValidReviewSessionRecord(session, now)) reviewSessions.delete(hash);
  }
}

function isValidReviewSessionRecord(session, now) {
  if (!session || typeof session !== 'object' || Array.isArray(session)) return false;
  const prototype = Object.getPrototypeOf(session);
  if (prototype !== Object.prototype && prototype !== null) return false;
  const keys = Object.keys(session);
  if (keys.length !== 3 || keys[0] !== 'actorId' || keys[1] !== 'issuedAt' || keys[2] !== 'expiresAt') return false;
  if (!isTrackBOpaqueId(session.actorId)
    || !Number.isSafeInteger(session.issuedAt)
    || !Number.isSafeInteger(session.expiresAt)
    || !Number.isSafeInteger(now)) return false;
  return session.expiresAt - session.issuedAt === REVIEW_SESSION_TTL_MS
    && session.issuedAt <= now
    && session.expiresAt > now;
}
