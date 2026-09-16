import test from 'node:test';
import assert from 'node:assert/strict';
import { judicialCalendarsApi } from '../src/lib/judicial-calendars-api.mjs';
import {
  calendarId as id,
  calendarHash as hash,
  calendarPrepared,
  calendarRecord,
  calendarFixtureCommand,
  calendarOverview,
  calendarHistoryRow,
  calendarDayRows,
} from './fixtures/judicial-calendars.mjs';
test('global API sends exact routes queries and prepare-submit envelope without case context', async () => {
  const calls = [],
    prepared = calendarPrepared(),
    record = calendarRecord();
  const api = judicialCalendarsApi(async (path, options) => {
    calls.push({ path, options });
    if (path.endsWith('/prepare')) return prepared;
    if (options) return record;
    if (path.includes('/days?'))
      return {
        calendar_id: id,
        revision: 1,
        values_digest: hash,
        days: calendarDayRows(record.values, '2000-02-28', '2000-03-01'),
      };
    if (path.includes('/history?'))
      return {
        revisions: [calendarHistoryRow(record)],
        has_more: false,
        next_before_revision: null,
      };
    if (path.startsWith('/judicial-calendars?'))
      return { calendars: [calendarOverview(record)], has_more: false, next_after_id: null };
    return record;
  });
  await api.list({ status: 'all', jurisdiction: 'local', entityCode: '01', afterId: id });
  await api.history(id, { limit: 20, beforeRevision: 2 });
  await api.days(id, 1, { from: '2000-02-28', through: '2000-03-01' }, hash);
  await api.prepare(prepared.command, 'user');
  await api.submit(prepared);
  assert.match(calls[0].path, /^\/judicial-calendars\?/);
  assert.match(calls[0].path, /entity_code=01/);
  assert.match(calls[0].path, /after_id=/);
  assert.match(calls[1].path, /before_revision=2/);
  assert.equal(calls[4].options.method, 'POST');
  assert.deepEqual(calls[4].options.data, {
    command: prepared.command,
    expected_submission_digest: hash,
  });
  assert.ok(calls.every((c) => !c.path.includes('/cases/')));
});
test('prepared command values actor and scope must match normalized requested data', async () => {
  for (const change of [
    (v) => (v.actor_id = 'other'),
    (v) => (v.command.change.values.scope.title = 'changed'),
    (v) => (v.values.coverage.through = '2000-03-03'),
    (v) => (v.initial_scope.organ = 'other'),
    (v) => (v.result_revision = 2),
    (v) => (v.submission_digest = 'wrong'),
  ]) {
    const p = calendarPrepared();
    change(p);
    const api = judicialCalendarsApi(async () => p);
    await assert.rejects(api.prepare(calendarFixtureCommand(), 'user'));
  }
});
test('exact response identities and day ranges are validated without browser classification', async () => {
  const bad = { ...calendarRecord(), revision: 2 };
  await assert.rejects(judicialCalendarsApi(async () => bad).revision(id, 1));
  for (const change of [
    (v) => (v.calendar_id = 'other'),
    (v) => (v.values_digest = '55'.repeat(32)),
    (v) => v.days.pop(),
    (v) => (v.days[0].state = 'working_day'),
  ]) {
    const record = calendarRecord(),
      value = {
        calendar_id: id,
        revision: 1,
        values_digest: hash,
        days: calendarDayRows(record.values, '2000-02-28', '2000-03-01'),
      };
    change(value);
    await assert.rejects(
      judicialCalendarsApi(async () => value).days(
        id,
        1,
        { from: '2000-02-28', through: '2000-03-01' },
        hash,
      ),
    );
  }
});
test('list and history query bounds reject before network and pages stay lightweight', async () => {
  let calls = 0;
  const api = judicialCalendarsApi(async () => {
    calls++;
    return {};
  });
  for (const q of [{ limit: 0 }, { limit: 101 }, { entityCode: '1' }, { status: 'active' }])
    await assert.rejects(api.list(q));
  await assert.rejects(api.history(id, { limit: 21 }));
  await assert.rejects(api.days(id, 1, { from: '2000-01-01', through: '2000-03-03' }));
  assert.equal(calls, 0);
});
test('disposed sessions ignore late calendar responses and requests stay within 1 MiB', async () => {
  let finish,
    calls = 0;
  const api = judicialCalendarsApi(() => {
    calls++;
    return new Promise((r) => (finish = r));
  });
  const pending = api.get(id);
  api.dispose();
  finish(calendarRecord());
  await assert.rejects(pending);
  await assert.rejects(api.get(id));
  assert.equal(calls, 1);
  const large = calendarFixtureCommand();
  large.change.values.scope.title = 'a'.repeat(1024 * 1024);
  await assert.rejects(
    judicialCalendarsApi(async () => {
      throw new Error('must not call');
    }).prepare(large, 'user'),
    /1 MiB/,
  );
});
test('a successful HTTP response with another receipt remains unconfirmed and mutations never retry', async () => {
  let calls = 0;
  const record = calendarRecord();
  record.receipt.operation_id = '11111111-1111-1111-1111-111111111111';
  const api = judicialCalendarsApi(async () => {
    calls++;
    return record;
  });
  await assert.rejects(api.submit(calendarPrepared()), /recibo/);
  assert.equal(calls, 1);
});
