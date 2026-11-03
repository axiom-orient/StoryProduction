import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, readdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { publishSceneHtml } from '../packages/application/src/scene-html-output.js';

async function workspace(t) {
  const root = await mkdtemp(join(tmpdir(), 'smc-output-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}
function publish(root, outputPath, extra = {}) {
  return publishSceneHtml({ projectRoot: root, outputPath, html: '<html>new</html>', provenance: { htmlHash: 'fixture' }, ...extra });
}
test('writes nested HTML and provenance, replacing normal outputs without temp residue', async (t) => {
  const root = await workspace(t);
  const first = await publish(root, 'render/scene.html');
  assert.equal(await readFile(first.outputPath, 'utf8'), '<html>new</html>');
  assert.deepEqual(JSON.parse(await readFile(first.provenancePath)), { htmlHash: 'fixture' });
  await publish(root, 'render/scene.html', { html: 'replacement' });
  assert.equal(await readFile(first.outputPath, 'utf8'), 'replacement');
  assert.deepEqual((await readdir(join(root, 'render'))).sort(), ['scene.html', 'scene.html.provenance.json']);
});
test('rejects ancestor symlink without writing outside', async (t) => {
  const root = await workspace(t); const outside = await workspace(t);
  await symlink(outside, join(root, 'link'), 'dir');
  await assert.rejects(publish(root, 'link/scene.html'), /symlink/);
  assert.deepEqual(await readdir(outside), []);
});
test('rejects a leaf symlink and preserves its target', async (t) => {
  const root = await workspace(t);
  await writeFile(join(root, 'source.txt'), 'original');
  await symlink('source.txt', join(root, 'scene.html'));
  await assert.rejects(publish(root, 'scene.html'), /symlink/);
  assert.equal(await readFile(join(root, 'source.txt'), 'utf8'), 'original');
});
test('preflights provenance before replacing existing HTML', async (t) => {
  const root = await workspace(t);
  await writeFile(join(root, 'scene.html'), 'old');
  await mkdir(join(root, 'scene.html.provenance.json'));
  await assert.rejects(publish(root, 'scene.html'), /invalid file type/);
  assert.equal(await readFile(join(root, 'scene.html'), 'utf8'), 'old');
});
test('protects verified input and provenance-alias targets', async (t) => {
  const root = await workspace(t);
  for (const target of ['source.json', 'scene.html.provenance.json']) await writeFile(join(root, target), 'original');
  await assert.rejects(publish(root, 'source.json', { protectedPaths: ['source.json'] }), /verified input/);
  await assert.rejects(publish(root, 'scene.html', { protectedPaths: ['scene.html.provenance.json'] }), /verified input/);
  assert.equal(await readFile(join(root, 'source.json'), 'utf8'), 'original');
  assert.equal(await readFile(join(root, 'scene.html.provenance.json'), 'utf8'), 'original');
});
test('rejects escape, root-as-output, and NUL without effects', async (t) => {
  const root = await workspace(t);
  for (const target of ['../outside.html', root, 'bad\0path']) await assert.rejects(publish(root, target));
  assert.deepEqual(await readdir(root), []);
});
