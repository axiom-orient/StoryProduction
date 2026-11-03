import {
  TRACK_B_STUDIO_MAX_BODY_BYTES,
  TRACK_B_STUDIO_ENDPOINT,
  TRACK_B_STUDIO_SESSION_ENDPOINT,
  TRACK_B_STUDIO_REQUEST_FRAME_KEYS,
  isTrackBLoopbackOrigin,
  validateTrackBStudioExecuteBody
} from '../../contracts/src/trackb-contract.js';
import {
  STUDIO_APPROVAL_COMMAND
} from './command-registry.js';
import { isPlainObject } from './trackb-application-primitives.js';

export const TRACK_B_STUDIO_ROUTE = Object.freeze({
  method: 'POST',
  path: TRACK_B_STUDIO_ENDPOINT,
  command: STUDIO_APPROVAL_COMMAND,
  sessionBootstrap: Object.freeze({ method: 'GET', path: TRACK_B_STUDIO_SESSION_ENDPOINT }),
  requestFrameKeys: TRACK_B_STUDIO_REQUEST_FRAME_KEYS,
  contentType: 'application/json',
  maxBodyBytes: TRACK_B_STUDIO_MAX_BODY_BYTES,
  allowCredentials: true,
  cacheControl: 'no-store'
});

export function validateTrackBStudioRequest(body, frame, { now } = {}) {
  const errors = [];
  if (!isPlainObject(frame)) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame', message: 'Studio request frame is required' });
  else {
    exactKeys(frame, TRACK_B_STUDIO_REQUEST_FRAME_KEYS, errors);
    if (frame.method !== TRACK_B_STUDIO_ROUTE.method) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame/method', message: 'Studio method must be POST' });
    if (frame.path !== TRACK_B_STUDIO_ROUTE.path) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame/path', message: 'Studio path must be /api/execute' });
    if (frame.contentType !== TRACK_B_STUDIO_ROUTE.contentType) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame/contentType', message: 'Studio content-type must be application/json' });
    if (!Number.isInteger(frame.bodyBytes) || frame.bodyBytes < 0 || frame.bodyBytes > TRACK_B_STUDIO_MAX_BODY_BYTES) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame/bodyBytes', message: 'Studio bodyBytes must be an integer from 0 through 8 MiB' });
    if (!isTrackBLoopbackOrigin(frame.origin) || !isTrackBLoopbackOrigin(frame.allowedOrigin) || frame.origin !== frame.allowedOrigin) errors.push({ code: 'trackb-approval-forbidden', instancePath: '/frame/origin', message: 'Studio origin and allowedOrigin must be the same exact non-empty loopback origin' });
    if (frame.sessionValid !== true) errors.push({ code: 'trackb-approval-forbidden', instancePath: '/frame/sessionValid', message: 'Studio review session must be valid' });
  }
  const shape = validateTrackBStudioExecuteBody(body, { now });
  errors.push(...shape.errors);
  return { valid: errors.length === 0, errors };
}

function exactKeys(value, expected, errors) {
  const allowed = new Set(expected);
  for (const key of Object.keys(value)) if (!allowed.has(key)) errors.push({ code: 'trackb-input-invalid', instancePath: '/frame', message: `unsupported frame key: ${key}` });
  for (const key of expected) if (!Object.hasOwn(value, key)) errors.push({ code: 'trackb-input-invalid', instancePath: `/frame/${key}`, message: 'frame field is required' });
}
