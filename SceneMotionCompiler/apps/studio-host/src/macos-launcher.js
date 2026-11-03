import { realpath } from 'node:fs/promises';
import { userInfo } from 'node:os';
import { resolve } from 'node:path';
import { sha256Hex } from '../../../packages/core/src/canonical.js';
import { createStudioServer } from './server.js';
import { MacosStudioLaunchError } from './launch-error.js';
import { validateStudioProjectRoot } from './project-root.js';

export { MacosStudioLaunchError };

export function macosOperatorId(username = userInfo().username) {
  const digest = sha256Hex(Buffer.from(String(username), 'utf8')).slice(0, 16);
  return `operator.macos.${digest}`;
}

export async function startMacosStudio({
  repoRoot,
  projectRoot,
  createServer = createStudioServer,
  operatorId = macosOperatorId()
} = {}) {
  assertMacos();
  const canonicalRepoRoot = await realpath(resolve(repoRoot));
  const canonicalProjectRoot = await validateStudioProjectRoot(projectRoot);

  const server = createServer({
    port: 0,
    projectRoot: canonicalProjectRoot,
    repoRoot: canonicalRepoRoot,
    localOperatorId: operatorId
  });
  try {
    await listen(server);
    const address = server.address();
    if (!address || typeof address !== 'object') throw new Error('Studio address is unavailable');
    const origin = `http://127.0.0.1:${address.port}`;
    return {
      status: 'RUNNING',
      origin,
      projectRoot: canonicalProjectRoot,
      server,
      stop: () => close(server)
    };
  } catch (error) {
    await close(server).catch(() => {});
    throw new MacosStudioLaunchError('studio-start-failed', 'Studio를 시작하지 못했습니다.', { cause: error });
  }
}

function assertMacos() {
  if (process.platform !== 'darwin') {
    throw new MacosStudioLaunchError('platform-unsupported', 'SceneMotionCompiler Studio 런처는 현재 macOS만 지원합니다.');
  }
}

function listen(server) {
  return new Promise((resolveListen, reject) => {
    const onError = (error) => {
      server.off('listening', onListening);
      reject(error);
    };
    const onListening = () => {
      server.off('error', onError);
      resolveListen();
    };
    server.once('error', onError);
    server.once('listening', onListening);
    server.listen(0, '127.0.0.1');
  });
}

function close(server) {
  if (!server?.listening) return Promise.resolve();
  return new Promise((resolveClose, reject) => {
    server.close((error) => error ? reject(error) : resolveClose());
  });
}
