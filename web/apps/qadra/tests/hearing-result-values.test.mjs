import test from 'node:test';
import assert from 'node:assert/strict';
import {
  hearingResultDraft,
  hearingResultValues,
  hearingResultCommand,
} from '../src/lib/hearing-result-values.mjs';

const id = (n) => `00000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
const now = Date.parse('2026-09-16T12:00:00Z');
const options = {
  action: 'record',
  operationId: id(1),
  hearingId: id(2),
  resultId: id(3),
  anchorRevision: 7,
  now,
};
function draft() {
  return {
    ...hearingResultDraft(),
    occurrence: 'occurred',
    extent: 'partial',
    time: { precision: 'date', date: '2026-09-15', time: '', offset: '-06:00' },
    summary: ' Relato\r\ncomunicado ',
    provenance: { kind: 'operator_note', reference: '', support: null },
  };
}
function base() {
  return {
    id: id(3),
    hearing_id: id(2),
    revision: 4,
    status: 'recorded',
    values: hearingResultValues(draft(), now),
  };
}

test('new drafts do not infer occurrence extent attendance or an event time from a citation', () => {
  const empty = hearingResultDraft();
  assert.equal(empty.occurrence, '');
  assert.equal(empty.extent, '');
  assert.equal(empty.time.date, '');
  assert.equal(empty.time.time, '');
  assert.deepEqual(empty.attendees, []);
  assert.deepEqual(empty.agreements, []);
  assert.throws(() => hearingResultValues(empty, now), /ocurri|inici|selecciona/i);
});

test('normalization preserves Unicode interior and independent historical attendee capacities', () => {
  const input = draft();
  input.summary = '\u00a0A\r\n\u0065\u0301 \u{1f600}\u00a0';
  input.attendees = [
    {
      participant_id: id(9),
      revision: 3,
      capacity: '  Compareciente ',
      observation: ' Entrada\r\ntard\u00eda ',
    },
    { participant_id: id(4), revision: 1, capacity: 'Representante', observation: '   ' },
  ];
  const values = hearingResultValues(input, now);
  assert.equal(values.summary, 'A\ne\u0301 \u{1f600}');
  assert.deepEqual(values.attendees, [
    { participant_id: id(4), revision: 1, capacity: 'Representante', observation: null },
    {
      participant_id: id(9),
      revision: 3,
      capacity: 'Compareciente',
      observation: 'Entrada\ntard\u00eda',
    },
  ]);
  assert.equal(input.attendees[0].participant_id, id(9));
  assert.deepEqual(values.provenance, { kind: 'operator_note', reference: null, support: null });
});

test('no start allows explicit attendance and agreements but never partial or concluded extent', () => {
  const input = {
    ...draft(),
    occurrence: 'not_started',
    extent: 'unspecified',
    attendees: [{ participant_id: id(4), revision: 1, capacity: 'Compareciente' }],
    agreements: [{ id: id(10), text: ' Nueva fecha comunicada ' }],
  };
  assert.equal(hearingResultValues(input, now).attendees.length, 1);
  assert.equal(hearingResultValues(input, now).agreements[0].text, 'Nueva fecha comunicada');
  for (const extent of ['partial', 'concluded'])
    assert.throws(() => hearingResultValues({ ...input, extent }, now), /alcance|inicio|inici/i);
});

test('attendees require exact unique roots and bounded scalar text even for distinct names', () => {
  const person = { participant_id: id(4), revision: 1, capacity: 'Persona' };
  for (const attendees of [
    [person, { ...person, revision: 2 }],
    [{ ...person, revision: 0 }],
    [{ ...person, revision: 4294967296 }],
    [{ ...person, participant_id: 'not-a-uuid' }],
    [{ ...person, capacity: '' }],
    [{ ...person, capacity: 'A\nB' }],
    [{ ...person, observation: 'x'.repeat(501) }],
    Array.from({ length: 33 }, (_, n) => ({ ...person, participant_id: id(n + 20) })),
  ])
    assert.throws(() => hearingResultValues({ ...draft(), attendees }, now));
  assert.doesNotThrow(() =>
    hearingResultValues(
      {
        ...draft(),
        attendees: [
          { ...person, capacity: '\u{1f600}'.repeat(100) },
          { ...person, participant_id: id(5) },
        ],
      },
      now,
    ),
  );
});

test('agreements retain identifiers and user order with no generated or deduplicated content', () => {
  const agreements = [
    { id: id(9), text: ' Uno ' },
    { id: id(4), text: 'Dos' },
  ];
  const values = hearingResultValues({ ...draft(), agreements }, now);
  assert.deepEqual(values.agreements, [
    { id: id(9), text: 'Uno' },
    { id: id(4), text: 'Dos' },
  ]);
  for (const invalid of [
    [
      { id: id(9), text: 'a' },
      { id: id(9), text: 'b' },
    ],
    [{ text: 'No id' }],
    [{ id: id(9), text: '' }],
    Array.from({ length: 17 }, (_, n) => ({ id: id(n + 20), text: 'A' })),
  ])
    assert.throws(() => hearingResultValues({ ...draft(), agreements: invalid }, now));
});

test('provenance requires locator for oral or written sources and preserves optional exact support', () => {
  const support = { document_id: id(6), version: 1, digest: 'AB'.repeat(32) };
  for (const kind of ['oral_reference', 'written_record']) {
    assert.throws(() =>
      hearingResultValues({ ...draft(), provenance: { kind, reference: '  ' } }, now),
    );
    const value = hearingResultValues(
      { ...draft(), provenance: { kind, reference: ' Minuto 4 ', support } },
      now,
    );
    assert.deepEqual(value.provenance, {
      kind,
      reference: 'Minuto 4',
      support: { ...support, digest: support.digest.toLowerCase() },
    });
  }
  for (const invalid of [
    { ...support, version: 0 },
    { ...support, digest: 'x' },
    { ...support, document_id: '' },
  ])
    assert.throws(() =>
      hearingResultValues(
        { ...draft(), provenance: { kind: 'operator_note', support: invalid } },
        now,
      ),
    );
});

test('text limits use Unicode scalars and reject controls or lone surrogates', () => {
  assert.equal(
    [...hearingResultValues({ ...draft(), summary: '\u{1f600}'.repeat(1000) }, now).summary].length,
    1000,
  );
  for (const summary of ['', 'x'.repeat(1001), 'A\u0000B', 'A\rB', '\ud800'])
    assert.throws(() => hearingResultValues({ ...draft(), summary }, now));
});

test('record uses explicitly selected sources without administration or stage expectations', () => {
  const command = hearingResultCommand(draft(), {
    ...options,
    continuation: { result_id: id(8), revision: 2 },
  });
  assert.deepEqual(command, {
    operation_id: id(1),
    hearing_id: id(2),
    result_id: id(3),
    change: {
      action: 'record',
      expected_revision: 0,
      anchor_revision: 7,
      continuation: { result_id: id(8), revision: 2 },
      values: hearingResultValues(draft(), now),
    },
  });
  assert.throws(() => hearingResultCommand(draft(), { ...options, anchorRevision: undefined }));
  assert.throws(
    () =>
      hearingResultCommand(draft(), {
        ...options,
        continuation: { result_id: id(3), revision: 1 },
      }),
    /antecedente|propio|misma/i,
  );
});

test('correct serializes only mutable values and reason while preserving fixed sources in the base', () => {
  const record = base();
  record.anchor = { revision: 7 };
  record.continuation = { result_id: id(8), revision: 2 };
  const input = hearingResultDraft(record);
  input.reason = '  Precisi\u00f3n\r\ncomunicada ';
  input.summary = 'Corregido';
  const command = hearingResultCommand(input, { ...options, action: 'correct', base: record });
  assert.deepEqual(Object.keys(command.change).sort(), [
    'action',
    'expected_revision',
    'reason',
    'values',
  ]);
  assert.equal(command.change.expected_revision, 4);
  assert.equal(command.change.reason, 'Precisi\u00f3n\ncomunicada');
  assert.equal(record.values.summary, 'Relato\ncomunicado');
  assert.equal(record.anchor.revision, 7);
});

test('withdraw ignores editable content and historical time but requires exact base scope and reason', () => {
  const record = base();
  const input = {
    ...hearingResultDraft(record),
    reason: 'Captura duplicada',
    time: null,
    summary: '',
  };
  assert.deepEqual(
    hearingResultCommand(input, { ...options, action: 'withdraw', base: record, now: 0 }).change,
    { action: 'withdraw', expected_revision: 4, reason: 'Captura duplicada' },
  );
  for (const changed of [
    { ...record, status: 'withdrawn' },
    { ...record, hearing_id: id(30) },
    { ...record, id: id(31) },
    { ...record, revision: 4294967295 },
  ])
    assert.throws(() =>
      hearingResultCommand(input, { ...options, action: 'withdraw', base: changed }),
    );
  assert.throws(() =>
    hearingResultCommand(
      { ...input, reason: '' },
      { ...options, action: 'withdraw', base: record },
    ),
  );
});

test('editing a draft does not mutate exact attendee support or agreement evidence', () => {
  const record = base();
  record.values.attendees = [
    { participant_id: id(4), revision: 1, capacity: 'Persona', observation: null },
  ];
  record.values.agreements = [{ id: id(5), text: 'Texto' }];
  record.values.provenance.support = { document_id: id(6), version: 1, digest: 'a'.repeat(64) };
  const input = hearingResultDraft(record);
  input.attendees[0].capacity = 'Otro';
  input.agreements[0].text = 'Distinto';
  input.provenance.support.version = 2;
  assert.equal(record.values.attendees[0].capacity, 'Persona');
  assert.equal(record.values.agreements[0].text, 'Texto');
  assert.equal(record.values.provenance.support.version, 1);
});
