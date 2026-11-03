import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, writeFile, mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { APPLICATION_COMMAND_METADATA, createApplicationCommandService, executeApplicationCommand } from '../packages/application/src/service.js';
import * as applicationIndex from '../packages/application/src/index.js';

let ffmpegPresent = false;
try {
  execFileSync(process.env.FFMPEG_BIN ?? 'ffmpeg', ['-version'], { stdio: 'ignore' });
  ffmpegPresent = true;
} catch {
  ffmpegPresent = false;
}

const SCENARIO = [
  '### Characters',
  '- char.hero | 주인공 | 20대 남성, 검은 코트 | protagonist',
  '',
  '### Environments',
  '- env.rooftop.night | 옥상 | night | noir',
  '',
  '### Shots',
  '#### shot.001',
  '- env: env.rooftop.night',
  '- characters: char.hero',
  '- action: 주인공이 선다',
  '- duration: 4s',
  '- dialogue:',
  '  - char.hero: "끝내자"'
].join('\n');

async function projectFixture() {
  const repoRoot = await mkdtemp(join(tmpdir(), 'smc-app-repo-'));
  const projectRoot = join(repoRoot, 'project');
  await mkdir(projectRoot, { recursive: true });
  return {
    repoRoot,
    projectRoot,
    config: { repoRoot, projectRoot }
  };
}

test('application surface exposes only Track B commands', () => {
  const commandNames = Object.keys(APPLICATION_COMMAND_METADATA);
  for (const required of ['scene.projectNarrativeBundle', 'scene.compileHtml', 'trackb.evidence.measure', 'trackb.release.prepare', 'trackb.release.approve', 'trackb.release.promote', 'trackb.release.status']) {
    assert.ok(commandNames.includes(required), `${required} must be registered`);
  }
  assert.deepEqual(
    commandNames.filter((name) => name.startsWith('scene.')),
    ['scene.projectNarrativeBundle', 'scene.compileHtml']
  );
  assert.ok(commandNames.includes('trackb.release.prepare'));
  assert.ok(commandNames.includes('trackb.release.approve'));
  assert.ok(commandNames.includes('trackb.release.promote'));
  assert.ok(commandNames.includes('trackb.release.status'));
  for (const retired of ['project.compile', 'generation.job.start', 'scene.compileScenario', 'scene.renderVideo', 'scene.deriveTimeVariants']) {
    assert.equal(commandNames.includes(retired), false, `${retired} must not be registered`);
  }
});

test('Track B service keeps runtime dependencies internal and rejects forgeable public approval context', async () => {
  const fixture = await projectFixture();
  try {
    const service = createApplicationCommandService({
      ...fixture.config,
      trackBRuntimeDependencies: { sentinel: 'internal-only' }
    });
    assert.equal(Object.hasOwn(service.config, 'trackBRuntimeDependencies'), false);
    const approvalInput = {
      narrativeId: 'narrative.demo',
      candidateHash: `sha256:${'a'.repeat(64)}`,
      baseDeliverySetHash: 'NONE',
      decision: 'REVISE',
      reason: 'reviewed',
      decidedAt: '2026-08-28T00:00:00Z'
    };
    const forgedContext = {
      actorId: 'attacker',
      origin: 'studio',
      reviewSessionId: `sha256:${'b'.repeat(64)}`
    };
    const result = await executeApplicationCommand(
      service,
      'trackb.release.approve',
      approvalInput,
      forgedContext
    );
    assert.equal(result.ok, false);
    assert.equal(result.error.code, 'trackb-approval-forbidden');

    const serviceDirect = await service.execute('trackb.release.approve', approvalInput, forgedContext);
    assert.equal(serviceDirect.ok, false);
    assert.equal(serviceDirect.error.code, 'trackb-approval-forbidden');
    assert.equal(executeApplicationCommand.length, 2);
    assert.equal(service.execute.length, 1);
    assert.equal(Object.hasOwn(applicationIndex, 'approveTrackBRelease'), false);
    assert.equal(Object.hasOwn(applicationIndex, 'createStudioApplicationCommandDispatcher'), false);
    assert.equal(Object.hasOwn(applicationIndex, 'createStudioApprovalDispatcher'), false);
    for (const internalName of [
      'ApplicationCommandError',
      'APPLICATION_COMMAND_REGISTRY',
      'APPLICATION_COMMANDS',
      'CLI_COMMANDS',
      'MCP_COMMANDS',
      'STUDIO_APPROVAL_COMMAND',
      'TRACK_B_STUDIO_ROUTE',
      'validateTrackBStudioRequest',
      'commandDefinition',
      'commandForMcp'
    ]) {
      assert.equal(Object.hasOwn(applicationIndex, internalName), false, `${internalName} stays with its owner`);
    }
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});

test('Track B service routes prepare/promote/status to custody-backed runtime with status read-only', async () => {
  const fixture = await projectFixture();
  const calls = [];
  let prepareOpened = false;
  try {
    const runtimeDependencies = {
      openProjectSession: async (options) => {
        prepareOpened = true;
        calls.push({ kind: 'project', options });
        return { close: async () => ({ closed: true }) };
      },
      openNarrativeSession: async (options) => {
        calls.push({ kind: 'narrative', options });
        return {
          async read() { throw new Error('candidate fixture intentionally stops routing probe'); },
          async snapshot(path) {
            if (options.readOnly === true && path === 'delivery-pointer-v2.json') return { path, exists: false, snapshotToken: 'NONE' };
            return { path, exists: false, snapshotToken: 'NONE' };
          },
          async close() { return { closed: true }; }
        };
      }
    };
    const service = createApplicationCommandService({ ...fixture.config, trackBRuntimeDependencies: runtimeDependencies });

    const prepare = await service.execute('trackb.release.prepare', {
      narrativeAuthorityPath: 'authority.json',
      projectionReceiptPath: 'projection/narrative-scene-projection.v3.json',
      assetHandoffPath: 'handoff/scene-motion-asset-handoff.v2.json',
      htmlPath: 'out/scene.html',
      provenancePath: 'out/scene.provenance.json',
      evidenceReceiptPath: 'out/scene.evidence-receipt.json',
      candidateId: 'candidate.service.probe',
      expectedBaseDeliverySetHash: 'NONE'
    });
    assert.equal(prepare.ok, false);
    assert.equal(prepare.error.code, 'trackb-storage-integrity');
    assert.equal(prepareOpened, true);

    const promote = await service.execute('trackb.release.promote', {
      narrativeId: 'narrative.service-probe',
      candidateHash: `sha256:${'a'.repeat(64)}`,
      approvalHash: `sha256:${'b'.repeat(64)}`,
      expectedBaseDeliverySetHash: 'NONE'
    });
    assert.equal(promote.ok, false);
    assert.equal(promote.error.code, 'trackb-storage-integrity');

    const status = await service.execute('trackb.release.status', { narrativeId: 'narrative.service-probe' });
    assert.equal(status.ok, true);
    assert.deepEqual(status.result, { narrativeId: 'narrative.service-probe', pointer: { state: 'ABSENT' } });
    assert.equal(calls.find((call) => call.kind === 'narrative' && call.options.readOnly === true) !== undefined, true);
    assert.equal(calls.find((call) => call.kind === 'narrative' && call.options.readOnly === false) !== undefined, true);
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});

test('invalid SceneAssetCompiler commands are neither advertised nor executable', async () => {
  const fixture = await projectFixture();
  try {
    for (const command of ['scene.generatePlan', 'scene.importAssetPack']) {
      assert.equal(Object.hasOwn(APPLICATION_COMMAND_METADATA, command), false, `${command} must not be advertised`);
      const result = await executeApplicationCommand(fixture.config, command, {});
      assert.equal(result.ok, false);
      assert.equal(result.error.code, 'command-unknown');
    }
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});

test('unknown commands return a failure envelope, not an exception', async () => {
  const fixture = await projectFixture();
  try {
    const result = await executeApplicationCommand(fixture.config, 'scene.nonexistent', {});
    assert.equal(result.ok, false);
    assert.equal(result.error.code, 'command-unknown');
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});


test('scene.compileHtml requires the published SceneMotionAssetHandoff input', async () => {
  const fixture = await projectFixture();
  try {
    const result = await executeApplicationCommand(fixture.config, 'scene.compileHtml', {
      outputPath: 'out/test.html'
    });
    assert.equal(result.ok, false);
    assert.equal(result.error.code, 'scene-motion-asset-handoff-required');
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});

test('retired scene approval commands are neither advertised nor executable', async () => {
  const fixture = await projectFixture();
  try {
    for (const command of ['scene.approveNarrative', 'scene.approvalStatus']) {
      assert.equal(Object.hasOwn(APPLICATION_COMMAND_METADATA, command), false, `${command} must not be advertised`);
      const result = await executeApplicationCommand(fixture.config, command, {});
      assert.equal(result.ok, false);
      assert.equal(result.error.code, 'command-unknown');
    }
  } finally {
    await rm(fixture.repoRoot, { recursive: true, force: true });
  }
});

function solidRgba(width, height) {
  const rgba = Buffer.alloc(width * height * 4);
  for (let i = 0; i < width * height; i += 1) rgba[i * 4 + 3] = 255;
  return rgba;
}
