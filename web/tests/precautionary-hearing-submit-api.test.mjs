import test from 'node:test';
import assert from 'node:assert/strict';
import { precautionaryHearingsApi } from '../src/lib/precautionary-hearing-api.mjs';
import {
  precautionaryHearingOperation,
  preparedHearing,
  hearingBase,
  workflowClient,
  foreign,
  clone,
} from './fixtures/precautionary-hearing-workflow.mjs';

const client = (reply) => workflowClient(precautionaryHearingsApi, reply);
const confirmation = (draft) => ({
  command: draft.command,
  expected_submission_digest: draft.submission_digest,
  expected_review_digest: draft.review_digest,
});

test('submit sends only the retained complete command and both digests once for each action', async () => {
  for (const revision of [1, 2, 3]) {
    const draft = preparedHearing(revision),
      operation = precautionaryHearingOperation({ revision });
    const { api, calls } = client(operation);
    assert.deepEqual(await api.submit(draft), operation);
    assert.deepEqual(calls, [
      {
        path: `${hearingBase}/submit`,
        options: { method: 'POST', data: confirmation(draft) },
      },
    ]);
  }
});

test('submit and receipt lookup reject malformed retained reviews before transport', async () => {
  for (const method of ['submit', 'readSubmission']) {
    for (const change of [
      (v) => {
        v.case_id = foreign;
      },
      (v) => {
        v.command.case_id = foreign;
      },
      (v) => {
        v.submission_digest = 'bad';
      },
      (v) => {
        delete v.review_digest;
      },
      (v) => {
        v.recorded_at = '2026-01-01T00:00:00Z';
      },
      (v) => {
        v.command.change.values.venue = 'Changed without review';
      },
    ]) {
      const draft = preparedHearing(),
        { api, calls } = client(precautionaryHearingOperation({ revision: 1 }));
      change(draft);
      await assert.rejects(api[method](draft), change.toString());
      assert.equal(calls.length, 0);
    }
  }
});

test('submit and recovery reject another internally consistent operation or reviewed material', async () => {
  for (const method of ['submit', 'readSubmission']) {
    for (const change of [
      (v) => {
        v.review.command.operation_id = foreign;
      },
      (v) => {
        v.review.actor.email = 'other@example.test';
      },
      (v) => {
        v.review.submission_digest = 'b'.repeat(64);
      },
      (v) => {
        v.review.review_digest = 'c'.repeat(64);
      },
      (v) => {
        v.review.resolved_values.venue = 'Different reviewed venue';
        v.review.command.change.values.venue = v.review.resolved_values.venue;
      },
    ]) {
      const operation = precautionaryHearingOperation({ revision: 1 });
      change(operation.capture);
      operation.history.captures[0] = clone(operation.capture);
      Object.assign(operation.history.origin, {
        operation_id: operation.capture.review.command.operation_id,
        submission_digest: operation.capture.review.submission_digest,
        review_digest: operation.capture.review.review_digest,
      });
      const { api, calls } = client(operation);
      await assert.rejects(api[method](preparedHearing()), change.toString());
      assert.equal(calls.length, 1);
    }
  }
});

test('an uncertain response is reconciled by exact operation GET without resending', async () => {
  const draft = preparedHearing(2),
    operation = precautionaryHearingOperation({ revision: 2 });
  const failure = Object.assign(new Error('Response interrupted'), { status: 503 });
  const { api, calls } = client((path) => {
    if (path.endsWith('/submit')) throw failure;
    return clone(operation);
  });
  await assert.rejects(api.submit(draft), (error) => error === failure);
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.readSubmission(draft), { state: 'confirmed', operation });
  assert.deepEqual(calls[1], {
    path: `${hearingBase}/operations/${draft.command.operation_id}`,
    options: undefined,
  });
  assert.equal(calls.length, 2);
});

test('only the exact missing receipt is unconfirmed and a later explicit send preserves both digests', async () => {
  const draft = preparedHearing(),
    operation = precautionaryHearingOperation({ revision: 1 });
  const missing = Object.assign(new Error('Missing'), {
    status: 404,
    code: 'precautionary_hearing_not_found',
  });
  const { api, calls } = client((path) => {
    if (path.includes('/operations/')) throw missing;
    return clone(operation);
  });
  assert.deepEqual(await api.readSubmission(draft), { state: 'unconfirmed' });
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.submit(draft), operation);
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[1].options.data, confirmation(draft));
});

test('transport and authorization failures remain errors without retries or false absence', async () => {
  for (const [status, code] of [
    [401, 'unauthenticated'],
    [403, 'forbidden'],
    [404, 'case_not_found'],
    [409, 'precautionary_hearing_operation_conflict'],
    [500, 'internal_error'],
    [503, 'precautionary_hearing_not_found'],
  ]) {
    for (const method of ['submit', 'readSubmission']) {
      const failure = Object.assign(new Error(code), { status, code });
      const { api, calls } = client(() => {
        throw failure;
      });
      await assert.rejects(api[method](preparedHearing()), (error) => error === failure);
      assert.equal(calls.length, 1);
    }
  }
});

test('pending sends and receipt reads retain the reviewed bytes across caller edits and disposal', async () => {
  for (const method of ['submit', 'readSubmission']) {
    for (const disposed of [false, true]) {
      const draft = preparedHearing(3),
        operation = precautionaryHearingOperation({ revision: 3 });
      let release;
      const { api, calls } = client(
        () =>
          new Promise((resolve) => {
            release = resolve;
          }),
      );
      const pending = api[method](draft);
      draft.command.change.reason = 'Changed after request';
      draft.review_digest = 'a'.repeat(64);
      if (disposed) api.dispose();
      release(clone(operation));
      if (disposed) await assert.rejects(pending);
      else
        assert.deepEqual(
          await pending,
          method === 'submit' ? operation : { state: 'confirmed', operation },
        );
      assert.equal(calls.length, 1);
    }
  }
});

test('disposed receipt recovery cannot turn a late missing response into unconfirmed', async () => {
  let reject;
  const { api, calls } = client(
    () =>
      new Promise((_, fail) => {
        reject = fail;
      }),
  );
  const draft = preparedHearing(),
    pending = api.readSubmission(draft);
  api.dispose();
  reject(
    Object.assign(new Error('Missing'), { status: 404, code: 'precautionary_hearing_not_found' }),
  );
  await assert.rejects(pending);
  await assert.rejects(api.readSubmission(draft));
  await assert.rejects(api.submit(draft));
  assert.equal(calls.length, 1);
});
