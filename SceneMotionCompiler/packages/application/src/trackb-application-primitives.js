/** Small value and diagnostic primitives shared inside the Track B application boundary. */
export function boundedMessage(message, fallback = 'Track B operation failed') {
  return String(message ?? fallback).slice(0, 512);
}

export function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}
