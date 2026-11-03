import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { SchemaValidationError } from '../../contracts/src/schema-validator.js';
import { loadSmcContractRegistry } from '../../contracts/src/closed-schema-set.js';
import { loadTrackBContractRegistry } from '../../contracts/src/trackb-registry.js';
import { ValidationError } from './errors.js';

const repoRoot = fileURLToPath(new URL('../../../', import.meta.url));
const schemaDir = join(repoRoot, 'contracts', 'schemas');
const trackBSchemaDir = join(schemaDir, 'trackb');
let registryPromise;
let trackBRegistryPromise;

export const SCHEMAS = Object.freeze({
  continuityBible: 'https://narrative-courier.local/contracts/continuity-bible-v1.schema.json',
  cinematicDirectorPlan: 'https://narrative-courier.local/contracts/cinematic-director-plan-v1.schema.json',
  cinematicApproval: 'https://narrative-courier.local/contracts/cinematic-approval-v1.schema.json',
  sceneGraph: 'https://scene-motion-compiler.local/schemas/scene-graph.schema.json',
  narrativeCourierNarrativePackageV2: 'https://narrative-courier.local/contracts/narrative-package/v2.schema.json',
  narrativeSceneMap: 'https://narrative-courier.local/contracts/narrative-scene-map/v2.schema.json',
  narrativeReleaseBundle: 'https://narrative-courier.local/contracts/release-bundle/v1.schema.json',
  narrativeSceneProjection: 'https://scene-motion-compiler.local/schemas/narrative-scene-projection.schema.json',
  assetManifest: 'https://scene-motion-compiler.local/schemas/asset-manifest.schema.json',
  trackBNarrativeAuthority: 'https://scene-motion-compiler.local/contracts/trackb/narrative-authority/v2',
  trackBReleaseCandidate: 'https://scene-motion-compiler.local/contracts/trackb/release-candidate/v4',
  trackBMeasuredEvidenceReceipt: 'https://scene-motion-compiler.local/contracts/trackb/measured-evidence-receipt/v2',
  trackBReleaseApprovalReceipt: 'https://scene-motion-compiler.local/contracts/trackb/release-approval-receipt/v2',
  trackBDeliverySet: 'https://scene-motion-compiler.local/contracts/trackb/delivery-set/v2',
  trackBDeliveryPointer: 'https://scene-motion-compiler.local/contracts/trackb/delivery-pointer/v2'
});

const TRACK_B_SCHEMA_IDS = new Set([
  SCHEMAS.trackBNarrativeAuthority,
  SCHEMAS.trackBReleaseCandidate,
  SCHEMAS.trackBMeasuredEvidenceReceipt,
  SCHEMAS.trackBReleaseApprovalReceipt,
  SCHEMAS.trackBDeliverySet,
  SCHEMAS.trackBDeliveryPointer
]);

export async function validateSchema(schemaId, value) {
  const registry = TRACK_B_SCHEMA_IDS.has(schemaId)
    ? await (trackBRegistryPromise ??= loadTrackBContractRegistry(trackBSchemaDir))
    : await (registryPromise ??= loadSmcContractRegistry(schemaDir));
  try {
    registry.assert(schemaId, value);
  } catch (error) {
    if (error instanceof SchemaValidationError) {
      const wrapped = new ValidationError(error.message);
      wrapped.errors = error.errors;
      throw wrapped;
    }
    throw error;
  }
  return { value, schemaId };
}
