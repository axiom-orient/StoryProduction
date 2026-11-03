import { randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import {
  TRACKB_CUSTODY_SHA256_PATTERN,
  TRACKB_CUSTODY_PROTOCOL_VERSION,
  TRACKB_CUSTODY_MAX_LINE_BYTES,
  TRACKB_CUSTODY_MAX_MEASUREMENT_RECEIPT_BYTES,
  TRACKB_CUSTODY_MAX_WRITE_BYTES,
  TRACKB_CUSTODY_MAX_POINTER_BYTES,
  TRACKB_PROJECT_ARTIFACT_KINDS,
  TRACKB_MEASUREMENT_ARTIFACT_KINDS,
  TrackBCustodyError,
  verifyMeasurementArtifactResult,
  verifyMeasurementPublishResult,
  verifyProjectArtifactResult,
  verifyInitializeResult,
  verifyAcquireResult,
  verifySnapshotResult,
  verifyWriteResult,
  verifyPublishResult,
  verifyCloseResult,
  validateNarrativeId,
  validateOpaqueId,
  validateRelativePath,
  validateImmutablePath,
  boundedBuffer,
  normalizeHelperError,
  storageIntegrity,
  inputInvalid
} from './trackb-custody-protocol.js';
import { resolveTrackBCustodyBinary } from './trackb-custody-binary.js';
import { validateProjectRoot } from './trackb-custody-files.js';

export class TrackBCustodySession {
  static async open(options = {}) {
    const session = await TrackBCustodySession.#openWithPurpose(options, 'narrative');
    try {
      await session.initializeNarrative({
        narrativeId: options.narrativeId,
        operationId: options.operationId ?? `op.${randomUUID()}`,
        readOnly: options.readOnly === true,
      });
      return session;
    } catch (error) {
      await session.#abort();
      throw error;
    }
  }

  static async openProjectSession(options = {}) {
    return TrackBCustodySession.#openWithPurpose(options, 'project-ingress');
  }

  static async openMeasurementSession(options = {}) {
    return TrackBCustodySession.#openWithPurpose(options, 'measurement');
  }

  static async #openWithPurpose(options = {}, purpose) {
    const projectRoot = await validateProjectRoot(options.projectRoot);
    if (options.readOnly !== undefined && typeof options.readOnly !== 'boolean') {
      throw inputInvalid('readOnly must be boolean when provided');
    }
    if (process.platform === 'darwin') {
      throw storageIntegrity(
        'protected signed install unavailable: descriptor-backed custody execution is unsupported on Darwin',
      );
    }
    // Copy only resolver-owned fields.  In particular, do not spread the
    // caller's environment getter before verification: the replacement test
    // and the production boundary both require pathname mutation to occur
    // only after the verified descriptor has been acquired.
    if (Object.hasOwn(options, 'applicationConfigPath')) {
      throw storageIntegrity('applicationConfigPath is not a custody option; configure binaryPath');
    }
    const resolved = await resolveTrackBCustodyBinary({
      targetTriple: options.targetTriple,
      binaryPath: options.binaryPath,
      runtimeRoot: options.runtimeRoot,
      receiptPath: options.receiptPath,
      retainBinaryHandle: true,
    });
    const binaryHandle = resolved.binaryHandle;
    if (!binaryHandle || !Number.isInteger(binaryHandle.fd) || binaryHandle.fd < 0) {
      throw storageIntegrity('custody helper verification did not retain an executable file descriptor');
    }
    // The child receives a duplicate of this verified descriptor at a fixed
    // number.  Executing the descriptor-backed path prevents a replacement of
    // the configured pathname after verification from changing the image that
    // is launched.  The parent handle is closed immediately after spawn; the
    // child owns the duplicate for the lifetime of its exec.
    const childFd = 3;
    let child;
    let binaryHandleClosed = false;
    try {
      child = spawn(`/dev/fd/${childFd}`, [
        '--project-root', projectRoot,
        '--purpose', purpose,
        '--protocol-version', String(TRACKB_CUSTODY_PROTOCOL_VERSION),
      ], {
        cwd: projectRoot,
        env: options.env === undefined ? process.env : options.env,
        shell: false,
        stdio: ['pipe', 'pipe', 'pipe', binaryHandle.fd],
        windowsHide: true,
      });
    } catch (error) {
      await binaryHandle.close().catch(() => {});
      throw storageIntegrity(`custody helper spawn failed: ${error.message}`, { cause: error });
    }
    let session = null;
    try {
      session = new TrackBCustodySession({ child, projectRoot, resolved, purpose, requestTimeoutMs: options.requestTimeoutMs });
      await binaryHandle.close();
      binaryHandleClosed = true;
      return session;
    } catch (error) {
      if (session) await session.#abort();
      else if (child.exitCode === null && child.signalCode === null) {
        try { child.kill('SIGTERM'); } catch {}
      }
      if (!binaryHandleClosed) await binaryHandle.close().catch(() => {});
      throw error;
    }
  }

  constructor({ child, projectRoot, resolved, purpose = 'narrative', requestTimeoutMs = 10_000 }) {
    this.child = child;
    this.projectRoot = projectRoot;
    this.resolved = resolved;
    if (purpose !== 'narrative' && purpose !== 'project-ingress' && purpose !== 'measurement') throw inputInvalid('custody session purpose is invalid');
    this.purpose = purpose;
    this.readOnly = false;
    this.initialized = false;
    this.narrativeId = null;
    this.projectArtifactReads = new Map();
    this.projectArtifactSequenceIndex = 0;
    this.measurementArtifactReads = new Map();
    this.measurementArtifactSequenceIndex = 0;
    if (!Number.isSafeInteger(requestTimeoutMs) || requestTimeoutMs < 100 || requestTimeoutMs > 60_000) throw new TrackBCustodyError('trackb-input-invalid', 'requestTimeoutMs must be an integer between 100 and 60000');
    this.requestTimeoutMs = requestTimeoutMs;
    this.operationId = null;
    this.closed = false;
    this.nextRequestId = 1;
    this.pending = new Map();
    this.stdoutBuffer = Buffer.alloc(0);
    this.stderrBuffer = Buffer.alloc(0);
    this.#wireChild();
  }

  async initializeNarrative({ narrativeId, operationId = `op.${randomUUID()}`, readOnly = false } = {}) {
    if (this.initialized) throw inputInvalid('narrative is already initialized');
    if (typeof readOnly !== 'boolean') throw inputInvalid('readOnly must be boolean when provided');
    validateNarrativeId(narrativeId);
    validateOpaqueId(operationId, 'operationId');
    if (this.purpose === 'project-ingress') {
      if (this.projectArtifactSequenceIndex !== TRACKB_PROJECT_ARTIFACT_KINDS.length
        || this.projectArtifactReads.size !== TRACKB_PROJECT_ARTIFACT_KINDS.length
        || TRACKB_PROJECT_ARTIFACT_KINDS.some((kind) => !this.projectArtifactReads.get(kind)?.exists)) {
        throw inputInvalid('project-ingress requires all seven artifacts present in the fixed read order before initialize');
      }
    }
    const initialized = await this.#request('initialize', {
      mode: readOnly ? 'readOnly' : 'mutable',
      narrativeId,
      operationId,
    });
    verifyInitializeResult(initialized, {
      narrativeId,
      mode: readOnly ? 'readOnly' : 'mutable',
    });
    this.initialized = true;
    this.narrativeId = narrativeId;
    if (!readOnly) {
      const acquired = await this.#request('acquire', { narrativeId, operationId: this.operationId });
      verifyAcquireResult(acquired, { narrativeId, operationId });
    }
    return initialized;
  }

  async readProjectArtifact({ kind, path, projectionReceiptHash } = {}) {
    if (this.purpose !== 'project-ingress') {
      throw inputInvalid('readProjectArtifact is only available in a project-ingress session');
    }
    if (!TRACKB_PROJECT_ARTIFACT_KINDS.includes(kind)) throw inputInvalid(`unsupported project artifact kind: ${kind}`);
    if (this.initialized) throw inputInvalid('readProjectArtifact is only available before narrative initialize');
    if (this.projectArtifactReads.has(kind)) throw inputInvalid(`project artifact kind may be read only once per session: ${kind}`);
    const expectedKind = TRACKB_PROJECT_ARTIFACT_KINDS[this.projectArtifactSequenceIndex];
    if (expectedKind === undefined) throw inputInvalid('project artifact sequence is already complete');
    if (kind !== expectedKind) throw inputInvalid(`project artifact kind must be read in fixed order; expected ${expectedKind}, got ${kind}`);
    const fields = { kind, path: validateRelativePath(path) };
    if (kind === 'sceneGraph') {
      if (typeof projectionReceiptHash !== 'string' || !TRACKB_CUSTODY_SHA256_PATTERN.test(projectionReceiptHash)) {
        throw inputInvalid('projectionReceiptHash must be a sha256:<hex> hash');
      }
      const cached = this.projectArtifactReads.get('projectionReceipt');
      if (!cached) throw inputInvalid('sceneGraph requires a prior projectionReceipt read');
      if (cached.sha256 !== projectionReceiptHash) throw inputInvalid('projectionReceiptHash does not match the cached raw receipt hash');
      fields.projectionReceiptHash = projectionReceiptHash;
    } else if (projectionReceiptHash !== undefined) {
      throw inputInvalid('projectionReceiptHash is only valid for sceneGraph');
    }
    const result = await this.#request('readProjectArtifact', fields);
    const verified = verifyProjectArtifactResult(result, { kind, path: fields.path });
    this.projectArtifactReads.set(kind, verified);
    this.projectArtifactSequenceIndex += 1;
    return verified;
  }

  async readMeasurementArtifact({ kind, path } = {}) {
    if (this.purpose !== 'measurement') {
      throw inputInvalid('readMeasurementArtifact is only available in a measurement session');
    }
    if (this.initialized) throw inputInvalid('readMeasurementArtifact is only available before initialize');
    if (!TRACKB_MEASUREMENT_ARTIFACT_KINDS.includes(kind)) {
      throw inputInvalid(`unsupported measurement artifact kind: ${kind}`);
    }
    const expectedKind = TRACKB_MEASUREMENT_ARTIFACT_KINDS[this.measurementArtifactSequenceIndex];
    if (expectedKind === undefined) throw inputInvalid('measurement artifact sequence is already complete');
    if (kind !== expectedKind) {
      throw inputInvalid(`measurement artifact kind must be read in fixed order; expected ${expectedKind}, got ${kind}`);
    }
    if (this.measurementArtifactReads.has(kind)) {
      throw inputInvalid(`measurement artifact kind may be read only once per session: ${kind}`);
    }
    const safePath = validateRelativePath(path);
    const result = await this.#request('readMeasurementArtifact', { kind, path: safePath });
    const verified = verifyMeasurementArtifactResult(result, { kind, path: safePath });
    this.measurementArtifactReads.set(kind, verified);
    this.measurementArtifactSequenceIndex += 1;
    return verified;
  }

  async snapshot(path) {
    const safePath = validateRelativePath(path);
    const result = await this.#request('snapshot', { path: safePath });
    return verifySnapshotResult(result, { path: safePath });
  }

  async read(path) {
    const safePath = validateRelativePath(path);
    const result = await this.#request('read', { path: safePath });
    return verifySnapshotResult(result, { path: safePath });
  }

  async writeImmutable(path, bytes) {
    if (this.readOnly) throw inputInvalid('writeImmutable is forbidden in a read-only custody session');
    const payload = boundedBuffer(bytes, TRACKB_CUSTODY_MAX_WRITE_BYTES, 'immutable write');
    const safePath = validateImmutablePath(path);
    const result = await this.#request('writeImmutable', { path: safePath, bytesBase64: payload.toString('base64url') });
    return verifyWriteResult(result, { path: safePath, bytes: payload });
  }

  async publishMeasurementReceipt(path, bytes) {
    if (path && typeof path === 'object' && bytes === undefined) {
      ({ path, bytes } = path);
    }
    if (this.purpose !== 'measurement') {
      throw inputInvalid('publishMeasurementReceipt is only available in a measurement session');
    }
    if (this.measurementArtifactSequenceIndex !== TRACKB_MEASUREMENT_ARTIFACT_KINDS.length
      || this.measurementArtifactReads.size !== TRACKB_MEASUREMENT_ARTIFACT_KINDS.length
      || TRACKB_MEASUREMENT_ARTIFACT_KINDS.some((kind) => !this.measurementArtifactReads.get(kind)?.exists)) {
      throw inputInvalid('publishMeasurementReceipt requires all three measurement artifacts to be read and present first');
    }
    const payload = boundedBuffer(bytes, TRACKB_CUSTODY_MAX_MEASUREMENT_RECEIPT_BYTES, 'measurement receipt publish');
    const safePath = validateRelativePath(path);
    const result = await this.#request('publishMeasurementReceipt', {
      path: safePath,
      bytesBase64: payload.toString('base64url'),
    });
    return verifyMeasurementPublishResult(result, { path: safePath, bytes: payload });
  }

  async publishPointer({ expectedBaseDeliverySetHash, expectedPointerSnapshotToken, pointerBytes, deliverySetPath, deliverySetBytes, deliverySetSnapshotToken } = {}) {
    if (this.readOnly) throw inputInvalid('publishPointer is forbidden in a read-only custody session');
    const pointerPayload = boundedBuffer(pointerBytes, TRACKB_CUSTODY_MAX_POINTER_BYTES, 'pointer publish');
    const deliverySetPayload = boundedBuffer(deliverySetBytes, TRACKB_CUSTODY_MAX_POINTER_BYTES, 'delivery-set publish');
    if (!/^(?:NONE|sha256:[0-9a-f]{64})$/.test(expectedBaseDeliverySetHash ?? '')) throw inputInvalid('expectedBaseDeliverySetHash must be NONE or a hash');
    if (typeof expectedPointerSnapshotToken !== 'string' || (expectedPointerSnapshotToken !== 'NONE' && expectedPointerSnapshotToken.length > 16 * 1024)) throw inputInvalid('expectedPointerSnapshotToken must be NONE or a bounded opaque token');
    const safeDeliverySetPath = validateRelativePath(deliverySetPath);
    if (typeof deliverySetSnapshotToken !== 'string' || deliverySetSnapshotToken === 'NONE' || deliverySetSnapshotToken.length > 16 * 1024) throw inputInvalid('deliverySetSnapshotToken must be a bounded present snapshot token');
    const result = await this.#request('publishPointer', {
      path: 'delivery-pointer-v2.json',
      expectedBaseDeliverySetHash,
      expectedPointerSnapshotToken,
      pointerBytesBase64: pointerPayload.toString('base64url'),
      deliverySetPath: safeDeliverySetPath,
      deliverySetBytesBase64: deliverySetPayload.toString('base64url'),
      deliverySetSnapshotToken,
    });
    return verifyPublishResult(result, { pointerBytes: pointerPayload });
  }

  async close() {
    if (this.closed) return { closed: true };
    try {
      const result = await this.#request('close', {});
      verifyCloseResult(result);
      this.closed = true;
      await this.#waitForExit();
      return result;
    } catch (error) {
      await this.#abort();
      throw error;
    }
  }

  #wireChild() {
    this.child.stdout.on('data', (chunk) => this.#consumeStdout(Buffer.from(chunk)));
    this.child.stderr.on('data', (chunk) => {
      this.stderrBuffer = Buffer.concat([this.stderrBuffer, Buffer.from(chunk)]).subarray(-TRACKB_CUSTODY_MAX_LINE_BYTES);
    });
    this.child.on('error', (error) => this.#failPending(storageIntegrity(`custody helper spawn failed: ${error.message}`, { cause: error })));
    this.child.on('exit', (code, signal) => {
      if (!this.closed) this.#failPending(storageIntegrity(`custody helper exited before session close (code=${code ?? 'null'}, signal=${signal ?? 'null'})`));
    });
  }

  #consumeStdout(chunk) {
    this.stdoutBuffer = Buffer.concat([this.stdoutBuffer, chunk]);
    if (this.stdoutBuffer.length > TRACKB_CUSTODY_MAX_LINE_BYTES * 2) {
      this.#failPending(storageIntegrity('custody helper protocol output exceeded its bounded buffer'));
      void this.#abort();
      return;
    }
    let newline;
    while ((newline = this.stdoutBuffer.indexOf(0x0a)) >= 0) {
      const line = this.stdoutBuffer.subarray(0, newline);
      this.stdoutBuffer = this.stdoutBuffer.subarray(newline + 1);
      if (line.length > TRACKB_CUSTODY_MAX_LINE_BYTES) {
        this.#failPending(storageIntegrity('custody helper protocol line exceeded its bound'));
        void this.#abort();
        return;
      }
      let response;
      try { response = JSON.parse(line.toString('utf8')); }
      catch (error) {
        this.#failPending(storageIntegrity(`custody helper protocol response is invalid JSON: ${error.message}`, { cause: error }));
        void this.#abort();
        return;
      }
      const requestId = typeof response?.id === 'string' ? response.id : null;
      const pending = requestId === null ? null : this.pending.get(requestId);
      if (!pending) {
        this.#failPending(storageIntegrity('custody helper returned an unknown protocol request id'));
        void this.#abort();
        return;
      }
      this.pending.delete(requestId);
      if (response.ok === true) pending.resolve(response.result);
      else pending.reject(normalizeHelperError(response.error));
    }
  }

  #request(op, fields) {
    if (this.closed) return Promise.reject(inputInvalid('custody session is closed'));
    if (op === 'initialize') {
      if (typeof fields.narrativeId !== 'string') return Promise.reject(inputInvalid('narrativeId is required'));
      validateNarrativeId(fields.narrativeId);
      if (fields.mode !== 'mutable' && fields.mode !== 'readOnly') return Promise.reject(inputInvalid('mode must be exactly mutable or readOnly'));
      this.readOnly = fields.mode === 'readOnly';
      this.operationId = fields.operationId;
      validateOpaqueId(this.operationId, 'operationId');
    }
    const id = `r${this.nextRequestId++}`;
    const request = JSON.stringify({ id, op, ...fields });
    if (Buffer.byteLength(request) + 1 > TRACKB_CUSTODY_MAX_LINE_BYTES) return Promise.reject(inputInvalid('custody request exceeds its bounded line size'));
    return new Promise((resolvePromise, rejectPromise) => {
      const pending = {
        resolve: (value) => { clearTimeout(timer); resolvePromise(value); },
        reject: (error) => { clearTimeout(timer); rejectPromise(error); },
      };
      const timer = setTimeout(() => {
        if (!this.pending.delete(id)) return;
        rejectPromise(storageIntegrity(`custody helper request timed out: ${op}`));
        void this.#abort();
      }, this.requestTimeoutMs);
      this.pending.set(id, pending);
      this.child.stdin.write(`${request}\n`, (error) => {
        if (error) {
          this.pending.delete(id);
          clearTimeout(timer);
          rejectPromise(storageIntegrity(`custody helper request write failed: ${error.message}`, { cause: error }));
        }
      });
    });
  }

  #failPending(error) {
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
  }

  #waitForExit() {
    if (this.child.exitCode !== null || this.child.signalCode !== null) return Promise.resolve();
    return new Promise((resolvePromise) => this.child.once('exit', () => resolvePromise()));
  }

  async #abort() {
    this.#failPending(storageIntegrity('custody session aborted'));
    if (this.child.exitCode === null && this.child.signalCode === null) {
      try { this.child.stdin.destroy(); } catch {}
      try { this.child.kill('SIGTERM'); } catch {}
      await new Promise((resolvePromise) => {
        const timer = setTimeout(resolvePromise, 500);
        this.child.once('exit', () => { clearTimeout(timer); resolvePromise(); });
      });
      if (this.child.exitCode === null && this.child.signalCode === null) {
        try { this.child.kill('SIGKILL'); } catch {}
      }
    }
    this.closed = true;
  }
}

export async function openTrackBCustodySession(options = {}) {
  return TrackBCustodySession.open(options);
}

export async function openTrackBCustodyProjectSession(options = {}) {
  return TrackBCustodySession.openProjectSession(options);
}

export async function openMeasurementSession(options = {}) {
  return TrackBCustodySession.openMeasurementSession(options);
}
