import { access, lstat, mkdtemp, readFile, realpath, rm } from 'node:fs/promises';
import { constants } from 'node:fs';
import { arch, platform, tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { sha256Prefixed } from '../../core/src/canonical.js';

const BROWSER_CANDIDATES = [
  process.env.SMC_CHROMIUM_PATH,
  process.env.CHROMIUM_PATH,
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/usr/bin/google-chrome',
  '/usr/bin/google-chrome-stable',
  '/opt/google/chrome/chrome',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium'
].filter(Boolean);

const FONT_CANDIDATES = [
  process.env.SMC_FONT_PATH,
  '/System/Library/Fonts/AppleSDGothicNeo.ttc',
  '/System/Library/Fonts/Helvetica.ttc',
  '/System/Library/Fonts/ArialHB.ttc',
  '/Library/Fonts/Arial Unicode.ttf',
  '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  '/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf',
  '/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf'
].filter(Boolean);

const RENDERER_LOCK_KEYS = new Set([
  'schemaVersion', 'rendererLockId', 'browser', 'font', 'os', 'architecture',
  'locale', 'timezone', 'colorScheme', 'reducedMotion', 'networkPolicy',
  'screenshotFormat', 'fontPolicy', 'implementationVersion'
]);

export async function findChromium() {
  const discovered = [...BROWSER_CANDIDATES];
  for (const command of ['chromium', 'chromium-browser', 'google-chrome', 'google-chrome-stable', 'chrome']) {
    const found = spawnSync('which', [command], { encoding: 'utf8' });
    if (found.status === 0 && found.stdout.trim()) discovered.push(found.stdout.trim());
  }
  for (const candidate of unique(discovered)) {
    try {
      await access(candidate, constants.X_OK);
      return candidate;
    } catch { /* next */ }
  }
  throw new Error(`Chromium executable not found. Checked: ${unique(discovered).join(', ')}`);
}

export async function findFont() {
  return findFontForText('');
}

// Pick a single, byte-bound face that can render the document's actual copy.
// A generic Latin sans face may load successfully yet silently fall back for
// Korean (or another script), which makes the captured pixels non-reproducible
// and defeats the locked-font gate. The returned path is still hashed into the
// normal renderer lock; this function only chooses the admission-time face.
export async function findFontForText(text = '') {
  if (process.env.SMC_FONT_PATH) {
    // Configuration wins over discovery policy. A bad explicit font is an error,
    // not permission to select another font or embed different bytes silently.
    await readRegularFileBinding(process.env.SMC_FONT_PATH, 'configured font');
    return process.env.SMC_FONT_PATH;
  }
  const discovered = [];
  const language = languageForFontMatch(text);
  if (language) {
    const languageMatched = spawnSync('fc-match', ['-f', '%{file}\n', `:lang=${language}`], { encoding: 'utf8' });
    if (languageMatched.status === 0 && languageMatched.stdout.trim()) discovered.push(languageMatched.stdout.trim().split(/\r?\n/)[0]);
    if (language === 'ko') discovered.unshift('/System/Library/Fonts/AppleSDGothicNeo.ttc');
  }
  discovered.push(...FONT_CANDIDATES.filter((candidate) => candidate !== process.env.SMC_FONT_PATH));
  const matched = spawnSync('fc-match', ['-f', '%{file}\n', 'sans-serif'], { encoding: 'utf8' });
  if (matched.status === 0 && matched.stdout.trim()) discovered.push(matched.stdout.trim().split(/\r?\n/)[0]);
  for (const candidate of unique(discovered)) {
    try {
      await access(candidate, constants.R_OK);
      return candidate;
    } catch { /* next */ }
  }
  throw new Error(`Sans-serif font file not found. Checked: ${unique(discovered).join(', ')}`);
}

function unique(values) { return [...new Set(values.filter(Boolean))]; }

/**
 * Validate the public lock shape without touching the referenced binaries.
 * This is intentionally exported so acceptance and runtime code can keep the
 * same contract while choosing their own byte/runtime checks.
 */
export function assertRendererLockShape(lock) {
  if (!lock || typeof lock !== 'object' || Array.isArray(lock)) throw new Error('Renderer lock must be an object');
  if (Object.keys(lock).some((name) => !RENDERER_LOCK_KEYS.has(name))) throw new Error('Renderer lock has unexpected fields');
  if (lock.schemaVersion !== 1) throw new Error('Renderer lock schemaVersion must be 1');
  if (typeof lock.rendererLockId !== 'string' || !/^[A-Za-z][A-Za-z0-9._:-]{1,127}$/.test(lock.rendererLockId)) throw new Error('Renderer lock rendererLockId is invalid');
  const browser = lock.browser;
  if (!browser || typeof browser !== 'object' || Array.isArray(browser) || Object.keys(browser).some((name) => !['name', 'version', 'binaryPath', 'binarySha256', 'binaryBytes'].includes(name))) throw new Error('Renderer lock browser binding is invalid');
  if (browser.name !== 'chromium' || typeof browser.version !== 'string' || browser.version.length === 0 || typeof browser.binaryPath !== 'string' || browser.binaryPath.length === 0 || !/^sha256:[a-f0-9]{64}$/.test(browser.binarySha256) || !Number.isInteger(browser.binaryBytes) || browser.binaryBytes < 0) throw new Error('Renderer lock browser binding is invalid');
  const font = lock.font;
  if (!font || typeof font !== 'object' || Array.isArray(font) || Object.keys(font).some((name) => !['path', 'sha256'].includes(name))) throw new Error('Renderer lock font binding is invalid');
  if (typeof font.path !== 'string' || font.path.length === 0 || !/^sha256:[a-f0-9]{64}$/.test(font.sha256)) throw new Error('Renderer lock font binding is invalid');
  if (typeof lock.os !== 'string' || lock.os.length === 0 || typeof lock.architecture !== 'string' || lock.architecture.length === 0) throw new Error('Renderer lock platform binding is invalid');
  if (typeof lock.locale !== 'string' || lock.locale.length < 2 || typeof lock.timezone !== 'string' || lock.timezone.length === 0) throw new Error('Renderer lock locale/timezone binding is invalid');
  if (!['light', 'dark'].includes(lock.colorScheme) || !['reduce', 'no-preference'].includes(lock.reducedMotion)) throw new Error('Renderer lock media binding is invalid');
  if (lock.networkPolicy !== 'deny-external' || lock.screenshotFormat !== 'png' || lock.fontPolicy !== 'force-locked-font' || typeof lock.implementationVersion !== 'string' || lock.implementationVersion.length === 0) throw new Error('Renderer lock policy binding is invalid');
  return lock;
}

export async function createRendererLock(profile = {}) {
  const browserPath = profile.browserPath ?? await findChromium();
  const fontPath = profile.fontPath ?? await findFontForText(profile.text ?? '');
  const browserBinding = await readRegularFileBinding(browserPath, 'browser binary', { executable: true });
  const fontBinding = await readRegularFileBinding(fontPath, 'font');
  const version = queryBrowserVersion(browserPath);
  const lock = {
    schemaVersion: 1,
    rendererLockId: profile.rendererLockId ?? `renderer.chromium.${platform()}.${arch()}`,
    browser: {
      name: 'chromium',
      version,
      binaryPath: browserPath,
      binarySha256: browserBinding.sha256,
      binaryBytes: browserBinding.bytes
    },
    font: {
      path: fontPath,
      sha256: fontBinding.sha256
    },
    os: platform(),
    architecture: arch(),
    locale: profile.locale ?? 'en-US',
    timezone: profile.timezone ?? 'UTC',
    colorScheme: profile.colorScheme ?? 'light',
    reducedMotion: profile.reducedMotion ?? 'reduce',
    networkPolicy: 'deny-external',
    screenshotFormat: 'png',
    fontPolicy: 'force-locked-font',
    implementationVersion: profile.implementationVersion ?? '0.1.0'
  };
  assertRendererLockShape(lock);
  return lock;
}

function languageForFontMatch(text) {
  const value = String(text ?? '');
  if (/[\u1100-\u11ff\u3130-\u318f\uac00-\ud7af]/u.test(value)) return 'ko';
  if (/[\u3040-\u30ff]/u.test(value)) return 'ja';
  if (/[\u4e00-\u9fff\u3400-\u4dbf]/u.test(value)) return 'zh-cn';
  if (/[\u0400-\u04ff]/u.test(value)) return 'ru';
  if (/[\u0600-\u06ff]/u.test(value)) return 'ar';
  return null;
}

/** Validate the lock and all bytes before a browser process is spawned. */
export async function validateRendererLockForLaunch(lock) {
  assertRendererLockShape(lock);
  if (lock.os !== platform() || lock.architecture !== arch()) throw new Error(`Renderer lock platform mismatch: lock=${lock.os}/${lock.architecture} actual=${platform()}/${arch()}`);
  const browserBinding = await readRegularFileBinding(lock.browser.binaryPath, 'browser binary', { executable: true });
  if (browserBinding.sha256 !== lock.browser.binarySha256 || browserBinding.bytes !== lock.browser.binaryBytes) throw new Error('Renderer lock browser bytes are not bound');
  const fontBinding = await readRegularFileBinding(lock.font.path, 'font');
  if (fontBinding.sha256 !== lock.font.sha256) throw new Error('Renderer lock font bytes are not bound');
  const version = queryBrowserVersion(lock.browser.binaryPath);
  if (version !== lock.browser.version) throw new Error(`Renderer lock browser version mismatch: lock=${lock.browser.version} actual=${version}`);
  return lock;
}

async function readRegularFileBinding(path, label, { executable = false } = {}) {
  let details;
  try { details = await lstat(path); } catch (error) { throw new Error(`Renderer lock ${label} is unavailable: ${error.message}`); }
  if (details.isSymbolicLink() || !details.isFile()) throw new Error(`Renderer lock ${label} must be a regular non-symlink file`);
  try { await access(path, executable ? constants.X_OK : constants.R_OK); } catch (error) { throw new Error(`Renderer lock ${label} is not accessible: ${error.message}`); }
  const bytes = await readFile(path);
  if (label === 'font' && !isSupportedFontBytes(bytes)) throw new Error('Renderer lock font is not a supported sfnt/OTTO/ttcf/WOFF/WOFF2 font');
  return { bytes: details.size, sha256: sha256Prefixed(bytes), data: bytes };
}

export async function readVerifiedFontBytes(rendererLock) {
  assertRendererLockShape(rendererLock);
  const binding = await readRegularFileBinding(rendererLock.font.path, 'font');
  if (binding.sha256 !== rendererLock.font.sha256) throw new Error('Renderer lock font bytes are not bound');
  return binding.data;
}

export function isSupportedFontBytes(bytes) {
  if (!bytes || bytes.length < 4) return false;
  const magic = bytes.subarray(0, 4);
  return (magic[0] === 0x00 && magic[1] === 0x01 && magic[2] === 0x00 && magic[3] === 0x00)
    || magic.toString('ascii') === 'OTTO'
    || magic.toString('ascii') === 'ttcf'
    || magic.toString('ascii') === 'wOFF'
    || magic.toString('ascii') === 'wOF2';
}

function queryBrowserVersion(browserPath) {
  const result = spawnSync(browserPath, ['--version'], { encoding: 'utf8' });
  if (result.error || result.status !== 0) throw new Error(`Failed to query Chromium version: ${result.error?.message ?? result.stderr ?? result.status}`);
  const version = String(result.stdout ?? '').trim() || String(result.stderr ?? '').trim();
  if (!version) throw new Error('Chromium --version returned no version');
  return version;
}

export async function launchChromium(rendererLock) {
  // All lock/path/hash/version checks happen before mkdtemp and spawn. A
  // tampered lock therefore cannot create a browser or leave a profile behind.
  await validateRendererLockForLaunch(rendererLock);
  const userDataDir = await mkdtemp(join(tmpdir(), 'smc-chromium-'));
  const stderr = [];
  const args = [
    '--headless=new',
    '--no-sandbox',
    '--disable-gpu',
    '--disable-dev-shm-usage',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-default-apps',
    '--disable-extensions',
    '--disable-sync',
    '--metrics-recording-only',
    '--no-first-run',
    '--remote-debugging-port=0',
    `--user-data-dir=${userDataDir}`,
    'about:blank'
  ];
  let processHandle;
  try {
    processHandle = spawn(rendererLock.browser.binaryPath, args, { stdio: ['ignore', 'ignore', 'pipe'] });
    processHandle.stderr.setEncoding('utf8');
    processHandle.stderr.on('data', (chunk) => stderr.push(String(chunk)));
    // Keep a listener for late spawn errors so Node cannot surface an
    // unhandled process event after DevTools has become available.
    processHandle.on('error', (error) => stderr.push(`spawn-error: ${error.message}`));
    const activePortFile = join(userDataDir, 'DevToolsActivePort');
    const { port } = await waitForActivePort(activePortFile, processHandle);
    let closePromise;
    return {
      processHandle,
      port,
      userDataDir,
      stderr,
      async close() {
        if (!closePromise) closePromise = closeChromiumProcess(processHandle, userDataDir);
        return closePromise;
      }
    };
  } catch (error) {
    try {
      await closeChromiumProcess(processHandle, userDataDir);
    } catch (cleanupError) {
      // Preserve the launch cause while exposing that no-survivor cleanup was
      // not proven; callers can report both facts instead of seeing a fake
      // successful cleanup.
      error.cleanupError = cleanupError;
    }
    throw error;
  }
}

async function waitForActivePort(path, processHandle) {
  const deadline = Date.now() + 12_000;
  let spawnError;
  const onError = (error) => { spawnError = error; };
  processHandle.once('error', onError);
  try {
    while (Date.now() < deadline) {
      if (spawnError) throw new Error(`Failed to spawn Chromium: ${spawnError.message}`);
      if (processHandle.exitCode !== null) throw new Error(`Chromium exited before DevTools became available: ${processHandle.exitCode}`);
      try {
        const lines = (await readFile(path, 'utf8')).trim().split(/\r?\n/);
        const port = Number(lines[0]);
        if (Number.isInteger(port) && port > 0) return { port };
      } catch { /* wait */ }
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error('Timed out waiting for Chromium DevToolsActivePort');
  } finally {
    processHandle.removeListener('error', onError);
  }
}

async function closeChromiumProcess(processHandle, userDataDir) {
  try {
    if (processHandle && processHandle.exitCode === null) processHandle.kill('SIGTERM');
    if (processHandle) await waitForExit(processHandle, 2_000);
    if (processHandle && processHandle.exitCode === null) {
      processHandle.kill('SIGKILL');
      await waitForExit(processHandle, 2_000);
    }
  } finally {
    await removeWithRetry(userDataDir, 6, 100);
  }
}

export const DEFAULT_PAGE_ENDPOINT_TIMEOUT_MS = 10_000;
const PAGE_ENDPOINT_TIMEOUT_LIMITS = Object.freeze({ min: 100, max: 60_000 });

export class ChromiumEndpointError extends Error {
  constructor(code, message, details = {}, cause = undefined) {
    super(message, cause === undefined ? undefined : { cause });
    this.name = 'ChromiumEndpointError';
    this.code = code;
    this.details = details;
  }
}

function validatePageEndpointPort(port) {
  if (!Number.isInteger(port) || port < 1 || port > 65_535) {
    throw new ChromiumEndpointError('E_CHROMIUM_ENDPOINT_CONFIG', 'Chromium DevTools port must be an integer between 1 and 65535', { port });
  }
  return port;
}

function validatePageEndpointTimeout(timeoutMs) {
  if (!Number.isInteger(timeoutMs) || timeoutMs < PAGE_ENDPOINT_TIMEOUT_LIMITS.min || timeoutMs > PAGE_ENDPOINT_TIMEOUT_LIMITS.max) {
    throw new ChromiumEndpointError(
      'E_CHROMIUM_ENDPOINT_CONFIG',
      `Chromium target endpoint timeout must be an integer between ${PAGE_ENDPOINT_TIMEOUT_LIMITS.min} and ${PAGE_ENDPOINT_TIMEOUT_LIMITS.max} milliseconds`,
      { timeoutMs, limits: PAGE_ENDPOINT_TIMEOUT_LIMITS },
    );
  }
  return timeoutMs;
}

/**
 * Create a Chromium page target with a deadline covering both response
 * headers and the response body.  The caller owns Chromium cleanup; a typed
 * error makes that existing finally path observable without retry/fallback.
 */
export async function createPageEndpoint(port, url = 'about:blank', options = {}) {
  const normalizedPort = validatePageEndpointPort(port);
  if (typeof url !== 'string' || url.length === 0 || url.length > 2_048) {
    throw new ChromiumEndpointError('E_CHROMIUM_ENDPOINT_CONFIG', 'Chromium target URL must be a non-empty string of at most 2048 characters');
  }
  if (options === null || typeof options !== 'object' || Array.isArray(options)) {
    throw new ChromiumEndpointError('E_CHROMIUM_ENDPOINT_CONFIG', 'Chromium target endpoint options must be an object');
  }
  const timeoutMs = validatePageEndpointTimeout(options.timeoutMs ?? DEFAULT_PAGE_ENDPOINT_TIMEOUT_MS);
  const endpoint = `http://127.0.0.1:${normalizedPort}/json/new?${encodeURIComponent(url)}`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    let response;
    try {
      response = await fetch(endpoint, { method: 'PUT', signal: controller.signal });
    } catch (error) {
      if (controller.signal.aborted) {
        throw new ChromiumEndpointError(
          'E_CHROMIUM_ENDPOINT_TIMEOUT',
          `Timed out creating Chromium target after ${timeoutMs} milliseconds`,
          { port: normalizedPort, timeoutMs },
          error,
        );
      }
      throw new ChromiumEndpointError(
        'E_CHROMIUM_ENDPOINT_NETWORK',
        'Failed to reach the local Chromium DevTools endpoint',
        { port: normalizedPort },
        error,
      );
    }
    let body;
    try {
      body = await response.text();
    } catch (error) {
      if (controller.signal.aborted) {
        throw new ChromiumEndpointError(
          'E_CHROMIUM_ENDPOINT_TIMEOUT',
          `Timed out reading Chromium target response after ${timeoutMs} milliseconds`,
          { port: normalizedPort, timeoutMs },
          error,
        );
      }
      throw new ChromiumEndpointError('E_CHROMIUM_ENDPOINT_RESPONSE', 'Failed to read Chromium target response', { port: normalizedPort }, error);
    }
    if (!response.ok) {
      throw new ChromiumEndpointError(
        'E_CHROMIUM_ENDPOINT_HTTP',
        `Chromium target endpoint returned HTTP ${response.status}`,
        { port: normalizedPort, status: response.status },
      );
    }
    try {
      return JSON.parse(body);
    } catch (error) {
      throw new ChromiumEndpointError('E_CHROMIUM_ENDPOINT_PROTOCOL', 'Chromium target endpoint returned invalid JSON', { port: normalizedPort }, error);
    }
  } finally {
    clearTimeout(timer);
  }
}

async function waitForExit(processHandle, timeoutMs) {
  if (processHandle.exitCode !== null) return;
  await Promise.race([
    new Promise((resolve) => processHandle.once('exit', resolve)),
    new Promise((resolve) => setTimeout(resolve, timeoutMs))
  ]);
}

async function removeWithRetry(path, attempts, delayMs) {
  if (!path) return;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await rm(path, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === attempts) throw error;
      await new Promise((resolve) => setTimeout(resolve, delayMs));
    }
  }
}
