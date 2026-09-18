import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factText as text,
} from './procedural-fact-primitives.mjs';
import { civilDate } from './judicial-calendar-time.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import { deadlineEnum as choice, deadlineInteger as integer } from './deadline-rule.mjs';
function optionalDate(value) {
  if (value !== null) civilDate(value);
}
function day(raw) {
  object(raw, ['date', 'origin', 'classification', 'explanation', 'source_ids']);
  civilDate(raw.date);
  if (!Array.isArray(raw.source_ids) || raw.source_ids.length > 16) invalid();
  const ids = raw.source_ids.map(uuid);
  if (new Set(ids).size !== ids.length) invalid();
  if (raw.origin === null) {
    if (raw.classification !== null || raw.explanation !== null || ids.length) invalid();
  } else {
    choice(raw.classification, ['countable', 'excluded', 'unresolved']);
    text(raw.explanation, 'Explicacion', 256);
    if (raw.origin.kind === 'weekly_pattern') {
      object(raw.origin, ['kind', 'weekday']);
      integer(raw.origin.weekday, 1, 7);
    } else {
      object(raw.origin, ['kind', 'id']);
      choice(raw.origin.kind, ['exception']);
      uuid(raw.origin.id);
    }
  }
}
function count(raw) {
  object(raw, ['first_included', 'quantity', 'accumulated', 'outcome', 'trace']);
  civilDate(raw.first_included);
  revision(raw.quantity);
  integer(raw.accumulated, 0, 4294967295);
  const outcome = raw.outcome,
    end = outcome?.kind === 'date_range_exhausted' ? 'after' : 'date';
  object(outcome, ['kind', end]);
  choice(outcome.kind, ['candidate', 'unresolved', 'outside_coverage', 'date_range_exhausted']);
  civilDate(outcome[end]);
  if (!Array.isArray(raw.trace) || raw.trace.length > 1097) invalid();
  for (const step of raw.trace) {
    object(step, ['day', 'accumulated']);
    day(step.day);
    integer(step.accumulated, 0, 4294967295);
  }
}
export function deadlineTrace(raw) {
  const kind = raw?.kind;
  if (['counted_days', 'final_day'].includes(kind)) {
    object(raw, ['kind', 'count']);
    count(raw.count);
  } else if (kind === 'natural_days') {
    object(raw, ['kind', 'first_included', 'quantity', 'candidate']);
    civilDate(raw.first_included);
    revision(raw.quantity);
    optionalDate(raw.candidate);
  } else if (kind === 'civil_months') {
    object(raw, [
      'kind',
      'anchor',
      'quantity',
      'target_year',
      'target_month',
      'requested_day',
      'candidate',
    ]);
    civilDate(raw.anchor);
    revision(raw.quantity);
    integer(raw.target_year, 0, 4294967295);
    integer(raw.target_month, 1, 12);
    integer(raw.requested_day, 1, 31);
    optionalDate(raw.candidate);
  } else if (kind === 'elapsed_hours') {
    object(raw, ['kind', 'start', 'quantity', 'candidate']);
    deadlineInstant(raw.start);
    revision(raw.quantity);
    if (raw.candidate !== null) deadlineInstant(raw.candidate);
  } else invalid();
  return raw;
}
