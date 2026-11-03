import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { loadSceneAssetCompilerContractRegistry, loadSmcContractRegistry } from '../packages/contracts/src/closed-schema-set.js';
import { loadTrackBArtifactContractRegistries } from '../packages/contracts/src/trackb-artifact-schemas.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import {
  validateTrackBNarrativeAuthority,
  validateTrackBMeasuredEvidenceReceipt,
  validateTrackBReleaseCandidate,
  validateTrackBApprovalReceipt,
  validateTrackBDeliverySet,
  validateTrackBDeliveryPointer
} from '../packages/contracts/src/trackb-contract.js';

const registry=await loadSmcContractRegistry('contracts/schemas');
const failures=[];
for (const name of (await readdir('contracts/examples')).filter((item)=>item.endsWith('.example.json')).sort()) {
  const schemaName=name.replace('.example.json','.schema.json');
  try { registry.schema(schemaName); } catch { continue; }
  const value=JSON.parse(await readFile(join('contracts/examples',name),'utf8'));
  const result=registry.validate(schemaName,value);
  if (!result.valid) failures.push({example:name,schema:schemaName,errors:result.errors});
}
if (failures.length) {
  console.error(JSON.stringify({status:'FAIL',failures},null,2));
  process.exit(1);
}

const trackBRegistry = await loadTrackBContractRegistry('contracts/schemas/trackb');
const trackBExamples = [
  ['trackBNarrativeAuthority', 'trackb-narrative-authority.example.json', validateTrackBNarrativeAuthority],
  ['trackBMeasuredEvidenceReceipt', 'trackb-measured-evidence-receipt.example.json', validateTrackBMeasuredEvidenceReceipt],
  ['trackBReleaseCandidate', 'trackb-release-candidate.example.json', validateTrackBReleaseCandidate],
  ['trackBReleaseApprovalReceipt', 'trackb-release-approval-receipt.example.json', validateTrackBApprovalReceipt],
  ['trackBDeliverySet', 'trackb-delivery-set.example.json', validateTrackBDeliverySet],
  ['trackBDeliveryPointer', 'trackb-delivery-pointer.example.json', validateTrackBDeliveryPointer]
];
const trackBValues = new Map();
for (const [key, name] of trackBExamples) {
  const value = JSON.parse(await readFile(join('contracts/examples/trackb', name), 'utf8'));
  const schemaResult = trackBRegistry.validate(key, value);
  if (!schemaResult.valid) failures.push({ example: `trackb/${name}`, schema: key, errors: schemaResult.errors });
  trackBValues.set(key, value);
}
const authorityResult = validateTrackBNarrativeAuthority(trackBValues.get('trackBNarrativeAuthority'), { registry: trackBRegistry });
if (!authorityResult.valid) failures.push({ example: 'trackb/trackb-narrative-authority.example.json', schema: 'trackBNarrativeAuthority semantic', errors: authorityResult.errors });
const receiptResult = validateTrackBMeasuredEvidenceReceipt(trackBValues.get('trackBMeasuredEvidenceReceipt'), { registry: trackBRegistry });
if (!receiptResult.valid) failures.push({ example: 'trackb/trackb-measured-evidence-receipt.example.json', schema: 'TrackBMeasuredEvidenceReceipt semantic', errors: receiptResult.errors });
const candidateResult = validateTrackBReleaseCandidate(trackBValues.get('trackBReleaseCandidate'), { registry: trackBRegistry, authority: trackBValues.get('trackBNarrativeAuthority') });
if (!candidateResult.valid) failures.push({ example: 'trackb/trackb-release-candidate.example.json', schema: 'trackBReleaseCandidate semantic', errors: candidateResult.errors });
const approvalResult = validateTrackBApprovalReceipt(trackBValues.get('trackBReleaseApprovalReceipt'), { registry: trackBRegistry, candidateHash: trackBValues.get('trackBReleaseCandidate').candidateHash, baseDeliverySetHash: 'NONE' });
if (!approvalResult.valid) failures.push({ example: 'trackb/trackb-release-approval-receipt.example.json', schema: 'TrackBReleaseApprovalReceipt semantic', errors: approvalResult.errors });
const deliveryResult = validateTrackBDeliverySet(trackBValues.get('trackBDeliverySet'), { registry: trackBRegistry });
if (!deliveryResult.valid) failures.push({ example: 'trackb/trackb-delivery-set.example.json', schema: 'TrackBDeliverySet semantic', errors: deliveryResult.errors });
const pointerResult = validateTrackBDeliveryPointer(trackBValues.get('trackBDeliveryPointer'), { registry: trackBRegistry });
if (!pointerResult.valid) failures.push({ example: 'trackb/trackb-delivery-pointer.example.json', schema: 'TrackBDeliveryPointer semantic', errors: pointerResult.errors });

const artifactRegistries = await loadTrackBArtifactContractRegistries({ smcSchemaDir: 'contracts/schemas', sceneAssetCompilerSchemaDir: 'contracts/schemas/scene-asset-compiler' });
const projectionReceipt = JSON.parse(await readFile('contracts/examples/narrative-scene-projection.example.json', 'utf8'));
const sceneGraphExample = JSON.parse(await readFile('contracts/examples/scene-graph.example.json', 'utf8'));
const handoffExample = JSON.parse(await readFile('contracts/examples/scene-motion-asset-handoff-v2.example.json', 'utf8'));
for (const [schema, value, example] of [
  ['narrative-scene-projection.schema.json', projectionReceipt, 'trackb/embedded-projection-receipt'],
  ['scene-graph.schema.json', sceneGraphExample, 'trackb/embedded-scene-graph'],
  ['scene-motion-asset-handoff-v2.schema.json', handoffExample, 'trackb/embedded-asset-handoff']
]) {
  const owner = schema === 'scene-motion-asset-handoff-v2.schema.json' ? artifactRegistries.sceneAssetCompilerRegistry : artifactRegistries.smcRegistry;
  const result = owner.validate(schema, value);
  if (!result.valid) failures.push({ example, schema, errors: result.errors });
}
for (const [label, value] of [
  ['embedded projection receipt', projectionReceipt],
  ['SceneMotionAssetHandoff v2 example', handoffExample]
]) {
  if (value.sourceSemanticManifestHash === value.semanticManifestHash) {
    failures.push({ example: label, schema: 'semantic pin invariant', errors: [{ message: 'sourceSemanticManifestHash must differ from semanticManifestHash' }] });
  }
}
if (failures.length) {
  console.error(JSON.stringify({ status: 'FAIL', failures }, null, 2));
  process.exit(1);
}
console.log(JSON.stringify({status:'PASS',validated:'all schema-backed examples'},null,2));
