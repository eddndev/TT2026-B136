import test from 'node:test';
import assert from 'node:assert/strict';
import { precautionaryHearingsApi } from '../src/lib/precautionary-hearing-api.mjs';
import {
  precautionaryCaseId,
  precautionaryHearingOperation,
  preparedHearing,
  hearingPage,
  hearingBase,
  workflowClient,
  foreign,
  clone,
} from './fixtures/precautionary-hearing-workflow.mjs';

const client = (reply) => workflowClient(precautionaryHearingsApi, reply);

test('context reads preserve exact case context including administrative closure', async () => {
  for (const status of ['active', 'closed']) {
    const context = clone(preparedHearing().observed_context);
    if (status === 'closed') {
      context.administration.administrative_status = status;
      context.administration.revision = 2;
      context.administration.values_digest = 'c'.repeat(64);
      context.administration.changed_at = '2026-01-02T00:00:00Z';
      context.context_digest = 'd'.repeat(64);
      context.expectation.administration_revision = 2;
      context.expectation.context_digest = context.context_digest;
    }
    const { api, calls } = client(context);
    assert.deepEqual(await api.context(), context);
    assert.deepEqual(calls, [
      { path: `/cases/${precautionaryCaseId}/precautionary-context`, options: undefined },
    ]);
    assert.equal(context.stage_administration.administrative_status, 'active');
  }
});

test('context rejects a foreign case or inconsistent observed revisions', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreign;
    },
    (v) => {
      v.administration.case_id = foreign;
    },
    (v) => {
      v.expectation.stage_revision++;
    },
    (v) => {
      v.expectation.context_digest = 'a'.repeat(64);
    },
  ]) {
    const context = clone(preparedHearing().observed_context);
    change(context);
    await assert.rejects(client(context).api.context());
  }
});

test('current hearing GET preserves its full returned revision without assuming R1', async () => {
  for (const revision of [1, 2, 3]) {
    const operation = precautionaryHearingOperation({ revision }),
      { api, calls } = client(operation);
    const id = operation.capture.review.command.hearing_id;
    assert.deepEqual(await api.get(id), operation);
    assert.deepEqual(calls, [{ path: `${hearingBase}/${id}`, options: undefined }]);
  }
  const { api, calls } = client(precautionaryHearingOperation());
  await assert.rejects(api.get('invalid'));
  assert.equal(calls.length, 0);
  await assert.rejects(api.get(foreign));
  assert.equal(calls.length, 1);
});

test('hearing collection uses exclusive ascending cursors and retains cancelled heads', async () => {
  const page = hearingPage(3);
  page.has_more = true;
  page.next_after_id = page.items.at(-1).capture.review.command.hearing_id;
  const { api, calls } = client(page);
  assert.deepEqual(await api.list({ limit: 3 }), page);
  assert.deepEqual(calls, [{ path: `${hearingBase}?limit=3`, options: undefined }]);
  assert.equal(page.items[2].capture.review.status, 'cancelled');
  const next = hearingPage(4);
  next.items = next.items.slice(3);
  const second = client(next);
  assert.deepEqual(await second.api.list({ afterId: page.next_after_id }), next);
  assert.equal(second.calls[0].path, `${hearingBase}?limit=10&after_id=${page.next_after_id}`);
  assert.deepEqual(await client(hearingPage(0)).api.list(), hearingPage(0));
  assert.deepEqual(await client(hearingPage(20)).api.list({ limit: 20 }), hearingPage(20));
});

test('collection rejects invalid query limits and unrelated filters before transport', async () => {
  for (const query of [
    { limit: 0 },
    { limit: 21 },
    { limit: 1.5 },
    { afterId: null },
    { afterId: foreign.toUpperCase() },
    { status: 'scheduled' },
  ]) {
    const { api, calls } = client(hearingPage());
    await assert.rejects(api.list(query));
    assert.equal(calls.length, 0);
  }
});

test('collection checks scope, order, page size, retained origin and cursor agreement', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreign;
    },
    (v) => {
      v.items.reverse();
    },
    (v) => {
      v.items[1] = clone(v.items[0]);
    },
    (v) => {
      v.items[0].history.origin.operation_id = foreign;
    },
    (v) => {
      v.next_after_id = foreign;
    },
    (v) => {
      v.has_more = 'false';
    },
    (v) => {
      v.has_more = true;
    },
    (v) => {
      v.has_more = true;
      v.next_after_id = v.items[0].capture.review.command.hearing_id;
    },
    (v) => {
      v.has_more = true;
      v.next_after_id = v.items[1].capture.review.command.hearing_id;
      v.items.shift();
    },
    (v) => {
      v.extra = true;
    },
  ]) {
    const page = hearingPage(2);
    change(page);
    await assert.rejects(client(page).api.list({ limit: 2 }), change.toString());
  }
  await assert.rejects(client(hearingPage(2)).api.list({ limit: 1 }));
  const page = hearingPage();
  await assert.rejects(
    client(page).api.list({ afterId: page.items[0].capture.review.command.hearing_id }),
  );
});

test('context collection and current detail discard responses after disposal', async () => {
  for (const method of ['context', 'list', 'get']) {
    const operation = precautionaryHearingOperation();
    const args = method === 'get' ? [operation.capture.review.command.hearing_id] : [];
    const response =
      method === 'context'
        ? preparedHearing().observed_context
        : method === 'list'
          ? hearingPage()
          : operation;
    let release;
    const { api, calls } = client(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const pending = api[method](...args);
    api.dispose();
    release(clone(response));
    await assert.rejects(pending);
    await assert.rejects(api[method](...args));
    assert.equal(calls.length, 1);
  }
});
