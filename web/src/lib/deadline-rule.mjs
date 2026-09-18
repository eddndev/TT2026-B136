import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { civilDate } from './judicial-calendar-time.mjs';
import { deadlineInstant } from './deadline-time.mjs';
export function deadlineEnum(value, choices) {
  if (!choices.includes(value)) invalid();
  return value;
}
export function deadlineInteger(value, minimum, maximum) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) invalid();
  return value;
}
const field = (v) =>
  deadlineEnum(v, [
    'resolution_issued_at',
    'notification_practiced_at',
    'notification_received_at',
    'notification_stated_effect_at',
    'hearing_session_event_time',
  ]);
const purpose = (v) => deadlineEnum(v, ['hearing_end', 'ordered_period_start']);
const family = (v) => deadlineEnum(v, ['resolution', 'notification', 'hearing_result']);
export function deadlineRequirement(raw) {
  if (raw?.kind === 'source_field') {
    object(raw, ['kind', 'field']);
    field(raw.field);
  } else {
    object(raw, ['kind', 'purpose', 'family']);
    deadlineEnum(raw.kind, ['qualified']);
    purpose(raw.purpose);
    family(raw.family);
  }
  return raw;
}
export function deadlineRule(raw, unit = false) {
  const kind = deadlineEnum(raw?.kind, ['days', 'civil_months', 'elapsed_hours']);
  object(raw, [
    'kind',
    ...(unit ? [] : ['quantity']),
    ...(kind === 'days' ? ['inclusion', 'basis'] : []),
    ...(kind !== 'elapsed_hours' ? ['final_day'] : []),
  ]);
  if (!unit) revision(raw.quantity);
  if (kind === 'days') {
    deadlineEnum(raw.inclusion, ['on_anchor', 'after_anchor']);
    deadlineEnum(raw.basis, ['natural', 'calendar_countable']);
  }
  if (kind !== 'elapsed_hours') deadlineEnum(raw.final_day, ['preserve', 'next_countable']);
  return raw;
}
export function deadlineRuleBlock(raw) {
  if (raw?.kind === 'ordered_quantity_exceeds_maximum') {
    object(raw, ['kind', 'maximum', 'supplied']);
    revision(raw.maximum);
    revision(raw.supplied);
  } else {
    object(raw, ['kind']);
    deadlineEnum(raw.kind, ['missing_ordered_quantity', 'unexpected_ordered_quantity']);
  }
  return raw;
}
export function deadlineTriggerBlock(raw) {
  const kind = raw?.kind;
  if (['unknown_source', 'unexpected_qualification'].includes(kind)) object(raw, ['kind']);
  else if (kind === 'absent_field') {
    object(raw, ['kind', 'field']);
    field(raw.field);
  } else if (kind === 'missing_qualification') {
    object(raw, ['kind', 'purpose']);
    purpose(raw.purpose);
  } else if (['incompatible_family', 'qualification_mismatch'].includes(kind)) {
    object(raw, ['kind', 'expected', 'actual']);
    const parse = kind === 'incompatible_family' ? family : purpose;
    parse(raw.expected);
    parse(raw.actual);
  } else invalid();
  return raw;
}
export function deadlineArithmeticBlock(raw) {
  const kind = raw?.kind;
  if (
    ['unknown_anchor', 'missing_offset', 'missing_calendar', 'date_range_exhausted'].includes(kind)
  )
    object(raw, ['kind']);
  else if (kind === 'insufficient_precision') {
    object(raw, ['kind', 'observed']);
    deadlineEnum(raw.observed, ['unknown', 'date', 'minute', 'second']);
  } else if (kind === 'missing_homologous_day') {
    object(raw, ['kind', 'year', 'month', 'requested_day']);
    deadlineInteger(raw.year, 0, 4294967295);
    deadlineInteger(raw.month, 1, 12);
    deadlineInteger(raw.requested_day, 1, 31);
  } else if (['unresolved_calendar_date', 'outside_calendar_coverage'].includes(kind)) {
    object(raw, ['kind', 'date']);
    civilDate(raw.date);
  } else invalid();
  return raw;
}
export function deadlineOutcome(raw) {
  if (raw?.kind === 'civil_candidate') {
    object(raw, ['kind', 'date']);
    civilDate(raw.date);
  } else if (raw?.kind === 'instant_candidate') {
    object(raw, ['kind', 'instant']);
    deadlineInstant(raw.instant);
  } else {
    object(raw, ['kind', 'block']);
    deadlineEnum(raw.kind, ['blocked']);
    deadlineArithmeticBlock(raw.block);
  }
  return raw;
}
export function deadlineEvaluationBlock(raw) {
  const kind = raw?.kind;
  if (
    [
      'scope_unknown',
      'scope_rejected',
      'incident_unknown',
      'unresolved_incident',
      'civil_cutoff_missing',
    ].includes(kind)
  )
    object(raw, ['kind']);
  else if (['condition_missing', 'condition_unknown', 'condition_rejected'].includes(kind)) {
    object(raw, ['kind', 'id']);
    uuid(raw.id);
  } else if (kind === 'cutoff_outside_coverage') {
    object(raw, ['kind', 'candidate']);
    civilDate(raw.candidate);
  } else if (['rule', 'trigger', 'arithmetic'].includes(kind)) {
    object(raw, ['kind', 'block']);
    ({
      rule: deadlineRuleBlock,
      trigger: deadlineTriggerBlock,
      arithmetic: deadlineArithmeticBlock,
    })[kind](raw.block);
  } else invalid();
  return raw;
}
