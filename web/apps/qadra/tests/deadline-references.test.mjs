import test from 'node:test';
import assert from 'node:assert/strict';
import { loadDeadlineReference } from '../src/components/deadline-reference-data.mjs';
import { id, hash, profile, prepared } from './fixtures/deadline-unit.mjs';
const source = () => ({
  reference: {
    family: 'notification',
    id: id(8),
    revision: 2,
    resolution: { id: id(9), revision: 1 },
  },
  values_digest: hash(),
  sources_digest: hash('b'),
  submission_digest: hash('c'),
});
function mock(value) {
  const calls = [];
  const scope = {
    revision: async (...args) => {
      calls.push(args);
      return value;
    },
    dispose: () => calls.push('disposed'),
  };
  return {
    calls,
    caseNotifications: (...args) => {
      calls.push(args);
      return scope;
    },
    deadlineProfiles: () => scope,
    caseHearingResults: () => scope,
    judicialCalendars: () => scope,
  };
}
test('notification evidence keeps its own exact parent revision and releases context', async () => {
  const ref = source(),
    row = {
      values_digest: hash(),
      receipt: { sources_digest: hash('b'), submission_digest: hash('c') },
      values: { resolution: { id: id(9), revision: 1 } },
    },
    api = mock(row);
  assert.equal(await loadDeadlineReference(api, id(1), 'source', ref), row);
  assert.deepEqual(api.calls, [[id(1), id(9)], [id(8), 2], 'disposed']);
  row.values.resolution.revision = 2;
  await assert.rejects(loadDeadlineReference(api, id(1), 'source', ref), /captura/);
});
test('captured profile digests and exact scope must match the profile fetched', async () => {
  const ref = prepared().calculation.profile,
    row = profile(),
    api = mock(row);
  assert.equal(await loadDeadlineReference(api, id(1), 'profile', ref), row);
  row.definition.scope.case_id = id(77);
  await assert.rejects(loadDeadlineReference(api, id(1), 'profile', ref), /captura/);
});
test('changed evidence cannot be displayed as the captured historical source', async () => {
  const ref = source(),
    api = mock({
      values_digest: hash('d'),
      receipt: { sources_digest: hash('b'), submission_digest: hash('c') },
      values: { resolution: { id: id(9), revision: 1 } },
    });
  await assert.rejects(loadDeadlineReference(api, id(1), 'source', ref), /captura/);
  assert.equal(api.calls.at(-1), 'disposed');
});
test('a nil hearing agreement remains an exact agreement instead of absence', async () => {
  const ref = {
    reference: {
      family: 'hearing_result',
      hearing_id: id(10),
      result_id: id(11),
      revision: 1,
      agreement_id: id(0),
    },
    values_digest: hash(),
    sources_digest: null,
    submission_digest: hash('b'),
  };
  const row = {
      values_digest: hash(),
      receipt: { submission_digest: hash('b') },
      values: { agreements: [] },
    },
    api = mock(row);
  await assert.rejects(loadDeadlineReference(api, id(1), 'source', ref), /captura/);
  row.values.agreements = [{ id: id(0), text: 'Acuerdo exacto' }];
  assert.equal(await loadDeadlineReference(api, id(1), 'source', ref), row);
});
