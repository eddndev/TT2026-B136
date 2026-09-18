import test from 'node:test';
import assert from 'node:assert/strict';
import { deadlinePreparedValue, deadlineRecordValue } from '../src/lib/deadline-validation.mjs';
import { deadlineMatches } from '../src/lib/deadline-submission.mjs';
import { prepared, detail, id, hash, known, instant } from './fixtures/deadline-unit.mjs';
function withSource(family) {
  const p = prepared();
  const reference =
    family === 'hearing_result'
      ? { family, hearing_id: id(7), result_id: id(8), revision: 1, agreement_id: id(0) }
      : {
          family,
          id: id(8),
          revision: 1,
          ...(family === 'notification' ? { resolution: { id: id(7), revision: 2 } } : {}),
        };
  const href =
    family === 'hearing_result'
      ? `/api/v1/cases/${id(1)}/hearings/${id(7)}/results/${id(8)}/revisions/1`
      : family === 'notification'
        ? `/api/v1/cases/${id(1)}/resolutions/${id(7)}/notifications/${id(8)}/revisions/1`
        : `/api/v1/cases/${id(1)}/resolutions/${id(8)}/revisions/1`;
  p.definition.input.selection.source = known(reference);
  p.command.change.definition = structuredClone(p.definition);
  const source = {
    case_id: id(1),
    reference,
    values_digest: hash('1'),
    sources_digest: family === 'hearing_result' ? null : hash('2'),
    submission_digest: hash('3'),
    status: 'recorded',
    href,
  };
  p.calculation.material.source = source;
  p.calculation.material.source_head = structuredClone(source);
  if (family === 'hearing_result') p.calculation.material.source_head.reference.agreement_id = null;
  const rule = p.calculation.result.rule,
    at = { precision: 'unknown' };
  p.calculation.result = {
    requirement: {
      kind: 'source_field',
      field:
        family === 'resolution'
          ? 'resolution_issued_at'
          : family === 'notification'
            ? 'notification_practiced_at'
            : 'hearing_session_event_time',
    },
    trigger_outcome: { kind: 'extracted', at },
    rule,
    arithmetic: {
      rule,
      anchor: at,
      outcome: { kind: 'blocked', block: { kind: 'unknown_anchor' } },
      trace: [],
    },
    due_at: null,
    blocks: [{ kind: 'arithmetic', block: { kind: 'unknown_anchor' } }],
  };
  return p;
}
test('known sources preserve hearing nil agreement and notification selected versus head parent revisions', () => {
  for (const family of ['resolution', 'notification', 'hearing_result']) {
    const p = withSource(family);
    assert.deepEqual(deadlinePreparedValue(p), p);
    assert.equal(deadlineMatches(detail(p), p), true);
    const head = p.calculation.material.source_head;
    head.reference.revision = 2;
    head.href = head.href.replace('/revisions/1', '/revisions/2');
    head.values_digest = hash('4');
    if (family === 'notification') head.reference.resolution.revision = 5;
    assert.deepEqual(deadlinePreparedValue(p), p);
    if (family === 'hearing_result')
      assert.equal(p.definition.input.selection.source.value.agreement_id, id(0));
    if (family === 'notification')
      assert.equal(p.definition.input.selection.source.value.resolution.revision, 2);
    const response = detail(p);
    response.calculation.material.source_head.values_digest = hash('5');
    assert.equal(deadlineMatches(response, p), false);
  }
});
test('source and calendar projections reject substituted roots, revisions, scopes and hyperlinks', () => {
  const p = withSource('notification');
  for (const mutate of [
    (p) => {
      p.calculation.material.source.reference.resolution.revision = 7;
    },
    (p) => {
      p.calculation.material.source_head.reference.resolution.id = id(9);
    },
    (p) => {
      p.calculation.material.source_head.case_id = id(9);
    },
    (p) => {
      p.calculation.material.source.href = 'https://example.test/private';
    },
    (p) => {
      p.calculation.material.source_head.values_digest = hash('8');
    },
  ]) {
    const next = structuredClone(p);
    mutate(next);
    assert.throws(() => deadlinePreparedValue(next));
  }
  p.definition.input.calendar = { id: id(9), revision: 2 };
  p.command.change.definition = structuredClone(p.definition);
  p.calculation.material.calendar = {
    id: id(9),
    revision: 2,
    values_digest: hash('6'),
    submission_digest: hash('7'),
    status: 'retired',
    title: 'Calendario historico',
    href: `/api/v1/judicial-calendars/${id(9)}/revisions/2`,
  };
  p.calculation.material.calendar_head = structuredClone(p.calculation.material.calendar);
  assert.deepEqual(deadlinePreparedValue(p), p);
  p.calculation.material.calendar_head.revision = 1;
  assert.throws(() => deadlinePreparedValue(p));
});
test('receipts reject body, attention and expected revision contradictions on reads', () => {
  const p = prepared();
  for (const mutate of [
    (d) => {
      d.unexpected = true;
    },
    (d) => {
      d.receipt.expected_revision = 1;
    },
    (d) => {
      d.status = 'retired';
    },
    (d) => {
      d.reason = 'Unexpected';
    },
    (d) => {
      d.recorded_at.nanosecond = -1;
    },
    (d) => {
      d.attention = {
        status: 'recorded',
        occurred_at: { precision: 'unknown' },
        statement: 'Dato',
        locator: 'Folio',
      };
    },
    (d) => {
      d.definition.input.selection.case_id = id(9);
    },
  ]) {
    const d = detail(p);
    mutate(d);
    assert.throws(() => deadlineRecordValue(d, id(1), id(6), 1));
  }
});
