import test from 'node:test';
import assert from 'node:assert/strict';
import { cp, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { findFontForText } from '../packages/browser-renderer/src/chromium.js';

test('explicit font configuration wins over Korean discovery and missing configured font fails without fallback', async () => {
  const previous = process.env.SMC_FONT_PATH;
  delete process.env.SMC_FONT_PATH;
  const root = await mkdtemp(join(tmpdir(), 'smc-configured-font-'));
  try {
    const discovered = await findFontForText('한국어');
    const configured = join(root, 'configured-font.ttf');
    await cp(discovered, configured);
    process.env.SMC_FONT_PATH = configured;
    assert.equal(await findFontForText('한국어'), configured);
    process.env.SMC_FONT_PATH = join(root, 'missing.ttf');
    await assert.rejects(findFontForText('한국어'), /configured font/);
  } finally {
    if (previous === undefined) delete process.env.SMC_FONT_PATH;
    else process.env.SMC_FONT_PATH = previous;
    await rm(root, { recursive: true, force: true });
  }
});
