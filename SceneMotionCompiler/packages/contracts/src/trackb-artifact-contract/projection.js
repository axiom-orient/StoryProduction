import projectionSchema from '../../../../contracts/schemas/narrative-scene-projection.schema.json' with { type: 'json' };
import { SchemaRegistry } from '../schema-validator.js';
import { assertResult, contractError, resultWithErrors } from '../trackb-contract-primitives.js';

const PROJECTION_SCHEMA_REGISTRY = new SchemaRegistry();
PROJECTION_SCHEMA_REGISTRY.add(projectionSchema);
export function validateTrackBProjectionReceiptShape(value) {
  const shape = PROJECTION_SCHEMA_REGISTRY.validate(projectionSchema.$id, value);
  const errors = shape.errors.map(error => contractError(error.instancePath, error.message, 'trackb-artifact-binding-invalid'));
  if (!shape.valid) return resultWithErrors(errors);
  if (value.sourceSemanticManifestHash === value.semanticManifestHash) errors.push(contractError('/', 'source and projected semantic hash domains must differ', 'trackb-artifact-binding-invalid'));
  if (value.sceneCount !== value.sceneIds.length || value.shotCount !== value.sceneToShot.length ||
      new Set(value.sceneToShot.map(x => x.shotId)).size !== value.shotCount ||
      JSON.stringify([...new Set(value.sceneToShot.map(x => x.sceneId))]) !== JSON.stringify(value.sceneIds) ||
      value.sceneToShot.some((x,i) => x.order !== i+1)) {
    errors.push(contractError('/sceneToShot', 'scene/shot count, identity or order is invalid', 'trackb-artifact-binding-invalid'));
  }
  return resultWithErrors(errors);
}

export function assertTrackBProjectionReceiptShape(value) {
  assertResult(validateTrackBProjectionReceiptShape(value));
  return value;
}
