import { sha256Prefixed } from '../../core/src/canonical.js';

/**
 * The renderer injects this source before every document.  The source is
 * hashed before its per-run binding name and nonce are materialized so the
 * receipt can carry a deterministic, contract-owned trust anchor without
 * exposing the nonce in the artifact.
 */
export const TRACK_B_NAVIGATION_GUARD_MECHANISM = 'cdp-binding-nonce';

export const TRACK_B_NAVIGATION_GUARD_SOURCE_TEMPLATE = String.raw`(() => {
  const bindingName = "__SMC_NAVIGATION_GUARD_BINDING__";
  const nonce = "__SMC_NAVIGATION_GUARD_NONCE__";
  const binding = globalThis[bindingName];
  const report = (type, details = {}) => {
    if (typeof binding !== 'function') return;
    try { binding(JSON.stringify({ nonce, type, ...details })); } catch { /* CDP owns the evidence boundary. */ }
  };
  const allow = (value) => {
    const raw = String(value ?? '');
    if (/^(?:about:blank|about:srcdoc|data:)/i.test(raw)) return true;
    try {
      const resolved = new URL(raw, location.href).href;
      return /^(?:about:blank|about:srcdoc|data:)/i.test(resolved);
    } catch { return false; }
  };
  const attempt = (url, kind) => report('attempt', { url: String(url ?? 'forbidden-navigation'), kind });
  try {
    if (typeof binding !== 'function') throw new Error('navigation guard binding unavailable');
    if (typeof globalThis.navigation !== 'object' || typeof globalThis.navigation.addEventListener !== 'function') {
      throw new Error('Navigation API unavailable');
    }
    globalThis.navigation.addEventListener('navigate', (event) => {
      const url = String(event.destination?.url ?? 'forbidden-navigation');
      if (allow(url)) return;
      attempt(url, 'navigation-api');
      try { event.preventDefault(); } catch { /* CDP event evidence remains authoritative. */ }
    });
    const originalOpen = globalThis.open;
    if (typeof originalOpen === 'function') {
      globalThis.open = function guardedOpen(url, ...args) {
        if (!allow(url)) { attempt(url, 'window.open'); return null; }
        return originalOpen.call(this, url, ...args);
      };
    }
    const guardHistory = (name) => {
      const original = globalThis.history?.[name];
      if (typeof original !== 'function') return;
      globalThis.history[name] = function guardedHistory(...args) {
        const url = args.length > 2 ? args[2] : undefined;
        if (name === 'go' || name === 'back' || name === 'forward' || (url !== undefined && !allow(url))) {
          attempt(url, 'history.' + name);
          return undefined;
        }
        return original.apply(this, args);
      };
    };
    for (const name of ['pushState', 'replaceState', 'go', 'back', 'forward']) guardHistory(name);
    const formPrototype = globalThis.HTMLFormElement?.prototype;
    if (formPrototype) {
      for (const name of ['submit', 'requestSubmit']) {
        const original = formPrototype[name];
        if (typeof original !== 'function') continue;
        formPrototype[name] = function guardedSubmit(...args) {
          const url = this?.action || this?.getAttribute?.('action') || location.href;
          if (!allow(url)) { attempt(url, 'form.' + name); return undefined; }
          return original.apply(this, args);
        };
      }
    }
    report('ready', { url: String(location.href) });
  } catch (error) {
    report('error', { message: String(error?.message ?? error), url: String(location.href) });
  }
})();`;

export const TRACK_B_NAVIGATION_GUARD_SOURCE_HASH = sha256Prefixed(TRACK_B_NAVIGATION_GUARD_SOURCE_TEMPLATE);

export function materializeTrackBNavigationGuardSource(bindingName, nonce) {
  if (typeof bindingName !== 'string' || bindingName.length === 0 || typeof nonce !== 'string' || nonce.length === 0) {
    throw new TypeError('navigation guard bindingName and nonce are required');
  }
  return TRACK_B_NAVIGATION_GUARD_SOURCE_TEMPLATE
    .replace('"__SMC_NAVIGATION_GUARD_BINDING__"', JSON.stringify(bindingName))
    .replace('"__SMC_NAVIGATION_GUARD_NONCE__"', JSON.stringify(nonce));
}
