import test from 'node:test';
import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  APPLICATION_COMMAND_REGISTRY,
  CLI_COMMAND_REGISTRY,
  MCP_COMMAND_REGISTRY,
  STUDIO_COMMANDS
} from '../packages/application/src/command-registry.js';
import { TRACK_B_HANDLER_IMPLEMENTATIONS } from '../packages/application/src/trackb-commands.js';
import {
  loadSceneAssetCompilerContractRegistry,
  loadSmcContractRegistry
} from '../packages/contracts/src/closed-schema-set.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import * as applicationContract from '../packages/application/src/trackb-release-contract.js';
import * as trackBContract from '../packages/contracts/src/trackb-contract.js';
import * as measuredEvidence from '../packages/browser-renderer/src/index.js';
import * as custody from '../packages/store/src/index.js';

test('closed schema sets load only active files and reject archival/retired lookup', async () => {
  const smc = await loadSmcContractRegistry('contracts/schemas');
  const sceneAssetCompiler = await loadSceneAssetCompilerContractRegistry('contracts/schemas/scene-asset-compiler');
  assert.equal(smc.schema('asset-manifest.schema.json').schema.title, 'AssetManifest');
  assert.equal(sceneAssetCompiler.schema('scene-motion-asset-handoff-v2.schema.json').schema.title, 'SceneMotionAssetHandoff v2');
  for (const retired of ['evaluation-report.schema.json', 'narrative-authority.schema.json']) {
    assert.throws(() => smc.schema(retired), /Schema not found/);
  }
  assert.throws(() => sceneAssetCompiler.schema('scene-asset-release-request-v1.schema.json'), /Schema not found/);
});

test('closed schema sets reject unclassified files and dangling active refs', async () => {
  const root = await mkdtemp(join(tmpdir(), 'smc-closed-schema-'));
  try {
    const smcDir = join(root, 'smc');
    const sceneAssetCompilerDir = join(root, 'scene-asset-compiler');
    await cp('contracts/schemas', smcDir, { recursive: true });
    await cp('contracts/schemas/scene-asset-compiler', sceneAssetCompilerDir, { recursive: true });

    await writeFile(join(smcDir, 'unexpected.schema.json'), '{}');
    await assert.rejects(
      () => loadSmcContractRegistry(smcDir),
      (error) => error.code === 'closed-schema-set-unclassified'
    );
    await rm(join(smcDir, 'unexpected.schema.json'));

    await writeFile(join(sceneAssetCompilerDir, 'unexpected.schema.json'), '{}');
    await assert.rejects(
      () => loadSceneAssetCompilerContractRegistry(sceneAssetCompilerDir),
      (error) => error.code === 'closed-schema-set-unclassified'
    );
    await rm(join(sceneAssetCompilerDir, 'unexpected.schema.json'));

    const bindingPath = join(sceneAssetCompilerDir, 'binding-manifest-v1.schema.json');
    const binding = JSON.parse(await readFile(bindingPath, 'utf8'));
    binding.$defs.requirement.properties.kind.$ref = 'missing.schema.json#/$defs/kind';
    await writeFile(bindingPath, `${JSON.stringify(binding)}\n`);
    await assert.rejects(
      () => loadSceneAssetCompilerContractRegistry(sceneAssetCompilerDir),
      (error) => error.code === 'closed-schema-set-invalid' && /unresolved \$ref/.test(error.message)
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('Track B registry rejects files outside its exact registry-declared directory set', async () => {
  const root = await mkdtemp(join(tmpdir(), 'smc-trackb-registry-'));
  try {
    const schemaDir = join(root, 'trackb');
    await cp('contracts/schemas/trackb', schemaDir, { recursive: true });
    await writeFile(join(schemaDir, 'legacy.schema.json'), '{}');
    await assert.rejects(
      () => loadTrackBContractRegistry(schemaDir),
      (error) => error.code === 'trackb-registry-invalid' && /unclassified entry: legacy\.schema\.json/.test(error.message)
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('public Track B surfaces do not export retired forwarding aliases', () => {
  const retired = [
    ['Track B contract owner', trackBContract, ['loadTrackBArtifactRegistries', 'hashTrackBProjectionReceiptBytes', 'hashTrackBAssetHandoffBytes', 'hashTrackBEvidenceReceipt', 'hashTrackBSemanticQualification', 'hashTrackBNarrativeAuthority', 'hashTrackBReleaseApprovalReceipt']],
    ['application contract', applicationContract, ['projectTrackBCliSuccess', 'projectTrackBCliFailure', 'validateTrackBTransportEnvelope', 'validateTrackBTransportStatusResult']],
    ['measured evidence', measuredEvidence, [
      'measureTrackBMeasuredEvidence',
      'runTrackBMeasuredEvidence',
      'TrackBMeasuredEvidenceError',
      'TRACK_B_MEASURED_EVIDENCE_MAX_HTML_BYTES',
      'TRACK_B_MEASURED_EVIDENCE_MAX_SHOTS',
      'TRACK_B_MEASURED_EVIDENCE_DEFAULT_TIMEOUT_MS',
      'TRACK_B_MEASURED_EVIDENCE_TIMEOUT_LIMITS',
      'TRACK_B_NAVIGATION_GUARD_MECHANISM',
      'TRACK_B_NAVIGATION_GUARD_SOURCE_HASH',
      'TRACK_B_NAVIGATION_GUARD_SOURCE_TEMPLATE',
      'ChromiumEndpointError',
      'DEFAULT_PAGE_ENDPOINT_TIMEOUT_MS',
      'createPageEndpoint',
      'createRendererLock',
      'launchChromium',
      'installTrustedNavigationGuard',
      'validateNavigationGuardBindingEvent',
      'classifyRuntimeLogs',
      'collectNavigationGuardLogs',
      'configureCdpSession',
      'createRuntimeLogs'
    ]],
    ['custody', custody, [
      'defaultTrackBCustodyTargetTriple',
      'resolveTrackBCustodyBinary',
      'verifyTrackBCustodyReceipt',
      'TrackBCustodyError',
      'TRACKB_CUSTODY_BINARY_NAME',
      'TRACKB_CUSTODY_MAX_LINE_BYTES',
      'TRACKB_CUSTODY_MAX_POINTER_BYTES',
      'TRACKB_CUSTODY_MAX_RECEIPT_BYTES',
      'TRACKB_CUSTODY_MAX_BINARY_BYTES',
      'TRACKB_CUSTODY_RECEIPT_NAME',
      'TRACKB_PROJECT_ARTIFACT_KINDS',
      'TRACKB_MEASUREMENT_ARTIFACT_KINDS',
      'validateMeasurementArtifactResult',
      'validateMeasurementPublishResult'
    ]]
  ];
  for (const [surface, namespace, names] of retired) {
    for (const name of names) assert.equal(Object.hasOwn(namespace, name), false, `${surface} must not export ${name}`);
  }
});

test('AssetManifest uses the approval subject hash and rejects the retired visual target field', async () => {
  const registry = await loadSmcContractRegistry('contracts/schemas');
  const assetManifest = {
    schemaVersion: 1,
    assetManifestId: 'handoff.demo',
    approvalSubjectHash: `sha256:${'a'.repeat(64)}`,
    assets: []
  };
  assert.equal(registry.validate('asset-manifest.schema.json', assetManifest).valid, true);
  const retired = { ...assetManifest, visualTargetHash: assetManifest.approvalSubjectHash };
  delete retired.approvalSubjectHash;
  assert.equal(registry.validate('asset-manifest.schema.json', retired).valid, false);
});

test('command registry owns exact application and transport surfaces with handler coverage', () => {
  assert.equal(APPLICATION_COMMAND_REGISTRY.length, 12);
  assert.equal(CLI_COMMAND_REGISTRY.length, 11);
  assert.equal(MCP_COMMAND_REGISTRY.length, 7);
  assert.equal(STUDIO_COMMANDS.length, 3);
  assert.equal(new Set(APPLICATION_COMMAND_REGISTRY.map(({ command }) => command)).size, 12);
  assert.equal(new Set(APPLICATION_COMMAND_REGISTRY.map(({ implementation }) => implementation)).size, 12);
  assert.deepEqual(
    CLI_COMMAND_REGISTRY.map(({ command }) => command),
    APPLICATION_COMMAND_REGISTRY.filter(({ cli }) => cli).map(({ command }) => command)
  );
  assert.deepEqual(
    MCP_COMMAND_REGISTRY.map(({ command }) => command),
    APPLICATION_COMMAND_REGISTRY.filter(({ mcp }) => mcp).map(({ command }) => command)
  );
  for (const command of ['scene.projectNarrativeBundle', 'scene.compileHtml', 'trackb.evidence.measure']) {
    const definition = APPLICATION_COMMAND_REGISTRY.find((entry) => entry.command === command);
    assert.ok(definition);
    assert.equal(typeof TRACK_B_HANDLER_IMPLEMENTATIONS[definition.implementation], 'function');
  }
  assert.equal(APPLICATION_COMMAND_REGISTRY.find(({ command }) => command === 'trackb.release.approve').cli, null);
  assert.equal(APPLICATION_COMMAND_REGISTRY.find(({ command }) => command === 'trackb.release.approve').mcp, null);
});
