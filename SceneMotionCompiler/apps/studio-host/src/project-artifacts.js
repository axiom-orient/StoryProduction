import { isAbsolute, relative, resolve, sep } from 'node:path';
import { constants as fsConstants, readFileSync } from 'node:fs';
import { open } from 'node:fs/promises';

const SCENE_OUTPUT_SECURITY_HEADERS = Object.freeze({
  'Content-Security-Policy': "sandbox allow-scripts; default-src 'none'; connect-src 'none'; form-action 'none'; object-src 'none'; frame-src 'none'; child-src 'none'; worker-src 'none'; media-src 'none'; font-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'",
  'Referrer-Policy': 'no-referrer',
  'X-Content-Type-Options': 'nosniff'
});

export async function serveProjectArtifact(req, res, pathname, projectRoot) {
  // The Studio origin is privileged by the Track B review-session boundary.
  // Serve only the fixed compiler output; arbitrary project HTML/media could
  // execute in this origin and manufacture browser-shaped approval requests.
  if (pathname !== '/scene-output.html') {
    res.writeHead(404); return res.end('Not Found');
  }
  let decoded;
  try { decoded = decodeURIComponent(pathname); }
  catch { res.writeHead(400); return res.end('Invalid path'); }
  const target = resolve(projectRoot, `.${decoded}`);
  const root = resolve(projectRoot);
  const relativePath = relative(root, target);
  if (!relativePath || relativePath === '..' || relativePath.startsWith(`..${sep}`) || isAbsolute(relativePath) || decoded.includes('\0')) {
    res.writeHead(404); return res.end('Not Found');
  }
  let handle;
  try {
    handle = await open(target, fsConstants.O_RDONLY | fsConstants.O_NOFOLLOW);
  } catch (error) {
    res.writeHead(error?.code === 'ENOENT' ? 404 : 403);
    return res.end(error?.code === 'ENOENT' ? 'Not Found' : 'Forbidden');
  }
  try {
    const entry = await handle.stat();
    if (!entry.isFile()) {
      res.writeHead(403);
      return res.end('Forbidden');
    }
    let bytes;
    try {
      bytes = await handle.readFile();
    } catch {
      res.writeHead(500);
      return res.end('Artifact read failed');
    }
    res.writeHead(200, {
      'Content-Type': artifactContentType(target),
      'Cache-Control': 'no-store',
      ...SCENE_OUTPUT_SECURITY_HEADERS
    });
    return res.end(bytes);
  } finally {
    await handle.close().catch(() => {});
  }
}

export async function hasSafeSceneOutput(projectRoot) {
  const target = resolve(projectRoot, 'scene-output.html');
  let handle;
  try {
    handle = await open(target, fsConstants.O_RDONLY | fsConstants.O_NOFOLLOW);
    return (await handle.stat()).isFile();
  } catch {
    return false;
  } finally {
    await handle?.close().catch(() => {});
  }
}

export function renderDashboard() {
  return readFileSync(resolve(import.meta.dirname, 'index.html'), 'utf8');
}

function artifactContentType(path) {
  if (/\.html$/i.test(path)) return 'text/html; charset=utf-8';
  if (/\.png$/i.test(path)) return 'image/png';
  if (/\.mp4$/i.test(path)) return 'video/mp4';
  return 'video/webm';
}
