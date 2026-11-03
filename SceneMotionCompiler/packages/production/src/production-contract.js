import { SchemaRegistry } from '../../contracts/src/schema-validator.js';
import packSchema from '../../../contracts/schemas/scene-asset-compiler/visual-reference-pack-v1.schema.json' with { type: 'json' };
import approvalSchema from '../../../contracts/schemas/scene-asset-compiler/visual-reference-approval-v1.schema.json' with { type: 'json' };
import graphSchema from '../../../contracts/schemas/scene-graph.schema.json' with { type: 'json' };
import productionSchema from '../../../contracts/schemas/production/production-v1.schema.json' with { type: 'json' };
import { cinematicContentHash } from '../../adapters/src/cinematic-contract.js';

const registry = new SchemaRegistry();
for (const schema of [packSchema, approvalSchema, graphSchema, productionSchema]) {
  registry.add(schema);
}

export const check = (condition, code) => {
  if (!condition) throw Object.assign(new Error(code), { code: 'production-contract-invalid' });
};

export function exact(value, required, label) {
  check(value && typeof value === 'object' && !Array.isArray(value), `${label}:OBJECT`);
  const keys = required.split(' ');
  check(Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key)), `${label}:FIELDS`);
  return value;
}

export function integer(value, min, max, label) {
  check(Number.isSafeInteger(value) && value >= min && value <= max, `${label}:INTEGER`);
  return value;
}

function seal(value) {
  return { ...value, contentHash: cinematicContentHash(value) };
}

export function assertProductionContract(value) {
  return registry.assert(productionSchema.$id, value);
}

export function assertVisualReferencePack(value) {
  return registry.assert(packSchema.$id, value);
}

export function assertVisualReferenceApproval(value) {
  return registry.assert(approvalSchema.$id, value);
}

export function artifact(value) {
  return assertProductionContract(seal(value));
}

export function sealed(value, kind) {
  check(
    value.kind === kind && value.schemaVersion === 1 && value.contentHash === seal(value).contentHash,
    `${kind}:HASH_OR_VERSION`
  );
}
