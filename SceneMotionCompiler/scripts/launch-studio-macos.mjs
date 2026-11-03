#!/usr/bin/env node
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { startMacosStudio } from '../apps/studio-host/src/macos-launcher.js';

const repoRoot = resolve(fileURLToPath(new URL('../', import.meta.url)));
const { projectRoot, parentPid } = readRequiredLaunchArguments(process.argv.slice(2));
let launched;

try {
  if (process.ppid !== parentPid) {
    throw Object.assign(new Error('Studio 런처 부모 프로세스를 확인할 수 없습니다.'), { code: 'launcher-parent-missing' });
  }
  launched = await startMacosStudio({ repoRoot, projectRoot });
  console.log(`SMC_STUDIO_READY\t${JSON.stringify({
    origin: launched.origin,
    projectRoot: launched.projectRoot
  })}`);
} catch (error) {
  console.error(`SMC_STUDIO_ERROR\t${JSON.stringify({
    code: typeof error?.code === 'string' ? error.code : 'studio-start-failed',
    message: error instanceof Error ? error.message : 'Studio를 시작하지 못했습니다.'
  })}`);
  process.exitCode = 1;
}

if (launched) {
  await new Promise((resolveShutdown) => {
    let stopping = false;
    const finish = () => {
      clearInterval(parentWatchdog);
      resolveShutdown();
    };
    const stop = async () => {
      if (stopping) return;
      stopping = true;
      clearInterval(parentWatchdog);
      await launched.stop().catch(() => {});
      resolveShutdown();
    };
    const parentWatchdog = setInterval(() => {
      if (process.ppid !== parentPid) void stop();
    }, 250);
    for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.once(signal, stop);
    launched.server.once('close', finish);
  });
}

function readRequiredLaunchArguments(args) {
  const parentPid = Number(args[3]);
  if (args.length !== 4
    || args[0] !== '--project'
    || typeof args[1] !== 'string'
    || args[1].trim().length === 0
    || args[2] !== '--parent-pid'
    || !Number.isSafeInteger(parentPid)
    || parentPid < 2) {
    console.error('사용법: launch-studio-macos.mjs --project <프로젝트 폴더> --parent-pid <PID>');
    process.exit(64);
  }
  return { projectRoot: args[1], parentPid };
}
