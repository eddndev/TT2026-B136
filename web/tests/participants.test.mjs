import test from 'node:test';
import assert from 'node:assert/strict';
import {
  participantDraft,
  participantValues,
  participantFilters,
  canParticipants,
} from '../src/lib/participants.mjs';
import { participantsApi } from '../src/lib/participants-api.mjs';
const caseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const values = {
  display_name: 'Ana',
  procedural_role: 'Defensa',
  organization: null,
  legal_status: null,
};
const snapshot = { ...values, case_id: caseId, id, revision: 1, directory_status: 'active' };

test('participant text preserves identity spelling and counts Unicode scalars', () => {
  const draft = participantDraft({
    ...snapshot,
    display_name: '\u2003Jos\u00e9,  Mu\u00f1oz\u00a0',
    organization: '  ',
  });
  assert.deepEqual(participantValues(draft), { ...values, display_name: 'Jos\u00e9,  Mu\u00f1oz' });
  assert.equal(
    participantValues({ ...draft, display_name: '\u{1f600}'.repeat(200) }).display_name.length,
    400,
  );
  for (const [field, limit] of [
    ['display_name', 200],
    ['procedural_role', 80],
    ['organization', 200],
    ['legal_status', 160],
  ]) {
    assert.throws(
      () => participantValues({ ...draft, [field]: 'x'.repeat(limit + 1) }),
      (error) => error.field === field,
    );
    assert.throws(
      () => participantValues({ ...draft, [field]: '\ttext' }),
      (error) => error.field === field,
    );
    assert.throws(() => participantValues({ ...draft, [field]: 'text\u0085' }));
  }
  assert.throws(() => participantValues({ ...draft, display_name: '  ' }));
  assert.notEqual(participantValues({ ...draft, display_name: 'e\u0301' }).display_name, '\u00e9');
  draft.display_name = 'Changed';
  assert.equal(snapshot.display_name, 'Ana');
});

test('participant permissions and literal filters are independent of account or document roles', () => {
  for (const role of ['owner', 'litigator']) assert.equal(canParticipants(role, 'manage'), true);
  assert.equal(canParticipants('paralegal', 'read'), true);
  assert.equal(canParticipants('paralegal', 'manage'), false);
  assert.equal(canParticipants('client', 'read'), false);
  assert.deepEqual(
    participantFilters({ name: '  Ana%_  ', procedural_role: ' Defensa ', status: 'all' }),
    { name: 'Ana%_', procedural_role: 'Defensa', status: 'all' },
  );
  assert.deepEqual(participantFilters({ name: '', procedural_role: ' ', status: 'active' }), {
    status: 'active',
  });
  assert.throws(() => participantFilters({ status: ' Active ' }));
  assert.throws(() => participantFilters({ name: '\nAna', status: 'active' }));
});

test('participant requests use exclusive cursors, full edits and precise status-only bodies', async () => {
  const calls = [];
  const api = participantsApi(async (path, options) => {
    calls.push({ path, options });
    if (path.includes('/history'))
      return { revisions: [snapshot], has_more: false, next_before_revision: null };
    if (path.includes('?')) return { participants: [snapshot], has_more: true, next_after_id: id };
    return snapshot;
  }, caseId);
  await api.list({ afterId: id, name: 'Ana%_', procedural_role: 'Defensa', status: 'all' });
  await api.create(values);
  await api.get(id);
  await api.replace(id, 3, { ...values, directory_status: 'archived' });
  await api.changeStatus(id, 4, 'active');
  await api.history(id, { beforeRevision: 5 });
  const url = new URL(calls[0].path, 'http://test');
  assert.equal(url.searchParams.get('after_id'), id);
  assert.equal(url.searchParams.get('name'), 'Ana%_');
  assert.equal(url.searchParams.get('status'), 'all');
  assert.deepEqual(calls[1].options, { method: 'POST', data: values });
  assert.deepEqual(calls[3].options.data, {
    expected_revision: 3,
    ...values,
    directory_status: 'archived',
  });
  assert.equal(calls[4].path, `/cases/${caseId}/participants/${id}/directory-status`);
  assert.deepEqual(calls[4].options.data, { expected_revision: 4, directory_status: 'active' });
  assert.equal(
    calls[5].path,
    `/cases/${caseId}/participants/${id}/history?limit=50&before_revision=5`,
  );
});

test('participant scope rejects wrong contexts and responses after disposal', async () => {
  const wrong = participantsApi(async () => ({ ...snapshot, case_id: id }), caseId);
  await assert.rejects(wrong.get(id), /expediente/);
  let resolve;
  const api = participantsApi(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
    caseId,
  );
  const pending = api.get(id);
  api.dispose();
  resolve(snapshot);
  await assert.rejects(pending, /abierto/);
  await assert.rejects(api.get(id), /abierto/);
});
