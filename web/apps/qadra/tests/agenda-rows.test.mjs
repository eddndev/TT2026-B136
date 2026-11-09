import { test } from 'node:test';
import assert from 'node:assert/strict';
import { agendaRow, agendaCalendarDay } from '../src/components/agenda-rows.mjs';
import { agendaGroups } from '../src/components/agenda-presentation.mjs';

// Hand-built Agenda items with only the fields the rows read. 1767315600 is
// 2026-01-02T01:00:00Z, which is 2026-01-01 19:00 at -06:00.
const at = { unix_seconds: 1767315600, nanosecond: 0, offset_seconds: 0 };
const caseFields = { case_title: 'Defensa inicial', case_reference: 'NUC-123' };
const hearingItem = (status = 'scheduled', caseStatus = 'active') => ({
  kind: 'hearing',
  at,
  hearing: {
    id: 'h-1',
    revision: 2,
    kind: 'initial',
    status,
    scheduled_at: '2026-01-02T01:00:00Z',
    case_status: caseStatus,
    ...caseFields,
  },
});
const deadlineItem = (attention = false) => ({
  kind: 'deadline',
  at,
  ...caseFields,
  case_status: 'active',
  deadline: {
    id: 'd-1',
    revision: 1,
    title: 'Vencimiento combinado',
    responsible: { email: 'responsable@example.test' },
    attention_recorded: attention,
  },
});
const resourceItem = {
  kind: 'resource_hearing',
  at,
  ...caseFields,
  case_status: 'active',
  resource_hearing: {
    id: 'r-1',
    revision: 1,
    kind: 'appeal_arguments',
    scheduled_at: '2026-01-02T01:00:00Z',
  },
};
const precautionaryItem = (status) => ({
  kind: 'precautionary_hearing',
  at,
  ...caseFields,
  case_status: 'closed',
  precautionary_hearing: {
    id: 'p-1',
    revision: 3,
    purpose: 'imposition',
    status,
    scheduled_at: '2026-01-02T01:00:00Z',
  },
});

test('agenda rows keep one exact accessible name and kind per activity', () => {
  const cases = [
    [hearingItem(), 'Audiencia', 'Consultar audiencia h-1'],
    [deadlineItem(), 'Plazo', 'Consultar plazo d-1'],
    [resourceItem, 'Audiencia de recurso', 'Consultar audiencia de recurso r-1'],
    [precautionaryItem('scheduled'), 'Audiencia cautelar', 'Consultar audiencia cautelar p-1'],
  ];
  for (const [item, badge, label] of cases) {
    const row = agendaRow(item, '+00:00');
    assert.equal(row.badge, badge);
    assert.equal(row.label, label);
    assert.deepEqual(row.attrs, { 'data-agenda-kind': item.kind });
    assert.equal(row.key, `${item.kind}:${item[item.kind].id}`);
    assert.equal(row.value, item);
    assert.equal(row.pressed, undefined);
  }
});

test('agenda rows project the instant onto the queried offset', () => {
  const row = agendaRow(hearingItem(), '-06:00');
  assert.equal(row.date, '2026-01-01');
  assert.equal(row.dateText, '1 de enero de 2026');
  assert.deepEqual(row.time, {
    datetime: '2026-01-01T19:00:00-06:00',
    text: '19:00:00 UTC-06:00',
  });
  assert.ok(row.meta.includes('Hora en la agenda: 2026-01-01 19:00:00.000000000 / UTC-06:00'));
  assert.equal(row.reference, 'NUC-123');
  assert.equal(row.meta[0], 'Defensa inicial');
  assert.ok(!row.meta.some((line) => line.includes('NUC-123')));
  assert.ok(row.meta.includes('Revisi\u00f3n 2'));
  const fraction = agendaRow({ ...hearingItem(), at: { ...at, nanosecond: 120000000 } }, '+00:00');
  assert.deepEqual(fraction.time, {
    datetime: '2026-01-02T01:00:00+00:00',
    text: '01:00:00.120000000 UTC+00:00',
  });
});

test('agenda rows state cancellation and closure as text', () => {
  const cancelled = agendaRow(precautionaryItem('cancelled'), '+00:00');
  assert.equal(cancelled.status, 'Cancelada');
  assert.ok(cancelled.meta.includes('Expediente cerrado administrativamente'));
  assert.ok(cancelled.meta.some((line) => line.startsWith('Cancelada / ')));
  assert.equal(agendaRow(hearingItem('cancelled'), '+00:00').status, 'Cancelada');
  assert.equal(agendaRow(hearingItem(), '+00:00').status, undefined);
  assert.equal(agendaRow(resourceItem, '+00:00').status, undefined);
  assert.ok(
    agendaRow(hearingItem('scheduled', 'closed'), '+00:00').meta.includes(
      'Expediente cerrado administrativamente',
    ),
  );
  assert.ok(
    agendaRow(resourceItem, '+00:00').meta.some((line) => line.startsWith('Horario declarado / ')),
  );
});

test('deadline rows carry the responsible person and the attention state', () => {
  const pending = agendaRow(deadlineItem(), '+00:00');
  assert.equal(pending.title, 'Vencimiento combinado');
  assert.ok(
    pending.meta.includes('Vencimiento operativo: 2026-01-02 01:00:00.000000000 / UTC+00:00'),
  );
  assert.ok(pending.meta.includes('Responsable: responsable@example.test'));
  assert.ok(pending.meta.includes('Atenci\u00f3n pendiente'));
  assert.ok(agendaRow(deadlineItem(true), '+00:00').meta.includes('Atenci\u00f3n declarada'));
});

test('agenda calendar days count what the period holds and mark only its days', () => {
  const rows = [
    hearingItem(),
    deadlineItem(),
    { ...resourceItem, at: { ...at, unix_seconds: 1767398400 } },
  ];
  const groups = agendaGroups(rows, '+00:00');
  const range = { from: '2026-01-02', until: '2026-01-05' };
  const full = agendaCalendarDay({ groups, range, complete: true });
  assert.deepEqual(full('2026-01-02'), {
    label: '2 de enero de 2026: 2 actividades',
    tone: 'period',
    marker: { dot: true },
    attrs: { 'data-agenda-day': '2026-01-02' },
  });
  assert.equal(full('2026-01-03').label, '3 de enero de 2026: 1 actividad');
  assert.deepEqual(full('2026-01-04'), {
    label: '4 de enero de 2026: sin actividades',
    tone: 'period',
    marker: null,
    attrs: { 'data-agenda-day': '2026-01-04' },
  });
  assert.deepEqual(full('2026-01-05'), {
    label: '5 de enero de 2026: fuera del periodo consultado',
    tone: null,
    marker: null,
    attrs: {},
  });
  const partial = agendaCalendarDay({ groups, range, complete: false });
  assert.equal(partial('2026-01-02').label, '2 de enero de 2026: 2 actividades cargadas');
  assert.equal(partial('2026-01-03').label, '3 de enero de 2026: 1 actividad cargada');
  assert.equal(partial('2026-01-04').label, '4 de enero de 2026: sin actividades cargadas');
  assert.equal(partial('2026-01-01').label, '1 de enero de 2026: fuera del periodo consultado');
});
