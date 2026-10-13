import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import {
  captureMeasureAdministrationDraft,
  createMeasureAdministrationDraft,
  pendingMeasureAdministrationDrafts,
} from '../src/lib/measure-administration-draft.mjs';
import {
  preparedAdministration,
  measureCaseId as caseId,
  clone,
} from './fixtures/measure-administrations.mjs';
import { measureRecord } from './fixtures/measure-records.mjs';
import { deferred } from './fixtures/draft-registry.mjs';

const actions = ['correct', 'entered_in_error', 'replace_entered_in_error'];
const otherId = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
function state(action = 'correct', mode = 'uncertain') {
  const last = preparedAdministration({ action }),
    base = measureRecord(),
    values = last.command.action.values ?? base.record.capture.result.values;
  return {
    operationId: last.command.operation_id,
    action,
    replacementId: last.command.action.replacement_id ?? null,
    base,
    fields: {
      reason: last.command.reason,
      conditions: values.conditions,
      validity: clone(values.validity),
      supervisionText: values.supervision_text ?? values.supervision.reason,
    },
    subject: clone(last.replacement?.sources.subject ?? null),
    inputs: { start: { time: { date: '', clock: '', offset: '' } } },
    mode,
    last,
  };
}
function setup(value = state()) {
  const registry = createDraftRegistry(),
    calls = [];
  let principal = clone(value.last?.actor ?? state().last.actor),
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
    createMeasureAdministrationDraft({
      session,
      caseId,
      measureId: value.base.reference.id,
      action: value.action,
      capture: () => captureMeasureAdministrationDraft(value),
      ...overrides,
    });
  const first = controller();
  first.register(value.base.reference.revision);
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

test('administrative capture clones partial declared values and raw inputs without normalization', () => {
  const value = state('correct', 'draft');
  value.last = null;
  value.fields.reason = '  Motivo pendiente  ';
  value.fields.validity.start = {
    precision: 'second',
    year: undefined,
    month: 10,
    day: undefined,
    hour: 9,
    minute: undefined,
    second: NaN,
    offset_seconds: NaN,
  };
  value.inputs.start.time = { date: '2026-', clock: '09:', offset: '-0' };
  const captured = captureMeasureAdministrationDraft({ ...value, busy: true, approved: true });
  assert.deepEqual(captured, value);
  assert.equal(Object.hasOwn(captured.fields.validity.start, 'year'), true);
  assert.equal(captured.fields.validity.start.year, undefined);
  assert.equal(Number.isNaN(captured.fields.validity.start.offset_seconds), true);
  value.inputs.start.time.offset = 'Mutated';
  value.base.record.capture.result.values.conditions = 'Mutated';
  assert.equal(captured.inputs.start.time.offset, '-0');
  assert.notEqual(captured.base.record.capture.result.values.conditions, 'Mutated');
  assert.equal(captureMeasureAdministrationDraft(state('correct', 'review')).mode, 'draft');
});

test('each administrative uncertainty retains the original review bound to its exact target and action', () => {
  for (const action of actions) {
    const value = state(action),
      captured = captureMeasureAdministrationDraft(value);
    assert.deepEqual(captured.last, value.last);
    assert.deepEqual(captured.last.command.target, captured.base.reference);
    assert.equal(captured.last.command.action.kind, action);
    assert.equal(captured.replacementId, captured.last.command.action.replacement_id ?? null);
    assert.equal(captured.last.submission_digest, value.last.submission_digest);
    assert.equal(captured.last.review_digest, value.last.review_digest);
    value.last.command.reason = 'Mutated';
    assert.notEqual(captured.last.command.reason, 'Mutated');
  }
  for (const alter of [
    (value) => {
      value.last = null;
    },
    (value) => {
      value.operationId = otherId;
    },
    (value) => {
      value.action = 'entered_in_error';
    },
    (value) => {
      value.base = measureRecord({ family: 'm2' });
    },
  ]) {
    const value = state();
    alter(value);
    assert.throws(() => captureMeasureAdministrationDraft(value));
  }
  for (const replacementId of [null, otherId]) {
    const value = state('replace_entered_in_error');
    value.replacementId = replacementId;
    assert.throws(() => captureMeasureAdministrationDraft(value));
  }
  const correction = state();
  correction.replacementId = otherId;
  assert.throws(() => captureMeasureAdministrationDraft(correction));
});

test('restoration authorizes the case and fresh sources before exposing each full administrative draft', async () => {
  for (const action of actions) {
    const s = setup(state(action)),
      saved = s.suspend(),
      controller = s.controller();
    assert.equal(saved.editorKind, 'measure-administration');
    assert.equal(saved.resourceId, s.value.base.reference.id);
    assert.equal(saved.baseRevision, s.value.base.reference.revision);
    assert.equal(saved.action, action);
    assert.deepEqual(pendingMeasureAdministrationDrafts(s.session, caseId), [saved]);
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
  }
});

test('foreign principal, case, measure, action or stale admission cannot expose administrative drafts', async () => {
  for (const change of [
    { id: otherId },
    { email: 'changed@example.test' },
    { role: 'litigator' },
  ]) {
    const s = setup(),
      saved = s.suspend();
    s.changePrincipal(change);
    assert.deepEqual(pendingMeasureAdministrationDrafts(s.session, caseId), []);
    const result = await s.controller().restore(
      saved,
      async () => assert.fail('Foreign principal must not read sources.'),
      () => assert.fail('Foreign principal must not see retained values.'),
    );
    assert.notEqual(result.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const override of [
    { caseId: otherId },
    { measureId: otherId },
    { action: 'entered_in_error' },
  ]) {
    const s = setup(),
      saved = s.suspend();
    const result = await s.controller(override).restore(
      saved,
      async () => assert.fail('Another editor must not read retained sources.'),
      () => assert.fail('Another editor must not see retained values.'),
    );
    assert.notEqual(result.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const change of ['principal', 'admission', 'dispose']) {
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
    if (change === 'principal') s.changePrincipal({ role: 'paralegal' });
    else if (change === 'admission') s.setAllowed(false);
    else controller.dispose();
    gate.resolve({
      id: caseId,
      administration: { case_id: caseId, administrative_status: 'active' },
    });
    assert.notEqual((await pending).status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
  for (const alter of [
    (value) => {
      value.last.actor.email = 'another@example.test';
    },
    (value) => {
      value.subject.case_id = otherId;
    },
  ]) {
    const s = setup(state('replace_entered_in_error'));
    alter(s.value);
    assert.equal(s.registry.suspend().failed.length, 1);
  }
});

test('failed fresh reads retain uncertainty and a closed case allows only restoration of the original send', async () => {
  const s = setup(state('replace_entered_in_error')),
    saved = s.suspend(),
    controller = s.controller();
  const failure = Object.assign(new Error('Source unavailable'), { status: 503 });
  await assert.rejects(
    controller.restore(
      saved,
      async () => {
        throw failure;
      },
      () => assert.fail('Unavailable sources must not expose retained values.'),
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
  assert.deepEqual(restored.base, s.value.base);
  assert.equal(restored.operationId, s.value.operationId);
  assert.equal(restored.replacementId, s.value.replacementId);
});

test('navigation disposal captures partial drafts and explicit close prevents later recapture', async () => {
  const value = state('correct', 'draft');
  value.last = null;
  value.fields.validity.start = { precision: 'minute', hour: undefined, offset_seconds: NaN };
  const s = setup(value);
  s.first.dispose();
  const saved = s.registry.pending()[0],
    controller = s.controller();
  assert.ok(saved);
  let restored;
  const result = await controller.restore(
    saved,
    async () => ({}),
    (next) => {
      restored = next;
    },
  );
  assert.equal(result.status, 'restored');
  assert.deepEqual(restored, value);
  controller.close();
  controller.dispose();
  assert.deepEqual(s.registry.pending(), []);
  const cancelled = setup();
  cancelled.first.close();
  cancelled.first.dispose();
  assert.deepEqual(cancelled.registry.pending(), []);
});
