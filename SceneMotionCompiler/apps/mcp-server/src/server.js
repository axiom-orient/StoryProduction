import { McpServer } from '@modelcontextprotocol/server';
import { z } from 'zod';
import {
  APPLICATION_COMMAND_METADATA,
  MCP_COMMAND_REGISTRY,
  createApplicationCommandService,
  executeApplicationCommand,
  PRODUCT_VERSION
} from '../../../packages/application/src/index.js';
import {
  TRACK_B_BASE_HASH_PATTERN,
  TRACK_B_HASH_PATTERN,
  TRACK_B_NARRATIVE_ID_PATTERN,
  TRACK_B_OPAQUE_ID_PATTERN,
  TRACK_B_SAFE_PATH_PATTERN,
  validateTrackBApplicationEnvelope
} from '../../../packages/contracts/src/trackb-contract.js';

const EMPTY_INPUT = z.object({}).strict();
const SCENE_PROJECT_BUNDLE_INPUT = z.object({ releaseBundlePath: z.string().min(1).max(4096), continuityBiblePath: z.string().min(1).max(4096),
  directorPlanPath: z.string().min(1).max(4096), directorApprovalPath: z.string().min(1).max(4096) }).strict();
const SCENE_COMPILE_HTML_INPUT = z.object({
  sceneMotionAssetHandoffPath: z.string().min(1).max(4096),
  outputPath: z.string().min(1).max(4096).optional(),
  width: z.number().int().min(1).max(8192).optional(),
  height: z.number().int().min(1).max(8192).optional()
}).strict();
const TRACK_B_PREPARE_INPUT = z.object({
  narrativeAuthorityPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  projectionReceiptPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  assetHandoffPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  htmlPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  provenancePath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  evidenceReceiptPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  candidateId: z.string().regex(TRACK_B_OPAQUE_ID_PATTERN),
  expectedBaseDeliverySetHash: z.string().regex(TRACK_B_BASE_HASH_PATTERN)
}).strict();
const TRACK_B_MEASURE_INPUT = z.object({
  narrativeAuthorityPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  htmlPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  provenancePath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN),
  outputPath: z.string().min(1).max(1024).regex(TRACK_B_SAFE_PATH_PATTERN)
}).strict();
const TRACK_B_PROMOTE_INPUT = z.object({
  narrativeId: z.string().regex(TRACK_B_NARRATIVE_ID_PATTERN),
  candidateHash: z.string().regex(TRACK_B_HASH_PATTERN),
  approvalHash: z.string().regex(TRACK_B_HASH_PATTERN),
  expectedBaseDeliverySetHash: z.string().regex(TRACK_B_BASE_HASH_PATTERN)
}).strict();
const TRACK_B_STATUS_INPUT = z.object({
  narrativeId: z.string().regex(TRACK_B_NARRATIVE_ID_PATTERN),
  candidateHash: z.string().regex(TRACK_B_HASH_PATTERN).optional()
}).strict();

const INPUT_SCHEMAS_BY_IMPLEMENTATION = Object.freeze({
  prepareProduction: z.object({requestPath:z.string().min(1).max(4096),outputPath:z.string().min(1).max(4096)}).strict(),
  projectNarrativeBundle: SCENE_PROJECT_BUNDLE_INPUT,
  compileHtml: SCENE_COMPILE_HTML_INPUT,
  measureEvidence: TRACK_B_MEASURE_INPUT,
  prepareRelease: TRACK_B_PREPARE_INPUT,
  promoteRelease: TRACK_B_PROMOTE_INPUT,
  statusRelease: TRACK_B_STATUS_INPUT
});

const TOOL_DEFINITIONS = Object.freeze(MCP_COMMAND_REGISTRY.map(({ name, command, implementation, description, readOnly }) => {
  const inputSchema = INPUT_SCHEMAS_BY_IMPLEMENTATION[implementation];
  if (!inputSchema) throw new Error(`MCP command has no input schema: ${command}`);
  return tool(name, command, description, inputSchema, readOnly);
}));

function tool(name, command, description, inputSchema, readOnlyHint) {
  return Object.freeze({
    name,
    command,
    title: description,
    description,
    inputSchema,
    annotations: Object.freeze({ readOnlyHint, destructiveHint: !readOnlyHint, idempotentHint: readOnlyHint, openWorldHint: false })
  });
}

export function createSmcMcpServer(config) {
  const application = createApplicationCommandService(config);
  const server = new McpServer(
    { name: 'scene-motion-compiler', version: PRODUCT_VERSION },
    { capabilities: { tools: {} } }
  );
  let commandTail = Promise.resolve();

  for (const toolDefinition of TOOL_DEFINITIONS) {
    server.registerTool(
      toolDefinition.name,
      {
        title: toolDefinition.title,
        description: toolDefinition.description,
        inputSchema: toolDefinition.inputSchema ?? EMPTY_INPUT,
        annotations: toolDefinition.annotations
      },
      async (input) => enqueue(async () => {
        try {
          const envelope = await executeApplicationCommand(application, toolDefinition.command, input);
          return toToolResult(validateEnvelope(toolDefinition.command, input, envelope));
        } catch (error) {
          return toToolResult(unexpectedFailure(toolDefinition.command, error));
        }
      })
    );
  }

  return server;

  function enqueue(operation) {
    const pending = commandTail.then(operation, operation);
    commandTail = pending.then(() => undefined, () => undefined);
    return pending;
  }
}

function toToolResult(envelope) {
  return {
    content: [{ type: 'text', text: JSON.stringify(envelope, null, 2) }],
    structuredContent: envelope,
    isError: envelope?.ok !== true
  };
}

function validateEnvelope(command, input, envelope) {
  if (command.startsWith('trackb.')) {
    const validation = validateTrackBApplicationEnvelope(envelope, { command, input });
    if (!validation.valid) {
      const first = validation.errors?.[0];
      throw Object.assign(new Error(first?.message ?? `Track B application envelope is invalid: ${command}`), {
        code: 'MCP_APPLICATION_ENVELOPE_INVALID',
        details: { errors: validation.errors }
      });
    }
    return envelope;
  }
  if (envelope?.schemaVersion === 1 && typeof envelope.ok === 'boolean' && envelope.command === command) return envelope;
  throw Object.assign(new Error(`Application command returned an invalid envelope: ${command}`), {
    code: 'MCP_APPLICATION_ENVELOPE_INVALID'
  });
}

function unexpectedFailure(command, error) {
  return {
    schemaVersion: 1,
    ok: false,
    command,
    metadata: commandMetadata(command),
    error: { code: typeof error?.code === 'string' && error.code ? error.code : 'MCP_COMMAND_FAILED', message: error instanceof Error ? error.message : String(error), retryable: false }
  };
}

function commandMetadata(command) {
  return APPLICATION_COMMAND_METADATA?.[command] ?? {};
}
