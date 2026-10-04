import test from 'node:test';
import assert from 'node:assert/strict';
import { caseApi } from '../src/lib/case-api.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  clone,
} from './fixtures/hearing-derived-deadline-unit.mjs';

test('case closure notifies the shared watcher and still permits explicit origin recovery', async () => {
  const ready = derivedReady();
  const record = derivedRecord(ready);
  const calls = [];
  const closed = Object.assign(new Error('Case closed'), { status: 409, code: 'case_closed' });
  const api = caseApi(async (path, options) => {
    calls.push({ path, options: clone(options) });
    if (calls.length === 1) throw closed;
    return { state: 'replay', record: clone(record) };
  });
  const notifications = [];
  const unwatch = api.watchCase(ready.command.case_id, (error) => notifications.push(error));
  const scoped = api.caseHearingDerivedDeadlines(
    ready.command.case_id,
    ready.command.result.hearing_id,
  );
  await assert.rejects(scoped.prepare(ready.command, principal()), (error) => error === closed);
  assert.deepEqual(notifications, [closed]);
  assert.deepEqual(await scoped.prepare(ready.command, principal()), { state: 'replay', record });
  assert.equal(calls.length, 2);
  for (const call of calls) {
    assert.equal(
      call.path,
      `/cases/${ready.command.case_id}/hearings/${ready.command.result.hearing_id}/results/derived-deadline/prepare`,
    );
    assert.equal(call.options.method, 'POST');
    assert.deepEqual(call.options.data, ready.command);
  }
  unwatch();
  scoped.dispose();
  await assert.rejects(scoped.prepare(ready.command, principal()));
  assert.equal(calls.length, 2);
});
