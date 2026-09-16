import { readFileSync } from 'node:fs';
export const calendarId = '00000000-0000-0000-0000-000000000000';
export const calendarActor = 'user';
export const calendarHash = '77'.repeat(32);
const vectors = JSON.parse(
  readFileSync(
    new URL(
      '../../../crates/domain/tests/fixtures/judicial_calendar_vectors.json',
      import.meta.url,
    ),
  ),
);
export const calendarFixtureValues = () =>
  structuredClone(vectors.find((v) => v.name === 'leap_unicode_unordered').normalized);
export function calendarFixtureCommand(
  action = 'publish',
  revision = 0,
  values = calendarFixtureValues(),
) {
  const change = { action, expected_revision: revision };
  if (action !== 'retire') change.values = values;
  if (action !== 'publish') change.reason = 'Cambio declarado';
  return { operation_id: calendarId, calendar_id: calendarId, change };
}
export function calendarPrepared(command = calendarFixtureCommand(), base) {
  return {
    actor_id: calendarActor,
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values: structuredClone(command.change.values || base?.values || calendarFixtureValues()),
    values_digest: calendarHash,
    initial_scope: calendarFixtureValues().scope,
    submission_digest: calendarHash,
  };
}
export function calendarRecord(prepared = calendarPrepared()) {
  const c = prepared.command;
  return {
    id: c.calendar_id,
    revision: prepared.result_revision,
    status: c.change.action === 'retire' ? 'retired' : 'published',
    values: structuredClone(prepared.values),
    values_digest: prepared.values_digest,
    reason: c.change.reason || null,
    receipt: {
      operation_id: c.operation_id,
      action: c.change.action,
      expected_revision: c.change.expected_revision,
      submission_digest: prepared.submission_digest,
    },
    recorded_at: '2026-09-16T12:00:00Z',
    recorded_by: { id: prepared.actor_id, email: 'hatz@example.com' },
  };
}
export function calendarOverview(record) {
  return {
    id: record.id,
    revision: record.revision,
    status: record.status,
    scope: record.values.scope,
    coverage: record.values.coverage,
    values_digest: record.values_digest,
    has_unresolved: true,
  };
}
export function calendarHistoryRow(record) {
  const { values, ...rest } = record;
  return rest;
}
export function calendarDayRows(values, from, through) {
  const result = [];
  for (
    let at = Date.parse(`${from}T00:00:00Z`);
    at <= Date.parse(`${through}T00:00:00Z`);
    at += 86400000
  ) {
    const date = new Date(at).toISOString().slice(0, 10);
    let day = { date, state: 'outside_coverage', origin: null, source_ids: [], explanation: null };
    if (date >= values.coverage.from && date <= values.coverage.through) {
      const exception = values.exceptions.find((r) => date >= r.from && date <= r.through),
        weekday = new Date(at).getUTCDay() || 7,
        rule = exception || values.weekly_pattern.find((r) => r.weekday === weekday);
      day = {
        date,
        state: rule.classification,
        origin: exception ? 'exception' : 'weekly_pattern',
        source_ids: rule.source_ids,
        explanation: rule.explanation,
        ...(exception ? { exception_id: exception.id } : { weekday }),
      };
    }
    result.push(day);
  }
  return result;
}
