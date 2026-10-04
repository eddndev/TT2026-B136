import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingDerivedDeadlinesApi } from '../src/lib/hearing-derived-deadline-api.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  ids,
  digest,
  clone,
} from './fixtures/hearing-derived-deadline-unit.mjs';

function client(request, ready = derivedReady()) {
  return hearingDerivedDeadlinesApi(
    request,
    ready.command.case_id,
    ready.command.result.hearing_id,
  );
}
test('prepares a prospective source and submits only one compound instruction', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready),
    calls = [];
  const api = client(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return clone(path.endsWith('/prepare') ? ready : record);
  });
  assert.deepEqual(await api.prepare(ready.command, principal()), ready);
  assert.deepEqual(await api.submit(ready, principal()), record);
  const base = `/cases/${ready.command.case_id}/hearings/${ready.command.result.hearing_id}/results/derived-deadline`;
  assert.deepEqual(
    calls.map((c) => c.path),
    [`${base}/prepare`, `${base}/submit`],
  );
  assert.ok(calls.every((c) => c.options.method === 'POST'));
  assert.deepEqual(calls[1].options.data, {
    command: ready.command,
    expected_review_digest: ready.review_digest,
  });
  assert.notEqual(ready.review_digest, record.deadline.receipt.submission_digest);
  assert.equal(record.origin.source_event.sequence, '9007199254740993');
  assert.equal(ready.deadline.result.due_at, null);
  assert.ok(ready.deadline.result.blocks.length);
  for (const field of ['recorded_at', 'source_event', 'capture_digest', 'receipt'])
    assert.equal(Object.hasOwn(ready.deadline, field), false);
});
test('prepare returns historical Replay without reading current sources or writing again', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready),
    calls = [];
  const current = { ...principal(), email: 'current@example.test', role: 'litigator' };
  const api = client(async (path, options) => {
    calls.push({ path, options });
    return { state: 'replay', record: clone(record) };
  });
  const response = await api.prepare(ready.command, current);
  assert.deepEqual(response, { state: 'replay', record });
  assert.equal(response.record.origin.recorded_by.email, principal().email);
  assert.equal(response.record.origin.recorded_by.role, 'owner');
  assert.equal(calls.length, 1);
  assert.match(calls[0].path, /\/derived-deadline\/prepare$/);
  assert.equal(calls[0].options.method, 'POST');
});
test('explicit same-envelope submission retains original authority after account changes', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready),
    calls = [];
  const api = client(async (path, options) => {
    calls.push(clone(options.data));
    return clone(record);
  });
  const current = { ...principal(), email: 'new@example.test', role: 'litigator' };
  assert.deepEqual(await api.submit(ready, current), record);
  assert.deepEqual(await api.submit(ready, current), record);
  assert.deepEqual(calls[0], calls[1]);
  assert.deepEqual(calls[0], {
    command: ready.command,
    expected_review_digest: ready.review_digest,
  });
});
test('rejects wrong actors, denied roles and invalid command scope before transport', async () => {
  const ready = derivedReady();
  let calls = 0;
  const api = client(async () => {
    calls++;
    return clone(ready);
  });
  for (const actor of [
    null,
    { ...principal(), role: 'client' },
    { ...principal(), role: 'paralegal' },
  ])
    await assert.rejects(api.prepare(ready.command, actor));
  for (const [name, mutate] of [
    [
      'case',
      (c) => {
        c.case_id = ids(99);
      },
    ],
    [
      'hearing',
      (c) => {
        c.result.hearing_id = ids(99);
      },
    ],
    [
      'selection case',
      (c) => {
        c.deadline.change.definition.input.selection.case_id = ids(99);
      },
    ],
    [
      'unknown source',
      (c) => {
        c.deadline.change.definition.input.selection.source = {
          kind: 'unknown',
          reason: 'Unknown',
        };
      },
    ],
    [
      'foreign result',
      (c) => {
        c.deadline.change.definition.input.selection.source.value.result_id = ids(99);
      },
    ],
    [
      'wrong source hearing',
      (c) => {
        c.deadline.change.definition.input.selection.source.value.hearing_id = ids(99);
      },
    ],
    [
      'later revision',
      (c) => {
        c.deadline.change.definition.input.selection.source.value.revision = 2;
      },
    ],
    [
      'foreign agreement',
      (c) => {
        c.deadline.change.definition.input.selection.source.value.agreement_id = ids(99);
      },
    ],
    [
      'correction',
      (c) => {
        c.result.change.action = 'correct';
        c.result.change.expected_revision = 1;
      },
    ],
    [
      'deadline correction',
      (c) => {
        c.deadline.change.action = 'correct';
        c.deadline.change.expected_revision = 1;
      },
    ],
    [
      'missing policies',
      (c) => {
        delete c.deadline.change.tracking;
      },
    ],
    [
      'extra field',
      (c) => {
        c.expected_submission_digest = digest('1');
      },
    ],
  ]) {
    const command = clone(ready.command);
    mutate(command);
    await assert.rejects(api.prepare(command, principal()), name);
  }
  assert.equal(calls, 0);
  await assert.rejects(api.submit(ready, { ...principal(), id: ids(99) }));
  assert.equal(calls, 0);
});
test('rejects contradictory or fabricated Ready evidence', async () => {
  const ready = derivedReady();
  for (const [name, mutate] of [
    [
      'actor',
      (r) => {
        r.result.actor_id = ids(99);
      },
    ],
    [
      'command values',
      (r) => {
        r.command.result.change.values.summary = 'Changed';
      },
    ],
    [
      'draft values',
      (r) => {
        r.result.values.summary = 'Changed';
      },
    ],
    [
      'draft operation',
      (r) => {
        r.result.command.operation_id = ids(99);
      },
    ],
    [
      'anchor',
      (r) => {
        r.result.anchor.revision++;
      },
    ],
    [
      'deadline definition',
      (r) => {
        r.deadline.definition.title = 'Changed';
      },
    ],
    [
      'responsible',
      (r) => {
        r.deadline.responsible.id = ids(99);
      },
    ],
    [
      'policy',
      (r) => {
        r.deadline.tracking.profile = 'follow';
      },
    ],
    [
      'profile',
      (r) => {
        r.deadline.profile.id = ids(99);
      },
    ],
    [
      'profile head',
      (r) => {
        r.deadline.profile_head.definition_digest = digest('1');
      },
    ],
    [
      'missing calendar',
      (r) => {
        delete r.deadline.calendar;
      },
    ],
    [
      'invalid calculation',
      (r) => {
        r.deadline.result.blocks = 'blocked';
      },
    ],
    [
      'review digest',
      (r) => {
        r.review_digest = 'invalid';
      },
    ],
    [
      'invented capture',
      (r) => {
        r.deadline.capture_digest = digest('1');
      },
    ],
    [
      'invented source',
      (r) => {
        r.deadline.source_event = {};
      },
    ],
    [
      'mixed union',
      (r) => {
        r.record = derivedRecord(ready);
      },
    ],
  ]) {
    const reply = clone(ready);
    mutate(reply);
    await assert.rejects(client(async () => reply).prepare(ready.command, principal()), name);
  }
});
test('equal instants with different offsets are different instructions', async () => {
  const ready = derivedReady(),
    command = clone(ready.command);
  command.result.change.values.event_time.at = '2026-09-01T16:00:00Z';
  for (const reply of [ready, { state: 'replay', record: derivedRecord(ready) }])
    await assert.rejects(client(async () => clone(reply)).prepare(command, principal()));
});
test('wrong union variants and missing exact records are rejected', async () => {
  const ready = derivedReady();
  for (const reply of [
    null,
    [],
    {},
    { state: 'unknown' },
    { state: 'replay' },
    { state: 'ready', record: derivedRecord(ready) },
  ])
    await assert.rejects(client(async () => reply).prepare(ready.command, principal()));
  let calls = 0;
  const api = client(async () => {
    calls++;
    return derivedRecord(ready);
  });
  await assert.rejects(api.submit({ state: 'replay', record: derivedRecord(ready) }, principal()));
  assert.equal(calls, 0);
});
test('does not accept disposed responses or exceed the compound JSON budget', async () => {
  const ready = derivedReady();
  let resolve;
  const api = client(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const pending = api.prepare(ready.command, principal());
  api.dispose();
  resolve(clone(ready));
  await assert.rejects(pending);
  await assert.rejects(api.submit(ready, principal()));
  let calls = 0;
  const limited = client(async () => {
    calls++;
    return clone(ready);
  });
  const command = clone(ready.command);
  command.result.change.values.summary = 'x'.repeat(1024 * 1024);
  await assert.rejects(limited.prepare(command, principal()));
  assert.equal(calls, 0);
});
