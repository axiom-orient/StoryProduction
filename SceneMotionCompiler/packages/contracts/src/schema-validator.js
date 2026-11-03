import { readFile } from 'node:fs/promises';
import { basename, resolve, sep } from 'node:path';
import { canonicalizeJson } from '../../core/src/canonical.js';

export class SchemaValidationError extends Error {
  constructor(schemaId, errors) {
    super(`Schema validation failed for ${schemaId}:\n${errors.map((error) => `${error.instancePath || '/'} ${error.message}`).join('\n')}`);
    this.name = 'SchemaValidationError';
    this.schemaId = schemaId;
    this.errors = errors;
  }
}

export class SchemaRegistry {
  #schemasById = new Map();
  #schemasByFile = new Map();

  static async fromFiles(schemaDir, files) {
    if (!Array.isArray(files) || files.length === 0) throw new Error('An explicit non-empty schema file set is required');
    const root = resolve(schemaDir);
    const names = new Set();
    const registry = new SchemaRegistry();
    for (const file of files) {
      if (typeof file !== 'string' || file.length === 0 || file.includes('\\') || file.includes('\0') || file.startsWith('/') || file.split('/').some((part) => part === '' || part === '.' || part === '..')) {
        throw new Error(`Schema file must be a safe relative path: ${String(file)}`);
      }
      if (names.has(file)) throw new Error(`Schema file set contains a duplicate: ${file}`);
      names.add(file);
      const path = resolve(root, file);
      if (path !== root && !path.startsWith(`${root}${sep}`)) throw new Error(`Schema file escapes schema directory: ${file}`);
      const schema = JSON.parse(await readFile(path, 'utf8'));
      registry.add(schema, { file, path });
    }
    return registry;
  }

  add(schema, source = {}) {
    if (!schema || typeof schema !== 'object') throw new Error('Schema must be an object');
    const file = source.file ?? (schema.$id ? basename(new URL(schema.$id).pathname) : undefined);
    if (schema.$id) this.#schemasById.set(schema.$id, { schema, source: { ...source, file } });
    if (file) this.#schemasByFile.set(file, { schema, source: { ...source, file } });
  }

  schema(identifier) {
    const direct = this.#schemasById.get(identifier) ?? this.#schemasByFile.get(identifier) ?? this.#schemasByFile.get(basename(identifier));
    if (!direct) throw new Error(`Schema not found: ${identifier}`);
    return direct;
  }

  validate(identifier, value) {
    const root = this.schema(identifier);
    const errors = [];
    this.#visit(root.schema, value, '', root, errors, new Set());
    return { valid: errors.length === 0, errors };
  }

  assert(identifier, value) {
    const result = this.validate(identifier, value);
    if (!result.valid) throw new SchemaValidationError(identifier, result.errors);
    return value;
  }

  #visit(schema, value, instancePath, context, errors, refStack) {
    if (schema === true) return;
    if (schema === false) { errors.push(error(instancePath, 'boolean schema is false')); return; }
    if (!schema || typeof schema !== 'object') { errors.push(error(instancePath, 'invalid schema node')); return; }
    if (Object.prototype.hasOwnProperty.call(schema, '$defs')) {
      // $defs holds reusable subschemas; they apply only through $ref and never constrain the instance directly.
    } else if (Object.prototype.hasOwnProperty.call(schema, 'unevaluatedProperties')) throw new Error('Unsupported JSON Schema keyword: unevaluatedProperties');
    if (schema.format !== undefined && schema.format !== 'date-time') throw new Error(`Unsupported JSON Schema format: ${schema.format}`);
    if (schema.additionalProperties === false && (schema.allOf || schema.$ref)) throw new Error('Unsupported JSON Schema combination: allOf/$ref with additionalProperties=false');

    if (schema.$ref) {
      const resolved = this.#resolveRef(schema.$ref, context);
      const key = `${resolved.source.path ?? resolved.source.file ?? resolved.schema.$id ?? 'schema'}#${resolved.pointer}`;
      if (refStack.has(key)) throw new Error(`Circular JSON Schema $ref is unsupported: ${schema.$ref}`);
      const nextStack = new Set(refStack); nextStack.add(key);
      this.#visit(resolved.node, value, instancePath, resolved.root, errors, nextStack);
    }

    if (schema.allOf) for (const child of schema.allOf) this.#visit(child, value, instancePath, context, errors, refStack);
    if (schema.anyOf) {
      const branches = schema.anyOf.map((child) => { const branchErrors=[]; this.#visit(child,value,instancePath,context,branchErrors,refStack); return branchErrors; });
      if (!branches.some((branch) => branch.length === 0)) errors.push(error(instancePath, `must match at least one anyOf branch (${branches.map((branch)=>branch.length).join(',')})`));
    }
    if (schema.oneOf) {
      const validCount = schema.oneOf.reduce((count, child) => { const branchErrors=[]; this.#visit(child,value,instancePath,context,branchErrors,refStack); return count + (branchErrors.length === 0 ? 1 : 0); }, 0);
      if (validCount !== 1) errors.push(error(instancePath, `must match exactly one oneOf branch; matched ${validCount}`));
    }
    if (schema.not) {
      const branchErrors=[]; this.#visit(schema.not,value,instancePath,context,branchErrors,refStack);
      if (branchErrors.length === 0) errors.push(error(instancePath, 'must not match forbidden schema'));
    }
    if (schema.if) {
      const conditionErrors=[]; this.#visit(schema.if,value,instancePath,context,conditionErrors,refStack);
      if (conditionErrors.length === 0 && schema.then) this.#visit(schema.then,value,instancePath,context,errors,refStack);
      if (conditionErrors.length > 0 && schema.else) this.#visit(schema.else,value,instancePath,context,errors,refStack);
    }

    if (schema.type !== undefined && !matchesType(value, schema.type)) {
      errors.push(error(instancePath, `must be of type ${Array.isArray(schema.type) ? schema.type.join('|') : schema.type}`));
      return;
    }
    if (schema.const !== undefined && !deepEqual(value, schema.const)) errors.push(error(instancePath, `must equal const ${canonicalizeJson(schema.const)}`));
    if (schema.enum && !schema.enum.some((candidate) => deepEqual(value, candidate))) errors.push(error(instancePath, `must be one of ${schema.enum.map(canonicalizeJson).join(', ')}`));

    if (typeof value === 'string') this.#validateString(schema, value, instancePath, errors);
    if (typeof value === 'number') this.#validateNumber(schema, value, instancePath, errors);
    if (Array.isArray(value)) this.#validateArray(schema, value, instancePath, context, errors, refStack);
    if (isObject(value)) this.#validateObject(schema, value, instancePath, context, errors, refStack);
  }

  #validateString(schema, value, path, errors) {
    if (schema.minLength !== undefined && [...value].length < schema.minLength) errors.push(error(path, `length must be >= ${schema.minLength}`));
    if (schema.maxLength !== undefined && [...value].length > schema.maxLength) errors.push(error(path, `length must be <= ${schema.maxLength}`));
    if (schema.pattern !== undefined) {
      let pattern;
      try { pattern = new RegExp(schema.pattern, 'u'); } catch (cause) { throw new Error(`Invalid schema regex ${schema.pattern}: ${cause.message}`); }
      if (!pattern.test(value)) errors.push(error(path, `must match pattern ${schema.pattern}`));
    }
    if (schema.format === 'date-time' && !isRfc3339DateTime(value)) errors.push(error(path, 'must be a valid RFC 3339 date-time'));
  }

  #validateNumber(schema, value, path, errors) {
    if (schema.type === 'integer' && !Number.isInteger(value)) errors.push(error(path, 'must be an integer'));
    if (schema.minimum !== undefined && value < schema.minimum) errors.push(error(path, `must be >= ${schema.minimum}`));
    if (schema.maximum !== undefined && value > schema.maximum) errors.push(error(path, `must be <= ${schema.maximum}`));
    if (schema.exclusiveMinimum !== undefined && value <= schema.exclusiveMinimum) errors.push(error(path, `must be > ${schema.exclusiveMinimum}`));
    if (schema.exclusiveMaximum !== undefined && value >= schema.exclusiveMaximum) errors.push(error(path, `must be < ${schema.exclusiveMaximum}`));
    if (schema.multipleOf !== undefined && Math.abs(value / schema.multipleOf - Math.round(value / schema.multipleOf)) > 1e-10) errors.push(error(path, `must be multiple of ${schema.multipleOf}`));
  }

  #validateArray(schema, value, path, context, errors, refStack) {
    if (schema.minItems !== undefined && value.length < schema.minItems) errors.push(error(path, `must contain >= ${schema.minItems} items`));
    if (schema.maxItems !== undefined && value.length > schema.maxItems) errors.push(error(path, `must contain <= ${schema.maxItems} items`));
    if (schema.uniqueItems) {
      const seen = new Set();
      value.forEach((item, index) => { const key=canonicalizeJson(item); if (seen.has(key)) errors.push(error(`${path}/${index}`, 'must be unique')); seen.add(key); });
    }
    if (Array.isArray(schema.prefixItems)) {
      schema.prefixItems.forEach((child, index) => { if (index < value.length) this.#visit(child,value[index],`${path}/${index}`,context,errors,refStack); });
    }
    if (schema.items && !Array.isArray(schema.items)) {
      const start = Array.isArray(schema.prefixItems) ? schema.prefixItems.length : 0;
      for (let index=start; index<value.length; index+=1) this.#visit(schema.items,value[index],`${path}/${index}`,context,errors,refStack);
    }
    if (schema.contains) {
      let matches=0;
      for (let index=0; index<value.length; index+=1) { const childErrors=[]; this.#visit(schema.contains,value[index],`${path}/${index}`,context,childErrors,refStack); if (childErrors.length===0) matches+=1; }
      const minimum=schema.minContains ?? 1, maximum=schema.maxContains ?? Infinity;
      if (matches < minimum || matches > maximum) errors.push(error(path, `contains matches ${matches}, expected ${minimum}..${maximum}`));
    }
  }

  #validateObject(schema, value, path, context, errors, refStack) {
    const keys=Object.keys(value);
    if (schema.minProperties !== undefined && keys.length < schema.minProperties) errors.push(error(path, `must contain >= ${schema.minProperties} properties`));
    if (schema.maxProperties !== undefined && keys.length > schema.maxProperties) errors.push(error(path, `must contain <= ${schema.maxProperties} properties`));
    for (const required of schema.required ?? []) if (!Object.prototype.hasOwnProperty.call(value, required)) errors.push(error(`${path}/${escapePointer(required)}`, 'is required'));
    const evaluated=new Set();
    for (const [key, child] of Object.entries(schema.properties ?? {})) {
      if (Object.prototype.hasOwnProperty.call(value,key)) { evaluated.add(key); this.#visit(child,value[key],`${path}/${escapePointer(key)}`,context,errors,refStack); }
    }
    for (const [patternText, child] of Object.entries(schema.patternProperties ?? {})) {
      const pattern=new RegExp(patternText,'u');
      for (const key of keys) if (pattern.test(key)) { evaluated.add(key); this.#visit(child,value[key],`${path}/${escapePointer(key)}`,context,errors,refStack); }
    }
    if (schema.propertyNames) for (const key of keys) this.#visit(schema.propertyNames,key,`${path}/${escapePointer(key)}`,context,errors,refStack);
    const additional=keys.filter((key)=>!evaluated.has(key));
    if (schema.additionalProperties === false) for (const key of additional) errors.push(error(`${path}/${escapePointer(key)}`, 'additional property is not allowed'));
    else if (isObject(schema.additionalProperties) || schema.additionalProperties === true) {
      if (schema.additionalProperties !== true) for (const key of additional) this.#visit(schema.additionalProperties,value[key],`${path}/${escapePointer(key)}`,context,errors,refStack);
    }
    if (schema.dependentRequired) for (const [key,required] of Object.entries(schema.dependentRequired)) if (Object.prototype.hasOwnProperty.call(value,key)) for (const dependency of required) if (!Object.prototype.hasOwnProperty.call(value,dependency)) errors.push(error(path, `${dependency} is required when ${key} exists`));
  }

  #resolveRef(ref, context) {
    const [location, fragment=''] = ref.split('#',2);
    let root=context;
    if (location) {
      if (/^https?:/.test(location)) root=this.schema(location);
      else root=this.schema(basename(location));
    }
    const pointer=fragment ? `#${fragment}` : '#';
    let node=root.schema;
    if (fragment) {
      if (!fragment.startsWith('/')) throw new Error(`Unsupported JSON Schema fragment: ${ref}`);
      for (const token of fragment.slice(1).split('/').map(unescapePointer)) {
        if (!node || typeof node !== 'object' || !(token in node)) throw new Error(`Unresolved JSON Schema ref: ${ref}`);
        node=node[token];
      }
    }
    return { node, root, source: root.source, pointer };
  }
}

export async function loadContractRegistry(schemaDir, files) {
  return SchemaRegistry.fromFiles(resolve(schemaDir), files);
}

function matchesType(value,type) { const types=Array.isArray(type)?type:[type]; return types.some((candidate)=>candidate==='null'?value===null:candidate==='array'?Array.isArray(value):candidate==='object'?isObject(value):candidate==='integer'?Number.isInteger(value):candidate==='number'?typeof value==='number'&&Number.isFinite(value):typeof value===candidate); }
function isObject(value) { return value!==null && typeof value==='object' && !Array.isArray(value); }
function deepEqual(a,b) { return canonicalizeJson(a)===canonicalizeJson(b); }
function error(instancePath,message) { return { instancePath: instancePath || '/', message }; }
function escapePointer(value) { return value.replace(/~/g,'~0').replace(/\//g,'~1'); }
function unescapePointer(value) { return value.replace(/~1/g,'/').replace(/~0/g,'~'); }

function isRfc3339DateTime(value) {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)) return false;
  return Number.isFinite(Date.parse(value));
}
