import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import {
  capturePrecautionaryHearingDraft,
  createPrecautionaryHearingDraft,
  pendingPrecautionaryHearingDrafts,
} from '../src/lib/precautionary-hearing-draft.mjs';
import {
  clone,
  precautionaryCaseId as caseId,
  precautionaryHearingOperation,
} from './fixtures/precautionary-hearing-unit.mjs';
import { deferred } from './fixtures/draft-registry.mjs';

const otherId = 'f0000000-0000-4000-8000-000000000099';
function state(action = 'replace', mode = 'uncertain') {
  const revision = { schedule: 1, replace: 2, cancel: 3 }[action],
    last = precautionaryHearingOperation({ revision }).capture.review,
    values = last.resolved_values;
  return {
    hearingId: last.command.hearing_id,
    operationId: last.command.operation_id,
    action,
    base: revision === 1 ? null : precautionaryHearingOperation({ revision: revision - 1 }),
    fields: {
      purpose: values.purpose,
      date: values.scheduled_at.slice(0, 10),
      time: values.scheduled_at.slice(11, 19),
      offset: values.scheduled_at.slice(19),
      modality: values.modality,
      venue: values.venue,
      note: values.note,
      statement: values.scheduling_basis.statement,
      locator: values.scheduling_basis.locator,
      reason: last.command.change.reason ?? '',
    },
    participants: clone(last.sources.participants),
    support: clone(last.sources.support),
    reviewTargets: [],
    inputs: {
      choosingParticipant: true,
      participantPicker: { name: 'Busqueda pendiente', query: 'Testigo' },
      supportPicker: null,
      reviewTargetPicker: null,
    },
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
    createPrecautionaryHearingDraft({
      session,
      caseId,
      action: value.action,
      hearingId: value.hearingId,
      capture: () => capturePrecautionaryHearingDraft(value),
      ...overrides,
    });
  const first = controller(),
    baseRevision = value.base?.capture.review.result_revision ?? 0;
  first.register(baseRevision);
  return {
    value,
    registry,
    session,
    calls,
    first,
    controller,
    baseRevision,
    changePrincipal: (changes) => {
      principal = { ...principal, ...changes };
    },
    setAllowed: (next) => {
      allowed = next;
    },
    suspend() {
      const captured = registry.suspend();
      first.dispose();
      registry.activate(principal.id);
      assert.equal(captured.failed.length, 0);
      return registry.pending()[0];
    },
  };
}

test('capture retains incomplete inputs and exact uncertain submission without approval', () => {
  const original = state(),
    captured = capturePrecautionaryHearingDraft({ ...original, approved: true, busy: true });
  assert.deepEqual(captured, original);
  original.last.command.change.values.venue = 'Mutated';
  original.fields.venue = 'Mutated';
  original.inputs.participantPicker.name = 'Mutated';
  assert.notEqual(
    captured.last.command.change.values.venue,
    original.last.command.change.values.venue,
  );
  assert.notEqual(captured.fields.venue, original.fields.venue);
  assert.notEqual(captured.inputs.participantPicker.name, original.inputs.participantPicker.name);
  assert.equal(capturePrecautionaryHearingDraft(state('schedule', 'review')).mode, 'draft');
  const incomplete = state('schedule', 'draft');
  incomplete.last = null;
  incomplete.fields = Object.fromEntries(Object.keys(incomplete.fields).map((key) => [key, '']));
  incomplete.participants = [];
  incomplete.support = null;
  assert.deepEqual(capturePrecautionaryHearingDraft(incomplete), incomplete);
  for (const alter of [
    (s) => {
      s.last = null;
    },
    (s) => {
      s.operationId = otherId;
    },
    (s) => {
      s.hearingId = otherId;
    },
    (s) => {
      s.action = 'cancel';
    },
    (s) => {
      s.last.command.change.expected_revision = 2;
    },
    (s) => {
      s.last.command.change.expected_capture_digest = '0'.repeat(64);
    },
  ]) {
    const invalid = state();
    alter(invalid);
    assert.throws(() => capturePrecautionaryHearingDraft(invalid));
  }
});

test('schedule, replacement and cancellation restore their own base after current authorization', async () => {
  for (const action of ['schedule', 'replace', 'cancel']) {
    const s = setup(state(action)),
      saved = s.suspend(),
      controller = s.controller();
    assert.equal(saved.editorKind, 'precautionary-hearing');
    assert.equal(saved.resourceId, s.value.hearingId);
    assert.equal(saved.action, action);
    assert.equal(saved.baseRevision, s.baseRevision);
    assert.deepEqual(pendingPrecautionaryHearingDrafts(s.session, caseId), [saved]);
    let applied;
    const context = { head: clone(s.value.base), marker: 'fresh' };
    const outcome = await controller.restore(
      saved,
      async () => {
        assert.equal(applied, undefined);
        s.calls.push('references');
        return context;
      },
      (value, fresh) => {
        s.calls.push('apply');
        applied = { value, fresh };
      },
    );
    assert.equal(outcome.status, 'restored');
    assert.deepEqual(s.calls, ['case', 'references', 'apply']);
    assert.deepEqual(applied.value, s.value);
    assert.deepEqual(applied.fresh, { ...context, closed: false });
    assert.equal(s.registry.pending().length, 0);
  }
  const s = setup(state('schedule', 'review')),
    saved = s.suspend();
  await s.controller().restore(
    saved,
    async () => ({}),
    (value) => {
      assert.equal(value.mode, 'draft');
    },
  );
});

test('restoration and pending lists are bound to complete principal, case, action and hearing', async () => {
  for (const change of [
    { id: otherId },
    { email: 'changed@example.test' },
    { role: 'litigator' },
  ]) {
    const s = setup(),
      saved = s.suspend();
    s.changePrincipal(change);
    assert.deepEqual(pendingPrecautionaryHearingDrafts(s.session, caseId), []);
    const outcome = await s.controller().restore(
      saved,
      async () => {
        assert.fail('Foreign principal must not read retained references.');
      },
      () => assert.fail('Foreign principal must not see retained values.'),
    );
    assert.notEqual(outcome.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const override of [{ caseId: otherId }, { action: 'cancel' }, { hearingId: otherId }]) {
    const s = setup(),
      saved = s.suspend();
    assert.deepEqual(pendingPrecautionaryHearingDrafts(s.session, otherId), []);
    const outcome = await s.controller(override).restore(
      saved,
      async () => {
        assert.fail('Another editor must not read retained references.');
      },
      () => assert.fail('Another editor must not see retained values.'),
    );
    assert.notEqual(outcome.status, 'restored');
    assert.deepEqual(s.calls, []);
  }
  for (const role of ['paralegal', 'client']) {
    const s = setup();
    s.changePrincipal({ role });
    const controller = s.controller();
    assert.equal(controller.admitted(), false);
    assert.throws(() => controller.register(s.baseRevision));
  }
});

test('principal, admission or disposal changes while authorizing cannot restore stale values', async () => {
  for (const mutation of ['email', 'role', 'dispose', 'admission']) {
    const s = setup(),
      saved = s.suspend(),
      controller = s.controller(),
      gate = deferred();
    s.session.authorizeCase = () => gate.promise;
    const pending = controller.restore(
      saved,
      async () => {
        assert.fail('Stale authorization must not refresh references.');
      },
      () => assert.fail('Stale authorization must not apply values.'),
    );
    if (mutation === 'dispose') controller.dispose();
    else if (mutation === 'admission') s.setAllowed(false);
    else s.changePrincipal({ [mutation]: mutation === 'role' ? 'litigator' : 'new@example.test' });
    gate.resolve({
      id: caseId,
      administration: { case_id: caseId, administrative_status: 'active' },
    });
    assert.notEqual((await pending).status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('failed reference refresh preserves uncertain command and both confirmations for closed reentry', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  const failure = Object.assign(new Error('Missing exact reference'), {
    status: 404,
    code: 'precautionary_hearing_not_found',
  });
  await assert.rejects(
    controller.restore(
      saved,
      async () => {
        throw failure;
      },
      () => {
        assert.fail('Missing reference must not apply retained values.');
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
  const outcome = await controller.restore(
    saved,
    async () => ({ closed: false }),
    (value, fresh) => {
      restored = value;
      assert.equal(fresh.closed, true);
    },
  );
  assert.equal(outcome.status, 'restored');
  assert.equal(restored.mode, 'uncertain');
  assert.deepEqual(restored.last, s.value.last);
  assert.equal(restored.last.submission_digest, s.value.last.submission_digest);
  assert.equal(restored.last.review_digest, s.value.last.review_digest);
  assert.equal(restored.operationId, s.value.operationId);
});

test('explicit close removes the editor snapshot while dispose preserves its recovery', () => {
  const s = setup(state('schedule'));
  s.registry.suspend();
  s.registry.activate(s.session.principal().id);
  assert.equal(s.registry.pending().length, 1);
  s.first.close();
  assert.equal(s.registry.pending().length, 0);
  const another = setup(state('cancel'));
  another.suspend();
  assert.equal(another.registry.pending().length, 1);
});
