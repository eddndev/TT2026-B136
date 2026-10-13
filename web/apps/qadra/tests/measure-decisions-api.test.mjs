import test from 'node:test';
import assert from 'node:assert/strict';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  measureCaseId,
  decisionBase,
  foreignDecisionId,
  measureDecisionOperation,
  preparedDecision,
  decisionClient,
  clone,
} from './fixtures/measure-decision-workflow.mjs';

const client = (reply) => decisionClient(caseApi, reply);
const confirmation = ({ review }) => ({
  command: review.command,
  expected_submission_digest: review.submission_digest,
  expected_review_digest: review.review_digest,
});

test('decision preparation submission and original recovery retain their g1 or g2 family', async () => {
  for (const family of ['g1', 'g2']) {
    const prepared = preparedDecision({ family }),
      operation = measureDecisionOperation({ family });
    const { command, actor } = prepared.review;
    const prepare = client(prepared);
    assert.deepEqual(await prepare.api.prepare(command, actor), prepared);
    assert.deepEqual(prepare.calls, [
      { path: `${decisionBase}/prepare`, options: { method: 'POST', data: command } },
    ]);
    const write = client(operation);
    assert.deepEqual(await write.api.submit(prepared), operation);
    assert.deepEqual(write.calls[0], {
      path: `${decisionBase}/submit`,
      options: { method: 'POST', data: confirmation(prepared) },
    });
    assert.deepEqual(await write.api.readSubmission(prepared), { state: 'confirmed', operation });
    assert.deepEqual(write.calls[1], {
      path: `${decisionBase}/operations/${command.operation_id}`,
      options: undefined,
    });
    assert.deepEqual(await write.api.get(command.decision_id), operation);
    assert.deepEqual(write.calls[2], {
      path: `${decisionBase}/${command.decision_id}`,
      options: undefined,
    });
    assert.equal(write.calls.length, 3);
  }
});

test('decision prepare normalizes declared text and effect order and preserves explicit no change', async () => {
  const prepared = preparedDecision({ count: 2 }),
    command = clone(prepared.review.command);
  command.values.authority = ` ${command.values.authority} `;
  command.outcome.effects.reverse();
  const original = clone(command),
    preparedClient = client(prepared);
  assert.deepEqual(await preparedClient.api.prepare(command, prepared.review.actor), prepared);
  assert.deepEqual(command, original);
  assert.deepEqual(preparedClient.calls[0].options.data, prepared.review.command);
  const noChange = preparedDecision({ noChanges: true });
  noChange.review.actor.role = 'litigator';
  assert.deepEqual(
    await client(noChange).api.prepare(noChange.review.command, noChange.review.actor),
    noChange,
  );
  assert.equal(noChange.review.results.length, 0);
});

test('decision preparation rejects invalid scope principal and command before transport', async () => {
  for (const change of [
    (v) => {
      v.case_id = foreignDecisionId;
    },
    (v) => {
      v.operation_id = v.operation_id.toUpperCase();
    },
    (v) => {
      v.actor = preparedDecision().review.actor;
    },
    (v) => {
      delete v.anchor;
    },
    (v) => {
      v.context.stage_revision = 0;
    },
    (v) => {
      v.values.support.digest = 'bad';
    },
    (v) => {
      v.outcome.effects = [];
    },
    (v) => {
      v.outcome.effects[0].action = 'unknown';
    },
  ]) {
    const prepared = preparedDecision(),
      command = clone(prepared.review.command),
      { api, calls } = client(prepared);
    change(command);
    await assert.rejects(api.prepare(command, prepared.review.actor));
    assert.equal(calls.length, 0);
  }
  for (const role of ['paralegal', 'client']) {
    const prepared = preparedDecision(),
      { api, calls } = client(prepared);
    await assert.rejects(api.prepare(prepared.review.command, { ...prepared.review.actor, role }));
    assert.equal(calls.length, 0);
  }
});

test('decision responses bind family full command actor and visible results to the reviewed operation', async () => {
  for (const change of [
    (v) => {
      v.family = 'g3';
    },
    (v) => {
      v.review.command.operation_id = foreignDecisionId;
    },
    (v) => {
      v.review.actor.email = 'another@example.test';
    },
    (v) => {
      v.review.results[0].id = foreignDecisionId;
    },
    (v) => {
      v.review.material.result_sources[0].sources.subject.case_id = foreignDecisionId;
    },
    (v) => {
      v.review.review_digest = 'bad';
    },
  ]) {
    const original = preparedDecision(),
      changed = clone(original);
    change(changed);
    await assert.rejects(
      client(changed).api.prepare(original.review.command, original.review.actor),
    );
  }
  const prepared = preparedDecision();
  for (const method of ['submit', 'readSubmission']) {
    const invalid = clone(prepared),
      transport = client(measureDecisionOperation());
    invalid.review.command.outcome.effects[0].proposal.values.conditions =
      'Changed without preparation';
    await assert.rejects(transport.api[method](invalid));
    assert.equal(transport.calls.length, 0);
    const operation = measureDecisionOperation();
    operation.origin.review_digest = 'f'.repeat(64);
    await assert.rejects(client(operation).api[method](prepared));
  }
});

test('decision list and reads use authorized original identities with exclusive cursors', async () => {
  const first = measureDecisionOperation(),
    second = measureDecisionOperation({ family: 'g2' });
  const page = {
    case_id: measureCaseId,
    items: [first, second],
    has_more: true,
    next_after_id: second.origin.decision_id,
  };
  const { api, calls } = client(page);
  assert.deepEqual(await api.list({ limit: 2 }), page);
  assert.deepEqual(calls[0], { path: `${decisionBase}?limit=2`, options: undefined });
  const empty = { case_id: measureCaseId, items: [], has_more: false, next_after_id: null };
  const next = client(empty);
  assert.deepEqual(await next.api.list({ afterId: page.next_after_id }), empty);
  assert.equal(next.calls[0].path, `${decisionBase}?limit=10&after_id=${page.next_after_id}`);
  await assert.rejects(client(first).api.get(second.origin.decision_id));
  for (const query of [{ limit: 0 }, { limit: 21 }, { afterId: null }, { family: 'g1' }]) {
    const invalid = client(page);
    await assert.rejects(invalid.api.list(query));
    assert.equal(invalid.calls.length, 0);
  }
  for (const change of [
    (v) => {
      v.case_id = foreignDecisionId;
    },
    (v) => {
      v.items.reverse();
    },
    (v) => {
      v.next_after_id = first.origin.decision_id;
    },
  ]) {
    const invalid = clone(page);
    change(invalid);
    await assert.rejects(client(invalid).api.list({ limit: 2 }));
  }
});

test('decision receipt recovery never resends and only exact absence is unconfirmed', async () => {
  const prepared = preparedDecision(),
    operation = measureDecisionOperation();
  const missing = Object.assign(new Error('Missing'), {
    status: 404,
    code: 'measure_decision_not_found',
  });
  const { api, calls } = client((path) => {
    if (path.includes('/operations/')) throw missing;
    return clone(operation);
  });
  assert.deepEqual(await api.readSubmission(prepared), { state: 'unconfirmed' });
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.submit(prepared), operation);
  assert.deepEqual(calls[1].options.data, confirmation(prepared));
  for (const [status, code] of [
    [401, 'unauthenticated'],
    [403, 'forbidden'],
    [404, 'case_not_found'],
    [503, 'measure_decision_not_found'],
  ]) {
    const error = Object.assign(new Error(code), { status, code }),
      failed = client(() => {
        throw error;
      });
    await assert.rejects(failed.api.readSubmission(prepared), (failure) => failure === error);
    assert.equal(failed.calls.length, 1);
  }
});

test('decision pending requests retain command principal and prepared bytes and reject disposed responses', async () => {
  const prepared = preparedDecision(),
    operation = measureDecisionOperation();
  for (const method of ['prepare', 'submit', 'readSubmission', 'get', 'list']) {
    const retained = clone(prepared);
    let release;
    const { api, calls } = client(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const args =
      method === 'prepare'
        ? [retained.review.command, retained.review.actor]
        : method === 'get'
          ? [retained.review.command.decision_id]
          : method === 'list'
            ? []
            : [retained];
    const pending = api[method](...args);
    retained.review.command.values.authority = 'Changed after request';
    retained.review.actor.email = 'changed@example.test';
    api.dispose();
    release(
      clone(
        method === 'prepare'
          ? prepared
          : method === 'list'
            ? { case_id: measureCaseId, items: [operation], has_more: false, next_after_id: null }
            : operation,
      ),
    );
    await assert.rejects(pending);
    await assert.rejects(api[method](...args));
    assert.equal(calls.length, 1);
    if (method === 'prepare') assert.deepEqual(calls[0].options.data, prepared.review.command);
    if (method === 'submit') assert.deepEqual(calls[0].options.data, confirmation(prepared));
  }
  let reject;
  const late = client(
    () =>
      new Promise((_, fail) => {
        reject = fail;
      }),
  );
  const pending = late.api.readSubmission(prepared);
  late.api.dispose();
  reject(Object.assign(new Error('Missing'), { status: 404, code: 'measure_decision_not_found' }));
  await assert.rejects(pending);
});
