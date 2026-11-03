import { constants as fsConstants } from 'node:fs';
import { lstat, open, realpath } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { MacosStudioLaunchError } from './launch-error.js';

const HANDOFF_RELATIVE_PATH = 'handoff/scene-motion-asset-handoff.v2.json';

export async function validateStudioProjectRoot(selectedPath) {
  if (typeof selectedPath !== 'string' || selectedPath.trim().length === 0) {
    throw new MacosStudioLaunchError('project-not-selected', '프로젝트 폴더를 선택하지 않았습니다.');
  }
  const declaredRoot = resolve(selectedPath);
  let rootEntry;
  try {
    rootEntry = await lstat(declaredRoot);
  } catch (error) {
    throw new MacosStudioLaunchError('project-unreadable', '선택한 프로젝트 폴더를 읽을 수 없습니다.', { cause: error });
  }
  if (rootEntry.isSymbolicLink() || !rootEntry.isDirectory()) {
    throw new MacosStudioLaunchError('project-unsafe', '실제 프로젝트 폴더를 선택하세요. 별칭이나 심볼릭 링크는 사용할 수 없습니다.');
  }

  const canonicalRoot = await realpath(declaredRoot);
  const handoffDirectory = join(canonicalRoot, 'handoff');
  let handoffDirectoryEntry;
  try {
    handoffDirectoryEntry = await lstat(handoffDirectory);
  } catch (error) {
    throw new MacosStudioLaunchError(
      'project-not-ready',
      '이 프로젝트에는 승인·게시된 장면 자료가 없습니다. SceneAssetCompiler에서 이미지와 자산을 승인·게시한 뒤 다시 선택하세요.',
      { cause: error }
    );
  }
  if (handoffDirectoryEntry.isSymbolicLink() || !handoffDirectoryEntry.isDirectory()) {
    throw new MacosStudioLaunchError('project-unsafe', '프로젝트의 handoff 폴더가 안전한 실제 폴더가 아닙니다.');
  }

  const handoffPath = join(canonicalRoot, HANDOFF_RELATIVE_PATH);
  let handoffHandle;
  try {
    handoffHandle = await open(handoffPath, fsConstants.O_RDONLY | fsConstants.O_NOFOLLOW);
    const handoffEntry = await handoffHandle.stat();
    if (!handoffEntry.isFile()) throw new Error('handoff is not a regular file');
  } catch (error) {
    throw new MacosStudioLaunchError(
      'project-not-ready',
      '이 프로젝트에는 승인·게시된 장면 자료가 없습니다. SceneAssetCompiler에서 이미지와 자산을 승인·게시한 뒤 다시 선택하세요.',
      { cause: error }
    );
  } finally {
    await handoffHandle?.close().catch(() => {});
  }
  return canonicalRoot;
}
