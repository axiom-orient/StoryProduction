#!/usr/bin/env node
import { resolve } from 'node:path';
import {
  CLI_COMMAND_REGISTRY,
  commandForCli,
  PRODUCT_VERSION,
  createApplicationCommandService
} from '../../application/src/index.js';
import { runDoctor } from '../../source-integrity/src/index.js';

const COMMAND_OPTIONS = Object.freeze({
  doctor: optionSpec(),
  ...Object.fromEntries(CLI_COMMAND_REGISTRY.map(({ name, requiredOptions, optionalOptions }) => [
    name,
    optionSpec(requiredOptions, optionalOptions)
  ]))
});

async function main() {
  const [command, ...tokens] = process.argv.slice(2);
  if (tokens.length === 1 && ['--help', '-h'].includes(tokens[0])) {
    process.stdout.write(renderCommandHelp(command));
    return;
  }
  const options = parseOptions(command, tokens);
  if (command === undefined || ['help', '--help', '-h'].includes(command)) {
    process.stdout.write(renderHelp());
    return;
  }
  if (['version', '--version', '-v'].includes(command)) {
    console.log(PRODUCT_VERSION);
    return;
  }
  if (command === 'doctor') {
    const report = await runDoctor();
    console.log(JSON.stringify(report, null, 2));
    if (report.status !== 'PASS') process.exitCode = 1;
    return;
  }

  const service = createApplicationCommandService({
    repoRoot: resolve(options.root ?? process.cwd()),
    projectRoot: resolve(required(options, 'project')),
  });
  if (options.motion && !['full', 'reduced'].includes(options.motion)) throw new Error('--motion must be full or reduced');
  const input = command === 'animatic-render'
    ? { packetPath: required(options, 'packet'), assetPackPath: required(options, 'assets'),
        assetReviewPath: required(options, 'asset-review'), attemptId: required(options, 'attempt'),
        reducedMotion: options.motion === 'reduced' }
    : command.startsWith('animatic-')
    ? { attemptId: required(options, 'attempt') }
    : command === 'production-prepare'
    ? { requestPath: required(options, 'request'), outputPath: required(options, 'out') }
    : command === 'project-narrative-bundle'
    ? { releaseBundlePath: required(options, 'bundle'), continuityBiblePath: required(options, 'continuity'),
        directorPlanPath: required(options, 'director-plan'), directorApprovalPath: required(options, 'director-approval') }
    : command === 'compile-scene-html'
      ? {
          sceneMotionAssetHandoffPath: required(options, 'handoff'),
          outputPath: required(options, 'out'),
          ...(options.width ? { width: integerOption(options.width, 'width') } : {}),
          ...(options.height ? { height: integerOption(options.height, 'height') } : {})
        }
      : command === 'trackb-evidence-measure'
        ? {
            narrativeAuthorityPath: required(options, 'narrative-authority-path'),
            htmlPath: required(options, 'html-path'),
            provenancePath: required(options, 'provenance-path'),
            outputPath: required(options, 'out')
          }
        : command === 'trackb-release-prepare'
          ? {
              narrativeAuthorityPath: required(options, 'narrative-authority-path'),
              projectionReceiptPath: required(options, 'projection-receipt-path'),
              assetHandoffPath: required(options, 'asset-handoff-path'),
              htmlPath: required(options, 'html-path'),
              provenancePath: required(options, 'provenance-path'),
              evidenceReceiptPath: required(options, 'evidence-receipt-path'),
              candidateId: required(options, 'candidate-id'),
              expectedBaseDeliverySetHash: required(options, 'expected-base-delivery-set-hash')
            }
          : command === 'trackb-release-promote'
            ? {
                narrativeId: required(options, 'narrative-id'),
                candidateHash: required(options, 'candidate-hash'),
                approvalHash: required(options, 'approval-hash'),
                expectedBaseDeliverySetHash: required(options, 'expected-base-delivery-set-hash')
              }
            : {
                narrativeId: required(options, 'narrative-id'),
                ...(options['candidate-hash'] ? { candidateHash: options['candidate-hash'] } : {})
              };
  const result = await service.execute(commandToApplication(command), input);
  if (!result.ok) throw Object.assign(new Error(result.error.message), { code: result.error.code });
  console.log(JSON.stringify({ status: 'PASS', ...result.result }, null, 2));
  if (['failed', 'cancelled', 'outcome_unknown', 'not_started'].includes(result.result.status)) process.exitCode = 3;
}

function commandToApplication(command) {
  const applicationCommand = commandForCli(command);
  if (!applicationCommand) {
    throw cliFailure('cli-command-unmapped', `CLI command has no Application mapping: ${command}`);
  }
  return applicationCommand;
}

function parseOptions(command, tokens) {
  const specification = COMMAND_OPTIONS[command];
  if (!specification && !['help', '--help', '-h', 'version', '--version', '-v'].includes(command)) {
    throw cliFailure('cli-command-unknown', `Unknown command: ${command ?? '<none>'}`);
  }
  const options = {};
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token.startsWith('--')) throw cliFailure('cli-argument-invalid', `Unexpected argument: ${token}`);
    const name = token.slice(2);
    if (!specification?.allowed.has(name)) throw cliFailure('cli-option-unknown', `Unknown option for ${command}: --${name}`);
    const value = tokens[index + 1];
    if (value === undefined || value.startsWith('--')) throw cliFailure('cli-option-value-required', `Missing value for --${name}`);
    if (Object.hasOwn(options, name)) throw cliFailure('cli-option-duplicate', `Option may only be supplied once: --${name}`);
    options[name] = value;
    index += 1;
  }
  for (const name of specification?.required ?? []) {
    if (!options[name]) throw cliFailure('cli-option-required', `Missing required option --${name}`);
  }
  return options;
}

function renderHelp() {
  const commands = [
    '  doctor                         Verify local runtime dependencies',
    ...CLI_COMMAND_REGISTRY.map(({ name }) => `  ${name}`)
  ].join('\n');
  return `Scene Motion Compiler ${PRODUCT_VERSION}\n\nUsage: smc <command> [options]\n\nCommands:\n${commands}\n\nGlobal:\n  help, --help, -h                Show help\n  version, --version, -v          Print the package version\n`;
}

function renderCommandHelp(command) {
  const specification = COMMAND_OPTIONS[command];
  if (!specification) throw cliFailure('cli-command-unknown', `Unknown command: ${command}`);
  const requiredNames = [...specification.required].sort();
  const optionalNames = [...specification.allowed].filter((name) => !specification.required.has(name)).sort();
  const option = (name) => `  --${name} <value>`;
  return `${command}\n\nRequired:\n${requiredNames.length ? requiredNames.map(option).join('\n') : '  (none)'}\n\nOptional:\n${optionalNames.length ? optionalNames.map(option).join('\n') : '  (none)'}\n`;
}

function required(options, name) {
  if (!options[name]) throw new Error(`Missing required option --${name}`);
  return options[name];
}

function integerOption(value, name) {
  if (!/^(?:0|[1-9][0-9]*)$/.test(String(value))) throw new Error(`--${name} must be a non-negative integer`);
  const result = Number(value);
  if (!Number.isSafeInteger(result)) throw new Error(`--${name} is outside the safe integer range`);
  return result;
}

function optionSpec(required = [], optional = []) {
  return Object.freeze({ required: new Set(required), allowed: new Set([...required, ...optional]) });
}

function cliFailure(code, message) {
  return Object.assign(new Error(message), { code });
}

main().catch((error) => {
  process.stdout.write(`${JSON.stringify({ status: 'FAIL', error: error?.code ?? 'cli-command-failed', detail: error?.message ?? String(error) }, null, 2)}\n`);
  process.exitCode = 1;
});
