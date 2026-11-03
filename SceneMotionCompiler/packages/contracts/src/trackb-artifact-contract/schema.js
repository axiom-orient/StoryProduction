import {
  TrackBContractError,
  contractError,
  resultWithErrors
} from '../trackb-contract-primitives.js';
import { isTrackBSchemaIdentifier } from '../trackb-registry.js';

export function validateSchemaValue(identifier, value, registry) {
  if (!isTrackBSchemaIdentifier(identifier)) return resultWithErrors([contractError('/', `unknown Track B schema identifier: ${identifier}`, 'trackb-input-invalid')]);
  if (!registry || typeof registry.validate !== 'function') throw new TrackBContractError('trackb-registry-invalid', 'a loaded Track B registry is required for schema validation');
  const result = registry.validate(identifier, value);
  return result.valid ? { valid: true, errors: [] } : { valid: false, errors: result.errors.map((item) => ({ ...item, code: 'trackb-input-invalid' })) };
}
