import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resourceActivitiesApi } from '../src/lib/resource-activities-api.mjs';
import { resourceActivityCommand } from '../src/lib/resource-activity-values.mjs';
import { resourceHearingCreation, clone } from './fixtures/resource-hearing-unit.mjs';

function fixture() {
  const { association, hearing } = resourceHearingCreation({ withAct: true });
  const view = {
    association,
    checked_at: { unix_seconds: 1767398400, nanosecond: 0, offset_seconds: 0 },
    current_target: { kind: 'resource_hearing', record: clone(hearing) },
  };
  const page = {
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    associations: [view],
    has_more: false,
    next_after_id: null,
  };
  return { view, page, hearing };
}
const apiFor = (request, hearing) =>
  resourceActivitiesApi(request, hearing.case_id, hearing.resource_id);

test('activity list reads own hearings with an explicit family filter', async () => {
  const { view, page, hearing } = fixture(),
    calls = [];
  const api = apiFor(async (path) => {
    calls.push(path);
    return clone(page);
  }, hearing);
  assert.deepEqual(await api.list({ kind: 'resource_hearing' }), page);
  assert.match(calls[0], /kind=resource_hearing/);
  assert.equal(view.association.selection.resource.revision, 1);
  assert.equal(view.association.recorded_resource_head.revision, 5);
});

test('exact activity reads and history retain the original own hearing record', async () => {
  const { view, hearing } = fixture();
  const history = {
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    association_id: hearing.association_id,
    revisions: [view.association],
    has_more: false,
    next_before_revision: null,
  };
  const api = apiFor(async (path) => clone(path.includes('/history?') ? history : view), hearing);
  assert.deepEqual(await api.revision(hearing.association_id, 1), view);
  assert.deepEqual(await api.history(hearing.association_id), history);
});

test('unlink history preserves the hearing instead of treating unlink as cancellation', async () => {
  const { view, hearing } = fixture();
  const original = clone(view.association);
  const row = view.association;
  row.revision = 2;
  row.status = 'unlinked';
  row.reason = 'Relacion retirada';
  row.receipt.action = 'unlink';
  row.receipt.expected_revision = 1;
  row.receipt.operation_id = 'f0000000-0000-4000-8000-000000000009';
  row.receipt.previous = { revision: 1, capture_digest: original.receipt.capture_digest };
  row.receipt.capture_digest = '2'.repeat(64);
  const api = apiFor(async () => clone(view), hearing);
  const result = await api.get(hearing.association_id);
  assert.equal(result.association.status, 'unlinked');
  assert.deepEqual(result.current_target.record, hearing);
  assert.deepEqual(result.association.sources.target, original.sources.target);
});

test('own targets use capture digest and cannot exchange ordinary hearing receipt fields', () => {
  const { view, hearing } = fixture();
  const command = {
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    operation_id: hearing.operation_id,
    association_id: hearing.association_id,
    expected_resource_revision: 5,
    change: { action: 'link', expected_revision: 0, ...clone(view.association.selection) },
  };
  assert.deepEqual(resourceActivityCommand(command), command);
  command.change.target.submission_digest = command.change.target.capture_digest;
  delete command.change.target.capture_digest;
  assert.throws(() => resourceActivityCommand(command));
});

test('own target reads reject changed scope, identity, revision or capture', async () => {
  for (const mutate of [
    (v) => {
      v.association.sources.target.record.resource_id = 'f0000000-0000-4000-8000-000000000099';
    },
    (v) => {
      v.association.selection.target.id = 'f0000000-0000-4000-8000-000000000099';
    },
    (v) => {
      v.association.selection.target.revision = 2;
    },
    (v) => {
      v.association.selection.target.capture_digest = '1'.repeat(64);
    },
    (v) => {
      v.current_target.record.values.venue = 'Otra sala';
    },
  ]) {
    const { view, hearing } = fixture();
    mutate(view);
    await assert.rejects(apiFor(async () => view, hearing).get(hearing.association_id));
  }
});
