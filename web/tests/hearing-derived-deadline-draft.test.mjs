import test from 'node:test';
import assert from 'node:assert/strict';
import {
  captureHearingDerivedDeadlineDraft,
  pendingHearingDerivedDeadlineDrafts,
} from '../src/lib/hearing-derived-deadline-draft.mjs';
import { createStageSupportDraft } from '../src/lib/stage-support-draft.mjs';
import { descriptorKey } from '../src/lib/draft-descriptor.mjs';
import { draftState, setup } from './fixtures/hearing-derived-deadline-draft.mjs';
import { ids } from './fixtures/hearing-derived-deadline-unit.mjs';
import { deferred } from './fixtures/draft-registry.mjs';
import { initialDeadlinePolicies } from '../src/lib/deadline-editor-policies.mjs';
import { hearingResultDraft } from '../src/lib/hearing-result-values.mjs';
import { initialResourceDeadline } from '../src/lib/resource-deadline-draft.mjs';

test('editable policy keys survive uncertain capture and session restoration', async () => {
  const original = draftState();
  original.policies = initialDeadlinePolicies(
    original.definition,
    original.last.command.deadline.change.tracking,
  );
  assert.deepEqual(captureHearingDerivedDeadlineDraft(original), original);
  const s = setup(original),
    saved = s.suspend();
  let restored;
  const outcome = await s.controller().restore(saved, (value) => {
    restored = value;
  });
  assert.equal(outcome.status, 'restored');
  assert.deepEqual(restored.policies, original.policies);
  assert.deepEqual(restored.last, original.last);
});

test('empty result and deadline forms preserve unselected editable policies without formalizing', () => {
  const original = draftState('draft');
  const source = structuredClone(original.definition.input.selection.source);
  original.definition = initialResourceDeadline(original.definition.input.selection.case_id);
  original.definition.input.selection.source = source;
  original.draft = hearingResultDraft(null);
  original.policies = initialDeadlinePolicies(original.definition);
  original.last = null;
  original.inputs = null;
  assert.deepEqual(captureHearingDerivedDeadlineDraft(original), original);
  assert.deepEqual(original.policies.profile, { key: '', value: '' });
  assert.equal(original.policies.source.value, '');
});

test('capture preserves the complete uncertain Ready and never preserves approval', () => {
  const original = draftState();
  const captured = captureHearingDerivedDeadlineDraft({
    ...original,
    prepared: original.last,
    acknowledged: true,
    busy: true,
    retryAvailable: true,
  });
  assert.deepEqual(captured, original);
  original.last.deadline.profile.definition.title = 'Mutated';
  original.draft.summary = 'Mutated';
  assert.notEqual(
    captured.last.deadline.profile.definition.title,
    original.last.deadline.profile.definition.title,
  );
  assert.notEqual(captured.draft.summary, original.draft.summary);
  assert.equal(captureHearingDerivedDeadlineDraft(draftState('prepared')).mode, 'draft');
  const incomplete = draftState('draft');
  incomplete.last = null;
  incomplete.draft.summary = '';
  incomplete.definition.title = '';
  assert.deepEqual(captureHearingDerivedDeadlineDraft(incomplete), incomplete);
});

test('capture rejects an uncertain draft without its original exact Ready', () => {
  for (const alter of [
    (s) => {
      s.last = null;
    },
    (s) => {
      s.resultId = ids(99);
    },
    (s) => {
      s.deadlineId = ids(99);
    },
    (s) => {
      s.anchor.revision++;
    },
    (s) => {
      s.continuation = { result_id: ids(99), revision: 1 };
    },
    (s) => {
      s.draft.summary = 'Substituted';
    },
    (s) => {
      s.definition.title = 'Substituted';
    },
    (s) => {
      s.policies.profile = 'follow';
    },
    (s) => {
      s.last.deadline.source_event = { sequence: '1' };
    },
    (s) => {
      s.last.review_digest = 'invalid';
    },
  ]) {
    const value = draftState();
    alter(value);
    assert.throws(() => captureHearingDerivedDeadlineDraft(value));
  }
});

test('restore reauthorizes the case and retains uncertainty without refreshing catalogs', async () => {
  const s = setup(),
    saved = s.suspend();
  const controller = s.controller(async () => {
    assert.fail('No current source reads.');
  });
  assert.deepEqual(pendingHearingDerivedDeadlineDrafts(s.session, s.caseId, s.hearingId), [saved]);
  assert.equal(saved.editorKind, 'hearing-derived-deadline');
  assert.equal(saved.resourceId, s.hearingId);
  assert.equal(saved.instanceId, s.value.resultId);
  let applied;
  const outcome = await controller.restore(saved, (value, context) => {
    s.calls.push('apply');
    applied = { value, context };
  });
  assert.equal(outcome.status, 'restored');
  assert.deepEqual(s.calls, ['case', 'apply']);
  assert.deepEqual(applied.value, s.value);
  assert.equal(applied.context.closed, false);
  assert.equal(applied.value.mode, 'uncertain');
  assert.deepEqual(applied.value.last, s.value.last);
  assert.equal(s.registry.pending().length, 0);
});

test('fresh authorizes before reading current context and cannot hide a closed case', async () => {
  const s = setup();
  s.setStatus('closed');
  const context = await s.first.fresh();
  assert.deepEqual(s.calls, ['case', 'read']);
  assert.equal(context.closed, true);
  assert.equal(context.hearing.id, s.hearingId);
  const saved = s.suspend();
  let restored;
  await s.controller().restore(saved, (value, current) => {
    restored = { value, current };
  });
  assert.equal(restored.current.closed, true);
  assert.equal(restored.value.mode, 'uncertain');
  assert.deepEqual(s.calls, ['case', 'read', 'case']);
});

test('admission and pending drafts require the same identity and a current managing role', async () => {
  for (const change of [{ role: 'paralegal' }, { role: 'client' }, { id: ids(99) }]) {
    const s = setup(),
      saved = s.suspend();
    s.setPrincipal(change);
    const controller = s.controller();
    assert.equal(s.first.admitted(), false);
    assert.deepEqual(pendingHearingDerivedDeadlineDrafts(s.session, s.caseId, s.hearingId), []);
    assert.notEqual(
      (await controller.restore(saved, () => assert.fail('Denied apply'))).status,
      'restored',
    );
    assert.deepEqual(s.calls, []);
  }
  const s = setup(),
    saved = s.suspend();
  s.setPrincipal({ role: 'litigator', email: 'current@example.test' });
  let restored;
  assert.equal(
    (
      await s.controller().restore(saved, (value) => {
        restored = value;
      })
    ).status,
    'restored',
  );
  assert.deepEqual(restored.last, s.value.last);
});

test('authorization races cannot apply a draft after identity, admission or lifecycle loss', async () => {
  for (const change of ['identity', 'role', 'admission', 'dispose']) {
    const s = setup(),
      saved = s.suspend(),
      wait = deferred();
    const controller = s.controller();
    s.session.authorizeCase = () => wait.promise;
    const pending = controller.restore(saved, () => assert.fail('Stale apply'));
    if (change === 'identity') s.setPrincipal({ id: ids(99) });
    if (change === 'role') s.setPrincipal({ role: 'client' });
    if (change === 'admission') s.setAllowed(false);
    if (change === 'dispose') controller.dispose();
    wait.resolve(s.caseRecord());
    assert.notEqual((await pending).status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('restore revalidates stored descriptor boundaries and keeps invalid snapshots unapplied', async () => {
  for (const mutate of [
    (d) => {
      d.contextId = ids(99);
    },
    (d) => {
      d.resourceId = ids(99);
    },
    (d) => {
      d.editorKind = 'hearing-result';
    },
    (d) => {
      d.action = 'record';
    },
    (d) => {
      d.instanceId = ids(99);
    },
    (d) => {
      d.baseRevision++;
    },
    (d) => {
      d.schemaVersion++;
    },
  ]) {
    const s = setup();
    const saved = s.unsafeSnapshot(() => {}, mutate);
    const outcome = await s.controller().restore(saved, () => assert.fail('Foreign apply'));
    assert.notEqual(outcome.status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('restore validates retained scope, ids and all Ready fields even after raw registry capture', async () => {
  for (const mutate of [
    (v) => {
      v.deadlineId = ids(99);
    },
    (v) => {
      v.anchor.hearing_id = ids(99);
    },
    (v) => {
      v.definition.input.selection.case_id = ids(99);
    },
    (v) => {
      v.last.result.actor_id = ids(99);
    },
    (v) => {
      v.last.deadline.result.due_at = { unix_seconds: 0 };
    },
    (v) => {
      v.last.deadline.profile.definition.scope.case_id = ids(99);
    },
  ]) {
    const s = setup(),
      saved = s.unsafeSnapshot(mutate);
    const outcome = await s.controller().restore(saved, () => assert.fail('Corrupt apply'));
    assert.notEqual(outcome.status, 'restored');
    assert.equal(s.registry.pending().length, 1);
  }
});

test('failed or malformed case authorization preserves uncertainty for a later attempt', async () => {
  const s = setup(),
    saved = s.suspend(),
    controller = s.controller();
  const failure = Object.assign(new Error('Unavailable'), { status: 503 });
  s.session.authorizeCase = async () => {
    throw failure;
  };
  await assert.rejects(
    controller.restore(saved, () => assert.fail('Failed apply')),
    (error) => error === failure,
  );
  assert.equal(s.registry.pending().length, 1);
  for (const bad of [
    { ...s.caseRecord(), id: ids(99) },
    { id: s.caseId, administration: { case_id: ids(99), administrative_status: 'active' } },
    { id: s.caseId, administration: { case_id: s.caseId, administrative_status: 'invented' } },
  ]) {
    s.session.authorizeCase = async () => bad;
    await assert.rejects(controller.restore(saved, () => assert.fail('Wrong case apply')));
    assert.equal(s.registry.pending().length, 1);
  }
});

test('support recovery belongs to this editor and explicit discard leaves the parent draft', () => {
  const s = setup(),
    field = ['provenance', 'support'];
  const context = s.first.supportContext(
    field,
    () => true,
    () => {},
    () => {},
  );
  assert.equal(context.ownerDraftKey, descriptorKey(s.descriptor()));
  assert.deepEqual(context.fieldPath, field);
  assert.equal(context.caseId, s.caseId);
  assert.equal(context.rowId, null);
  const child = createStageSupportDraft({
    session: s.session,
    context: () => context,
    capture: () => ({ picker: { name: 'Support', query: 'Pending' } }),
  });
  child.register();
  s.registry.suspend();
  s.registry.activate(s.session.principal().id);
  assert.equal(s.registry.pending().length, 2);
  s.first.discardSupport(field);
  assert.equal(s.registry.pending().length, 1);
  assert.equal(s.registry.pending()[0].editorKind, 'hearing-derived-deadline');
  s.first.close();
  assert.equal(s.registry.pending().length, 0);
  assert.equal(context.canApply(), false);
});

test('support authorization rejects late responses after owner disposal', async () => {
  const s = setup(),
    wait = deferred();
  let observed = false;
  s.session.authorizeCase = () => wait.promise;
  const context = s.first.supportContext(
    ['provenance', 'support'],
    () => true,
    () => {
      observed = true;
    },
    () => assert.fail('Unexpected denial'),
  );
  const pending = context.authorize();
  s.first.dispose();
  wait.resolve(s.caseRecord());
  assert.equal(await pending, false);
  assert.equal(observed, false);
  assert.equal(context.canApply(), false);
});

test('context denial discards related drafts while dispose preserves their recovery', () => {
  const s = setup();
  const saved = s.suspend();
  assert.deepEqual(pendingHearingDerivedDeadlineDrafts(s.session, s.caseId, ids(99)), []);
  assert.deepEqual(pendingHearingDerivedDeadlineDrafts(s.session, ids(99), s.hearingId), []);
  assert.equal(s.registry.pending()[0].key, saved.key);
  const controller = s.controller();
  controller.deny({ status: 404, code: 'case_not_found' });
  assert.equal(s.registry.pending().length, 0);
});
