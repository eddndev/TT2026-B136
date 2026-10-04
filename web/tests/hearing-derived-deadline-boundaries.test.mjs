import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { hearingDerivedDeadlinesApi } from '../src/lib/hearing-derived-deadline-api.mjs';
import { deadlineProfileDefinition } from '../src/lib/deadline-profile-values.mjs';
import { calendarValues } from '../src/lib/judicial-calendar-values.mjs';
import {
  derivedReady,
  derivedRecord,
  principal,
  ids,
  digest,
  clone,
} from './fixtures/hearing-derived-deadline-unit.mjs';

const byteLength = (value) => Buffer.byteLength(JSON.stringify(value), 'utf8');
const oneMiB = 1024 * 1024;

function client(request, ready) {
  return hearingDerivedDeadlinesApi(
    request,
    ready.command.case_id,
    ready.command.result.hearing_id,
  );
}

function withAttendee(profile, directoryStatus) {
  const ready = derivedReady();
  const reference = {
    participant_id: ids(80),
    revision: 1,
    capacity: 'Declared capacity',
    observation: null,
  };
  for (const values of [
    ready.command.result.change.values,
    ready.result.command.change.values,
    ready.result.values,
  ])
    values.attendees = [clone(reference)];
  const typed = profile === 'typed';
  ready.result.attendees = [
    {
      id: reference.participant_id,
      revision: reference.revision,
      profile,
      display_name: 'Historical participant',
      procedural_role: typed ? 'defendant' : 'Declared role',
      kind: typed ? 'defendant' : null,
      subject: typed ? { id: ids(81), revision: 1, values_digest: digest('4') } : null,
      values_digest: digest('3'),
      subject_digest: typed ? digest('4') : null,
      directory_status: directoryStatus,
      capacity: reference.capacity,
      observation: reference.observation,
    },
  ];
  return ready;
}

async function responseFor(phase, ready) {
  const record = derivedRecord(ready);
  const reply =
    phase === 'ready' ? ready : phase === 'replay' ? { state: 'replay', record } : record;
  const api = client(async () => clone(reply), ready);
  return phase === 'submit'
    ? api.submit(ready, principal())
    : api.prepare(ready.command, principal());
}

for (const phase of ['ready', 'replay', 'submit']) {
  test(`${phase} preserves archived manual and typed historical attendees`, async () => {
    for (const profile of ['manual', 'typed']) {
      const ready = withAttendee(profile, 'archived');
      const response = await responseFor(phase, ready);
      const result = phase === 'replay' ? response.record.result : response.result;
      assert.deepEqual(result.attendees, ready.result.attendees);
      assert.deepEqual(result.values.attendees, ready.result.values.attendees);
    }
  });

  test(`${phase} rejects retired as an unsupported participant directory status`, async () => {
    for (const profile of ['manual', 'typed'])
      await assert.rejects(responseFor(phase, withAttendee(profile, 'retired')));
  });
}

function withLargeProfile() {
  const vectors = JSON.parse(
    readFileSync(
      new URL('../../crates/domain/tests/fixtures/judicial_calendar_vectors.json', import.meta.url),
      'utf8',
    ),
  );
  const vector = vectors.find((value) => value.name === 'maximum_utf8');
  assert.ok(vector, 'The independent maximum calendar vector must exist.');
  const calendar = clone(vector.normalized);
  assert.deepEqual(calendarValues(calendar), calendar);
  const ready = derivedReady();
  const definition = ready.deadline.profile.definition;
  const example = clone(definition.examples[0]);
  assert.deepEqual(definition.template, {
    kind: 'fixed',
    rule: { kind: 'elapsed_hours', quantity: 24 },
  });
  // Elapsed-hour examples preserve their successful expected instant; calendars are optional.
  definition.examples = Array.from({ length: 16 }, (_, index) => ({
    ...clone(example),
    id: ids(100 + index),
    calendar: clone(calendar),
  }));
  assert.equal(deadlineProfileDefinition(definition, ready.command.case_id), definition);
  assert.ok(byteLength(definition) < 16 * oneMiB);
  ready.deadline.profile_head = clone(ready.deadline.profile);
  return ready;
}

test('a legitimate large Ready submits only the bounded command and review digest', async () => {
  const ready = withLargeProfile();
  const record = derivedRecord(ready);
  const envelope = { command: ready.command, expected_review_digest: ready.review_digest };
  assert.ok(byteLength(ready) > oneMiB);
  assert.ok(byteLength(ready.command) < oneMiB);
  assert.ok(byteLength(envelope) < oneMiB);
  const calls = [];
  const api = client(async (path, options) => {
    calls.push({ path, options: clone(options) });
    return clone(path.endsWith('/prepare') ? ready : record);
  }, ready);
  const preview = await api.prepare(ready.command, principal());
  assert.equal(preview.review_digest, ready.review_digest);
  assert.equal(preview.deadline.profile.definition.examples.length, 16);
  assert.equal(byteLength(preview), byteLength(ready));
  assert.deepEqual(await api.submit(preview, principal()), record);
  assert.equal(calls.length, 2);
  assert.ok(calls[1].path.endsWith('/derived-deadline/submit'));
  assert.equal(calls[1].options.method, 'POST');
  assert.deepEqual(calls[1].options.data, envelope);
  assert.ok(calls.every(({ options }) => byteLength(options.data) < oneMiB));
});
