import test from 'node:test';
import assert from 'node:assert/strict';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  measureCaseId,
  administrationBase as base,
  foreignMeasureId,
  laterOperationId,
  measureAdministrationOperation,
  preparedAdministration,
  administrationClient,
  clone,
} from './fixtures/measure-administrations.mjs';

const client = (reply) => administrationClient(caseApi, reply);
const confirmation = (review) => ({
  command: review.command,
  expected_submission_digest: review.submission_digest,
  expected_review_digest: review.review_digest,
});
const page = (items = [], hasMore = false) => ({
  case_id: measureCaseId,
  items,
  has_more: hasMore,
  next_after_operation_id: hasMore ? items.at(-1).origin.operation_id : null,
});

test('administrative actions prepare a direct review and preserve original captures and histories', async () => {
  for (const action of ['correct', 'entered_in_error', 'replace_entered_in_error']) {
    const prepared = preparedAdministration({ action });
    const operation = measureAdministrationOperation({ action });
    const read = client(prepared);
    assert.deepEqual(await read.api.prepare(prepared.command, prepared.actor), prepared);
    assert.deepEqual(read.calls, [
      { path: `${base}/prepare`, options: { method: 'POST', data: prepared.command } },
    ]);
    const write = client(operation);
    assert.deepEqual(await write.api.submit(prepared), operation);
    assert.deepEqual(write.calls[0], {
      path: `${base}/submit`,
      options: { method: 'POST', data: confirmation(prepared) },
    });
    assert.deepEqual(await write.api.readSubmission(prepared), { state: 'confirmed', operation });
    assert.deepEqual(await write.api.get(prepared.command.operation_id), operation);
    assert.deepEqual(write.calls.slice(1), [
      { path: `${base}/${prepared.command.operation_id}`, options: undefined },
      { path: `${base}/${prepared.command.operation_id}`, options: undefined },
    ]);
    assert.equal(operation.record_history.records.administrative.length, 0);
    assert.equal(operation.record_history.records.judicial.groups.length, 1);
    assert.equal(operation.capture.records.length, action === 'replace_entered_in_error' ? 2 : 1);
    assert.equal(operation.capture.records[0].result.values.validity.start.offset_seconds, null);
  }
});

test('administrative preparation retains normalized commands and requires full managing principal', async () => {
  const prepared = preparedAdministration(),
    command = clone(prepared.command);
  command.reason = ` ${command.reason} `;
  const original = clone(command),
    transport = client(prepared);
  assert.deepEqual(await transport.api.prepare(command, prepared.actor), prepared);
  assert.deepEqual(command, original);
  assert.deepEqual(transport.calls[0].options.data, prepared.command);
  const litigator = preparedAdministration();
  litigator.actor.role = 'litigator';
  assert.deepEqual(
    await client(litigator).api.prepare(litigator.command, litigator.actor),
    litigator,
  );
  for (const change of [
    (v) => {
      v.case_id = foreignMeasureId;
    },
    (v) => {
      v.operation_id = v.operation_id.toUpperCase();
    },
    (v) => {
      v.target.revision = 0;
    },
    (v) => {
      v.context.stage_revision = 0;
    },
    (v) => {
      v.action.kind = 'revoke';
    },
    (v) => {
      v.actor = prepared.actor;
    },
  ]) {
    const invalid = clone(prepared.command),
      rejected = client(prepared);
    change(invalid);
    await assert.rejects(rejected.api.prepare(invalid, prepared.actor));
    assert.equal(rejected.calls.length, 0);
  }
  for (const role of ['paralegal', 'client']) {
    const rejected = client(prepared);
    await assert.rejects(rejected.api.prepare(prepared.command, { ...prepared.actor, role }));
    assert.equal(rejected.calls.length, 0);
  }
  for (const change of [
    (v) => {
      v.actor.id = foreignMeasureId;
    },
    (v) => {
      v.actor.email = 'another@example.test';
    },
    (v) => {
      v.actor.role = 'litigator';
    },
    (v) => {
      v.command.operation_id = foreignMeasureId;
    },
    (v) => {
      v.result.previous.capture_digest = 'f'.repeat(64);
    },
    (v) => {
      v.case_id = foreignMeasureId;
    },
  ]) {
    const invalid = clone(prepared);
    change(invalid);
    await assert.rejects(client(invalid).api.prepare(prepared.command, prepared.actor));
  }
});

test('administrative lists use the original operation cursor and reject inconsistent pages', async () => {
  const first = measureAdministrationOperation();
  const second = measureAdministrationOperation({ operationId: laterOperationId });
  const expected = page([first, second], true),
    transport = client(expected);
  assert.deepEqual(await transport.api.list({ limit: 2 }), expected);
  assert.equal(transport.calls[0].path, `${base}?limit=2`);
  const next = client(page());
  assert.deepEqual(await next.api.list({ afterId: laterOperationId }), page());
  assert.equal(next.calls[0].path, `${base}?limit=10&after_operation_id=${laterOperationId}`);
  await assert.rejects(client(first).api.get(laterOperationId));
  for (const query of [{ limit: 0 }, { limit: 21 }, { afterId: null }, { action: 'correct' }]) {
    const rejected = client(expected);
    await assert.rejects(rejected.api.list(query));
    assert.equal(rejected.calls.length, 0);
  }
  for (const change of [
    (v) => {
      v.case_id = foreignMeasureId;
    },
    (v) => {
      v.items.reverse();
    },
    (v) => {
      v.next_after_operation_id = first.origin.operation_id;
    },
    (v) => {
      v.items[1] = clone(v.items[0]);
    },
  ]) {
    const invalid = clone(expected);
    change(invalid);
    await assert.rejects(client(invalid).api.list({ limit: 2 }));
  }
});

test('administrative confirmations bind both digests original rows and the explicit replacement link', async () => {
  const prepared = preparedAdministration({ action: 'replace_entered_in_error' });
  for (const method of ['submit', 'readSubmission']) {
    for (const change of [
      (v) => {
        v.origin.operation_id = foreignMeasureId;
      },
      (v) => {
        v.origin.submission_digest = 'f'.repeat(64);
      },
      (v) => {
        v.origin.review_digest = 'f'.repeat(64);
      },
      (v) => {
        v.capture.records[0].actor.email = 'another@example.test';
      },
      (v) => {
        v.capture.records[0].result.values.conditions = 'Changed capture';
      },
      (v) => {
        v.capture.replacement_link = null;
      },
      (v) => {
        v.capture.replacement_link.replacement.id = foreignMeasureId;
      },
      (v) => {
        v.record_history.records.administrative = null;
      },
    ]) {
      const invalid = measureAdministrationOperation({ action: 'replace_entered_in_error' });
      change(invalid);
      await assert.rejects(client(invalid).api[method](prepared), change.toString());
    }
    const invalid = preparedAdministration(),
      rejected = client(measureAdministrationOperation());
    invalid.command.action.values.conditions = 'Edited without another preparation';
    await assert.rejects(rejected.api[method](invalid));
    assert.equal(rejected.calls.length, 0);
  }
});

test('administrative recovery only reports exact absence and never resends automatically', async () => {
  const prepared = preparedAdministration(),
    operation = measureAdministrationOperation();
  const missing = Object.assign(new Error('Missing'), {
    status: 404,
    code: 'measure_administrative_not_found',
  });
  const transport = client((path, options) => {
    if (!options) throw missing;
    return clone(operation);
  });
  assert.deepEqual(await transport.api.readSubmission(prepared), { state: 'unconfirmed' });
  assert.deepEqual(transport.calls, [
    { path: `${base}/${prepared.command.operation_id}`, options: undefined },
  ]);
  assert.deepEqual(await transport.api.submit(prepared), operation);
  assert.deepEqual(transport.calls[1].options.data, confirmation(prepared));
  for (const [status, code] of [
    [401, 'invalid_session'],
    [403, 'permission_denied'],
    [404, 'case_not_found'],
    [404, 'measure_record_not_found'],
    [503, 'measure_administrative_not_found'],
  ]) {
    const error = Object.assign(new Error(code), { status, code });
    const failed = client(() => {
      throw error;
    });
    await assert.rejects(failed.api.readSubmission(prepared), (failure) => failure === error);
    assert.equal(failed.calls.length, 1);
  }
});

test('pending administration calls retain their inputs and disposal rejects stale success or absence', async () => {
  const prepared = preparedAdministration(),
    operation = measureAdministrationOperation();
  for (const method of ['prepare', 'submit', 'readSubmission']) {
    const retained = clone(prepared);
    let release;
    const transport = client(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const args = method === 'prepare' ? [retained.command, retained.actor] : [retained];
    const pending = transport.api[method](...args);
    retained.command.reason = 'Changed while awaiting response';
    retained.actor.email = 'changed@example.test';
    release(clone(method === 'prepare' ? prepared : operation));
    assert.deepEqual(
      await pending,
      method === 'prepare'
        ? prepared
        : method === 'submit'
          ? operation
          : { state: 'confirmed', operation },
    );
    if (method === 'prepare') assert.deepEqual(transport.calls[0].options.data, prepared.command);
    if (method === 'submit')
      assert.deepEqual(transport.calls[0].options.data, confirmation(prepared));
  }
  for (const method of ['prepare', 'submit', 'readSubmission', 'get', 'list']) {
    let release;
    const transport = client(
      () =>
        new Promise((resolve) => {
          release = resolve;
        }),
    );
    const args =
      method === 'prepare'
        ? [prepared.command, prepared.actor]
        : method === 'get'
          ? [prepared.command.operation_id]
          : method === 'list'
            ? []
            : [prepared];
    const pending = transport.api[method](...args);
    transport.api.dispose();
    release(
      clone(method === 'prepare' ? prepared : method === 'list' ? page([operation]) : operation),
    );
    await assert.rejects(pending);
    await assert.rejects(transport.api[method](...args));
    assert.equal(transport.calls.length, 1);
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
  reject(
    Object.assign(new Error('Missing'), { status: 404, code: 'measure_administrative_not_found' }),
  );
  await assert.rejects(pending);
});
