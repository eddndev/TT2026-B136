import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingDerivedDeadlinesApi } from '../src/lib/hearing-derived-deadline-api.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  clone,
} from './fixtures/hearing-derived-deadline-unit.mjs';

test('edits made while transport is pending cannot replace the sent joint instruction', async () => {
  const ready = derivedReady();
  const record = derivedRecord(ready);
  let complete;
  const calls = [];
  const api = hearingDerivedDeadlinesApi(
    (path, options) => {
      calls.push({ path, options });
      return new Promise((resolve) => {
        complete = resolve;
      });
    },
    ready.command.case_id,
    ready.command.result.hearing_id,
  );
  const command = clone(ready.command);
  const preparation = api.prepare(command, principal());
  command.result.change.values.summary = 'Edited during preparation';
  complete(clone(ready));
  assert.deepEqual(await preparation, ready);
  assert.deepEqual(calls[0].options.data, ready.command);

  const preview = clone(ready);
  const submission = api.submit(preview, principal());
  preview.command.deadline.change.definition.title = 'Edited during submission';
  preview.review_digest = '1'.repeat(64);
  complete(clone(record));
  assert.deepEqual(await submission, record);
  assert.deepEqual(calls[1].options.data, {
    command: ready.command,
    expected_review_digest: ready.review_digest,
  });
  assert.equal(calls.length, 2);
});
