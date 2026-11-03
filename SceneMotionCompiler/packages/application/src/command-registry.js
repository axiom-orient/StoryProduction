/**
 * The single Track B command registry.
 *
 * Transport adapters may add their own framing/input validation, but they do
 * not define another command list or command-to-application mapping.  Keep
 * this module data-only so the application, CLI, MCP server, and Studio can
 * all consume the same immutable surface without a dependency cycle.
 */

const DEFINITIONS = [
  ...['render', 'status', 'cancel', 'recover'].map((operation) => ({
    command: `animatic.${operation}`,
    implementation: `animatic${operation[0].toUpperCase()}${operation.slice(1)}`,
    metadata: { effects: { read: true, write: operation !== 'status' }, longRunning: operation === 'render', authority: 'operator' },
    cli: { name: `animatic-${operation}`, requiredOptions: operation === 'render'
      ? ['project', 'packet', 'assets', 'asset-review', 'attempt'] : ['project', 'attempt'],
      optionalOptions: operation === 'render' ? ['root', 'motion'] : ['root'] },
    mcp: null,
    studio: false
  })),
  {
    command: 'production.prepare',
    implementation: 'prepareProduction',
    metadata: { effects: { read: true, write: true }, longRunning: true, authority: 'operator' },
    cli: { name: 'production-prepare', requiredOptions: ['project', 'request', 'out'], optionalOptions: ['root'] },
    mcp: { name: 'production_prepare', description: 'Prepare motion requests, inspected clip/QC candidates, or editorial interchange; never approve or invoke a generation provider', readOnly: false },
    studio: false
  },
  {
    command: 'scene.projectNarrativeBundle',
    implementation: 'projectNarrativeBundle',
    metadata: { effects: { read: true, write: true }, longRunning: false, authority: 'operator' },
    cli: {
      name: 'project-narrative-bundle',
      requiredOptions: ['project', 'bundle', 'continuity', 'director-plan', 'director-approval'],
      optionalOptions: ['root']
    },
    mcp: {
      name: 'scene_project_narrative_bundle',
      description: 'Project a Completed NarrativeCourier release bundle',
      readOnly: false
    },
    studio: false
  },
  {
    command: 'scene.compileHtml',
    implementation: 'compileHtml',
    metadata: { effects: { read: true, write: true }, longRunning: false, authority: 'operator' },
    cli: {
      name: 'compile-scene-html',
      requiredOptions: ['project', 'handoff', 'out'],
      optionalOptions: ['root', 'width', 'height']
    },
    mcp: {
      name: 'compile_scene_html',
      description: 'Compile a verified SceneAssetCompiler handoff into moving scene HTML',
      readOnly: false
    },
    studio: true
  },
  {
    command: 'trackb.evidence.measure',
    implementation: 'measureEvidence',
    metadata: { effects: { read: true, write: true }, longRunning: true, authority: 'operator' },
    cli: {
      name: 'trackb-evidence-measure',
      requiredOptions: ['project', 'narrative-authority-path', 'html-path', 'provenance-path', 'out'],
      optionalOptions: ['root']
    },
    mcp: {
      name: 'trackb_evidence_measure',
      description: 'Persist pinned Chromium Track B measured evidence',
      readOnly: false
    },
    studio: false
  },
  {
    command: 'trackb.release.prepare',
    implementation: 'prepareRelease',
    metadata: { effects: { read: true, write: true }, longRunning: false, authority: 'operator' },
    cli: {
      name: 'trackb-release-prepare',
      requiredOptions: [
        'project',
        'narrative-authority-path',
        'projection-receipt-path',
        'asset-handoff-path',
        'html-path',
        'provenance-path',
        'evidence-receipt-path',
        'candidate-id',
        'expected-base-delivery-set-hash'
      ],
      optionalOptions: ['root']
    },
    mcp: {
      name: 'trackb_release_prepare',
      description: 'Stage a validated Track B release candidate',
      readOnly: false
    },
    studio: false
  },
  {
    command: 'trackb.release.approve',
    implementation: 'approveRelease',
    metadata: { effects: { read: true, write: true }, longRunning: false, authority: 'human-studio' },
    cli: null,
    mcp: null,
    studio: true,
    studioApproval: true
  },
  {
    command: 'trackb.release.promote',
    implementation: 'promoteRelease',
    metadata: { effects: { read: true, write: true }, longRunning: false, authority: 'operator' },
    cli: {
      name: 'trackb-release-promote',
      requiredOptions: ['project', 'narrative-id', 'candidate-hash', 'approval-hash', 'expected-base-delivery-set-hash'],
      optionalOptions: ['root']
    },
    mcp: {
      name: 'trackb_release_promote',
      description: 'CAS-publish an approved Track B DeliverySet',
      readOnly: false
    },
    studio: false
  },
  {
    command: 'trackb.release.status',
    implementation: 'statusRelease',
    metadata: { effects: { read: true, write: false }, longRunning: false, authority: 'none' },
    cli: {
      name: 'trackb-release-status',
      requiredOptions: ['project', 'narrative-id'],
      optionalOptions: ['root', 'candidate-hash']
    },
    mcp: {
      name: 'trackb_release_status',
      description: 'Read Track B release status',
      readOnly: true
    },
    studio: true
  }
];

export const APPLICATION_COMMAND_REGISTRY = deepFreeze(DEFINITIONS);
export const APPLICATION_COMMAND_METADATA = deepFreeze(
  Object.fromEntries(APPLICATION_COMMAND_REGISTRY.map(({ command, metadata }) => [command, metadata]))
);

export const CLI_COMMAND_REGISTRY = deepFreeze(
  APPLICATION_COMMAND_REGISTRY
    .filter(({ cli }) => cli)
    .map(({ command, cli }) => ({ command, ...cli }))
);

export const MCP_COMMAND_REGISTRY = deepFreeze(
  APPLICATION_COMMAND_REGISTRY
    .filter(({ mcp }) => mcp)
    .map(({ command, implementation, mcp }) => ({ command, implementation, ...mcp }))
);

export const STUDIO_COMMANDS = Object.freeze(
  APPLICATION_COMMAND_REGISTRY.filter(({ studio }) => studio).map(({ command }) => command)
);
export const STUDIO_APPROVAL_COMMAND = APPLICATION_COMMAND_REGISTRY.find(({ studioApproval }) => studioApproval)?.command;

export function commandForCli(name) {
  return CLI_COMMAND_REGISTRY.find((definition) => definition.name === name)?.command ?? null;
}

function deepFreeze(value) {
  if (!value || typeof value !== 'object' || Object.isFrozen(value)) return value;
  for (const child of Object.values(value)) deepFreeze(child);
  return Object.freeze(value);
}
