import { lstat, mkdir, open, rename, unlink } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { basename, dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { ApplicationCommandError } from './command-error.js';

function unsafe(message) {
  return new ApplicationCommandError('configuration-path-unsafe', message);
}

async function inspectPath(root, target, { createParents = false } = {}) {
  const rootInfo = await lstat(root);
  if (rootInfo.isSymbolicLink() || !rootInfo.isDirectory()) throw unsafe('projectRoot must be a non-symlink directory');
  const parts = relative(root, target).split(sep);
  let current = root;
  for (const [index, part] of parts.entries()) {
    current = resolve(current, part);
    const last = index === parts.length - 1;
    let info;
    try { info = await lstat(current); } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      if (!last && createParents) {
        try { await mkdir(current); } catch (error) { if (error.code !== 'EEXIST') throw error; }
        info = await lstat(current);
      } else { continue; }
    }
    if (info.isSymbolicLink()) throw unsafe(`output cannot traverse a symlink: ${current}`);
    if (last ? !info.isFile() : !info.isDirectory()) throw unsafe(`output path has an invalid file type: ${current}`);
  }
}

/** Publish HTML, then its hash-bound provenance. Not a multi-file transaction.
 * Cooperative workspace ownership is required; this is not a hostile directory-swap sandbox.
 * Both targets are checked before effects; each file is replaced without following a leaf link.
 */
export async function publishSceneHtml({ projectRoot, outputPath, html, provenance, protectedPaths = [] }) {
  const root = resolve(projectRoot);
  if (typeof outputPath !== 'string' || !outputPath.trim() || outputPath.includes('\0')) throw unsafe('outputPath is required');
  const target = isAbsolute(outputPath) ? resolve(outputPath) : resolve(root, outputPath);
  const outputRelative = relative(root, target);
  if (!outputRelative || outputRelative === ".."
    || outputRelative.startsWith(".." + sep) || isAbsolute(outputRelative)) {
    throw unsafe("outputPath must be a file inside projectRoot");
  }
  const provenancePath = `${target}.provenance.json`;
  const protectedSet = new Set(protectedPaths.map((path) => resolve(root, path)));
  for (const path of [target, provenancePath]) {
    if (protectedSet.has(path)) throw unsafe('HTML output cannot replace a verified input artifact');
    await inspectPath(root, path);
  }
  const pending = new Set();
  let outputCommitted = false;
  let provenanceCommitted = false;
  async function stage(path, data) {
    await inspectPath(root, path, { createParents: true });
    const temp = resolve(dirname(path), `.${basename(path)}.${randomUUID()}.tmp`);
    const file = await open(temp, 'wx', 0o600);
    pending.add(temp);
    try { await file.writeFile(data, 'utf8'); await file.sync(); } finally { await file.close(); }
    return temp;
  }
  try {
    const htmlTemp = await stage(target, html);
    const provenanceTemp = await stage(provenancePath, `${JSON.stringify(provenance, null, 2)}\n`);
    // Recheck just before replacement. Hash-bound provenance lets readers reject a torn pair.
    await inspectPath(root, target);
    await inspectPath(root, provenancePath);
    await rename(htmlTemp, target); pending.delete(htmlTemp); outputCommitted = true;
    await rename(provenanceTemp, provenancePath); pending.delete(provenanceTemp); provenanceCommitted = true;
    const directory = await open(dirname(target), 'r');
    try { await directory.sync(); } finally { await directory.close(); }
    return { outputPath: target, provenancePath };
  } catch (error) {
    throw new ApplicationCommandError('scene-html-output-incomplete', `HTML publication failed: ${error.message}`, {
      cause: error, details: { outputPath: target, provenancePath, outputCommitted, provenanceCommitted }
    });
  } finally {
    for (const temp of pending) {
      try { await unlink(temp); } catch (error) { if (error.code !== 'ENOENT') { /* Preserve the primary publication error. */ } }
    }
  }
}
