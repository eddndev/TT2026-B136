import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftRegistry } from '../src/lib/draft-registry.mjs';
import {
  captureResourceHearingDraft,
  createResourceHearingDraft,
  pendingResourceHearingDrafts,
} from '../src/lib/resource-hearing-draft.mjs';
import {
  clone,
  resourceHearingCreation,
  resourceHearingPrepared,
} from './fixtures/resource-hearing-unit.mjs';
import {
  resourceRecord,
  resourcePrepared,
  resourceCommandFixture,
} from './fixtures/procedural-resource-unit.mjs';

function state(mode = 'uncertain') {
  const last = resourceHearingPrepared(resourceHearingCreation({ withAct: true }));
  const command = resourceCommandFixture('correct');
  command.change.expected_revision = 4;
  const base = resourceRecord(resourcePrepared(command));
  base.receipt.capture_digest = '9'.repeat(64);
  return {
    hearingId: last.command.hearing_id,
    associationId: last.command.association_id,
    operationId: last.command.operation_id,
    base,
    resource: clone(last.resource),
    act: clone(last.act),
    fields: {
      kind: 'appeal_arguments',
      date: '2026-01-01',
      time: '18:00:00',
      offset: '-06:00',
      modality: 'in_person',
      venue: last.command.values.venue,
      note: last.command.values.note,
      statement: last.command.values.scheduling_basis.statement,
      supportKey: 'act-support',
    },
    participants: clone(last.sources.participants),
    mode,
    last,
    inputs: {
      choosingAct: false,
      choosingParticipant: true,
      actPicker: null,
      participantPicker: { name: 'Busqueda pendiente', query: 'Testigo' },
    },
  };
}
function setup(value = state()) {
  const registry = createDraftRegistry();
  let principal = { ...value.last.recorded_by, role: 'owner' },
    allowed = true;
  registry.activate(principal.id);
  const calls = [];
  const caseId = value.resource.case_id,
    resourceId = value.resource.id;
  const session = {
    registry,
    principal: () => principal,
    canAdmit: () => allowed,
    authorizeCase: async () => {
      calls.push('case');
      return { id: caseId, administration: { case_id: caseId, administrative_status: 'active' } };
    },
  };
  const controller = () =>
    createResourceHearingDraft({
      session,
      caseId,
      resourceId,
      capture: () => captureResourceHearingDraft(value),
    });
  const first = controller();
  first.register(value.base.revision);
  return {
    value,
    registry,
    session,
    calls,
    first,
    controller,
    caseId,
    resourceId,
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

test('capture retains exact uncertain command and inputs without preserving approval', () => {
  const original = state(),
    captured = captureResourceHearingDraft({ ...original, approved: true, busy: true });
  assert.deepEqual(captured, original);
  original.last.command.values.venue = 'Mutated';
  original.fields.venue = 'Mutated';
  assert.notEqual(captured.last.command.values.venue, original.last.command.values.venue);
  assert.notEqual(captured.fields.venue, original.fields.venue);
  assert.equal(captureResourceHearingDraft(state('review')).mode, 'draft');
  const incomplete = state('draft');
  incomplete.last = null;
  incomplete.fields.venue = '';
  assert.deepEqual(captureResourceHearingDraft(incomplete), incomplete);
  const invalid = state();
  invalid.last = null;
  assert.throws(() => captureResourceHearingDraft(invalid));
});

test('restoration reauthorizes case before fresh references and preserves historical selection', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  assert.deepEqual(pendingResourceHearingDrafts(s.session, s.caseId, s.resourceId), [saved]);
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
  assert.equal(applied.value.resource.revision, 1);
  assert.equal(applied.value.act.revision, 4);
  assert.equal(applied.value.base.revision, 5);
  assert.equal(s.registry.pending().length, 0);
});

test('reentry with changed id, email or role cannot expose the former principal draft', async () => {
  for (const change of [
    { email: 'changed@example.test' },
    { role: 'litigator' },
    { id: 'f0000000-0000-4000-8000-000000000099' },
  ]) {
    const s = setup(),
      saved = s.suspend();
    s.changePrincipal(change);
    const controller = s.controller();
    let applied = false;
    assert.deepEqual(pendingResourceHearingDrafts(s.session, s.caseId, s.resourceId), []);
    const result = await controller.restore(
      saved,
      async () => {
        assert.fail('Foreign identity must not read retained references.');
      },
      () => {
        applied = true;
      },
    );
    assert.notEqual(result.status, 'restored');
    assert.equal(applied, false);
    assert.deepEqual(s.calls, []);
  }
});

test('principal or disposal changes during authorization invalidate pending restoration', async () => {
  for (const mutation of ['email', 'role', 'dispose', 'admission']) {
    const s = setup(),
      saved = s.suspend(),
      controller = s.controller();
    let release;
    s.session.authorizeCase = () =>
      new Promise((resolve) => {
        release = resolve;
      });
    const pending = controller.restore(
      saved,
      async () => {
        assert.fail('Stale read');
      },
      () => {
        assert.fail('Stale apply');
      },
    );
    if (mutation === 'dispose') controller.dispose();
    else if (mutation === 'admission') s.setAllowed(false);
    else s.changePrincipal({ [mutation]: mutation === 'role' ? 'litigator' : 'new@example.test' });
    release({
      id: s.caseId,
      administration: { case_id: s.caseId, administrative_status: 'active' },
    });
    assert.notEqual((await pending).status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('missing retained reference does not discard uncertain snapshot or imply absence', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  const failure = Object.assign(new Error('Missing exact reference'), {
    status: 404,
    code: 'resource_activity_not_found',
  });
  await assert.rejects(
    controller.restore(
      saved,
      async () => {
        throw failure;
      },
      () => {
        assert.fail('Unexpected apply');
      },
    ),
    (error) => error === failure,
  );
  assert.equal(s.registry.pending().length, 1);
  let restored;
  await controller.restore(
    saved,
    async () => ({ head: s.value.base }),
    (value) => {
      restored = value;
    },
  );
  assert.deepEqual(restored.last, s.value.last);
  assert.equal(restored.mode, 'uncertain');
});

test('closed current case is supplied explicitly while its retained creation remains uncertain', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  s.session.authorizeCase = async () => ({
    id: s.caseId,
    administration: { case_id: s.caseId, administrative_status: 'closed' },
  });
  let applied;
  await controller.restore(
    saved,
    async () => ({ closed: false }),
    (value, context) => {
      applied = { value, context };
    },
  );
  assert.equal(applied.context.closed, true);
  assert.equal(applied.value.mode, 'uncertain');
  assert.deepEqual(applied.value.last, s.value.last);
});

test('capture rejects substitution of retained operation, parent, head or historical sources', () => {
  for (const alter of [
    (s) => {
      s.operationId = 'f0000000-0000-4000-8000-000000000099';
    },
    (s) => {
      s.associationId = 'f0000000-0000-4000-8000-000000000099';
    },
    (s) => {
      s.hearingId = 'f0000000-0000-4000-8000-000000000099';
    },
    (s) => {
      s.resource.id = 'f0000000-0000-4000-8000-000000000099';
    },
    (s) => {
      s.base.receipt.capture_digest = '0'.repeat(64);
    },
    (s) => {
      s.resource.receipt.capture_digest = '0'.repeat(64);
    },
    (s) => {
      s.act = null;
    },
  ]) {
    const value = state();
    alter(value);
    assert.throws(() => captureResourceHearingDraft(value));
  }
});

test('explicit close discards a registered snapshot but dispose alone preserves recovery', () => {
  const s = setup();
  s.registry.suspend();
  s.registry.activate(s.session.principal().id);
  assert.equal(s.registry.pending().length, 1);
  s.first.close();
  assert.equal(s.registry.pending().length, 0);
  const another = setup();
  another.suspend();
  assert.equal(another.registry.pending().length, 1);
});
