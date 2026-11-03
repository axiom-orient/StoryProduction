#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { CLI_COMMAND_REGISTRY } from '../packages/application/src/command-registry.js';

const cli = resolve('packages/cli/src/main.js');
const help = execFileSync(process.execPath, [cli, '--help'], { encoding: 'utf8' });
const retained = CLI_COMMAND_REGISTRY.map(({ name }) => name);
for (const command of retained) {
  if (!help.includes(command)) throw new Error(`help missing retained command: ${command}`);
}
for (const command of [
  'compile-project', 'compile-prompt-catalog', 'start-generation', 'generation-status',
  'cancel-generation', 'recover-generation', 'resolve-unknowns', 'validate-screen',
  'approve-contract', 'propose-target-ir', 'derive-viewports', 'reconstruct-screen',
  'prepare-release', 'approve-release', 'accept-revision', 'export-revision',
  'compile-scene', 'compile-scene-briefs', 'evaluate-scene-gate', 'assemble-animatic',
  'assemble-final-cut', 'derive-time-variants'
]) {
  try {
    execFileSync(process.execPath, [cli, command], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    const output = `${error.stdout ?? ''}${error.stderr ?? ''}`;
    if (!/Unknown command|cli-command-unknown/.test(output)) throw new Error(`retired command did not fail canonically: ${command}`);
    continue;
  }
  throw new Error(`retired command unexpectedly succeeded: ${command}`);
}
console.log(JSON.stringify({ status: 'PASS', retainedCommands: retained.length, retiredCommands: 22 }, null, 2));
