import { resolve } from 'node:path';

export class McpLaunchConfigError extends Error {
  constructor(code, message) { super(message); this.name = 'McpLaunchConfigError'; this.code = code; }
}

export function parseMcpLaunchConfig(tokens, { cwd = process.cwd() } = {}) {
  const values = new Map();
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token.startsWith('--')) throw new McpLaunchConfigError('MCP_ARGUMENT_INVALID', `Unexpected argument: ${token}`);
    const name = token.slice(2);
    if (!['root', 'project'].includes(name)) throw new McpLaunchConfigError('MCP_ARGUMENT_UNKNOWN', `Unknown option: --${name}`);
    const value = tokens[index + 1];
    if (value === undefined || value.startsWith('--')) throw new McpLaunchConfigError('MCP_ARGUMENT_MISSING_VALUE', `Missing value for --${name}`);
    if (values.has(name)) throw new McpLaunchConfigError('MCP_ARGUMENT_DUPLICATE', `Option may only be supplied once: --${name}`);
    values.set(name, value);
    index += 1;
  }
  const root = requiredValue(values, 'root');
  const project = requiredValue(values, 'project');
  const repoRoot = resolve(cwd, root);
  return Object.freeze({
    repoRoot,
    projectRoot: resolve(cwd, project)
  });
}

function requiredValue(values, name) {
  const value = values.get(name);
  if (!value) throw new McpLaunchConfigError('MCP_ARGUMENT_REQUIRED', `Missing required option --${name}`);
  return value;
}
