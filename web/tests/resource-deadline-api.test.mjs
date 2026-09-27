import test from 'node:test';
import assert from 'node:assert/strict';
import { resourceDeadlinesApi } from '../src/lib/resource-deadline-api.mjs';
import {
  resourceDeadlinePrepared,
  resourceDeadlineResult,
  clone,
} from './fixtures/resource-deadline-unit.mjs';
const principal = (p) => ({ ...p.association.recorded_by, role: 'owner' });
function client(request, draft = resourceDeadlinePrepared()) {
  return resourceDeadlinesApi(request, draft.command.case_id, draft.command.resource_id);
}
test('prepares and submits a new deadline and exact link with one composite write', async () => {
  const draft = resourceDeadlinePrepared(),
    result = resourceDeadlineResult(draft),
    calls = [];
  const api = client(async (path, options) => {
    calls.push({ path, options });
    return clone(path.endsWith('/prepare') ? draft : result);
  });
  assert.deepEqual(await api.prepare(draft.command, principal(draft)), draft);
  assert.deepEqual(await api.submit(draft), result);
  assert.deepEqual(
    calls.map((r) => r.path),
    ['prepare', 'submit'].map(
      (suffix) =>
        `/cases/${draft.command.case_id}/procedural-resources/${draft.command.resource_id}/activities/deadlines/${suffix}`,
    ),
  );
  assert.equal(calls[1].options.method, 'POST');
  assert.deepEqual(calls[1].options.data, {
    command: draft.command,
    expected_submission_digest: draft.submission_digest,
  });
  assert.equal(draft.command.resource.revision, 1);
  assert.equal(draft.command.expected_resource_revision, 2);
  assert.equal(Object.hasOwn(draft.association, 'recorded_at'), false);
  assert.equal(Object.hasOwn(draft.association, 'target'), false);
});
test('rejects another scope, nonregistration and future captures before sending', async () => {
  const draft = resourceDeadlinePrepared();
  let calls = 0;
  const api = client(async () => {
    calls++;
    return clone(draft);
  });
  for (const mutate of [
    (c) => {
      c.case_id = '00000000-0000-0000-0000-000000000099';
    },
    (c) => {
      c.resource_id = c.association_id;
    },
    (c) => {
      c.deadline.change.action = 'correct';
    },
    (c) => {
      c.resource.revision = 3;
    },
    (c) => {
      c.deadline.change.definition.input.selection.case_id = c.association_id;
    },
  ]) {
    const command = clone(draft.command);
    mutate(command);
    await assert.rejects(api.prepare(command, principal(draft)));
  }
  assert.equal(calls, 0);
});
test('rejects swapped prepared actor, resource, operation, target or combined command', async () => {
  const draft = resourceDeadlinePrepared();
  for (const mutate of [
    (p) => {
      p.association.recorded_by.email = 'other@example.test';
    },
    (p) => {
      p.association.resource.receipt.capture_digest = '1'.repeat(64);
    },
    (p) => {
      p.association.command.operation_id = p.command.association_id;
    },
    (p) => {
      p.association.command.change.target.capture_digest = '1'.repeat(64);
    },
    (p) => {
      p.command.expected_resource_revision++;
    },
    (p) => {
      p.association.recorded_at = '2026-09-27T00:00:00Z';
    },
  ]) {
    const reply = clone(draft);
    mutate(reply);
    await assert.rejects(client(async () => reply).prepare(draft.command, principal(draft)));
  }
});
test('requires both exact receipts and the same captured deadline in the linked result', async () => {
  const draft = resourceDeadlinePrepared();
  for (const mutate of [
    (r) => {
      r.deadline.receipt.operation_id = draft.command.association_id;
    },
    (r) => {
      r.association.receipt.operation_id = draft.command.association_id;
    },
    (r) => {
      r.association.receipt.submission_digest = '1'.repeat(64);
    },
    (r) => {
      r.association.sources.target.record.definition.title = 'Other deadline';
    },
    (r) => {
      delete r.association;
    },
    (r) => {
      r.submission_digest = '1'.repeat(64);
    },
  ]) {
    const result = resourceDeadlineResult(draft);
    mutate(result);
    await assert.rejects(client(async () => result).submit(draft));
  }
});
test('does not accept late responses after the resource context is disposed', async () => {
  const draft = resourceDeadlinePrepared();
  let resolve;
  const api = client(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = api.prepare(draft.command, principal(draft));
  api.dispose();
  resolve(clone(draft));
  await assert.rejects(pending);
  await assert.rejects(api.submit(draft));
});
