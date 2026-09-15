import test from 'node:test';
import assert from 'node:assert/strict';
import { createApi } from '../src/lib/api.mjs';
import { declaredDate, dateInterval, dateLabel } from '../src/lib/stage-dates.mjs';
import { stageDraft, stagePayload, stageAction } from '../src/lib/case-stages.mjs';
const now = Date.parse('2026-09-15T12:00:00Z');
const date = { precision: 'date', date: '2026-09-01', time: '', offset: '-06:00' };
const support = {
  id: 'document',
  case_id: 'case',
  version: 1,
  digest: 'a'.repeat(64),
  name: 'A.pdf',
};
test('declared date preserves precision and explicit offset without inventing an instant', () => {
  assert.deepEqual(declaredDate(date, now), {
    precision: 'date',
    date: date.date,
    offset: '-06:00',
  });
  assert.deepEqual(declaredDate({ ...date, precision: 'instant', time: '10:30' }, now), {
    precision: 'instant',
    at: '2026-09-01T10:30:00-06:00',
  });
  assert.match(dateLabel(declaredDate(date, now)), /sin hora/);
  assert.match(dateLabel(declaredDate(date, now)), /-06:00/);
});
test('date intervals reject invalid calendars, unknown offsets and entirely future declarations', () => {
  for (const extra of [
    { date: '2026-02-30' },
    { offset: '-00:00' },
    { offset: 'Z' },
    { offset: '+14:01' },
    { offset: '' },
    { date: '2026-09-16' },
    { precision: 'instant', time: '' },
  ])
    assert.throws(() => declaredDate({ ...date, ...extra }, now));
  const today = declaredDate({ ...date, date: '2026-09-15', offset: '-06:00' }, now);
  const interval = dateInterval(today);
  assert.equal(interval[1] - interval[0], 86400000 - 1);
  assert.equal(interval[0], Date.parse('2026-09-15T06:00:00Z'));
});
test('stage payloads contain only their variant fields and exact immutable references', () => {
  const draft = stageDraft(null);
  Object.assign(draft, { stage: 'intermediate', known_at: date, reason: ' Motivo ', support });
  assert.deepEqual(stagePayload(draft, null, now), {
    expected_revision: 0,
    stage: 'intermediate',
    known_at: declaredDate(date, now),
    reason: 'Motivo',
    support: { document_id: 'document', version: 1, digest: support.digest },
  });
  const current = { stage: 'investigation', stage_revision: 2 };
  const next = stageDraft(current);
  Object.assign(next, { accusation_declared_at: date, accusation: support });
  assert.deepEqual(stagePayload(next, current, now), {
    expected_revision: 2,
    target: 'intermediate',
    accusation_declared_at: declaredDate(date, now),
    accusation: { document_id: 'document', version: 1, digest: support.digest },
  });
  assert.equal(stageAction({ stage: 'trial' }), null);
});
test('trial order uses date intervals and never copies emission into receipt', () => {
  const current = { stage: 'intermediate', stage_revision: 2 };
  const draft = stageDraft(current);
  assert.equal(draft.received_at.date, '');
  Object.assign(draft, {
    opening_order_issued_at: { ...date, precision: 'instant', time: '18:00' },
    received_at: date,
    receiving_court: ' Tribunal ',
    opening_order: support,
  });
  assert.equal(stagePayload(draft, current, now).receiving_court, 'Tribunal');
  draft.received_at = { ...date, date: '2026-08-31' };
  assert.throws(() => stagePayload(draft, current, now));
});
test('stage text limits reject controls before trim and count Unicode scalar values', () => {
  const draft = stageDraft(null);
  Object.assign(draft, { known_at: date, reason: '\u{1f642}'.repeat(1000), support });
  assert.equal([...stagePayload(draft, null, now).reason].length, 1000);
  draft.reason += 'a';
  assert.throws(() => stagePayload(draft, null, now));
  draft.reason = '\tA';
  assert.throws(() => stagePayload(draft, null, now));
});
test('stage facade uses four exact routes, cursor and body with contextual validation', async () => {
  const calls = [];
  const api = createApi(async (path, options) => {
    calls.push({ path, options });
    return new Response(
      JSON.stringify(
        path.includes('history')
          ? { entries: [], has_more: false, next_before_revision: null }
          : { case_id: 'case', current: null },
      ),
    );
  });
  const scope = api.caseStages('case');
  await scope.get();
  await scope.history({ beforeRevision: 3 });
  await scope.adopt({ expected_revision: 0 });
  await scope.transition({ expected_revision: 2 });
  assert.deepEqual(
    calls.map((c) => c.path),
    [
      '/api/v1/cases/case/stage',
      '/api/v1/cases/case/stage/history?limit=20&before_revision=3',
      '/api/v1/cases/case/stage/adoption',
      '/api/v1/cases/case/stage/transitions',
    ],
  );
  assert.deepEqual(JSON.parse(calls[2].options.body), { expected_revision: 0 });
  assert.equal(calls[3].options.method, 'POST');
  scope.dispose();
  await assert.rejects(scope.get());
});
test('stage scope rejects foreign snapshots and late results after disposal', async () => {
  let release;
  const api = createApi(
    () =>
      new Promise((resolve) => {
        release = resolve;
      }),
  );
  const scope = api.caseStages('case');
  const pending = scope.get();
  scope.dispose();
  release(new Response(JSON.stringify({ case_id: 'case', current: null })));
  await assert.rejects(pending);
  const foreign = createApi(
    async () => new Response(JSON.stringify({ case_id: 'other', current: null })),
  );
  await assert.rejects(foreign.caseStages('case').get());
});
test('stage reason and note preserve normalized lines while tribunal stays single line', () => {
  const draft = stageDraft(null);
  Object.assign(draft, { known_at: date, support, reason: ' Una\r\nOtra\n ' });
  assert.equal(stagePayload(draft, null, now).reason, 'Una\nOtra');
  const current = { stage: 'investigation', stage_revision: 1 };
  Object.assign(draft, { accusation_declared_at: date, accusation: support, note: ' Uno\r\nDos ' });
  assert.equal(stagePayload(draft, current, now).note, 'Uno\nDos');
  Object.assign(draft, {
    opening_order_issued_at: date,
    opening_order: support,
    received_at: date,
    receiving_court: 'Uno\nDos',
  });
  assert.throws(() => stagePayload(draft, { stage: 'intermediate', stage_revision: 2 }, now));
});
