import { test } from 'node:test';
import assert from 'node:assert/strict';
import { resourceActivitiesApi } from '../src/lib/resource-activities-api.mjs';
import { resourceActivityMatches } from '../src/lib/resource-activity-validation.mjs';
import {
  activityPrepared,
  activityRecord,
  activityView,
  activityActor,
  clone,
} from './fixtures/resource-activity-unit.mjs';

const apiFor = (request, draft = activityPrepared()) =>
  resourceActivitiesApi(request, draft.case_id, draft.resource_id);
test('preparation and confirmation preserve historical selection and the current expected resource head', async () => {
  const draft = activityPrepared(),
    calls = [];
  const api = apiFor(async (path, options) => {
    calls.push({ path, options });
    return path.endsWith('/prepare') ? clone(draft) : activityRecord(draft);
  });
  assert.deepEqual(await api.prepare(draft.command, activityActor), draft);
  const record = await api.submit(draft);
  assert.equal(resourceActivityMatches(record, draft), true);
  assert.equal(record.selection.resource.revision, 1);
  assert.equal(record.recorded_resource_head.revision, 2);
  assert.deepEqual(calls[1].options.data, {
    command: draft.command,
    expected_submission_digest: draft.submission_digest,
  });
  assert.equal(calls[1].options.method, 'POST');
});
test('current deadline state stays separate from the historical calculated date', async () => {
  const draft = activityPrepared('deadline'),
    view = activityView(draft);
  const api = apiFor(async () => clone(view), draft);
  const value = await api.revision(view.association.id, 1);
  assert.notEqual(value.association.sources.target.record.calculation.result.due_at, null);
  assert.equal(value.current_target.record.operational.due_at, null);
  assert.equal(value.current_target.record.revision, 2);
  assert.equal(value.association.selection.target.revision, 1);
});
test('reads reject scope, target, observed time and historical capture substitutions', async () => {
  for (const alter of [
    (v) => {
      v.association.resource_id = 'e0000000-0000-4000-8000-000000000099';
    },
    (v) => {
      v.association.sources.resource.receipt.capture_digest = '1'.repeat(64);
    },
    (v) => {
      v.current_target.record.id = 'e0000000-0000-4000-8000-000000000099';
    },
    (v) => {
      v.current_target.record.operational.checked_at.nanosecond = 1;
    },
    (v) => {
      v.association.sources.target.record.operational = clone(v.current_target.record.operational);
    },
  ]) {
    const draft = activityPrepared('deadline'),
      view = activityView(draft);
    alter(view);
    await assert.rejects(apiFor(async () => view, draft).get(draft.command.association_id));
  }
});
test('confirmation rejects a changed author or source despite a matching submission digest', async () => {
  const draft = activityPrepared();
  for (const alter of [
    (row) => {
      row.recorded_by.email = 'another@example.test';
    },
    (row) => {
      row.sources.target.record.values.venue = 'Changed after review';
    },
    (row) => {
      row.recorded_resource_head.capture_digest = '2'.repeat(64);
    },
  ]) {
    const row = activityRecord(draft);
    alter(row);
    assert.equal(resourceActivityMatches(row, draft), false);
    await assert.rejects(apiFor(async () => row).submit(draft));
  }
});
test('foreign commands and invalid cursors are rejected before network access', async () => {
  let calls = 0;
  const draft = activityPrepared(),
    api = apiFor(async () => {
      calls++;
    });
  draft.command.case_id = 'e0000000-0000-4000-8000-000000000099';
  await assert.rejects(api.prepare(draft.command, activityActor));
  await assert.rejects(api.list({ afterId: 'invalid' }));
  await assert.rejects(api.history(draft.command.association_id, { limit: 21 }));
  assert.equal(calls, 0);
});
test('closing the scoped client rejects an in-flight protected response', async () => {
  let finish;
  const draft = activityPrepared(),
    api = apiFor(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
  const pending = api.get(draft.command.association_id);
  api.dispose();
  finish(activityView(draft));
  await assert.rejects(pending);
});
test('list and history validate their own scope, order and exclusive continuation', async () => {
  const draft = activityPrepared(),
    view = activityView(draft);
  const page = {
    case_id: draft.case_id,
    resource_id: draft.resource_id,
    associations: [view],
    has_more: true,
    next_after_id: view.association.id,
  };
  assert.deepEqual(
    await apiFor(async () => clone(page)).list({ limit: 1, status: 'linked' }),
    page,
  );
  await assert.rejects(
    apiFor(async () => clone(page)).list({ limit: 1, afterId: view.association.id }),
  );
  const history = {
    case_id: draft.case_id,
    resource_id: draft.resource_id,
    association_id: view.association.id,
    revisions: [view.association],
    has_more: false,
    next_before_revision: null,
  };
  assert.deepEqual(await apiFor(async () => clone(history)).history(view.association.id), history);
  history.resource_id = 'e0000000-0000-4000-8000-000000000099';
  await assert.rejects(apiFor(async () => history).history(view.association.id));
});
