import { accessSync, constants, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, dirname, isAbsolute, join, resolve, sep } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const STORY_ROOT = resolve(ROOT, '..');
const TOOLCHAIN_VERSION = '1.97.1';
const TEST_DEPS_DIR = join(ROOT, '.tmp', 'test-prerequisites');
const PYTHON_ENV_DIR = join(TEST_DEPS_DIR, 'nvc-cinematic-python');
const PYTHON_REQUIREMENTS = join(STORY_ROOT, 'NarrativeCourier', 'tools', 'requirements-cinematic.txt');
const PYTHON_REQUIREMENTS_VERSION = '4.26.0';
const PREPARATION_TIMEOUT_MS = 900_000;
const TEST_SUITE_TIMEOUT_MS = 900_000;
let preparationDeadlineAt = null;

function fail(message, code = 1) {
  const error = new Error(message);
  error.exitCode = code;
  throw error;
}

function pathCandidates(name, pathValue = process.env.PATH ?? '') {
  return pathValue.split(delimiter).filter(Boolean).map((directory) => join(directory, name));
}

function findExecutable(name, preferred = []) {
  const candidates = [...preferred, ...pathCandidates(name)];
  for (const candidate of candidates) {
    try {
      accessSync(candidate, constants.X_OK);
      return resolve(candidate);
    } catch {
      // Continue through explicit toolchain and PATH candidates.
    }
  }
  return null;
}

function run(label, executable, args, {
  cwd = ROOT,
  env = process.env,
  stdio = 'inherit',
  timeoutMs = 600_000,
} = {}) {
  const remainingMs = preparationDeadlineAt === null ? timeoutMs : preparationDeadlineAt - Date.now();
  const effectiveTimeoutMs = Math.min(timeoutMs, remainingMs);
  if (effectiveTimeoutMs <= 0) {
    fail(`test prerequisite preparation exceeded ${PREPARATION_TIMEOUT_MS} ms before ${label}`, 124);
  }
  const result = spawnSync(executable, args, {
    cwd,
    env,
    stdio,
    encoding: 'utf8',
    timeout: effectiveTimeoutMs,
  });
  if (result.error?.code === 'ETIMEDOUT') {
    fail(`${label} timed out after ${effectiveTimeoutMs} ms`, 124);
  }
  if (result.error) {
    fail(`${label} could not start: ${result.error.message}`);
  }
  if (result.signal) {
    fail(`${label} was interrupted by ${result.signal}`, 1);
  }
  if (result.status !== 0) {
    fail(`${label} exited with status ${result.status ?? 'unknown'}`, result.status || 1);
  }
  return result;
}

function commandOutput(executable, args, { cwd = ROOT, env = process.env, timeoutMs = 10_000 } = {}) {
  const remainingMs = preparationDeadlineAt === null ? timeoutMs : preparationDeadlineAt - Date.now();
  const effectiveTimeoutMs = Math.min(timeoutMs, remainingMs);
  if (effectiveTimeoutMs <= 0) {
    fail(`test prerequisite preparation exceeded ${PREPARATION_TIMEOUT_MS} ms before ${executable}`, 124);
  }
  const result = spawnSync(executable, args, { cwd, env, encoding: 'utf8', timeout: effectiveTimeoutMs });
  if (result.error?.code === 'ETIMEDOUT') fail(`${executable} ${args.join(' ')} timed out after ${effectiveTimeoutMs} ms`, 124);
  if (result.error || result.status !== 0) return null;
  return `${result.stdout ?? ''}${result.stderr ?? ''}`.trim();
}

function rustupToolchainBin() {
  const arch = process.arch === 'arm64' ? 'aarch64' : process.arch === 'x64' ? 'x86_64' : null;
  const host = process.platform === 'darwin' && arch ? `${arch}-apple-darwin` : null;
  return host ? join(homedir(), '.rustup', 'toolchains', `${TOOLCHAIN_VERSION}-${host}`, 'bin') : null;
}

function resolveCargo() {
  if (process.env.SMC_TEST_CARGO) {
    const explicit = resolve(process.env.SMC_TEST_CARGO);
    try {
      accessSync(explicit, constants.X_OK);
      return explicit;
    } catch {
      fail(`SMC_TEST_CARGO is not executable: ${explicit}`);
    }
  }

  const toolchainBin = rustupToolchainBin();
  const cargo = findExecutable('cargo', toolchainBin ? [join(toolchainBin, 'cargo')] : []);
  if (!cargo) {
    fail(`Rust ${TOOLCHAIN_VERSION} Cargo is required for native test prerequisites; install/select that toolchain or set SMC_TEST_CARGO`);
  }
  return cargo;
}

function rustEnvironment(cargo) {
  const toolchainBin = rustupToolchainBin();
  const paths = [dirname(cargo), toolchainBin, dirname(process.execPath), join(ROOT, 'node_modules', '.bin'), process.env.PATH]
    .filter(Boolean);
  const env = {
    ...process.env,
    PATH: [...new Set(paths.flatMap((entry) => entry.split(delimiter).filter(Boolean)))].join(delimiter),
    RUSTUP_TOOLCHAIN: TOOLCHAIN_VERSION,
  };
  return env;
}

function verifyRustToolchain(cargo, env) {
  const cargoVersion = commandOutput(cargo, ['--version'], { env });
  if (!cargoVersion || !new RegExp(`^cargo ${TOOLCHAIN_VERSION.replaceAll('.', '\\.')}\\b`).test(cargoVersion)) {
    fail(`test prerequisites require Cargo ${TOOLCHAIN_VERSION}; found ${cargoVersion ?? 'no working Cargo'}`);
  }

  const rustc = findExecutable('rustc', [rustupToolchainBin() ? join(rustupToolchainBin(), 'rustc') : null].filter(Boolean));
  const rustcVersion = rustc ? commandOutput(rustc, ['--version'], { env }) : null;
  if (!rustcVersion || !new RegExp(`^rustc ${TOOLCHAIN_VERSION.replaceAll('.', '\\.')}\\b`).test(rustcVersion)) {
    fail(`test prerequisites require rustc ${TOOLCHAIN_VERSION}; found ${rustcVersion ?? 'no working rustc'}`);
  }
}

function buildNativePrerequisites() {
  const cargo = resolveCargo();
  const env = rustEnvironment(cargo);
  verifyRustToolchain(cargo, env);

  const ownerBuilds = [
    {
      label: 'SMC animatic native executable',
      manifestPath: join(ROOT, 'Cargo.toml'),
      targetDir: join(ROOT, 'target'),
      package: 'smc-animatic',
      binary: 'smc-animatic',
    },
    {
      label: 'NarrativeCourier release CLI used by animatic integration tests',
      manifestPath: join(STORY_ROOT, 'NarrativeCourier', 'Cargo.toml'),
      targetDir: join(STORY_ROOT, 'NarrativeCourier', 'target'),
      package: 'narrative-courier',
      binary: 'narrative-courier-cli',
    },
    {
      label: 'SceneAssetCompiler animatic CLI used by animatic integration tests',
      manifestPath: join(STORY_ROOT, 'SceneAssetCompiler', 'Cargo.toml'),
      targetDir: join(STORY_ROOT, 'SceneAssetCompiler', 'target'),
      package: 'sac-animatic',
      binary: 'sac-animatic',
    },
  ];

  const artifacts = {};
  for (const build of ownerBuilds) {
    const buildEnv = { ...env, CARGO_TARGET_DIR: build.targetDir };
    run(`build ${build.label}`, cargo, [
      'build',
      '--manifest-path', build.manifestPath,
      '--package', build.package,
      '--bin', build.binary,
      '--release',
      '--locked',
      '--offline',
    ], { cwd: ROOT, env: buildEnv });

    const artifact = join(build.targetDir, 'release', build.binary);
    try {
      accessSync(artifact, constants.X_OK);
    } catch {
      fail(`${build.label} build returned success but did not produce executable ${artifact}`);
    }
    artifacts[build.binary] = artifact;
  }
  return artifacts;
}

function bundledPythonCandidate() {
  // The desktop workspace runtime places Node and Python under sibling folders.
  return resolve(dirname(process.execPath), '..', '..', 'python', 'bin', 'python3');
}

function pythonVersion(executable) {
  const output = commandOutput(executable, ['--version']);
  const match = output?.match(/Python (\d+)\.(\d+)\.(\d+)/);
  return match ? { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]), output } : null;
}

function selectPython() {
  if (process.env.SMC_TEST_PYTHON) {
    const explicit = isAbsolute(process.env.SMC_TEST_PYTHON) || process.env.SMC_TEST_PYTHON.includes(sep)
      ? resolve(process.env.SMC_TEST_PYTHON)
      : findExecutable(process.env.SMC_TEST_PYTHON);
    if (!explicit) fail(`SMC_TEST_PYTHON does not resolve to an interpreter: ${process.env.SMC_TEST_PYTHON}`);
    const version = pythonVersion(explicit);
    if (!version) fail(`SMC_TEST_PYTHON is not a working Python interpreter: ${explicit}`);
    if (version.major < 3 || (version.major === 3 && version.minor < 10)) {
      fail(`NarrativeCourier pins jsonschema==${PYTHON_REQUIREMENTS_VERSION}, which requires Python >=3.10; SMC_TEST_PYTHON is ${version.output}`);
    }
    return explicit;
  }

  const candidates = [bundledPythonCandidate()];
  for (const name of ['python3.14', 'python3.13', 'python3.12', 'python3.11', 'python3.10', 'python3', 'python']) {
    const executable = findExecutable(name);
    if (executable) candidates.push(executable);
  }

  const seen = new Set();
  const tooOld = [];
  for (const candidate of candidates) {
    const executable = resolve(candidate);
    if (seen.has(executable)) continue;
    seen.add(executable);
    const version = pythonVersion(executable);
    if (!version) continue;
    if (version.major > 3 || (version.major === 3 && version.minor >= 10)) return executable;
    tooOld.push(version.output);
  }

  const found = tooOld.length > 0 ? ` Found only ${tooOld.join(', ')}.` : '';
  fail(`NarrativeCourier pins jsonschema==${PYTHON_REQUIREMENTS_VERSION}, which requires Python >=3.10.${found} Install/select a compatible test interpreter or set SMC_TEST_PYTHON.`);
}

function virtualEnvironmentPython() {
  return process.platform === 'win32'
    ? join(PYTHON_ENV_DIR, 'Scripts', 'python.exe')
    : join(PYTHON_ENV_DIR, 'bin', 'python');
}

function installedJsonschemaVersion(python) {
  const remainingMs = preparationDeadlineAt === null ? 10_000 : preparationDeadlineAt - Date.now();
  const effectiveTimeoutMs = Math.min(10_000, remainingMs);
  if (effectiveTimeoutMs <= 0) fail(`test prerequisite preparation exceeded ${PREPARATION_TIMEOUT_MS} ms before jsonschema version check`, 124);
  const result = spawnSync(python, [
    '-c',
    "from importlib.metadata import version; print(version('jsonschema'))",
  ], { cwd: ROOT, encoding: 'utf8', timeout: effectiveTimeoutMs });
  if (result.error?.code === 'ETIMEDOUT') fail(`isolated Python jsonschema version check timed out after ${effectiveTimeoutMs} ms`, 124);
  return result.status === 0 ? result.stdout.trim() : null;
}

function preparePythonPrerequisite() {
  const basePython = selectPython();
  const envPython = virtualEnvironmentPython();
  mkdirSync(TEST_DEPS_DIR, { recursive: true });

  if (!existsSync(envPython)) {
    run('create isolated NarrativeCourier cinematic test Python environment', basePython, ['-m', 'venv', PYTHON_ENV_DIR], { timeoutMs: 60_000 });
  }

  if (installedJsonschemaVersion(envPython) !== PYTHON_REQUIREMENTS_VERSION) {
    run('install NarrativeCourier pinned cinematic test dependencies', envPython, [
      '-m', 'pip', 'install', '--disable-pip-version-check', '--no-input', '--upgrade',
      '--requirement', PYTHON_REQUIREMENTS,
    ], { timeoutMs: 180_000 });
  }

  const installed = installedJsonschemaVersion(envPython);
  if (installed !== PYTHON_REQUIREMENTS_VERSION) {
    fail(`isolated test environment has jsonschema ${installed ?? 'missing'}; expected ${PYTHON_REQUIREMENTS_VERSION}`);
  }
  run('validate isolated NarrativeCourier cinematic test dependencies', envPython, ['-m', 'pip', 'check'], { timeoutMs: 60_000 });
  return envPython;
}

function configuredExecutable(label, envNames) {
  const configured = envNames
    .filter((name) => process.env[name] !== undefined && process.env[name] !== '')
    .map((name) => ({ name, value: process.env[name] }));
  const resolved = configured.map(({ name, value }) => {
    const candidate = isAbsolute(value) || value.includes(sep)
      ? resolve(value)
      : findExecutable(value);
    if (!candidate) fail(`${name} does not resolve to an executable: ${value}`);
    try {
      accessSync(candidate, constants.X_OK);
    } catch {
      fail(`${name} is not executable: ${candidate}`);
    }
    return { name, path: candidate };
  });

  const uniqueConfiguredPaths = [...new Set(resolved.map((item) => resolve(item.path)))];
  if (uniqueConfiguredPaths.length > 1) {
    fail(`conflicting ${label} paths in ${resolved.map((item) => item.name).join(' and ')}`);
  }
  return uniqueConfiguredPaths[0] ?? null;
}

function requiredExecutable(label, names, envNames = []) {
  const found = configuredExecutable(label, envNames) ?? names.map((name) => findExecutable(name)).find(Boolean);
  if (!found) fail(`${label} is required for the complete functional-test suite; install it or configure ${envNames.join('/')}`);
  return resolve(found);
}

const SYSTEM_CHROMIUM_CANDIDATES = [
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/usr/bin/google-chrome',
  '/usr/bin/google-chrome-stable',
  '/opt/google/chrome/chrome',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
];
const CHROMIUM_COMMANDS = ['chromium', 'chromium-browser', 'google-chrome', 'google-chrome-stable', 'chrome'];

function findChromiumForTests() {
  const candidates = [
    ...SYSTEM_CHROMIUM_CANDIDATES,
    ...CHROMIUM_COMMANDS.map((name) => findExecutable(name)).filter(Boolean),
  ];
  for (const candidate of candidates) {
    try {
      accessSync(candidate, constants.X_OK);
      return resolve(candidate);
    } catch {
      // Continue through local fixed-path and PATH candidates.
    }
  }
  fail(`Chromium is required by the complete functional-test suite. Checked: ${candidates.join(', ')}`);
}

function validateRequiredTestInputs() {
  const sacPythonEntrypoint = join(
    STORY_ROOT,
    'SceneAssetCompiler',
    '.agents',
    'skills',
    'scene-asset-compiler',
    'scripts',
    'scene_asset_compiler.py',
  );
  if (!existsSync(sacPythonEntrypoint)) {
    fail(`SceneAssetCompiler producer source is required by the full SMC suite: ${sacPythonEntrypoint}`);
  }

  const ffmpeg = requiredExecutable('FFmpeg', ['ffmpeg'], ['FFMPEG_BIN', 'SMC_FFMPEG_BIN']);
  const ffprobe = requiredExecutable('ffprobe', ['ffprobe'], ['SMC_FFPROBE_BIN']);
  const chromium = configuredExecutable('Chromium', ['SMC_CHROMIUM_PATH', 'CHROMIUM_PATH']) ?? findChromiumForTests();
  return { ffmpeg, ffprobe, chromium };
}

function runAllTests(testPython, artifacts, requiredInputs) {
  const testFiles = readdirSync(join(ROOT, 'tests'))
    .filter((name) => name.endsWith('.test.js'))
    .sort()
    .map((name) => join('tests', name));
  if (testFiles.length === 0) fail('no tests/*.test.js files were found');

  const selectedBinaries = {
    SMC_ANIMATIC_BIN: artifacts['smc-animatic'],
    STORY_NVC_BIN: artifacts['narrative-courier-cli'],
    STORY_SAC_ANIMATIC_BIN: artifacts['sac-animatic'],
  };
  for (const [name, expectedPath] of Object.entries(selectedBinaries)) {
    const configuredPath = process.env[name];
    if (configuredPath && resolve(configuredPath) !== resolve(expectedPath)) {
      fail(`${name} selects ${resolve(configuredPath)}; npm test builds and tests ${expectedPath}`);
    }
  }
  for (const name of ['STORY_TEST_PYTHON', 'STORY_NVC_TEST_PYTHON', 'SCENE_ASSET_COMPILER_PYTHON']) {
    const configuredValue = process.env[name];
    if (configuredValue) {
      const configuredPath = isAbsolute(configuredValue) || configuredValue.includes(sep)
        ? resolve(configuredValue)
        : findExecutable(configuredValue);
      if (configuredPath !== resolve(testPython)) {
        fail(`${name} selects ${configuredPath ?? configuredValue}; the complete suite uses its isolated Python at ${testPython}`);
      }
    }
  }

  const pathEntries = [
    dirname(testPython),
    join(ROOT, 'node_modules', '.bin'),
    dirname(process.execPath),
    process.env.PATH,
  ].filter(Boolean);
  const env = {
    ...process.env,
    PATH: [...new Set(pathEntries.flatMap((entry) => entry.split(delimiter).filter(Boolean)))].join(delimiter),
    FFMPEG_BIN: requiredInputs.ffmpeg,
    SMC_FFMPEG_BIN: requiredInputs.ffmpeg,
    SMC_FFPROBE_BIN: requiredInputs.ffprobe,
    SMC_CHROMIUM_PATH: requiredInputs.chromium,
    CHROMIUM_PATH: requiredInputs.chromium,
    STORY_TEST_PYTHON: testPython,
    ...selectedBinaries,
    STORY_NVC_TEST_PYTHON: testPython,
    SCENE_ASSET_COMPILER_PYTHON: testPython,
  };

  const result = spawnSync(process.execPath, [
    '--test',
    '--test-concurrency=1',
    ...process.argv.slice(2),
    ...testFiles,
  ], { cwd: ROOT, env, stdio: 'inherit', timeout: TEST_SUITE_TIMEOUT_MS });
  if (result.error?.code === 'ETIMEDOUT') {
    fail(`Node test suite timed out after ${TEST_SUITE_TIMEOUT_MS} ms`, 124);
  }
  if (result.error) fail(`Node test runner could not start: ${result.error.message}`);
  if (result.signal) fail(`Node test runner was interrupted by ${result.signal}`);
  process.exitCode = result.status ?? 1;
}

try {
  preparationDeadlineAt = Date.now() + PREPARATION_TIMEOUT_MS;
  const requiredInputs = validateRequiredTestInputs();
  const artifacts = buildNativePrerequisites();
  const testPython = preparePythonPrerequisite();
  preparationDeadlineAt = null;
  runAllTests(testPython, artifacts, requiredInputs);
} catch (error) {
  console.error(`FAIL test prerequisites: ${error.message}`);
  process.exitCode = error.exitCode ?? 1;
}
