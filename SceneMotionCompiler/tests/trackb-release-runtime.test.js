import { singleSceneProjectionFixture } from './support/narrative-fixture.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { ApplicationCommandError } from '../packages/application/src/command-error.js';
import { approveTrackBRelease } from '../packages/application/src/trackb-release-runtime-approve.js';
import { prepareTrackBRelease } from '../packages/application/src/trackb-release-runtime-prepare.js';
import { promoteTrackBRelease } from '../packages/application/src/trackb-release-runtime-promote.js';
import { statusTrackBRelease } from '../packages/application/src/trackb-release-runtime-status.js';
import * as applicationIndex from '../packages/application/src/index.js';
import { buildTrackBReleaseCandidate } from '../packages/application/src/trackb-release-candidate.js';
import {
  TRACK_B_PROJECTION_RECEIPT_PATH,
  loadTrackBReleaseInputsFromArtifacts
} from '../packages/application/src/trackb-release-input-contract.js';
import { loadTrackBArtifactContractRegistries } from '../packages/contracts/src/trackb-artifact-schemas.js';
import { loadTrackBContractRegistry } from '../packages/contracts/src/trackb-registry.js';
import {
  hashTrackBAuthority,
  hashTrackBApprovalReceipt,
  hashTrackBDeliveryPointer,
  hashTrackBDeliverySet,
  hashTrackBEvidenceCell,
  hashTrackBMeasuredEvidenceReceipt,
  hashTrackBRawBytes,
  hashTrackBReleaseCandidate
} from '../packages/contracts/src/trackb-contract.js';
import { canonicalHash, canonicalizeJson } from '../packages/core/src/canonical.js';
import { composeSceneHtml } from '../packages/composer/src/scene-html.js';

const EXAMPLE_ROOT = 'contracts/examples';

test('Track B durable prepare reads seven custody artifacts in order and stages canonical immutable bytes', async () => {
  const fixture = await createFixture();
  try {
    const custody = createCustodyDouble(fixture);
    const result = await prepareTrackBRelease(fixture.request, {
      openProjectSession: custody.openProjectSession,
      workflowDeps: fixture.workflowDeps
    });

    assert.deepEqual(Object.keys(result).sort(), [
      'authorityHash',
      'baseDeliverySetHash',
      'candidateHash',
      'candidatePath',
      'candidateVerdict',
      'evidenceCellCount',
      'evidenceReceiptHash',
      'meaningQualificationHash'
    ]);
    assert.equal(result.baseDeliverySetHash, 'NONE');
    assert.equal(result.candidateVerdict, 'PROMOTE');
    assert.equal(result.evidenceCellCount, fixture.authority.shots.length);
    assert.equal(
      result.candidatePath,
      `scenes/trackb/${fixture.authority.narrativeId}/candidates/${result.candidateHash.slice('sha256:'.length)}/trackb-release-candidate-v4.json`
    );

    assert.deepEqual(custody.events.map((event) => event.op), [
      'open',
      'read:narrativeAuthority',
      'read:projectionReceipt',
      'read:assetHandoff',
      'read:html',
      'read:provenance',
      'read:evidenceReceipt',
      'read:sceneGraph',
      'initialize',
      'snapshot:delivery-pointer-v2.json',
      'write:authority',
      'write:candidate',
      'close'
    ]);
    assert.equal(custody.events.filter((event) => event.op === 'open').length, 1, 'all custody operations must use one session');
    assert.equal(custody.events.findIndex((event) => event.op === 'initialize') > custody.events.findIndex((event) => event.op === 'read:sceneGraph'), true);
    assert.equal(custody.events.findIndex((event) => event.op === 'write:authority') > custody.events.findIndex((event) => event.op === 'snapshot:delivery-pointer-v2.json'), true);
    assert.equal(custody.events.at(-1).op, 'close');
    assert.equal(custody.pointerPublishCalled, false);
    assert.equal(custody.baseChecked, true);

    const authorityWrite = custody.writes.find((write) => write.path.startsWith('authority/'));
    const candidateWrite = custody.writes.find((write) => write.path.startsWith('candidates/'));
    assert.ok(authorityWrite);
    assert.ok(candidateWrite);
    assert.equal(authorityWrite.path, `authority/${result.authorityHash.slice('sha256:'.length)}/trackb-narrative-authority-v2.json`);
    assert.equal(authorityWrite.bytes.toString('utf8'), canonicalizeJson(fixture.authority));
    assert.equal(candidateWrite.path, `candidates/${result.candidateHash.slice('sha256:'.length)}/trackb-release-candidate-v4.json`);
    assert.equal(hashTrackBRawBytes(candidateWrite.bytes), hashTrackBRawBytes(Buffer.from(candidateWrite.bytes)));
    assert.equal(JSON.parse(candidateWrite.bytes.toString('utf8')).candidateVerdict, 'PROMOTE');

    const sceneGraphRead = custody.events.find((event) => event.op === 'read:sceneGraph');
    assert.equal(sceneGraphRead.path, fixture.projection.sceneGraphPath);
    assert.equal(sceneGraphRead.projectionReceiptHash, hashTrackBRawBytes(fixture.projectionBytes));
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B production prepare rejects in-memory measuredEvidence injection', async () => {
  const fixture = await createFixture();
  try {
    await assert.rejects(
      prepareTrackBRelease(fixture.request, {
        measuredEvidence: { [fixture.authority.shots[0].shotId]: { pass: true } },
        openProjectSession: async () => { throw new Error('injected evidence must be rejected before custody opens'); },
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-evidence-incomplete' && error.retryable === false
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable prepare compares absent current base before any immutable write', async () => {
  const fixture = await createFixture();
  try {
    const custody = createCustodyDouble(fixture);
    await assert.rejects(
      prepareTrackBRelease({
        ...fixture.request,
        input: {
          ...fixture.request.input,
          expectedBaseDeliverySetHash: `sha256:${'a'.repeat(64)}`
        }
      }, {
        openProjectSession: custody.openProjectSession,
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-cas-conflict' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0, 'base conflict must precede immutable writes');
    assert.equal(custody.pointerPublishCalled, false);
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable prepare does not assume a present pointer is NONE', async () => {
  const fixture = await createFixture();
  try {
    const custody = createCustodyDouble(fixture, { pointerPresent: true });
    await assert.rejects(
      prepareTrackBRelease(fixture.request, {
        openProjectSession: custody.openProjectSession,
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-storage-integrity' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.equal(custody.pointerPublishCalled, false);
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable prepare permits exact immutable write reuse and keeps result deterministic', async () => {
  const fixture = await createFixture();
  const persistentWrites = new Map();
  try {
    const firstCustody = createCustodyDouble(fixture, { persistentWrites });
    const secondCustody = createCustodyDouble(fixture, { persistentWrites });
    const dependencies = {
      workflowDeps: fixture.workflowDeps
    };
    const first = await prepareTrackBRelease(fixture.request, {
      ...dependencies,
      openProjectSession: firstCustody.openProjectSession
    });
    const second = await prepareTrackBRelease(fixture.request, {
      ...dependencies,
      openProjectSession: secondCustody.openProjectSession
    });
    assert.deepEqual(second, first);
    assert.deepEqual(firstCustody.writes.map((write) => write.reused), [false, false]);
    assert.deepEqual(secondCustody.writes.map((write) => write.reused), [true, true]);
    assert.equal(firstCustody.pointerPublishCalled || secondCustody.pointerPublishCalled, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable prepare closes custody on each typed failure path', async () => {
  const fixture = await createFixture();
  try {
    for (const failureAt of ['read:projectionReceipt', 'initialize', 'snapshot', 'write:authority', 'write:candidate']) {
      const custody = createCustodyDouble(fixture, { failureAt });
      await assert.rejects(
        prepareTrackBRelease(fixture.request, {
          openProjectSession: custody.openProjectSession,
          workflowDeps: fixture.workflowDeps
        }),
        (error) => error.code === (failureAt === 'read:projectionReceipt' ? 'trackb-lock-busy' : 'trackb-storage-integrity')
          && error.retryable === (failureAt === 'read:projectionReceipt')
      );
      assert.equal(custody.events.at(-1).op, 'close', `custody must close after ${failureAt}`);
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable prepare rejects malformed custody responses at the runtime boundary', async () => {
  const malformedCases = [
    {
      name: 'read extra field',
      operation: 'read:projectionReceipt',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 0
    },
    {
      name: 'initialize missing field',
      operation: 'initialize',
      mutate: (result) => {
        const malformed = { ...result };
        delete malformed.mode;
        return malformed;
      },
      expectedWrites: 0
    },
    {
      name: 'snapshot extra field',
      operation: 'snapshot',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 0
    },
    {
      name: 'authority write wrong snapshot hash',
      operation: 'write:authority',
      mutate: (result) => ({
        ...result,
        snapshot: { ...result.snapshot, sha256: `sha256:${'0'.repeat(64)}` }
      }),
      expectedWrites: 1
    },
    {
      name: 'candidate write extra field',
      operation: 'write:candidate',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 2
    }
  ];
  for (const testCase of malformedCases) {
    const fixture = await createFixture();
    try {
      const custody = createCustodyDouble(fixture, {
        responseMutation: { [testCase.operation]: testCase.mutate }
      });
      await assert.rejects(
        prepareTrackBRelease(fixture.request, {
          openProjectSession: custody.openProjectSession,
          workflowDeps: fixture.workflowDeps
        }),
        (error) => error.code === 'trackb-storage-integrity' && error.retryable === false,
        testCase.name
      );
      assert.equal(custody.writes.length, testCase.expectedWrites, `${testCase.name} must not hide the attempted write`);
      assert.equal(custody.events.at(-1).op, 'close', `${testCase.name} must close custody`);
    } finally {
      await rm(fixture.root, { recursive: true, force: true });
    }
  }
});

test('Track B durable prepare preserves the primary typed failure and records bounded close failure', async () => {
  const fixture = await createFixture();
  try {
    const successCloseFailure = createCustodyDouble(fixture, {
      closeFailure: new ApplicationCommandError('trackb-lock-busy', 'close lock failure', { retryable: true })
    });
    await assert.rejects(
      prepareTrackBRelease(fixture.request, {
        openProjectSession: successCloseFailure.openProjectSession,
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-storage-integrity'
        && error.retryable === false
        && error.details?.closeCode === 'trackb-lock-busy'
    );

    const primaryFailureCloseFailure = createCustodyDouble(fixture, {
      failureAt: 'read:projectionReceipt',
      closeFailure: new ApplicationCommandError('trackb-storage-integrity', 'close integrity failure')
    });
    await assert.rejects(
      prepareTrackBRelease(fixture.request, {
        openProjectSession: primaryFailureCloseFailure.openProjectSession,
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-lock-busy'
        && error.retryable === true
        && error.details?.custodyClose?.code === 'trackb-storage-integrity'
        && error.details.custodyClose.retryable === false
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B application index exposes only the consumer command surface', () => {
  assert.deepEqual(Object.keys(applicationIndex).sort(), [
    'APPLICATION_COMMAND_METADATA',
    'CLI_COMMAND_REGISTRY',
    'MCP_COMMAND_REGISTRY',
    'PRODUCT_VERSION',
    'STUDIO_COMMANDS',
    'commandForCli',
    'createApplicationCommandService',
    'executeApplicationCommand'
  ]);
});

test('Track B durable approval records an exact human Studio receipt for a PROMOTE candidate', async () => {
  const fixture = await createApprovalFixture();
  try {
    const custody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted
    });
    const input = approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' });
    const context = approvalContext();
    const result = await approveTrackBRelease(input, context, approvalDependencies(fixture, custody));

    assert.deepEqual(Object.keys(result).sort(), ['approvalHash', 'approvalPath', 'decision', 'reused', 'subject']);
    assert.equal(result.approvalPath, `approvals/${fixture.promoted.candidateHash.slice('sha256:'.length)}/NONE/trackb-release-approval-receipt-v2.json`);
    assert.equal(result.approvalHash.startsWith('sha256:'), true);
    assert.deepEqual(result.subject, {
      candidateHash: fixture.promoted.candidateHash,
      meaningQualificationHash: fixture.promoted.candidate.meaningQualificationHash,
      baseDeliverySetHash: 'NONE'
    });
    assert.equal(result.decision, 'APPROVE');
    assert.equal(result.reused, false);
    assert.deepEqual(custody.events.map((event) => event.op), [
      'open',
      'read:candidate',
      'read:authority',
      'snapshot:delivery-pointer-v2.json',
      'snapshot:approval',
      'write:approval',
      'close'
    ]);
    assert.equal(custody.pointerPublishCalled, false);
    assert.equal(custody.writes.length, 1);
    const write = custody.writes[0];
    assert.equal(write.path, result.approvalPath);
    assert.equal(write.bytes.toString('utf8'), canonicalizeJson(JSON.parse(write.bytes.toString('utf8'))));
    const receipt = JSON.parse(write.bytes.toString('utf8'));
    assert.deepEqual(receipt.subject, result.subject);
    assert.equal(receipt.decision, 'APPROVE');
    assert.deepEqual(receipt.actor, { actorId: context.actorId, actorKind: 'human' });
    assert.equal(receipt.origin, 'studio');
    assert.equal(receipt.reviewSessionId, context.reviewSessionId);
    assert.equal(receipt.reason, input.reason);
    assert.equal(receipt.decidedAt, input.decidedAt);
    assert.equal(receipt.approvalHash, result.approvalHash);
    assert.equal(hashTrackBApprovalReceipt(receipt), result.approvalHash);
    assert.equal(receipt.approvalId, `approval.${canonicalHash({
      narrativeId: input.narrativeId,
      candidateHash: input.candidateHash,
      meaningQualificationHash: fixture.promoted.candidate.meaningQualificationHash,
      baseDeliverySetHash: input.baseDeliverySetHash
    }).slice('sha256:'.length)}`);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval blocks APPROVE for ITERATE before any approval write', async () => {
  const fixture = await createApprovalFixture();
  try {
    const custody = createApprovalCustodyDouble(fixture, { candidate: fixture.iterate });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.iterate, { decision: 'APPROVE' }),
        approvalContext(),
        approvalDependencies(fixture, custody)
      ),
      (error) => error.code === 'trackb-delivery-not-ready' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.deepEqual(custody.events.map((event) => event.op), ['open', 'read:candidate', 'read:authority', 'close']);
    assert.equal(custody.pointerPublishCalled, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval records REVISE and REJECT for any valid candidate verdict', async () => {
  const fixture = await createApprovalFixture();
  try {
    for (const decision of ['REVISE', 'REJECT']) {
      const custody = createApprovalCustodyDouble(fixture, { candidate: fixture.iterate });
      const result = await approveTrackBRelease(
        approvalInput(fixture, fixture.iterate, { decision }),
        approvalContext(),
        approvalDependencies(fixture, custody)
      );
      assert.equal(result.decision, decision);
      assert.equal(result.reused, false);
      assert.equal(custody.writes.length, 1);
      assert.equal(custody.pointerPublishCalled, false);
      assert.equal(custody.events.at(-1).op, 'close');
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval rejects stale candidate/base subjects before immutable writes', async () => {
  const fixture = await createApprovalFixture();
  try {
    const candidateBase = `sha256:${'a'.repeat(64)}`;
    const staleBuilt = buildTrackBReleaseCandidate({
      ...fixture.loaded,
      candidateId: fixture.request.input.candidateId,
      expectedBaseDeliverySetHash: candidateBase
    });
    const staleCandidate = staleBuilt.candidate;
    const custody = createApprovalCustodyDouble(fixture, {
      candidate: { candidate: staleCandidate, bytes: staleBuilt.candidateBytes }
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, staleCandidate, { baseDeliverySetHash: candidateBase }),
        approvalContext(),
        approvalDependencies(fixture, custody)
      ),
      (error) => error.code === 'trackb-cas-conflict' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.equal(custody.baseChecked, true);

    const mismatchedInputCustody = createApprovalCustodyDouble(fixture, { candidate: fixture.iterate });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.iterate, { baseDeliverySetHash: candidateBase }),
        approvalContext(),
        approvalDependencies(fixture, mismatchedInputCustody)
      ),
      (error) => error.code === 'trackb-approval-stale' && error.retryable === false
    );
    assert.equal(mismatchedInputCustody.writes.length, 0);
    assert.equal(mismatchedInputCustody.baseChecked, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval rejects missing and tampered candidate bytes', async () => {
  const fixture = await createApprovalFixture();
  try {
    const missing = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      missingCandidate: true
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.promoted),
        approvalContext(),
        approvalDependencies(fixture, missing)
      ),
      (error) => error.code === 'trackb-candidate-missing' && error.retryable === false
    );
    assert.equal(missing.writes.length, 0);

    const tamperedCandidate = structuredClone(fixture.promoted.candidate);
    tamperedCandidate.candidateVerdict = 'ITERATE';
    const tampered = createApprovalCustodyDouble(fixture, {
      candidate: {
        candidate: tamperedCandidate,
        bytes: Buffer.from(canonicalizeJson(tamperedCandidate), 'utf8')
      }
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.promoted),
        approvalContext(),
        approvalDependencies(fixture, tampered)
      ),
      (error) => error.code === 'trackb-candidate-invalid' && error.retryable === false
    );
    assert.equal(tampered.writes.length, 0);
    assert.equal(tampered.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval rejects malformed custody responses before trusting or writing bytes', async () => {
  const cases = [
    {
      name: 'candidate response extra field',
      operation: 'read:candidate',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 0
    },
    {
      name: 'candidate response hash mismatch',
      operation: 'read:candidate',
      mutate: (result) => ({ ...result, sha256: `sha256:${'0'.repeat(64)}` }),
      expectedWrites: 0
    },
    {
      name: 'pointer response extra field',
      operation: 'snapshot:pointer',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 0
    },
    {
      name: 'approval response extra field',
      operation: 'snapshot:approval',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 0,
      existing: true
    },
    {
      name: 'approval write response hash mismatch',
      operation: 'write:approval',
      mutate: (result) => ({ ...result, snapshot: { ...result.snapshot, sha256: `sha256:${'0'.repeat(64)}` } }),
      expectedWrites: 1
    },
    {
      name: 'approval write response extra field',
      operation: 'write:approval',
      mutate: (result) => ({ ...result, extra: true }),
      expectedWrites: 1
    }
  ];
  const fixture = await createApprovalFixture();
  try {
    for (const testCase of cases) {
      const persistentApprovals = new Map();
      if (testCase.existing) {
        const seedCustody = createApprovalCustodyDouble(fixture, {
          candidate: fixture.promoted,
          persistentApprovals
        });
        await approveTrackBRelease(
          approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
          approvalContext(),
          approvalDependencies(fixture, seedCustody)
        );
      }
      const custody = createApprovalCustodyDouble(fixture, {
        candidate: fixture.promoted,
        persistentApprovals,
        responseMutation: { [testCase.operation]: testCase.mutate }
      });
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
          approvalContext(),
          approvalDependencies(fixture, custody)
        ),
        (error) => error.code === 'trackb-storage-integrity' && error.retryable === false,
        testCase.name
      );
      assert.equal(custody.writes.length, testCase.expectedWrites, `${testCase.name} write count`);
      assert.equal(custody.events.at(-1).op, 'close', `${testCase.name} close`);
      assert.equal(custody.pointerPublishCalled, false, `${testCase.name} pointer publish`);
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval reuses an identical receipt and conflicts on any immutable difference', async () => {
  const fixture = await createApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const input = approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' });
    const context = approvalContext();
    const firstCustody = createApprovalCustodyDouble(fixture, { candidate: fixture.promoted, persistentApprovals });
    const first = await approveTrackBRelease(input, context, approvalDependencies(fixture, firstCustody));
    assert.equal(first.reused, false);
    assert.equal(firstCustody.writes.length, 1);

    const secondCustody = createApprovalCustodyDouble(fixture, { candidate: fixture.promoted, persistentApprovals });
    const second = await approveTrackBRelease(input, context, approvalDependencies(fixture, secondCustody));
    assert.deepEqual(second, { ...first, reused: true });
    assert.equal(secondCustody.writes.length, 0);
    assert.equal(secondCustody.events.at(-2).op, 'snapshot:approval');

    const conflicts = [
      {
        input: { decision: 'REVISE' },
        context: {}
      },
      {
        input: {},
        context: { actorId: 'operator.other' }
      },
      {
        input: { reason: 'different reason' },
        context: {}
      },
      {
        input: { decidedAt: '2026-08-28T00:00:02Z' },
        context: {}
      }
    ];
    for (const variant of conflicts) {
      const custody = createApprovalCustodyDouble(fixture, { candidate: fixture.promoted, persistentApprovals });
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, fixture.promoted, variant.input),
          approvalContext(variant.context),
          approvalDependencies(fixture, custody)
        ),
        (error) => error.code === 'trackb-approval-conflict' && error.retryable === false
      );
      assert.equal(custody.writes.length, 0);
      assert.equal(custody.pointerPublishCalled, false);
      assert.equal(custody.events.at(-1).op, 'close');
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval classifies a valid existing wrong-subject receipt as stale without writing', async () => {
  const fixture = await createApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const seedCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
      approvalContext(),
      approvalDependencies(fixture, seedCustody)
    );
    const [approvalPath, originalBytes] = persistentApprovals.entries().next().value;
    const wrongSubject = JSON.parse(originalBytes.toString('utf8'));
    wrongSubject.subject.candidateHash = `sha256:${'0'.repeat(64)}`;
    wrongSubject.approvalHash = hashTrackBApprovalReceipt(wrongSubject);
    persistentApprovals.set(approvalPath, Buffer.from(canonicalizeJson(wrongSubject), 'utf8'));

    const custody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
        approvalContext(),
        approvalDependencies(fixture, custody)
      ),
      (error) => error.code === 'trackb-approval-stale' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.equal(custody.events.at(-1).op, 'close');
    assert.equal(custody.pointerPublishCalled, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval rejects spoofed or malformed Studio context and body context fields', async () => {
  const fixture = await createApprovalFixture();
  try {
    const invalidContexts = [
      undefined,
      { actorId: 'operator.demo', origin: 'http://127.0.0.1:4173', reviewSessionId: `sha256:${'6'.repeat(64)}` },
      { actorId: '../operator', origin: 'studio', reviewSessionId: `sha256:${'6'.repeat(64)}` },
      { actorId: 'operator.demo', origin: 'studio', reviewSessionId: 'not-a-hash' },
      { actorId: 'operator.demo', origin: 'studio', reviewSessionId: `sha256:${'6'.repeat(64)}`, extra: true }
    ];
    for (const context of invalidContexts) {
      let opened = false;
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, fixture.promoted),
          context,
          approvalDependencies(fixture, {
            openNarrativeSession: async () => {
              opened = true;
              throw new Error('context must be rejected before custody opens');
            }
          })
        ),
        (error) => error.code === 'trackb-approval-forbidden' && error.retryable === false
      );
      assert.equal(opened, false);
    }

    const bodyContext = approvalInput(fixture, fixture.promoted, { actorId: 'operator.demo' });
    await assert.rejects(
      approveTrackBRelease(
        bodyContext,
        approvalContext(),
        approvalDependencies(fixture, {
          openNarrativeSession: async () => { throw new Error('body context must be rejected before custody opens'); }
        })
      ),
      (error) => error.code === 'trackb-input-invalid' && error.retryable === false
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B durable approval closes custody on every typed failure and preserves primary close precedence', async () => {
  const fixture = await createApprovalFixture();
  try {
    for (const failureAt of ['read:candidate', 'snapshot:pointer', 'snapshot:approval', 'write:approval']) {
      const custody = createApprovalCustodyDouble(fixture, {
        candidate: fixture.promoted,
        failureAt
      });
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
          approvalContext(),
          approvalDependencies(fixture, custody)
        ),
        (error) => error.code === (failureAt === 'read:candidate' ? 'trackb-lock-busy' : 'trackb-storage-integrity')
          && error.retryable === (failureAt === 'read:candidate')
      );
      assert.equal(custody.events.at(-1).op, 'close', `custody must close after ${failureAt}`);
    }

    const closeAfterSuccess = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      closeFailure: new ApplicationCommandError('trackb-lock-busy', 'close lock failure', { retryable: true })
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
        approvalContext(),
        approvalDependencies(fixture, closeAfterSuccess)
      ),
      (error) => error.code === 'trackb-storage-integrity'
        && error.retryable === false
        && error.details?.closeCode === 'trackb-lock-busy'
    );

    const closeAfterPrimary = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      failureAt: 'read:candidate',
      closeFailure: new ApplicationCommandError('trackb-storage-integrity', 'close integrity failure')
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.promoted),
        approvalContext(),
        approvalDependencies(fixture, closeAfterPrimary)
      ),
      (error) => error.code === 'trackb-lock-busy'
        && error.retryable === true
        && error.details?.custodyClose?.code === 'trackb-storage-integrity'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B prepare resolves and binds a valid present pointer DeliverySet before writing', async () => {
  const fixture = await createApprovalFixture();
  try {
    const currentState = createCurrentDeliveryState(fixture, fixture.iterate);
    const custody = createCustodyDouble(fixture, { currentState });
    const request = {
      ...fixture.request,
      input: {
        ...fixture.request.input,
        expectedBaseDeliverySetHash: currentState.baseDeliverySetHash
      }
    };
    const result = await prepareTrackBRelease(request, {
      openProjectSession: custody.openProjectSession,
      workflowDeps: fixture.workflowDeps
    });
    assert.equal(result.baseDeliverySetHash, currentState.baseDeliverySetHash);
    assert.deepEqual(custody.events.map((event) => event.op), [
      'open',
      'read:narrativeAuthority',
      'read:projectionReceipt',
      'read:assetHandoff',
      'read:html',
      'read:provenance',
      'read:evidenceReceipt',
      'read:sceneGraph',
      'initialize',
      'snapshot:delivery-pointer-v2.json',
      'read:delivery-set',
      'write:authority',
      'write:candidate',
      'close'
    ]);
    const candidateWrite = custody.writes.find((write) => write.path.startsWith('candidates/'));
    assert.equal(JSON.parse(candidateWrite.bytes.toString('utf8')).baseDeliverySetHash, currentState.baseDeliverySetHash);
    assert.equal(custody.pointerPublishCalled, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B prepare rejects a valid present pointer when expected base is stale before immutable writes', async () => {
  const fixture = await createApprovalFixture();
  try {
    const currentState = createCurrentDeliveryState(fixture, fixture.iterate);
    const custody = createCustodyDouble(fixture, { currentState });
    await assert.rejects(
      prepareTrackBRelease(fixture.request, {
        openProjectSession: custody.openProjectSession,
        workflowDeps: fixture.workflowDeps
      }),
      (error) => error.code === 'trackb-cas-conflict' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.equal(custody.baseChecked, true);
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B current-base resolver fails closed for corrupt, unsafe, dangling, and cross-bound pointer state', async () => {
  const fixture = await createApprovalFixture();
  try {
    const baseState = createCurrentDeliveryState(fixture, fixture.iterate);
    const cases = [
      {
        name: 'pointer self-hash mismatch',
        state: {
          ...baseState,
          pointerBytes: Buffer.from(canonicalizeJson({ ...baseState.pointer, pointerHash: `sha256:${'0'.repeat(64)}` }), 'utf8')
        }
      },
      {
        name: 'pointer wrong prefix',
        state: pointerStateWith(baseState, { deliverySetPath: `other/${baseState.deliverySetPath}` })
      },
      {
        name: 'dangling delivery set',
        state: baseState,
        deliverySetMissing: true
      },
      {
        name: 'delivery set self-hash mismatch',
        state: {
          ...baseState,
          deliverySetBytes: Buffer.from(canonicalizeJson({ ...baseState.deliverySet, deliverySetHash: `sha256:${'0'.repeat(64)}` }), 'utf8')
        }
      },
      {
        name: 'pointer DeliverySet cross-binding mismatch',
        state: {
          ...baseState,
          pointerBytes: Buffer.from(canonicalizeJson({
            ...baseState.pointer,
            approvalHash: `sha256:${'f'.repeat(64)}`,
            pointerHash: hashTrackBDeliveryPointer({ ...baseState.pointer, approvalHash: `sha256:${'f'.repeat(64)}` })
          }), 'utf8')
        }
      }
    ];
    for (const testCase of cases) {
      const custody = createCustodyDouble(fixture, {
        currentState: testCase.state,
        deliverySetMissing: testCase.deliverySetMissing === true
      });
      await assert.rejects(
        prepareTrackBRelease({
          ...fixture.request,
          input: { ...fixture.request.input, expectedBaseDeliverySetHash: testCase.state.baseDeliverySetHash }
        }, {
          openProjectSession: custody.openProjectSession,
          workflowDeps: fixture.workflowDeps
        }),
        (error) => error.code === 'trackb-storage-integrity' && error.retryable === false,
        testCase.name
      );
      assert.equal(custody.writes.length, 0, `${testCase.name} must not write`);
      assert.equal(custody.events.at(-1).op, 'close', `${testCase.name} must close`);
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B approval resolves a valid present pointer DeliverySet and uses its current base', async () => {
  const fixture = await createPresentBaseApprovalFixture();
  try {
    const custody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      currentState: fixture.currentState
    });
    const result = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, {
        decision: 'APPROVE',
        baseDeliverySetHash: fixture.currentState.baseDeliverySetHash
      }),
      approvalContext(),
      approvalDependencies(fixture, custody)
    );
    assert.equal(result.subject.baseDeliverySetHash, fixture.currentState.baseDeliverySetHash);
    assert.equal(result.approvalPath, `approvals/${fixture.promoted.candidateHash.slice('sha256:'.length)}/${fixture.currentState.baseDeliverySetHash.slice('sha256:'.length)}/trackb-release-approval-receipt-v2.json`);
    assert.deepEqual(custody.events.map((event) => event.op), [
      'open',
      'read:candidate',
      'read:authority',
      'snapshot:delivery-pointer-v2.json',
      'read:delivery-set',
      'snapshot:approval',
      'write:approval',
      'close'
    ]);
    assert.equal(custody.writes.length, 1);
    assert.equal(custody.pointerPublishCalled, false);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B approval compares a present current base before immutable approval writes', async () => {
  const fixture = await createApprovalFixture();
  try {
    const currentState = createCurrentDeliveryState(fixture, fixture.iterate);
    const custody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.iterate,
      currentState
    });
    await assert.rejects(
      approveTrackBRelease(
        approvalInput(fixture, fixture.iterate),
        approvalContext(),
        approvalDependencies(fixture, custody)
      ),
      (error) => error.code === 'trackb-cas-conflict' && error.retryable === false
    );
    assert.equal(custody.writes.length, 0);
    assert.equal(custody.baseChecked, true);
    assert.equal(custody.pointerPublishCalled, false);
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B approval rejects corrupt present current-base state before immutable approval writes', async () => {
  const fixture = await createPresentBaseApprovalFixture();
  try {
    const baseState = fixture.currentState;
    const cases = [
      {
        name: 'pointer self-hash mismatch',
        state: {
          ...baseState,
          pointerBytes: Buffer.from(canonicalizeJson({ ...baseState.pointer, pointerHash: `sha256:${'0'.repeat(64)}` }), 'utf8')
        }
      },
      {
        name: 'pointer path traversal',
        state: pointerStateWith(baseState, { deliverySetPath: `${baseState.pointer.deliverySetPath}/../escape.json` })
      },
      {
        name: 'dangling delivery set',
        state: baseState,
        deliverySetMissing: true
      },
      {
        name: 'delivery set cross-binding mismatch',
        state: {
          ...baseState,
          deliverySetBytes: Buffer.from(canonicalizeJson({ ...baseState.deliverySet, revision: 2 }), 'utf8')
        }
      }
    ];
    for (const testCase of cases) {
      const custody = createApprovalCustodyDouble(fixture, {
        candidate: fixture.promoted,
        currentState: testCase.state,
        deliverySetMissing: testCase.deliverySetMissing === true
      });
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, fixture.promoted, {
            decision: 'APPROVE',
            baseDeliverySetHash: baseState.baseDeliverySetHash
          }),
          approvalContext(),
          approvalDependencies(fixture, custody)
        ),
        (error) => error.code === 'trackb-storage-integrity' && error.retryable === false,
        testCase.name
      );
      assert.equal(custody.writes.length, 0, `${testCase.name} must not write`);
      assert.equal(custody.events.at(-1).op, 'close', `${testCase.name} must close`);
      assert.equal(custody.pointerPublishCalled, false, `${testCase.name} pointer publish`);
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B approval binds the candidate to a canonical authority under the narrative lock', async () => {
  const fixture = await createApprovalFixture();
  try {
    const invalidAuthority = structuredClone(fixture.authority);
    invalidAuthority.story.theme = 'tampered authority';
    const cases = [
      {
        name: 'missing authority',
        options: { missingAuthority: true },
        expected: 'trackb-authority-missing'
      },
      {
        name: 'tampered authority self-hash',
        options: { authorityBytes: Buffer.from(canonicalizeJson(invalidAuthority), 'utf8') },
        expected: 'trackb-storage-integrity'
      },
      {
        name: 'candidate authority mismatch',
        candidate: (() => {
          const value = structuredClone(fixture.promoted.candidate);
          value.authorityHash = `sha256:${'e'.repeat(64)}`;
          value.candidateHash = hashTrackBReleaseCandidate(value);
          return { candidate: value, candidateBytes: Buffer.from(canonicalizeJson(value), 'utf8'), candidateHash: value.candidateHash };
        })(),
        expected: 'trackb-storage-integrity'
      }
    ];
    for (const testCase of cases) {
      const candidate = testCase.candidate ?? fixture.promoted;
      const custody = createApprovalCustodyDouble(fixture, {
        candidate,
        ...(testCase.options ?? {})
      });
      await assert.rejects(
        approveTrackBRelease(
          approvalInput(fixture, candidate),
          approvalContext(),
          approvalDependencies(fixture, custody)
        ),
        (error) => error.code === testCase.expected && error.retryable === false,
        testCase.name
      );
      assert.equal(custody.writes.length, 0, `${testCase.name} must not write`);
      assert.equal(custody.events.at(-1).op, 'close', `${testCase.name} must close`);
    }
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B promote stages a canonical DeliverySet, publishes the pointer last, and status reconciles PROMOTED', async () => {
  const fixture = await createApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const approvalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    const approval = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
      approvalContext(),
      approvalDependencies(fixture, approvalCustody)
    );
    const custody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    const result = await promoteTrackBRelease(
      promoteInput(fixture, fixture.promoted, approval),
      promoteDependencies(fixture, custody)
    );

    assert.deepEqual(Object.keys(result).sort(), [
      'deliverySetHash',
      'deliverySetPath',
      'meaningQualificationHash',
      'pointerHash',
      'pointerPath',
      'reused',
      'revision',
      'state'
    ]);
    assert.equal(result.state, 'PROMOTED');
    assert.equal(result.revision, 1);
    assert.equal(result.reused, false);
    assert.equal(result.deliverySetPath, `scenes/trackb/${fixture.authority.narrativeId}/delivery-sets/${result.deliverySetHash.slice('sha256:'.length)}/trackb-delivery-set-v2.json`);
    assert.equal(result.pointerPath, `scenes/trackb/${fixture.authority.narrativeId}/delivery-pointer-v2.json`);
    assert.deepEqual(custody.events.map((event) => event.op), [
      'open',
      'snapshot:delivery-pointer-v2.json',
      'read:candidate',
      'read:authority',
      'snapshot:approval',
      'write:delivery-set',
      'publish:pointer',
      'close'
    ]);
    assert.equal(custody.events.findIndex((event) => event.op === 'publish:pointer') > custody.events.findIndex((event) => event.op === 'write:delivery-set'), true);
    assert.equal(custody.pointerPublishCalled, true);
    assert.equal(custody.writes.length, 1);
    const deliveryWrite = custody.writes[0];
    assert.equal(deliveryWrite.path, `delivery-sets/${result.deliverySetHash.slice('sha256:'.length)}/trackb-delivery-set-v2.json`);
    assert.equal(deliveryWrite.bytes.toString('utf8'), canonicalizeJson(JSON.parse(deliveryWrite.bytes.toString('utf8'))));
    const deliverySet = JSON.parse(deliveryWrite.bytes.toString('utf8'));
    assert.equal(deliverySet.deliverySetHash, result.deliverySetHash);
    assert.equal(deliverySet.candidateHash, fixture.promoted.candidateHash);
    assert.equal(deliverySet.approvalHash, approval.approvalHash);
    assert.equal(deliverySet.meaningQualificationHash, fixture.promoted.candidate.meaningQualificationHash);
    assert.equal(hashTrackBDeliverySet(deliverySet), result.deliverySetHash);

    const statusCustody = custody;
    const status = await statusTrackBRelease(
      statusInput(fixture, fixture.promoted),
      statusDependencies(fixture, statusCustody)
    );
    assert.deepEqual(status.pointer, {
      state: 'PROMOTED',
      currentDeliverySetHash: result.deliverySetHash,
      candidateHash: fixture.promoted.candidateHash,
      approvalHash: approval.approvalHash,
      meaningQualificationHash: fixture.promoted.candidate.meaningQualificationHash,
      pointerRevision: 1,
      pointerHash: result.pointerHash
    });
    assert.deepEqual(status.candidateStatus, {
      state: 'PROMOTED',
      candidateHash: fixture.promoted.candidateHash,
      baseDeliverySetHash: 'NONE',
      meaningQualificationHash: fixture.promoted.candidate.meaningQualificationHash,
      candidateVerdict: 'PROMOTE',
      deliverySetHash: result.deliverySetHash,
      approvalHash: approval.approvalHash
    });
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B promote reuses the exact committed subject before checking a changed current base', async () => {
  const fixture = await createApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const approvalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    const approval = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
      approvalContext(),
      approvalDependencies(fixture, approvalCustody)
    );
    const custody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    const input = promoteInput(fixture, fixture.promoted, approval);
    const first = await promoteTrackBRelease(input, promoteDependencies(fixture, custody));
    const second = await promoteTrackBRelease(input, promoteDependencies(fixture, custody));
    assert.deepEqual(second, { ...first, reused: true });
    assert.equal(custody.writes.length, 1);
    assert.equal(custody.pointerPublishCount, 1);
    assert.equal(second.revision, first.revision);
    assert.equal(custody.events.filter((event) => event.op === 'write:delivery-set').length, 1);
    assert.equal(custody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B promote increments the verified present pointer revision and binds the new base exactly', async () => {
  const fixture = await createPresentBaseApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const approvalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      currentState: fixture.currentState,
      persistentApprovals
    });
    const approval = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, {
        decision: 'APPROVE',
        baseDeliverySetHash: fixture.currentState.baseDeliverySetHash
      }),
      approvalContext(),
      approvalDependencies(fixture, approvalCustody)
    );
    const custody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals,
      pointerState: fixture.currentState
    });
    const result = await promoteTrackBRelease(
      promoteInput(fixture, fixture.promoted, approval, {
        expectedBaseDeliverySetHash: fixture.currentState.baseDeliverySetHash
      }),
      promoteDependencies(fixture, custody)
    );
    assert.equal(result.revision, 2);
    const pointer = custody.pointerState.pointer;
    assert.equal(pointer.revision, 2);
    assert.equal(pointer.baseDeliverySetHash, fixture.currentState.baseDeliverySetHash);
    assert.equal(pointer.candidateHash, fixture.promoted.candidateHash);
    assert.equal(pointer.approvalHash, approval.approvalHash);
    assert.equal(custody.pointerPublishCalled, true);
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B promote fails closed for missing, stale, non-promotable, and conflicting subjects before pointer mutation', async () => {
  const fixture = await createApprovalFixture();
  try {
    const missingCustody = createPromotionCustodyDouble(fixture, { candidate: fixture.promoted });
    await assert.rejects(
      promoteTrackBRelease(
        promoteInput(fixture, fixture.promoted, { approvalHash: `sha256:${'a'.repeat(64) }` }),
        promoteDependencies(fixture, missingCustody)
      ),
      (error) => error.code === 'trackb-approval-required' && error.retryable === false
    );
    assert.equal(missingCustody.writes.length, 0);
    assert.equal(missingCustody.pointerPublishCalled, false);

    const approvalStorage = new Map();
    const approvalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals: approvalStorage
    });
    const approval = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
      approvalContext(),
      approvalDependencies(fixture, approvalCustody)
    );
    const staleCustody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals: approvalStorage
    });
    await assert.rejects(
      promoteTrackBRelease(
        promoteInput(fixture, fixture.promoted, { approvalHash: `sha256:${'b'.repeat(64) }` }),
        promoteDependencies(fixture, staleCustody)
      ),
      (error) => error.code === 'trackb-approval-stale' && error.retryable === false
    );
    assert.equal(staleCustody.writes.length, 0);

    const reviseStorage = new Map();
    const reviseApprovalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.iterate,
      persistentApprovals: reviseStorage
    });
    const revise = await approveTrackBRelease(
      approvalInput(fixture, fixture.iterate, { decision: 'REVISE' }),
      approvalContext(),
      approvalDependencies(fixture, reviseApprovalCustody)
    );
    const reviseCustody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.iterate,
      persistentApprovals: reviseStorage
    });
    await assert.rejects(
      promoteTrackBRelease(
        promoteInput(fixture, fixture.iterate, revise),
        promoteDependencies(fixture, reviseCustody)
      ),
      (error) => error.code === 'trackb-approval-not-promotable' && error.retryable === false
    );
    assert.equal(reviseCustody.writes.length, 0);
    assert.equal(reviseCustody.pointerPublishCalled, false);

    const currentState = createCurrentDeliveryState(fixture, fixture.iterate);
    const conflictCustody = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals: approvalStorage,
      pointerState: currentState
    });
    await assert.rejects(
      promoteTrackBRelease(
        promoteInput(fixture, fixture.promoted, approval),
        promoteDependencies(fixture, conflictCustody)
      ),
      (error) => error.code === 'trackb-cas-conflict' && error.retryable === false
    );
    assert.equal(conflictCustody.writes.length, 0);
    assert.equal(conflictCustody.pointerPublishCalled, false);
    assert.equal(conflictCustody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B status returns strict READY, APPROVED, REVISE, REJECT, ITERATE, FAIL, and ABSENT variants without writes', async () => {
  const fixture = await createApprovalFixture();
  try {
    const readyCustody = createPromotionCustodyDouble(fixture, { candidate: fixture.promoted });
    const ready = await statusTrackBRelease(statusInput(fixture, fixture.promoted), statusDependencies(fixture, readyCustody));
    assert.equal(ready.candidateStatus.state, 'READY');
    assert.equal(Object.hasOwn(ready.candidateStatus, 'approvalHash'), false);
    assert.equal(readyCustody.writes.length, 0);
    assert.equal(readyCustody.events.some((event) => event.op === 'read:delivery-set'), false);

    for (const decision of ['APPROVE', 'REVISE', 'REJECT']) {
      const approvals = new Map();
      const approvalCustody = createApprovalCustodyDouble(fixture, {
        candidate: fixture.promoted,
        persistentApprovals: approvals
      });
      const approval = await approveTrackBRelease(
        approvalInput(fixture, fixture.promoted, { decision }),
        approvalContext(),
        approvalDependencies(fixture, approvalCustody)
      );
      const statusCustody = createPromotionCustodyDouble(fixture, {
        candidate: fixture.promoted,
        persistentApprovals: approvals
      });
      const status = await statusTrackBRelease(statusInput(fixture, fixture.promoted), statusDependencies(fixture, statusCustody));
      const expectedState = decision === 'APPROVE' ? 'APPROVED' : decision;
      assert.equal(status.candidateStatus.state, expectedState);
      assert.equal(status.candidateStatus.approvalHash, approval.approvalHash);
      assert.equal(statusCustody.writes.length, 0);
      assert.equal(statusCustody.events.at(-1).op, 'close');
    }

    const iterateCustody = createPromotionCustodyDouble(fixture, { candidate: fixture.iterate });
    const iterate = await statusTrackBRelease(statusInput(fixture, fixture.iterate), statusDependencies(fixture, iterateCustody));
    assert.deepEqual(iterate.candidateStatus, {
      state: 'ITERATE',
      candidateHash: fixture.iterate.candidateHash,
      candidateVerdict: 'ITERATE'
    });
    assert.equal(Object.hasOwn(iterate.candidateStatus, 'baseDeliverySetHash'), false);

    const failedCandidate = makeFailedCandidate(fixture.iterate);
    const failCustody = createPromotionCustodyDouble(fixture, { candidate: failedCandidate });
    const failed = await statusTrackBRelease(statusInput(fixture, failedCandidate), statusDependencies(fixture, failCustody));
    assert.deepEqual(failed.candidateStatus, {
      state: 'FAIL',
      candidateHash: failedCandidate.candidateHash,
      candidateVerdict: 'FAIL'
    });

    const absentCustody = createPromotionCustodyDouble(fixture);
    const absent = await statusTrackBRelease(statusInput(fixture), statusDependencies(fixture, absentCustody));
    assert.deepEqual(absent, {
      narrativeId: fixture.authority.narrativeId,
      pointer: { state: 'ABSENT' }
    });
    assert.equal(absentCustody.events.some((event) => event.op === 'read:delivery-set'), false);
    assert.equal(absentCustody.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B promote preserves pre-publish pointer state, typed reconcile errors, and close precedence', async () => {
  const fixture = await createApprovalFixture();
  const persistentApprovals = new Map();
  try {
    const approvalCustody = createApprovalCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals
    });
    const approval = await approveTrackBRelease(
      approvalInput(fixture, fixture.promoted, { decision: 'APPROVE' }),
      approvalContext(),
      approvalDependencies(fixture, approvalCustody)
    );
    const writeFailure = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals,
      failureAt: 'write:delivery-set'
    });
    await assert.rejects(
      promoteTrackBRelease(promoteInput(fixture, fixture.promoted, approval), promoteDependencies(fixture, writeFailure)),
      (error) => error.code === 'trackb-storage-integrity' && error.retryable === false
    );
    assert.equal(writeFailure.pointerPublishCalled, false);
    assert.deepEqual(writeFailure.pointerBytes, null);

    const reconcileFailure = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals,
      failureAt: 'publish:pointer'
    });
    await assert.rejects(
      promoteTrackBRelease(promoteInput(fixture, fixture.promoted, approval), promoteDependencies(fixture, reconcileFailure)),
      (error) => error.code === 'trackb-pointer-reconcile-required' && error.retryable === false
    );
    assert.equal(reconcileFailure.pointerPublishCalled, true);
    assert.equal(reconcileFailure.pointerBytes, null);

    const closeFailure = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      persistentApprovals,
      closeFailure: new ApplicationCommandError('trackb-lock-busy', 'close lock failure', { retryable: true })
    });
    await assert.rejects(
      promoteTrackBRelease(promoteInput(fixture, fixture.promoted, approval), promoteDependencies(fixture, closeFailure)),
      (error) => error.code === 'trackb-storage-integrity'
        && error.retryable === false
        && error.details?.closeCode === 'trackb-lock-busy'
    );
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

test('Track B status validates every consumed custody snapshot and remains read-only', async () => {
  const fixture = await createApprovalFixture();
  try {
    const malformedPointer = createPromotionCustodyDouble(fixture, {
      responseMutation: { 'snapshot:pointer': (result) => ({ ...result, extra: true }) }
    });
    await assert.rejects(
      statusTrackBRelease(statusInput(fixture), statusDependencies(fixture, malformedPointer)),
      (error) => error.code === 'trackb-storage-integrity' && error.retryable === false
    );
    assert.equal(malformedPointer.writes.length, 0);
    assert.equal(malformedPointer.events.at(-1).op, 'close');

    const malformedCandidate = createPromotionCustodyDouble(fixture, {
      candidate: fixture.promoted,
      responseMutation: { 'read:candidate': (result) => ({ ...result, extra: true }) }
    });
    await assert.rejects(
      statusTrackBRelease(statusInput(fixture, fixture.promoted), statusDependencies(fixture, malformedCandidate)),
      (error) => error.code === 'trackb-storage-integrity' && error.retryable === false
    );
    assert.equal(malformedCandidate.writes.length, 0);
    assert.equal(malformedCandidate.events.at(-1).op, 'close');
  } finally {
    await rm(fixture.root, { recursive: true, force: true });
  }
});

async function createApprovalFixture() {
  const fixture = await createFixture();
  const loaded = loadTrackBReleaseInputsFromArtifacts({
    input: fixture.request.input,
    artifacts: {
      authorityBytes: fixture.authorityBytes,
      projectionReceiptBytes: fixture.projectionBytes,
      handoffBytes: fixture.handoffBytes,
      htmlBytes: fixture.htmlBytes,
      provenanceBytes: fixture.provenanceBytes,
      evidenceReceiptBytes: fixture.evidenceReceiptBytes,
      sceneGraphBytes: fixture.sceneGraphBytes
    },
    registry: fixture.workflowDeps.registry,
    artifactRegistries: fixture.workflowDeps.artifactRegistries
  });
  const built = buildTrackBReleaseCandidate({
    ...loaded,
    candidateId: fixture.request.input.candidateId,
    expectedBaseDeliverySetHash: 'NONE'
  });
  const iterateCandidate = structuredClone(built.candidate);
  for (const cell of iterateCandidate.evidenceCells) {
    cell.gateVerdict = 'ITERATE';
    cell.evidenceCellHash = hashTrackBEvidenceCell(cell);
  }
  iterateCandidate.candidateVerdict = 'ITERATE';
  iterateCandidate.candidateHash = hashTrackBReleaseCandidate(iterateCandidate);
  const iterate = {
    candidate: iterateCandidate,
    candidateBytes: Buffer.from(canonicalizeJson(iterateCandidate), 'utf8'),
    candidateHash: iterateCandidate.candidateHash
  };
  const promotedCandidate = structuredClone(built.candidate);
  for (const cell of promotedCandidate.evidenceCells) {
    cell.gateVerdict = 'PROMOTE';
    for (const meaning of cell.meaningCoverage) meaning.status = 'SUPPORTED';
    cell.evidenceCellHash = hashTrackBEvidenceCell(cell);
  }
  promotedCandidate.candidateVerdict = 'PROMOTE';
  promotedCandidate.candidateHash = hashTrackBReleaseCandidate(promotedCandidate);
  const promoted = {
    candidate: promotedCandidate,
    candidateBytes: Buffer.from(canonicalizeJson(promotedCandidate), 'utf8'),
    candidateHash: promotedCandidate.candidateHash
  };
  return { ...fixture, loaded, iterate, promoted };
}

async function createPresentBaseApprovalFixture() {
  const fixture = await createApprovalFixture();
  const currentState = createCurrentDeliveryState(fixture, fixture.iterate);
  const built = buildTrackBReleaseCandidate({
    ...fixture.loaded,
    candidateId: fixture.request.input.candidateId,
    expectedBaseDeliverySetHash: currentState.baseDeliverySetHash
  });
  const iterateCandidate = structuredClone(built.candidate);
  for (const cell of iterateCandidate.evidenceCells) {
    cell.gateVerdict = 'ITERATE';
    cell.evidenceCellHash = hashTrackBEvidenceCell(cell);
  }
  iterateCandidate.candidateVerdict = 'ITERATE';
  iterateCandidate.candidateHash = hashTrackBReleaseCandidate(iterateCandidate);
  const iterate = {
    candidate: iterateCandidate,
    candidateBytes: Buffer.from(canonicalizeJson(iterateCandidate), 'utf8'),
    candidateHash: iterateCandidate.candidateHash
  };
  const promotedCandidate = structuredClone(built.candidate);
  for (const cell of promotedCandidate.evidenceCells) {
    cell.gateVerdict = 'PROMOTE';
    for (const meaning of cell.meaningCoverage) meaning.status = 'SUPPORTED';
    cell.evidenceCellHash = hashTrackBEvidenceCell(cell);
  }
  promotedCandidate.candidateVerdict = 'PROMOTE';
  promotedCandidate.candidateHash = hashTrackBReleaseCandidate(promotedCandidate);
  const promoted = {
    candidate: promotedCandidate,
    candidateBytes: Buffer.from(canonicalizeJson(promotedCandidate), 'utf8'),
    candidateHash: promotedCandidate.candidateHash
  };
  return { ...fixture, currentState, iterate, promoted };
}

function createCurrentDeliveryState(fixture, candidateArtifact) {
  const candidate = candidateArtifact?.candidate ?? candidateArtifact;
  const approvalHash = `sha256:${'b'.repeat(64)}`;
  const deliverySet = {
    schemaVersion: 2,
    deliverySetId: 'delivery.runtime.v2',
    narrativeId: candidate.narrativeId,
    candidateHash: candidate.candidateHash,
    authorityHash: candidate.authorityHash,
    meaningQualificationHash: candidate.meaningQualificationHash,
    baseDeliverySetHash: 'NONE',
    approvalHash,
    artifacts: structuredClone(candidate.artifacts),
    evidenceCellHashes: candidate.evidenceCells.map((cell) => cell.evidenceCellHash),
    revision: 1,
    state: 'PROMOTED'
  };
  deliverySet.deliverySetHash = hashTrackBDeliverySet(deliverySet);
  const narrativePrefix = `scenes/trackb/${fixture.authority.narrativeId}/`;
  const deliverySetRelativePath = `delivery-sets/${deliverySet.deliverySetHash.slice('sha256:'.length)}/trackb-delivery-set-v2.json`;
  const pointer = {
    schemaVersion: 2,
    pointerKey: `pointer.${fixture.authority.narrativeId}`,
    narrativeId: fixture.authority.narrativeId,
    revision: deliverySet.revision,
    state: deliverySet.state,
    deliverySetHash: deliverySet.deliverySetHash,
    candidateHash: deliverySet.candidateHash,
    approvalHash: deliverySet.approvalHash,
    meaningQualificationHash: deliverySet.meaningQualificationHash,
    baseDeliverySetHash: deliverySet.baseDeliverySetHash,
    deliverySetPath: `${narrativePrefix}${deliverySetRelativePath}`
  };
  pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
  return {
    pointer,
    pointerBytes: Buffer.from(canonicalizeJson(pointer), 'utf8'),
    deliverySet,
    deliverySetBytes: Buffer.from(canonicalizeJson(deliverySet), 'utf8'),
    deliverySetRelativePath,
    baseDeliverySetHash: deliverySet.deliverySetHash
  };
}

function pointerStateWith(baseState, changes) {
  const pointer = { ...baseState.pointer, ...changes };
  pointer.pointerHash = hashTrackBDeliveryPointer(pointer);
  return {
    ...baseState,
    pointer,
    pointerBytes: Buffer.from(canonicalizeJson(pointer), 'utf8')
  };
}

function approvalInput(fixture, candidateArtifact, overrides = {}) {
  const candidate = candidateArtifact.candidate ?? candidateArtifact;
  return {
    narrativeId: fixture.authority.narrativeId,
    candidateHash: candidateArtifact.candidateHash ?? candidate.candidateHash,
    baseDeliverySetHash: candidate.baseDeliverySetHash,
    decision: 'REVISE',
    reason: 'reviewed by Studio',
    decidedAt: '2026-08-28T00:00:00Z',
    ...overrides
  };
}

function approvalContext(overrides = {}) {
  return {
    actorId: 'operator.demo',
    origin: 'studio',
    reviewSessionId: `sha256:${'6'.repeat(64)}`,
    ...overrides
  };
}

function approvalDependencies(fixture, custody, overrides = {}) {
  return {
    projectRoot: fixture.root,
    operationId: 'op.approval',
    openNarrativeSession: custody.openNarrativeSession,
    workflowDeps: { registry: fixture.workflowDeps.registry },
    now: Date.parse('2026-08-28T00:00:01Z'),
    ...overrides
  };
}

function createApprovalCustodyDouble(fixture, {
  candidate,
  persistentApprovals = new Map(),
  pointerPresent = false,
  currentState = null,
  missingCandidate = false,
  missingAuthority = false,
  deliverySetMissing = false,
  authorityBytes = null,
  failureAt = null,
  closeFailure = null,
  responseMutation = {}
} = {}) {
  const candidateValue = candidate?.candidate ?? candidate;
  const candidateHash = candidate?.candidateHash ?? candidateValue?.candidateHash;
  const candidateBytes = Buffer.from(
    candidate?.candidateBytes
      ?? candidate?.bytes
      ?? canonicalizeJson(candidateValue),
    candidate?.candidateBytes || candidate?.bytes ? undefined : 'utf8'
  );
  const candidatePath = `candidates/${candidateHash.slice('sha256:'.length)}/trackb-release-candidate-v4.json`;
  const authorityPath = `authority/${candidateValue.authorityHash.slice('sha256:'.length)}/trackb-narrative-authority-v2.json`;
  const authorityPayload = Buffer.from(authorityBytes ?? canonicalizeJson(fixture.authority), 'utf8');
  const events = [];
  const writes = [];
  let opened = false;
  let lockHeld = false;
  let closed = false;
  let baseChecked = false;
  let pointerPublishCalled = false;

  const session = {
    async read(path) {
      assert.equal(opened, true);
      assert.equal(closed, false);
      assert.equal(lockHeld, true);
      if (path === candidatePath) {
        events.push({ op: 'read:candidate', path });
        if (failureAt === 'read:candidate') {
          throw new ApplicationCommandError('trackb-lock-busy', 'test approval lock is busy', { retryable: true });
        }
        if (missingCandidate) return { path, exists: false, snapshotToken: 'NONE' };
        return mutateResponse('read:candidate', snapshotFor(path, candidateBytes, 'Y2FuZGlkYXRl', 2));
      }
      if (path === authorityPath) {
        events.push({ op: 'read:authority', path });
        if (failureAt === 'read:authority') {
          throw new ApplicationCommandError('trackb-storage-integrity', 'test authority read failure');
        }
        if (missingAuthority) return { path, exists: false, snapshotToken: 'NONE' };
        return mutateResponse('read:authority', snapshotFor(path, authorityPayload, 'YXV0aG9yaXR5', 3));
      }
      if (currentState && path === currentState.deliverySetRelativePath) {
        events.push({ op: 'read:delivery-set', path });
        if (failureAt === 'read:delivery-set') {
          throw new ApplicationCommandError('trackb-storage-integrity', 'test delivery set read failure');
        }
        if (deliverySetMissing) return { path, exists: false, snapshotToken: 'NONE' };
        baseChecked = true;
        return mutateResponse('read:delivery-set', snapshotFor(path, currentState.deliverySetBytes, 'ZGVsaXZlcnk', 4));
      }
      assert.fail(`unexpected narrative custody read path: ${path}`);
    },

    async snapshot(path) {
      assert.equal(opened, true);
      assert.equal(closed, false);
      assert.equal(lockHeld, true);
      events.push({ op: path === 'delivery-pointer-v2.json' ? 'snapshot:delivery-pointer-v2.json' : 'snapshot:approval', path });
      if (path === 'delivery-pointer-v2.json') {
        if (failureAt === 'snapshot:pointer') {
          throw new ApplicationCommandError('trackb-storage-integrity', 'test pointer snapshot failure');
        }
        if (currentState) {
          return mutateResponse('snapshot:pointer', snapshotFor(path, currentState.pointerBytes, 'cG9pbnRlcg', 5));
        }
        if (pointerPresent) {
          return mutateResponse('snapshot:pointer', snapshotFor(path, Buffer.from('{}'), 'cG9pbnRlcg', 5));
        }
        baseChecked = true;
        return mutateResponse('snapshot:pointer', { path, exists: false, snapshotToken: 'NONE' });
      }
      assert.equal(baseChecked, true, 'approval snapshot requires a prior base check');
      if (failureAt === 'snapshot:approval') {
        throw new ApplicationCommandError('trackb-storage-integrity', 'test approval snapshot failure');
      }
      const existing = persistentApprovals.get(path);
      if (existing === undefined) return mutateResponse('snapshot:approval', { path, exists: false, snapshotToken: 'NONE' });
      return mutateResponse('snapshot:approval', snapshotFor(path, existing, 'ZXhpc3Rpbmc', 4));
    },

    async writeImmutable(path, bytes) {
      assert.equal(opened, true);
      assert.equal(closed, false);
      assert.equal(lockHeld, true);
      assert.equal(baseChecked, true, 'approval write requires a prior base check');
      assert.equal(path.startsWith('approvals/'), true);
      events.push({ op: 'write:approval', path, bytes: Buffer.from(bytes) });
      if (failureAt === 'write:approval') {
        throw new ApplicationCommandError('trackb-storage-integrity', 'test approval write failure');
      }
      const payload = Buffer.from(bytes);
      const previous = persistentApprovals.get(path);
      if (previous !== undefined) {
        throw new ApplicationCommandError('trackb-storage-integrity', 'unexpected overwrite of immutable approval');
      }
      persistentApprovals.set(path, payload);
      writes.push({ path, bytes: payload, reused: false });
      return mutateResponse('write:approval', {
        path,
        reused: false,
        snapshot: {
          exists: true,
          snapshotToken: 'bmV3',
          bytesBase64: payload.toString('base64url'),
          byteLength: payload.length,
          sha256: hashTrackBRawBytes(payload),
          device: 1,
          inode: 6
        }
      });
    },

    async publishPointer() {
      pointerPublishCalled = true;
      throw new ApplicationCommandError('trackb-storage-integrity', 'approval must never publish the delivery pointer');
    },

    async close() {
      if (closed) return { closed: true };
      events.push({ op: 'close' });
      closed = true;
      lockHeld = false;
      if (closeFailure) throw closeFailure;
      return { closed: true };
    }
  };

  function mutateResponse(operation, result) {
    const mutate = responseMutation?.[operation];
    return typeof mutate === 'function' ? mutate(result) : result;
  }

  return {
    openNarrativeSession: async ({ projectRoot, narrativeId, readOnly, operationId }) => {
      assert.equal(projectRoot, fixture.root);
      assert.equal(narrativeId, fixture.authority.narrativeId);
      assert.equal(readOnly, false);
      assert.equal(operationId, 'op.approval');
      events.push({ op: 'open', projectRoot, narrativeId, readOnly, operationId });
      opened = true;
      lockHeld = true;
      return session;
    },
    events,
    writes,
    get baseChecked() { return baseChecked; },
    get pointerPublishCalled() { return pointerPublishCalled; }
  };
}

function promoteInput(fixture, candidateArtifact, approval, overrides = {}) {
  const candidate = candidateArtifact.candidate ?? candidateArtifact;
  return {
    narrativeId: fixture.authority.narrativeId,
    candidateHash: candidateArtifact.candidateHash ?? candidate.candidateHash,
    approvalHash: approval.approvalHash,
    expectedBaseDeliverySetHash: candidate.baseDeliverySetHash,
    ...overrides
  };
}

function statusInput(fixture, candidateArtifact, overrides = {}) {
  const candidate = candidateArtifact?.candidate ?? candidateArtifact;
  return {
    narrativeId: fixture.authority.narrativeId,
    ...(candidate ? { candidateHash: candidateArtifact.candidateHash ?? candidate.candidateHash } : {}),
    ...overrides
  };
}

function promoteDependencies(fixture, custody, overrides = {}) {
  return {
    projectRoot: fixture.root,
    operationId: custody.operationId ?? 'op.promote',
    openNarrativeSession: custody.openNarrativeSession,
    workflowDeps: { registry: fixture.workflowDeps.registry },
    ...overrides
  };
}

function statusDependencies(fixture, custody, overrides = {}) {
  return {
    projectRoot: fixture.root,
    operationId: custody.operationId ?? 'op.status',
    openNarrativeSession: custody.openNarrativeSession,
    workflowDeps: { registry: fixture.workflowDeps.registry },
    ...overrides
  };
}

function makeFailedCandidate(candidateArtifact) {
  const candidate = structuredClone(candidateArtifact.candidate ?? candidateArtifact);
  candidate.evidenceCells[0].runtimeEvidence.pass = false;
  candidate.evidenceCells[0].evidenceCellHash = hashTrackBEvidenceCell(candidate.evidenceCells[0]);
  candidate.candidateVerdict = 'FAIL';
  candidate.candidateHash = hashTrackBReleaseCandidate(candidate);
  return {
    candidate,
    candidateBytes: Buffer.from(canonicalizeJson(candidate), 'utf8'),
    candidateHash: candidate.candidateHash
  };
}

function createPromotionCustodyDouble(fixture, {
  candidate,
  persistentApprovals = new Map(),
  pointerState = null,
  persistentDeliverySets = new Map(),
  missingCandidate = false,
  missingAuthority = false,
  deliverySetMissing = false,
  authorityBytes = null,
  failureAt = null,
  responseMutation = {},
  closeFailure = null,
  operationId = 'op.promote'
} = {}) {
  const candidateValue = candidate?.candidate ?? candidate ?? fixture.promoted.candidate;
  const candidateHash = candidate?.candidateHash ?? candidateValue.candidateHash;
  const candidatePayload = Buffer.from(
    candidate?.candidateBytes
      ?? candidate?.bytes
      ?? canonicalizeJson(candidateValue),
    candidate?.candidateBytes || candidate?.bytes ? undefined : 'utf8'
  );
  const candidatePath = `candidates/${candidateHash.slice('sha256:'.length)}/trackb-release-candidate-v4.json`;
  const authorityPath = `authority/${candidateValue.authorityHash.slice('sha256:'.length)}/trackb-narrative-authority-v2.json`;
  const authorityPayload = Buffer.from(authorityBytes ?? canonicalizeJson(fixture.authority), 'utf8');
  const deliverySets = persistentDeliverySets;
  let currentPointerState = pointerState;
  if (currentPointerState?.deliverySetRelativePath && currentPointerState.deliverySetBytes && !deliverySetMissing) {
    deliverySets.set(currentPointerState.deliverySetRelativePath, Buffer.from(currentPointerState.deliverySetBytes));
  }
  const events = [];
  const writes = [];
  let pointerPublishCount = 0;

  function mutateResponse(operation, result) {
    const mutate = responseMutation?.[operation];
    return typeof mutate === 'function' ? mutate(result) : result;
  }

  function sessionFor(readOnly) {
    let opened = true;
    let closed = false;
    const lockHeld = readOnly === false;
    const assertSession = ({ requireMutation = false } = {}) => {
      assert.equal(opened, true);
      assert.equal(closed, false);
      if (requireMutation) {
        assert.equal(readOnly, false);
        assert.equal(lockHeld, true);
      }
    };
    return {
      async read(path) {
        assertSession();
        if (path === candidatePath) {
          events.push({ op: 'read:candidate', path });
          if (failureAt === 'read:candidate') throw new ApplicationCommandError('trackb-lock-busy', 'test candidate lock busy', { retryable: true });
          if (missingCandidate) return { path, exists: false, snapshotToken: 'NONE' };
          return mutateResponse('read:candidate', snapshotFor(path, candidatePayload, 'Y2FuZGlkYXRl', 2));
        }
        if (path === authorityPath) {
          events.push({ op: 'read:authority', path });
          if (failureAt === 'read:authority') throw new ApplicationCommandError('trackb-storage-integrity', 'test authority read failure');
          if (missingAuthority) return { path, exists: false, snapshotToken: 'NONE' };
          return mutateResponse('read:authority', snapshotFor(path, authorityPayload, 'YXV0aG9yaXR5', 3));
        }
        if (path.startsWith('approvals/')) {
          events.push({ op: 'read:approval', path });
          if (failureAt === 'read:approval') throw new ApplicationCommandError('trackb-storage-integrity', 'test approval read failure');
          const payload = persistentApprovals.get(path);
          if (payload === undefined) return mutateResponse('read:approval', { path, exists: false, snapshotToken: 'NONE' });
          return mutateResponse('read:approval', snapshotFor(path, payload, 'YXBwcm92YWw', 4));
        }
        if (path.startsWith('delivery-sets/')) {
          events.push({ op: 'read:delivery-set', path });
          if (failureAt === 'read:delivery-set') throw new ApplicationCommandError('trackb-storage-integrity', 'test delivery set read failure');
          const payload = deliverySets.get(path);
          if (payload === undefined) return mutateResponse('read:delivery-set', { path, exists: false, snapshotToken: 'NONE' });
          return mutateResponse('read:delivery-set', snapshotFor(path, payload, 'ZGVsaXZlcnktc2V0', 5));
        }
        assert.fail(`unexpected promote/status custody read path: ${path}`);
      },

      async snapshot(path) {
        assertSession();
        if (path === 'delivery-pointer-v2.json') {
          events.push({ op: 'snapshot:delivery-pointer-v2.json', path });
          if (failureAt === 'snapshot:pointer') throw new ApplicationCommandError('trackb-storage-integrity', 'test pointer snapshot failure');
          if (!currentPointerState) return mutateResponse('snapshot:pointer', { path, exists: false, snapshotToken: 'NONE' });
          return mutateResponse('snapshot:pointer', snapshotFor(path, currentPointerState.pointerBytes, 'cG9pbnRlci1jdXJyZW50', 6));
        }
        if (path.startsWith('approvals/')) {
          events.push({ op: 'snapshot:approval', path });
          if (failureAt === 'snapshot:approval') throw new ApplicationCommandError('trackb-storage-integrity', 'test approval snapshot failure');
          const payload = persistentApprovals.get(path);
          if (payload === undefined) return mutateResponse('snapshot:approval', { path, exists: false, snapshotToken: 'NONE' });
          return mutateResponse('snapshot:approval', snapshotFor(path, payload, 'YXBwcm92YWwtc2V0', 7));
        }
        assert.fail(`unexpected promote/status custody snapshot path: ${path}`);
      },

      async writeImmutable(path, bytes) {
        assertSession({ requireMutation: true });
        assert.equal(path.startsWith('delivery-sets/'), true);
        events.push({ op: 'write:delivery-set', path, bytes: Buffer.from(bytes) });
        if (failureAt === 'write:delivery-set') throw new ApplicationCommandError('trackb-storage-integrity', 'test DeliverySet write failure');
        const payload = Buffer.from(bytes);
        const previous = deliverySets.get(path);
        if (previous !== undefined && !previous.equals(payload)) throw new ApplicationCommandError('trackb-storage-integrity', 'test immutable DeliverySet mismatch');
        const reused = previous !== undefined;
        if (!reused) deliverySets.set(path, payload);
        writes.push({ path, bytes: payload, reused });
        return mutateResponse('write:delivery-set', {
          path,
          reused,
          snapshot: writeSnapshotFor(payload, reused ? 'ZGVsaXZlcnktcmV1c2Vk' : 'ZGVsaXZyeS1uZXc', reused ? 8 : 9)
        });
      },

      async publishPointer(options) {
        assertSession({ requireMutation: true });
        events.push({ op: 'publish:pointer', options: { ...options, pointerBytes: Buffer.from(options.pointerBytes), deliverySetBytes: Buffer.from(options.deliverySetBytes) } });
        pointerPublishCount += 1;
        if (failureAt === 'publish:pointer') throw new ApplicationCommandError('trackb-pointer-reconcile-required', 'test pointer durability is unknown');
        const pointerPayload = Buffer.from(options.pointerBytes);
        const deliveryPayload = Buffer.from(options.deliverySetBytes);
        const parsedPointer = JSON.parse(pointerPayload.toString('utf8'));
        const parsedDeliverySet = JSON.parse(deliveryPayload.toString('utf8'));
        const previous = currentPointerState?.pointerBytes;
        const reused = previous !== undefined && previous.equals(pointerPayload);
        const narrativePrefix = `scenes/trackb/${fixture.authority.narrativeId}/`;
        const deliverySetRelativePath = options.deliverySetPath.startsWith(narrativePrefix)
          ? options.deliverySetPath.slice(narrativePrefix.length)
          : options.deliverySetPath;
        currentPointerState = {
          pointer: parsedPointer,
          pointerBytes: pointerPayload,
          deliverySet: parsedDeliverySet,
          deliverySetBytes: deliveryPayload,
          deliverySetRelativePath
        };
        deliverySets.set(deliverySetRelativePath, deliveryPayload);
        return mutateResponse('publish:pointer', {
          path: 'delivery-pointer-v2.json',
          reused,
          snapshot: writeSnapshotFor(pointerPayload, 'cHVibGlzaGVkLXBvaW50ZXI', 10)
        });
      },

      async close() {
        if (closed) return { closed: true };
        events.push({ op: 'close' });
        closed = true;
        opened = false;
        if (closeFailure) throw closeFailure;
        return { closed: true };
      }
    };
  }

  return {
    operationId,
    openNarrativeSession: async ({ projectRoot, narrativeId, readOnly, operationId: requestedOperationId }) => {
      assert.equal(projectRoot, fixture.root);
      assert.equal(narrativeId, fixture.authority.narrativeId);
      assert.equal(typeof readOnly, 'boolean');
      assert.equal(requestedOperationId, operationId);
      events.push({ op: 'open', projectRoot, narrativeId, readOnly, operationId: requestedOperationId });
      return sessionFor(readOnly);
    },
    events,
    writes,
    get pointerPublishCalled() { return pointerPublishCount > 0; },
    get pointerPublishCount() { return pointerPublishCount; },
    get pointerBytes() { return currentPointerState?.pointerBytes ?? null; },
    get pointerState() { return currentPointerState; }
  };
}

function snapshotFor(path, bytes, snapshotToken, inode) {
  const payload = Buffer.from(bytes);
  return {
    path,
    exists: true,
    snapshotToken,
    bytesBase64: payload.toString('base64url'),
    byteLength: payload.length,
    sha256: hashTrackBRawBytes(payload),
    device: 1,
    inode
  };
}

function writeSnapshotFor(bytes, snapshotToken, inode) {
  const payload = Buffer.from(bytes);
  return {
    exists: true,
    snapshotToken,
    bytesBase64: payload.toString('base64url'),
    byteLength: payload.length,
    sha256: hashTrackBRawBytes(payload),
    device: 1,
    inode
  };
}

async function createFixture() {
  const root = await mkdtemp(join(tmpdir(), 'smc-trackb-runtime-'));
  const [authorityExample, sceneGraphExample, handoffExample] = await Promise.all([
    readExample('trackb/trackb-narrative-authority.example.json'),
    readExample('scene-graph.example.json'),
    readExample('scene-motion-asset-handoff-v2.example.json')
  ]);
  const authority = structuredClone(authorityExample);
  const { sceneGraph, projection } = singleSceneProjectionFixture(authority.narrativeId);
  const sceneGraphHash = canonicalHash(sceneGraph);
  const projectionBytes = Buffer.from(`${JSON.stringify(projection)}\n`, 'utf8');
  const projectionHash = hashTrackBRawBytes(projectionBytes);
  const handoff = structuredClone(handoffExample);
  handoff.projectionReceiptSha256 = projectionHash.slice('sha256:'.length);
  handoff.sceneGraphHash = sceneGraphHash;
  handoff.sourceSemanticManifestHash = sceneGraph.semanticManifest.sourceSemanticManifestHash;
  handoff.semanticManifestHash = sceneGraph.semanticManifest.semanticManifestHash;
  const handoffBytes = Buffer.from(`${JSON.stringify(handoff)}\n`, 'utf8');
  const htmlBytes = Buffer.from(composeSceneHtml({ sceneGraph }), 'utf8');
  const htmlHash = hashTrackBRawBytes(htmlBytes);
  authority.sourceRefs.projectionReceiptHash = projectionHash;
  authority.sourceRefs.sceneGraphHash = sceneGraphHash;
  authority.sourceRefs.nvcReleaseBundleHash = projection.bundleSha256;
  authority.sourceRefs.narrativePackageHash = `sha256:${projection.packageSha256}`;
  authority.sourceRefs.narrativeSceneMapHash = `sha256:${projection.sceneMapSha256}`;
  authority.sourceRefs.sourceSemanticManifestHash = sceneGraph.semanticManifest.sourceSemanticManifestHash;
  authority.sourceRefs.semanticManifestHash = sceneGraph.semanticManifest.semanticManifestHash;
  authority.meaningItems = sceneGraph.semanticManifest.entries.map((entry, index) => ({
    meaningId: `meaning.${entry.kind}.${String(index + 1).padStart(3, '0')}`,
    semanticRef: { semanticId: entry.semanticId, entryHash: entry.entryHash },
    shotId: entry.shotId,
    kind: entry.kind,
    sourceText: entry.sourceText
  }));
  authority.authorityHash = hashTrackBAuthority(authority);
  const assetManifest = { schemaVersion: 1, assetManifestId: 'manifest.runtime', assets: [] };
  const provenance = {
    schemaVersion: 2,
    narrativeId: authority.narrativeId,
    projectionReceiptHash: projectionHash,
    sceneGraphHash,
    sourceSemanticManifestHash: sceneGraph.semanticManifest.sourceSemanticManifestHash,
    semanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
    htmlHash,
    embedding: {
      anchor: 'trackb-semantic-manifest',
      encoding: 'base64url-rfc4648',
      payloadHash: hashTrackBRawBytes(Buffer.from(canonicalizeJson(sceneGraph.semanticManifest), 'utf8'))
    },
    handoffHash: hashTrackBRawBytes(handoffBytes),
    assetManifestHash: canonicalHash(assetManifest)
  };
  provenance.provenanceHash = canonicalHash(provenance);
  const authorityBytes = Buffer.from(`${JSON.stringify(authority)}\n`, 'utf8');
  const provenanceBytes = Buffer.from(`${JSON.stringify(provenance)}\n`, 'utf8');
  const sceneGraphBytes = Buffer.from(`${JSON.stringify(sceneGraph)}\n`, 'utf8');
  const assetManifestHash = canonicalHash(assetManifest);
  const evidenceReceipt = {
    schemaVersion: 2,
    receiptId: 'receipt.runtime.v2',
    narrativeId: authority.narrativeId,
    authorityHash: authority.authorityHash,
    htmlHash,
    assetManifestHash,
    semanticManifestHash: sceneGraph.semanticManifest.semanticManifestHash,
    rendererLockHash: `sha256:${'4'.repeat(64)}`,
    navigationGuard: {
      mechanism: 'cdp-binding-nonce',
      guardSourceHash: 'sha256:922cd3832fe5988739a4be38f2d4c15bde7a672fa92b7301ee212548a3bd1099'
    },
    shotIds: authority.shots.map((shot) => shot.shotId),
    shots: authority.shots.map((shot, ordinal) => ({
      shotId: shot.shotId,
      ordinal,
      screenshotHash: `sha256:${String(5 + ordinal).repeat(64)}`,
      runtime: {
        networkRequestCount: 0,
        externalRequestCount: 0,
        navigationAttemptCount: 0,
        exceptionCount: 0,
        semanticManifestVerified: true,
        pass: true
      },
      visual: {
        activeShotCount: 1,
        visibleShotCount: 1,
        activeShotId: shot.shotId,
        visibleShotId: shot.shotId,
        stageGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        shotGeometry: { x: 0, y: 0, width: 1280, height: 720 },
        pass: true
      }
    }))
  };
  evidenceReceipt.receiptHash = hashTrackBMeasuredEvidenceReceipt(evidenceReceipt);
  const evidenceReceiptBytes = Buffer.from(canonicalizeJson(evidenceReceipt), 'utf8');
  const [registry, artifactRegistries] = await Promise.all([
    loadTrackBContractRegistry(),
    loadTrackBArtifactContractRegistries()
  ]);
  return {
    root,
    authority,
    projection,
    projectionBytes,
    authorityBytes,
    handoffBytes,
    htmlBytes,
    provenanceBytes,
    sceneGraphBytes,
    evidenceReceipt,
    evidenceReceiptBytes,
    workflowDeps: { registry, artifactRegistries },
    request: {
      projectRoot: root,
      input: {
        narrativeAuthorityPath: 'authority.json',
        projectionReceiptPath: TRACK_B_PROJECTION_RECEIPT_PATH,
        assetHandoffPath: 'handoff/scene-motion-asset-handoff.v2.json',
        htmlPath: 'out/scene.html',
        provenancePath: 'out/scene.provenance.json',
        evidenceReceiptPath: 'out/scene.evidence-receipt.json',
        candidateId: 'candidate.runtime.v2',
        expectedBaseDeliverySetHash: 'NONE'
      }
    }
  };
}

function createCustodyDouble(fixture, {
  failureAt = null,
  pointerPresent = false,
  currentState = null,
  deliverySetMissing = false,
  persistentWrites = new Map(),
  responseMutation = {},
  closeFailure = null
} = {}) {
  const events = [];
  const writes = [];
  let readIndex = 0;
  let initialized = false;
  let lockHeld = false;
  let closed = false;
  let baseChecked = false;
  let pointerPublishCalled = false;
  const expectedReads = [
    ['narrativeAuthority', fixture.request.input.narrativeAuthorityPath, fixture.authorityBytes],
    ['projectionReceipt', fixture.request.input.projectionReceiptPath, fixture.projectionBytes],
    ['assetHandoff', fixture.request.input.assetHandoffPath, fixture.handoffBytes],
    ['html', fixture.request.input.htmlPath, fixture.htmlBytes],
    ['provenance', fixture.request.input.provenancePath, fixture.provenanceBytes],
    ['evidenceReceipt', fixture.request.input.evidenceReceiptPath, fixture.evidenceReceiptBytes],
    ['sceneGraph', fixture.projection.sceneGraphPath, fixture.sceneGraphBytes]
  ];

  const session = {
    async readProjectArtifact({ kind, path, projectionReceiptHash }) {
      assert.equal(closed, false);
      const [expectedKind, expectedPath, bytes] = expectedReads[readIndex] ?? [];
      events.push({ op: `read:${kind}`, kind, path, projectionReceiptHash });
      if (`read:${kind}` === failureAt) {
        throw new ApplicationCommandError('trackb-lock-busy', 'test custody lock is busy', { retryable: true });
      }
      assert.equal(kind, expectedKind);
      assert.equal(path, expectedPath);
      if (kind === 'sceneGraph') assert.equal(projectionReceiptHash, hashTrackBRawBytes(fixture.projectionBytes));
      readIndex += 1;
      return mutateResponse('read:' + kind, {
        kind,
        path,
        exists: true,
        bytesBase64: bytes.toString('base64url'),
        byteLength: bytes.length,
        sha256: hashTrackBRawBytes(bytes)
      });
    },

    async initializeNarrative({ narrativeId, readOnly }) {
      assert.equal(closed, false);
      events.push({ op: 'initialize', narrativeId, readOnly });
      assert.equal(readIndex, expectedReads.length, 'initialize must follow all seven reads');
      assert.equal(narrativeId, fixture.authority.narrativeId);
      assert.equal(readOnly, false);
      if (failureAt === 'initialize') throw new ApplicationCommandError('trackb-storage-integrity', 'test initialize failure');
      initialized = true;
      lockHeld = true;
      return mutateResponse('initialize', { protocolVersion: 1, narrativeId, mode: 'mutable', namespacePresent: true });
    },

    async read(path) {
      assert.equal(closed, false);
      assert.equal(initialized, true);
      assert.equal(lockHeld, true);
      assert.ok(currentState, 'narrative read is only expected for a present current pointer');
      assert.equal(path, currentState.deliverySetRelativePath);
      events.push({ op: 'read:delivery-set', path });
      if (failureAt === 'read:delivery-set') {
        throw new ApplicationCommandError('trackb-storage-integrity', 'test delivery set read failure');
      }
      if (deliverySetMissing) return { path, exists: false, snapshotToken: 'NONE' };
      baseChecked = true;
      return mutateResponse('read:delivery-set', snapshotFor(path, currentState.deliverySetBytes, 'ZGVsaXZlcnk', 4));
    },

    async snapshot(path) {
      assert.equal(closed, false);
      assert.equal(initialized, true);
      assert.equal(lockHeld, true);
      events.push({ op: `snapshot:${path}`, path });
      if (failureAt === 'snapshot') throw new ApplicationCommandError('trackb-storage-integrity', 'test snapshot failure');
      if (currentState) {
        return mutateResponse('snapshot', snapshotFor(path, currentState.pointerBytes, 'cG9pbnRlcg', 3));
      }
      if (pointerPresent) {
        return mutateResponse('snapshot', {
          path,
          exists: true,
          snapshotToken: 'cHJlc2VudA',
          bytesBase64: Buffer.from('{}').toString('base64url'),
          byteLength: 2,
          sha256: hashTrackBRawBytes(Buffer.from('{}')),
          device: 1,
          inode: 2
        });
      }
      assert.equal(path, 'delivery-pointer-v2.json');
      baseChecked = true;
      return mutateResponse('snapshot', { path, exists: false, snapshotToken: 'NONE' });
    },

    async writeImmutable(path, bytes) {
      assert.equal(closed, false);
      assert.equal(initialized, true);
      assert.equal(lockHeld, true);
      assert.equal(baseChecked, true, 'immutable writes require a prior base check');
      const payload = Buffer.from(bytes);
      const kind = path.startsWith('authority/') ? 'authority' : path.startsWith('candidates/') ? 'candidate' : 'unknown';
      events.push({ op: `write:${kind}`, path, bytes: payload });
      if (failureAt === `write:${kind}`) throw new ApplicationCommandError('trackb-storage-integrity', `test ${kind} write failure`);
      const previous = persistentWrites.get(path);
      if (previous && !previous.equals(payload)) throw new ApplicationCommandError('trackb-storage-integrity', `test immutable mismatch for ${path}`);
      const reused = previous !== undefined;
      if (!previous) persistentWrites.set(path, payload);
      writes.push({ path, bytes: payload, reused });
      return mutateResponse(`write:${kind}`, {
        path,
        reused,
        snapshot: {
          exists: true,
          bytesBase64: payload.toString('base64url'),
          byteLength: payload.length,
          sha256: hashTrackBRawBytes(payload),
          snapshotToken: reused ? 'cmV1c2Vk' : 'bmV3',
          device: 1,
          inode: 3
        }
      });
    },

    async close() {
      if (closed) return { closed: true };
      events.push({ op: 'close' });
      closed = true;
      lockHeld = false;
      if (closeFailure) throw closeFailure;
      return { closed: true };
    }
  };

  function mutateResponse(operation, result) {
    const mutate = responseMutation?.[operation];
    return typeof mutate === 'function' ? mutate(result) : result;
  }

  return {
    openProjectSession: async ({ projectRoot }) => {
      assert.equal(projectRoot, fixture.root);
      events.push({ op: 'open', projectRoot });
      return session;
    },
    events,
    writes,
    get baseChecked() { return baseChecked; },
    get pointerPublishCalled() { return pointerPublishCalled; }
  };
}

async function readExample(path) {
  return JSON.parse(await readFile(join(EXAMPLE_ROOT, path), 'utf8'));
}
