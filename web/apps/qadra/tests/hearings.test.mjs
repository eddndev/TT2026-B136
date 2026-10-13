import test from 'node:test';
import assert from 'node:assert/strict';
import { hearingDraft, hearingCommand, canHearings, hearingKinds } from '../src/lib/hearings.mjs';
import {
  hearingMatches,
  hearingRequest,
  readHearingSubmission,
} from '../src/lib/hearing-submission.mjs';
import { agendaQuery } from '../src/lib/hearing-agenda.mjs';
import {
  hearingContext,
  hearingValues,
  hearingPrepared,
  hearingRecord,
} from './fixtures/hearings.mjs';

test('hearing commands preserve exact participant revisions and normalize only declared text', () => {
  const record = hearingRecord();
  record.values.participants = [
    { participant_id: '20000000-0000-4000-8000-000000000002', revision: 2 },
    { participant_id: '10000000-0000-4000-8000-000000000001', revision: 4 },
  ];
  const draft = hearingDraft(record);
  draft.venue = '  Sala declarada  ';
  draft.note = ' A\r\nB ';
  draft.reason = '  Cambio comunicado ';
  const command = hearingCommand(draft, hearingContext, record, 'replace', 'operation', record.id);
  assert.deepEqual(command.change.values.participants, [...record.values.participants].reverse());
  assert.equal(command.change.values.scheduled_at, hearingValues.scheduled_at);
  assert.equal(command.change.values.venue, 'Sala declarada');
  assert.equal(command.change.values.note, 'A\nB');
  assert.equal(command.change.reason, 'Cambio comunicado');
  assert.equal(record.values.venue, hearingValues.venue);
});

test('hearing kind is immutable and cancellation does not require the current stage context', () => {
  const record = hearingRecord(),
    draft = hearingDraft(record);
  draft.reason = 'Cita cancelada';
  const command = hearingCommand(draft, null, record, 'cancel', 'operation', record.id);
  assert.deepEqual(command.change, {
    action: 'cancel',
    expected_revision: 1,
    reason: 'Cita cancelada',
  });
  draft.kind = 'intermediate';
  assert.throws(
    () => hearingCommand(draft, hearingContext, record, 'replace', 'operation', record.id),
    /tipo/i,
  );
});

test('hearing participant references are bounded, unique and must be exact', () => {
  for (const participants of [
    [{ participant_id: 'a', revision: 0 }],
    [
      { participant_id: 'a', revision: 1 },
      { participant_id: 'a', revision: 2 },
    ],
    Array.from({ length: 33 }, (_, i) => ({ participant_id: String(i), revision: 1 })),
  ]) {
    const draft = hearingDraft();
    Object.assign(draft, {
      kind: 'initial',
      time: { date: '2026-10-01', time: '09:00', offset: '-06:00' },
      venue: 'Sala',
      participants,
    });
    assert.throws(
      () => hearingCommand(draft, hearingContext, null, 'schedule', 'op', 'id'),
      /participante|ficha|referencia/i,
    );
  }
});

test('sentencing requires a declared antecedent and exact support', () => {
  const draft = hearingDraft();
  Object.assign(draft, {
    kind: 'sentencing',
    time: { date: '2026-10-01', time: '09:00', offset: '-06:00' },
    venue: 'Sala',
  });
  const context = { ...hearingContext, stage: 'trial' };
  assert.throws(
    () => hearingCommand(draft, context, null, 'schedule', 'op', 'id'),
    /antecedente|soporte/i,
  );
  draft.statement = 'Antecedente declarado';
  draft.support = { id: 'document', version: 1, digest: 'f'.repeat(64) };
  const command = hearingCommand(draft, context, null, 'schedule', 'op', 'id');
  assert.equal(command.change.values.conviction_basis.support.version, 1);
  assert.equal(command.change.values.conviction_basis.statement, draft.statement);
  assert.equal(hearingKinds.sentencing.stage, 'trial');
});

test('receipt matching binds scope, actor, action, operation, revision, context and digest', () => {
  const prepared = hearingPrepared(),
    record = hearingRecord(prepared);
  assert.equal(hearingMatches(record, prepared), true);
  for (const change of [
    { case_id: 'other' },
    { id: 'other' },
    { revision: 2 },
    { recorded_by: { id: 'other' } },
  ])
    assert.equal(hearingMatches({ ...record, ...change }, prepared), false);
  for (const change of [
    { operation_id: 'other' },
    { action: 'replace' },
    { expected_revision: 5 },
    { submission_digest: 'other' },
    { expected_context: { case_revision: 2, stage_revision: 1 } },
  ])
    assert.equal(
      hearingMatches({ ...record, receipt: { ...record.receipt, ...change } }, prepared),
      false,
    );
});

test('submitted hearing command is copied and exact absence remains uncertain without a mutation', async () => {
  const prepared = hearingPrepared(),
    request = hearingRequest(prepared);
  prepared.command.change.values.venue = 'Edited later';
  assert.equal(request.command.change.values.venue, hearingValues.venue);
  const api = {
    revision: async () => {
      throw Object.assign(new Error('absent'), { status: 404, code: 'hearing_not_found' });
    },
  };
  assert.deepEqual(await readHearingSubmission(api, prepared), { state: 'absent' });
});

test('agenda filters convert explicit local boundaries to UTC with a bounded half-open range', () => {
  assert.deepEqual(
    agendaQuery({ from: '2026-09-15', until: '2026-09-16', offset: '-06:00', status: 'scheduled' }),
    {
      from: '2026-09-15T06:00:00Z',
      until: '2026-09-16T06:00:00Z',
      status: 'scheduled',
    },
  );
  for (const change of [{ until: '2026-09-15' }, { until: '2028-01-01' }, { status: 'unknown' }])
    assert.throws(() =>
      agendaQuery({
        from: '2026-09-15',
        until: '2026-09-16',
        offset: '+00:00',
        status: 'scheduled',
        ...change,
      }),
    );
});

test('hearing permissions separate management, staff reading and client denial', () => {
  assert.equal(canHearings('owner', 'manage'), true);
  assert.equal(canHearings('litigator', 'manage'), true);
  assert.equal(canHearings('paralegal', 'read'), true);
  assert.equal(canHearings('paralegal', 'manage'), false);
  assert.equal(canHearings('client', 'read'), false);
});

test('a matched receipt also preserves prepared values and the status implied by its action', () => {
  const prepared = hearingPrepared(),
    record = hearingRecord(prepared);
  assert.equal(hearingMatches({ ...record, values_digest: 'f'.repeat(64) }, prepared), false);
  assert.equal(hearingMatches({ ...record, status: 'cancelled' }, prepared), false);
});
