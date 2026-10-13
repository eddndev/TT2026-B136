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

function client(reply, ready) {
  return hearingDerivedDeadlinesApi(
    async () => clone(reply),
    ready.command.case_id,
    ready.command.result.hearing_id,
  );
}
test('both Replay and submit require exact component identities and a consistent joint origin', async () => {
  const ready = derivedReady();
  for (const [name, mutate] of [
    [
      'case',
      (r) => {
        r.case_id = ids(99);
      },
    ],
    [
      'command',
      (r) => {
        r.command.deadline.change.definition.title = 'Changed';
      },
    ],
    [
      'result id',
      (r) => {
        r.result.id = ids(99);
      },
    ],
    [
      'result revision',
      (r) => {
        r.result.revision = 2;
      },
    ],
    [
      'result operation',
      (r) => {
        r.result.receipt.operation_id = ids(99);
      },
    ],
    [
      'result values digest',
      (r) => {
        r.result.values_digest = digest('1');
      },
    ],
    [
      'result submission digest',
      (r) => {
        r.result.receipt.submission_digest = digest('1');
      },
    ],
    [
      'result values',
      (r) => {
        r.result.values.summary = 'Changed';
      },
    ],
    [
      'result author',
      (r) => {
        r.result.recorded_by.id = ids(99);
      },
    ],
    [
      'result email',
      (r) => {
        r.result.recorded_by.email = 'different@example.test';
      },
    ],
    [
      'deadline id',
      (r) => {
        r.deadline.id = ids(99);
      },
    ],
    [
      'deadline revision',
      (r) => {
        r.deadline.revision = 2;
      },
    ],
    [
      'deadline operation',
      (r) => {
        r.deadline.receipt.operation_id = ids(99);
      },
    ],
    [
      'deadline definition',
      (r) => {
        r.deadline.definition.title = 'Changed';
      },
    ],
    [
      'deadline author',
      (r) => {
        r.deadline.recorded_by.id = ids(99);
      },
    ],
    [
      'deadline email',
      (r) => {
        r.deadline.recorded_by.email = 'different@example.test';
      },
    ],
    [
      'captured source',
      (r) => {
        r.deadline.calculation.material.source.reference.result_id = ids(99);
      },
    ],
    [
      'source head',
      (r) => {
        r.deadline.calculation.material.source_head.submission_digest = digest('1');
      },
    ],
    [
      'administration',
      (r) => {
        r.result.recorded_administration_digest = digest('1');
      },
    ],
    [
      'capture time',
      (r) => {
        r.deadline.recorded_at.nanosecond++;
      },
    ],
    [
      'origin case',
      (r) => {
        r.origin.case_id = ids(99);
      },
    ],
    [
      'origin result',
      (r) => {
        r.origin.result_id = ids(99);
      },
    ],
    [
      'origin result revision',
      (r) => {
        r.origin.result_revision = 2;
      },
    ],
    [
      'origin result operation',
      (r) => {
        r.origin.result_operation_id = ids(99);
      },
    ],
    [
      'origin deadline',
      (r) => {
        r.origin.deadline_id = ids(99);
      },
    ],
    [
      'origin deadline revision',
      (r) => {
        r.origin.deadline_revision = 2;
      },
    ],
    [
      'origin deadline operation',
      (r) => {
        r.origin.deadline_operation_id = ids(99);
      },
    ],
    [
      'origin actor',
      (r) => {
        r.origin.recorded_by.id = ids(99);
      },
    ],
    [
      'origin role',
      (r) => {
        r.origin.recorded_by.role = 'client';
      },
    ],
    [
      'origin digest',
      (r) => {
        r.origin.capture_digest = digest('1');
      },
    ],
    [
      'origin review',
      (r) => {
        r.origin.review_digest = digest('1');
      },
    ],
    [
      'ordinary pair',
      (r) => {
        delete r.origin;
      },
    ],
    [
      'unexpected field',
      (r) => {
        r.extra = true;
      },
    ],
  ]) {
    const record = derivedRecord(ready);
    mutate(record);
    await assert.rejects(
      client({ state: 'replay', record }, ready).prepare(ready.command, principal()),
      `Replay: ${name}`,
    );
    await assert.rejects(client(record, ready).submit(ready, principal()), `submit: ${name}`);
  }
});
test('source event remains a bounded decimal string and must identify the exact HRES R1', async () => {
  const ready = derivedReady();
  for (const sequence of ['1', '9007199254740993', '9223372036854775807']) {
    const record = derivedRecord(ready);
    record.origin.source_event.sequence = sequence;
    const result = await client(record, ready).submit(ready, principal());
    assert.equal(result.origin.source_event.sequence, sequence);
    assert.equal(typeof result.origin.source_event.sequence, 'string');
  }
  for (const sequence of [
    0,
    1,
    9007199254740992,
    '',
    '0',
    '01',
    '-1',
    '1.0',
    '1e3',
    '9223372036854775808',
  ]) {
    const record = derivedRecord(ready);
    record.origin.source_event.sequence = sequence;
    await assert.rejects(client(record, ready).submit(ready, principal()), String(sequence));
  }
  for (const [field, value] of [
    ['family', 'resolution'],
    ['source_id', ids(99)],
    ['revision', 0],
    ['revision', 2],
    ['case_id', ids(99)],
    ['case_id', null],
    ['hearing_id', ids(99)],
    ['hearing_id', null],
    ['operation_id', ids(99)],
  ]) {
    const record = derivedRecord(ready);
    record.origin.source_event[field] = value;
    await assert.rejects(client(record, ready).submit(ready, principal()), field);
  }
});
test('Replay needs the authenticated actor ID but preserves historical email and role', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready);
  const reply = { state: 'replay', record };
  await assert.rejects(
    client(reply, ready).prepare(ready.command, { ...principal(), id: ids(99) }),
  );
  for (const role of ['client', 'paralegal'])
    await assert.rejects(client(reply, ready).prepare(ready.command, { ...principal(), role }));
  const recovered = await client(reply, ready).prepare(ready.command, {
    ...principal(),
    role: 'litigator',
    email: 'new@example.test',
  });
  assert.deepEqual(recovered.record.origin.recorded_by, principal());
});
test('submit compares approved review separately from final deadline receipt and final capture', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready);
  const changed = clone(record);
  changed.review_digest = changed.origin.review_digest = digest('1');
  await assert.rejects(client(changed, ready).submit(ready, principal()));
  assert.deepEqual(await client(record, ready).submit(ready, principal()), record);
  assert.notEqual(record.review_digest, record.deadline.receipt.review_digest);
  assert.notEqual(record.capture_digest, record.deadline.receipt.capture_digest);
});
test('returned actual result offsets are not replaced by equal UTC instants', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready);
  record.result.values.event_time.at = '2026-09-01T16:00:00Z';
  await assert.rejects(
    client({ state: 'replay', record }, ready).prepare(ready.command, principal()),
  );
  await assert.rejects(client(record, ready).submit(ready, principal()));
});
test('accepts a shared capture offset without losing nanoseconds', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready);
  record.result.recorded_at = '2026-09-16T06:00:00.123456789-06:00';
  record.deadline.recorded_at.offset_seconds = -21600;
  assert.deepEqual(await client(record, ready).submit(ready, principal()), record);
});
test('rejects equal capture instants with discordant offsets', async () => {
  const ready = derivedReady(),
    record = derivedRecord(ready);
  record.deadline.recorded_at.offset_seconds = -21600;
  await assert.rejects(client(record, ready).submit(ready, principal()));
});
