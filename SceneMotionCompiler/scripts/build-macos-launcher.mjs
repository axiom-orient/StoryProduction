#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { chmod, cp, mkdir, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.platform !== 'darwin') {
  console.error('SceneMotionCompiler Studio 런처 빌드는 macOS만 지원합니다.');
  process.exit(1);
}

const repoRoot = resolve(fileURLToPath(new URL('../', import.meta.url)));
const packageRoot = join(repoRoot, 'apps/macos-launcher');
const appRoot = join(repoRoot, 'SceneMotionCompiler Studio.app');
const retiredAppRoot = join(repoRoot, '.tmp/SceneMotionCompiler Studio.app');
const executableName = 'SceneMotionCompilerLauncher';

if (process.argv.slice(2).some((argument) => argument !== '--open')) {
  console.error('사용법: build-macos-launcher.mjs [--open]');
  process.exit(64);
}

execFileSync('/usr/bin/swift', ['build', '--package-path', packageRoot, '--configuration', 'release'], {
  cwd: repoRoot,
  stdio: 'inherit'
});
const binPath = execFileSync('/usr/bin/swift', ['build', '--package-path', packageRoot, '--configuration', 'release', '--show-bin-path'], {
  cwd: repoRoot,
  encoding: 'utf8'
}).trim();

await rm(appRoot, { recursive: true, force: true });
await rm(retiredAppRoot, { recursive: true, force: true });
await mkdir(join(appRoot, 'Contents/MacOS'), { recursive: true });
await mkdir(join(appRoot, 'Contents/Resources'), { recursive: true });
await cp(join(binPath, executableName), join(appRoot, 'Contents/MacOS', executableName));
await chmod(join(appRoot, 'Contents/MacOS', executableName), 0o755);
await writeFile(join(appRoot, 'Contents/Resources/node-path.txt'), `${process.execPath}\n`, 'utf8');
await writeFile(join(appRoot, 'Contents/Info.plist'), infoPlist(), 'utf8');

console.log(appRoot);
if (process.argv.includes('--open')) {
  execFileSync('/usr/bin/open', [appRoot], { stdio: 'inherit' });
}

function infoPlist() {
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key><string>SceneMotionCompiler Studio</string>
  <key>CFBundleExecutable</key><string>${executableName}</string>
  <key>CFBundleIdentifier</key><string>dev.scenemotioncompiler.studio-launcher</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>SceneMotionCompiler Studio</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
`;
}
