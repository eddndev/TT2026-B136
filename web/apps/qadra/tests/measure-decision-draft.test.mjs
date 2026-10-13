import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import {
  captureMeasureDecisionDraft,
  createMeasureDecisionDraft,
  pendingMeasureDecisionDrafts,
} from '../src/lib/measure-decision-draft.mjs';
import {
  preparedDecision,
  measureCaseId as caseId,
  clone,
} from './fixtures/measure-decision-workflow.mjs';
import { hearingRecord } from './fixtures/hearings.mjs';
import { deferred } from './fixtures/draft-registry.mjs';

const otherId = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
function state(mode = 'uncertain', family = 'g1') {
  const last = preparedDecision({ family }),
    command = last.review.command;
  return {
    decisionId: command.decision_id,
    operationId: command.operation_id,
    fields: {
      authority: command.values.authority,
      justification: command.values.justification,
      locator: command.values.locator,
      declaredAt: clone(command.values.declared_at),
      outcome: command.outcome.kind,
      statement: '',
    },
    support: clone(last.review.material.support),
    anchor: null,
    effects: [
      {
        key: 'e1000000-0000-4000-8000-000000000001',
        action: 'impose',
        previous: null,
        proposal: {
          id: last.review.results[0].id,
          subject: null,
          supervisor: null,
          values: clone(last.review.results[0].values),
        },
        predecessors: [],
        successors: [],
      },
    ],
    inputs: { declaredAt: { date: '', clock: '', offset: '' }, subjectPicker: null },
    mode,
    last,
  };
}

function setup(value = state()) {
  const registry = createDraftRegistry(),
    calls = [];
  let principal = clone(value.last?.review.actor ?? state().last.review.actor),
    allowed = true;
  registry.activate(principal.id);
  const session = {
    registry,
    principal: () => principal,
    canAdmit: () => allowed,
    authorizeCase: async () => {
      calls.push('case');
      return { id: caseId, administration: { case_id: caseId, administrative_status: 'active' } };
    },
  };
  const controller = (overrides = {}) =>
    createMeasureDecisionDraft({
      session,
      caseId,
      decisionId: value.decisionId,
      capture: () => captureMeasureDecisionDraft(value),
      ...overrides,
    });
  const first = controller();
  first.register();
  return {
    value,
    registry,
    calls,
    session,
    first,
    controller,
    changePrincipal: (change) => {
      principal = { ...principal, ...change };
    },
    setAllowed: (next) => {
      allowed = next;
    },
    suspend() {
      const report = registry.suspend();
      first.dispose();
      registry.activate(principal.id);
      assert.equal(report.failed.length, 0);
      return registry.pending()[0];
    },
  };
}

test('capture retains incomplete times and effect rows without converting NaN or undefined', () => {
  const value = state('draft');
  value.last = null;
  value.support = null;
  value.fields.authority = '  Texto pendiente  ';
  value.fields.declaredAt = {
    precision: 'second',
    year: undefined,
    month: 10,
    day: undefined,
    hour: 9,
    minute: undefined,
    second: NaN,
    offset_seconds: NaN,
  };
  value.effects[0].proposal.values.validity.start = clone(value.fields.declaredAt);
  value.effects[0].proposal.values.kind = '';
  value.inputs.declaredAt = { date: '2026-', clock: '09:', offset: '-0' };
  value.anchor = { kind: 'initial', record: hearingRecord() };
  const captured = captureMeasureDecisionDraft({ ...value, busy: true, approved: true });
  assert.deepEqual(captured, value);
  assert.equal(Object.hasOwn(captured.fields.declaredAt, 'year'), true);
  assert.equal(captured.fields.declaredAt.year, undefined);
  assert.equal(Number.isNaN(captured.fields.declaredAt.offset_seconds), true);
  value.inputs.declaredAt.offset = 'Mutated';
  value.effects[0].proposal.values.conditions = 'Mutated';
  assert.equal(captured.inputs.declaredAt.offset, '-0');
  assert.notEqual(captured.effects[0].proposal.values.conditions, 'Mutated');
  value.effects = Array.from({ length: 32 }, (_, index) => ({
    ...clone(captured.effects[0]),
    key: `e1000000-0000-4000-8000-${String(index + 1).padStart(12, '0')}`,
  }));
  assert.equal(captureMeasureDecisionDraft(value).effects.length, 32);
  value.effects.push({ ...clone(captured.effects[0]), key: otherId });
  assert.throws(() => captureMeasureDecisionDraft(value));
});

test('capture preserves exact G1 or G2 uncertainty and invalidates review approval', () => {
  for (const family of ['g1', 'g2']) {
    const value = state('uncertain', family),
      captured = captureMeasureDecisionDraft(value);
    assert.deepEqual(captured.last, value.last);
    assert.equal(captured.last.family, family);
    assert.equal(captured.last.review.submission_digest, value.last.review.submission_digest);
    assert.equal(captured.last.review.review_digest, value.last.review.review_digest);
    value.last.review.command.values.authority = 'Mutated';
    assert.notEqual(captured.last.review.command.values.authority, 'Mutated');
  }
  assert.equal(captureMeasureDecisionDraft(state('review')).mode, 'draft');
  for (const alter of [
    (value) => {
      value.last = null;
    },
    (value) => {
      value.decisionId = otherId;
    },
    (value) => {
      value.operationId = otherId;
    },
  ]) {
    const value = state();
    alter(value);
    assert.throws(() => captureMeasureDecisionDraft(value));
  }
});

test('restoration reauthorizes the case before sources and preserves the full decision snapshot', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  assert.equal(saved.editorKind, 'measure-decision');
  assert.equal(saved.resourceId, s.value.decisionId);
  assert.deepEqual(pendingMeasureDecisionDrafts(s.session, caseId), [saved]);
  let restored;
  const result = await controller.restore(
    saved,
    async () => {
      assert.equal(restored, undefined);
      s.calls.push('sources');
      return { marker: 'fresh' };
    },
    (value, fresh) => {
      s.calls.push('apply');
      restored = value;
      assert.deepEqual(fresh, { marker: 'fresh', closed: false });
    },
  );
  assert.equal(result.status, 'restored');
  assert.deepEqual(s.calls, ['case', 'sources', 'apply']);
  assert.deepEqual(restored, s.value);
  assert.equal(s.registry.pending().length, 0);
});

test('principal, case, decision and in-flight admission changes cannot reveal retained values', async () => {
  for (const change of [
    { id: otherId },
    { email: 'changed@example.test' },
    { role: 'litigator' },
  ]) {
    const s = setup(),
      saved = s.suspend();
    s.changePrincipal(change);
    assert.deepEqual(pendingMeasureDecisionDrafts(s.session, caseId), []);
    const result = await s.controller().restore(
      saved,
      async () => assert.fail('Foreign principal must not read retained sources.'),
      () => assert.fail('Foreign principal must not see retained values.'),
    );
    assert.notEqual(result.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const override of [{ caseId: otherId }, { decisionId: otherId }]) {
    const s = setup(),
      saved = s.suspend();
    assert.deepEqual(pendingMeasureDecisionDrafts(s.session, otherId), []);
    const result = await s.controller(override).restore(
      saved,
      async () => assert.fail('Another editor must not read retained sources.'),
      () => assert.fail('Another editor must not see retained values.'),
    );
    assert.notEqual(result.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const change of ['role', 'admission', 'dispose']) {
    const s = setup(),
      saved = s.suspend(),
      controller = s.controller(),
      gate = deferred();
    s.session.authorizeCase = () => gate.promise;
    const pending = controller.restore(
      saved,
      async () => assert.fail('Stale admission must not read sources.'),
      () => assert.fail('Stale admission must not apply values.'),
    );
    if (change === 'role') s.changePrincipal({ role: 'paralegal' });
    else if (change === 'admission') s.setAllowed(false);
    else controller.dispose();
    gate.resolve({
      id: caseId,
      administration: { case_id: caseId, administrative_status: 'active' },
    });
    assert.notEqual((await pending).status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('failed source refresh preserves exact uncertainty and closed reentry does not approve it', async () => {
  const s = setup(state('uncertain', 'g2')),
    saved = s.suspend(),
    controller = s.controller();
  const failure = Object.assign(new Error('Missing selected source'), { status: 404 });
  await assert.rejects(
    controller.restore(
      saved,
      async () => {
        throw failure;
      },
      () => {
        assert.fail('Missing source must not expose retained values.');
      },
    ),
    (error) => error === failure,
  );
  assert.equal(s.registry.pending().length, 1);
  s.session.authorizeCase = async () => ({
    id: caseId,
    administration: { case_id: caseId, administrative_status: 'closed' },
  });
  let restored;
  const result = await controller.restore(
    saved,
    async () => ({ closed: false }),
    (value, fresh) => {
      restored = value;
      assert.equal(fresh.closed, true);
    },
  );
  assert.equal(result.status, 'restored');
  assert.equal(restored.mode, 'uncertain');
  assert.deepEqual(restored.last, s.value.last);
  assert.equal(restored.operationId, s.value.operationId);
});

test('navigation disposal captures the draft and explicit close prevents later recapture', async () => {
  const s = setup(state('draft'));
  s.value.last = null;
  s.value.fields.declaredAt = { precision: 'minute', hour: undefined, offset_seconds: NaN };
  s.first.dispose();
  const saved = s.registry.pending()[0];
  assert.ok(saved);
  const controller = s.controller();
  let restored;
  assert.equal(
    (
      await controller.restore(
        saved,
        async () => ({}),
        (value) => {
          restored = value;
        },
      )
    ).status,
    'restored',
  );
  assert.deepEqual(restored, s.value);
  controller.close();
  controller.dispose();
  assert.deepEqual(s.registry.pending(), []);
  const cancelled = setup();
  cancelled.first.close();
  cancelled.first.dispose();
  assert.deepEqual(cancelled.registry.pending(), []);
});
