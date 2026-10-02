import test from 'node:test';
import assert from 'node:assert/strict';
import { resourceDeadlinesApi } from '../src/lib/resource-deadline-api.mjs';
import { administration } from './fixtures/deadline-v2-unit.mjs';
import {
  resourceDeadlinePrepared,
  resourceDeadlineResult,
  clone,
} from './fixtures/resource-deadline-unit.mjs';
function recorded(nanosecond = 123456789, fraction = '.123456789') {
  const draft = resourceDeadlinePrepared(),
    value = administration();
  value.changed_at.nanosecond = nanosecond;
  draft.deadline.calculation.material.administration = clone(value);
  draft.deadline.tracking.administration = clone(value);
  draft.association.observed_administration = {
    ...clone(value),
    changed_at: `2026-01-01T00:00:00${fraction}Z`,
  };
  return draft;
}
const principal = (draft) => ({ ...draft.association.recorded_by, role: 'owner' });
function client(reply, draft) {
  return resourceDeadlinesApi(
    async (path) => clone(path.endsWith('/prepare') ? reply : resourceDeadlineResult(reply)),
    draft.command.case_id,
    draft.command.resource_id,
  );
}
test('recorded administration compares equal across UTC text and exact instant projections', async () => {
  for (const [nanos, fraction] of [
    [123456789, '.123456789'],
    [123456780, '.12345678'],
    [0, ''],
  ]) {
    const draft = recorded(nanos, fraction),
      api = client(draft, draft);
    assert.deepEqual(await api.prepare(draft.command, principal(draft)), draft);
    assert.deepEqual(await api.submit(draft), resourceDeadlineResult(draft));
  }
});
test('cross-projection comparison retains nanoseconds and all administrative identity fields', async () => {
  const draft = recorded();
  for (const mutate of [
    (v) => {
      v.changed_at = '2026-01-01T00:00:00.123456788Z';
    },
    (v) => {
      v.changed_at = '2026-01-01T00:00:01.123456789Z';
    },
    (v) => {
      v.title = 'Other title';
    },
    (v) => {
      v.reference = 'Other reference';
    },
    (v) => {
      v.revision++;
    },
    (v) => {
      v.values_digest = '1'.repeat(64);
    },
    (v) => {
      v.changed_by.email = 'other@example.test';
    },
    (v) => {
      v.changed_by.id = draft.command.association_id;
    },
  ]) {
    const reply = clone(draft);
    mutate(reply.association.observed_administration);
    await assert.rejects(client(reply, draft).prepare(draft.command, principal(draft)));
  }
});
test('invalid civil dates cannot match an instant by Date parser normalization', async () => {
  const draft = recorded(0, '');
  const seconds = Date.parse('2026-03-02T00:00:00Z') / 1000;
  for (const value of [
    draft.deadline.calculation.material.administration,
    draft.deadline.tracking.administration,
  ])
    value.changed_at.unix_seconds = seconds;
  draft.association.observed_administration.changed_at = '2026-02-30T00:00:00Z';
  await assert.rejects(client(draft, draft).prepare(draft.command, principal(draft)));
});
