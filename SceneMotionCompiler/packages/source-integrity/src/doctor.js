import { spawnSync } from 'node:child_process';
import { access } from 'node:fs/promises';
import { constants } from 'node:fs';
import { findChromium, findFont } from '../../browser-renderer/src/chromium.js';

export async function runDoctor() {
  const checks = [];
  const nodeMajor = Number(process.versions.node.split('.')[0]);
  checks.push(check('node-22', nodeMajor >= 22 ? 'PASS' : 'FAIL', process.version, true));
  checks.push(commandCheck('unzip', true));
  checks.push(alternativeCommandCheck('checksum-tool', ['sha256sum', 'shasum'], true));
  checks.push(commandCheck('python3', false));
  checks.push(commandCheck('cargo', false, 'NOT_RUN_TOOLCHAIN'));
  checks.push(commandCheck('rustc', false, 'NOT_RUN_TOOLCHAIN'));
  try {
    const path = await findChromium();
    await access(path, constants.X_OK);
    checks.push(check('chromium', 'PASS', path, true));
  } catch (error) {
    checks.push(check('chromium', 'FAIL', error.message, true));
  }
  try {
    const path = await findFont();
    await access(path, constants.R_OK);
    checks.push(check('font', 'PASS', path, true));
  } catch (error) {
    checks.push(check('font', 'FAIL', error.message, true));
  }
  const providerConfigured = Boolean(process.env.OPENAI_API_KEY);
  checks.push(check('external-image-provider', providerConfigured ? 'STATICALLY_REVIEWED' : 'NOT_RUN_EXTERNAL', providerConfigured ? 'OpenAI credential configured; not invoked by doctor' : 'no OpenAI credential configured; command providers are explicit launch options', false));
  return {
    schemaVersion: 1,
    status: checks.filter((item) => item.required).every((item) => item.status === 'PASS') ? 'PASS' : 'FAIL',
    checks
  };
}

function commandCheck(command, required, absentStatus = 'UNKNOWN') {
  const result = probeCommand(command);
  return !result.error
    ? check(command, 'PASS', firstLine(result.stdout) || firstLine(result.stderr) || `${command} available`, required)
    : check(command, absentStatus, `${command} not found`, required);
}
function alternativeCommandCheck(name, commands, required) {
  for (const command of commands) {
    const result = probeCommand(command);
    if (!result.error) return check(name, 'PASS', command, required);
  }
  return check(name, 'FAIL', `none found: ${commands.join(', ')}`, required);
}
function probeCommand(command) {
  const result = spawnSync(command, ['--version'], { encoding: 'utf8', timeout: 5_000 });
  return result.error?.code === 'ENOENT' ? { error: result.error } : { ...result, error: result.error ?? null };
}
function firstLine(value) { return String(value ?? '').trim().split(/\r?\n/, 1)[0]; }
function check(name, status, detail, required) { return { name, status, detail, required }; }
