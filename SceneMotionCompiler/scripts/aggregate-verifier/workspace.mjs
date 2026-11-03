import { randomUUID } from 'node:crypto';
import { lstat, mkdir, open, readFile, rename, rm, rmdir, unlink } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { VerifyError } from './errors.mjs';

function pathIsInside(parent, child) {
  const value = relative(resolve(parent), resolve(child));
  return value !== '' && value !== '..' && !value.startsWith(`..${sep}`) && !isAbsolute(value);
}

async function assertNoSymlinkComponents(target, label, base = null) {
  const absolute = resolve(target);
  const baseAbsolute = base ? resolve(base) : null;
  if (baseAbsolute && !pathIsInside(baseAbsolute, absolute) && absolute !== baseAbsolute) {
    throw new VerifyError('E_VERIFY_WORKSPACE', `${label} is outside its trusted base: ${absolute}`, { path: absolute, base: baseAbsolute });
  }
  const parts = baseAbsolute ? relative(baseAbsolute, absolute).split(sep).filter(Boolean) : absolute.split(sep).filter(Boolean);
  let current = baseAbsolute ?? (absolute.startsWith(sep) ? sep : '');
  if (baseAbsolute) {
    try {
      const baseStat = await lstat(baseAbsolute);
      if (baseStat.isSymbolicLink()) {
        throw new VerifyError('E_VERIFY_SYMLINK', `${label} must not contain a symlink: ${baseAbsolute}`, { path: baseAbsolute });
      }
    } catch (error) {
      if (error instanceof VerifyError) throw error;
      if (error?.code === 'ENOENT') return;
      throw error;
    }
  }
  for (const part of parts) {
    current = current === sep ? join(current, part) : join(current, part);
    try {
      const stat = await lstat(current);
      if (stat.isSymbolicLink()) {
        throw new VerifyError('E_VERIFY_SYMLINK', `${label} must not contain a symlink: ${current}`, { path: current });
      }
    } catch (error) {
      if (error instanceof VerifyError) throw error;
      if (error?.code === 'ENOENT') return;
      throw error;
    }
  }
}

async function assertDirectoryOrMissing(path, label) {
  try {
    const stat = await lstat(path);
    if (stat.isSymbolicLink()) {
      throw new VerifyError('E_VERIFY_SYMLINK', `${label} must not be a symlink: ${path}`, { path });
    }
    if (!stat.isDirectory()) {
      throw new VerifyError('E_VERIFY_WORKSPACE', `${label} must be a directory: ${path}`, { path });
    }
  } catch (error) {
    if (error?.code === 'ENOENT') return;
    throw error;
  }
}

async function assertWorkspace(root, outputDir, lockPath) {
  const absoluteRoot = resolve(root);
  const absoluteOutput = resolve(outputDir);
  const absoluteLock = resolve(lockPath);
  await assertDirectoryOrMissing(absoluteRoot, 'repository root');
  if (!pathIsInside(absoluteRoot, absoluteOutput) || relative(absoluteRoot, absoluteOutput).split(sep)[0] !== '.tmp') {
    throw new VerifyError('E_VERIFY_WORKSPACE', `verify output must be a child of <root>/.tmp: ${absoluteOutput}`);
  }
  if (dirname(absoluteLock) !== resolve(absoluteRoot, '.tmp')) {
    throw new VerifyError('E_VERIFY_WORKSPACE', `verify lock must be directly under <root>/.tmp: ${absoluteLock}`);
  }
  await assertNoSymlinkComponents(resolve(absoluteRoot, '.tmp'), 'verify parent workspace', absoluteRoot);
  await assertNoSymlinkComponents(absoluteOutput, 'verify workspace', absoluteRoot);
  await assertNoSymlinkComponents(absoluteLock, 'verify lock', absoluteRoot);
  return { root: absoluteRoot, outputDir: absoluteOutput, lockPath: absoluteLock };
}

export async function prepareWorkspace(root, outputDir, lockPath) {
  const workspace = await assertWorkspace(root, outputDir, lockPath);
  await mkdir(resolve(workspace.root, '.tmp'), { recursive: true });
  await assertNoSymlinkComponents(resolve(workspace.root, '.tmp'), 'verify parent workspace', workspace.root);
  const lock = await acquireLock(workspace.lockPath);
  return { ...workspace, lock };
}

export async function prepareOutputDir(outputDir) {
  await assertDirectoryOrMissing(outputDir, 'verify workspace');
  await rm(outputDir, { recursive: true, force: true });
  await mkdir(outputDir, { recursive: false });
  await assertDirectoryOrMissing(outputDir, 'verify workspace');
}

async function acquireLock(lockPath) {
  try {
    await mkdir(lockPath, { recursive: false });
  } catch (error) {
    if (error?.code === 'EEXIST') {
      let owner = null;
      try {
        owner = (await readFile(join(lockPath, 'owner'), 'utf8')).trim();
      } catch (ownerError) {
        owner = `unreadable:${ownerError.code ?? ownerError.message}`;
      }
      throw new VerifyError('E_VERIFY_LOCKED', `aggregate verification is already running: ${lockPath}`, { lockPath, owner });
    }
    throw error;
  }
  const token = `${process.pid}:${Date.now()}:${randomUUID()}`;
  try {
    await open(join(lockPath, 'owner'), 'wx').then(async (handle) => {
      try {
        await handle.writeFile(`${token}\n`, 'utf8');
        await handle.sync();
      } finally {
        await handle.close();
      }
    });
  } catch (error) {
    // The lock was created by this invocation, but ownership could not be
    // recorded.  Remove only the exact empty lock we just created.
    try {
      await rmdir(lockPath);
    } catch {
      // Preserve an ambiguous lock for operator recovery.
    }
    throw new VerifyError('E_VERIFY_LOCK', `could not establish aggregate verification ownership: ${error.message}`, { lockPath });
  }
  return { token, acquiredAt: new Date().toISOString() };
}

export async function releaseLock(lockPath, token) {
  const ownerPath = join(lockPath, 'owner');
  const result = {
    attempted: true,
    ownerTokenMatched: false,
    ownerRemoved: false,
    lockDirectoryRemoved: false,
    lockAbsentAfterRelease: false,
    error: null,
  };
  try {
    const ownerStat = await lstat(ownerPath);
    if (ownerStat.isSymbolicLink() || !ownerStat.isFile()) {
      result.error = 'lock owner file is not a regular file';
      return result;
    }
    const owner = (await readFile(ownerPath, 'utf8')).trim();
    result.ownerTokenMatched = owner === token;
    if (!result.ownerTokenMatched) {
      result.error = 'lock ownership changed; preserving lock';
      return result;
    }
    await unlink(ownerPath);
    result.ownerRemoved = true;
    await rmdir(lockPath);
    result.lockDirectoryRemoved = true;
    try {
      await lstat(lockPath);
      result.error = 'lock directory still exists after release';
    } catch (error) {
      if (error?.code === 'ENOENT') result.lockAbsentAfterRelease = true;
      else result.error = `lock absence check failed: ${error.message}`;
    }
  } catch (error) {
    if (error?.code === 'ENOENT') {
      result.error = 'lock ownership could not be proven because owner or lock is missing';
    } else {
      result.error = error.message;
    }
  }
  return result;
}

export async function writeAtomicJson(path, value) {
  const directory = dirname(path);
  const temporary = join(directory, `.${path.split(sep).pop()}.${process.pid}.${randomUUID()}.tmp`);
  const payload = `${JSON.stringify(value, null, 2)}\n`;
  let handle;
  try {
    handle = await open(temporary, 'wx');
    await handle.writeFile(payload, 'utf8');
    await handle.sync();
    await handle.close();
    handle = null;
    await rename(temporary, path);
  } catch (error) {
    if (handle) {
      try { await handle.close(); } catch { /* preserve original error */ }
    }
    try { await unlink(temporary); } catch { /* no partial proof is published */ }
    throw error;
  }
}
