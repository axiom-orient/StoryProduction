import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';

const CLI = resolve('packages/cli/src/main.js');
const run = (args) => execFileSync(process.execPath, [CLI, ...args], { encoding: 'utf8' });
const fail = (args) => {
  try { run(args); } catch (error) { return `${error.stdout ?? ''}${error.stderr ?? ''}`; }
  throw new Error(`expected CLI failure: ${args.join(' ')}`);
};

test('help exposes exactly the seven production and Track B product commands', () => {
  const help = run(['--help']);
  for (const command of ['production-prepare', 'project-narrative-bundle', 'compile-scene-html', 'trackb-evidence-measure', 'trackb-release-prepare', 'trackb-release-promote', 'trackb-release-status']) {
    assert.match(help, new RegExp(command));
  }
  for (const retired of ['compile-project', 'compile-scene', 'start-generation', 'assemble-final-cut', 'derive-time-variants']) {
    assert.doesNotMatch(help, retired === 'compile-scene' ? /compile-scene(?!-)/ : new RegExp(`\\b${retired}\\b`));
  }
});

test('retired commands fail with the canonical unknown-command error', () => {
  for (const command of ['compile-project', 'compile-scene', 'approve-contract', 'render-video', 'assemble-animatic', 'derive-time-variants']) {
    assert.match(fail([command]), /cli-command-unknown|Unknown command/);
  }
});

test('release command help keeps the Track B option contract', () => {
  const help = run(['trackb-release-prepare', '--help']);
  for (const option of ['narrative-authority-path', 'projection-receipt-path', 'asset-handoff-path', 'html-path', 'provenance-path', 'candidate-id', 'expected-base-delivery-set-hash']) {
    assert.match(help, new RegExp(`--${option} <value>`));
  }
});
