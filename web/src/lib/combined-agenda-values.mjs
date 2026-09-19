import {
  factObject as object,
  factUuid as uuid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import { deadlineListRow } from './deadline-validation.mjs';
import { validateHearing } from './hearings-api.mjs';
import {
  agendaInvalid,
  agendaKey,
  compareAgendaKeys,
  agendaCursor,
} from './combined-agenda-query.mjs';

function instant(value) {
  deadlineInstant(value);
  if (
    value.offset_seconds !== 0 ||
    value.unix_seconds < -62135596800 ||
    value.unix_seconds > 253402300799
  )
    agendaInvalid();
  return value;
}
function caseMetadata(
  row,
  title = 'case_title',
  reference = 'case_reference',
  status = 'case_status',
) {
  if (
    typeof row[title] !== 'string' ||
    !row[title].trim() ||
    typeof row[reference] !== 'string' ||
    !row[reference].trim() ||
    !['active', 'closed'].includes(row[status])
  )
    agendaInvalid();
}
function hearingValue(row, query) {
  object(row, [
    'case_id',
    'case_title',
    'case_reference',
    'case_status',
    'id',
    'revision',
    'kind',
    'scheduled_at',
    'modality',
    'status',
    'participant_count',
  ]);
  uuid(row.case_id);
  uuid(row.id);
  validateHearing(row);
  caseMetadata(row);
  if (
    !['initial', 'intermediate', 'oral_trial', 'sentencing'].includes(row.kind) ||
    !['in_person', 'videoconference'].includes(row.modality) ||
    !['scheduled', 'cancelled'].includes(row.status) ||
    !Number.isInteger(row.participant_count) ||
    row.participant_count < 0 ||
    row.participant_count > 32 ||
    (query.hearing_status !== 'all' && row.status !== query.hearing_status)
  )
    agendaInvalid();
  return { unix_seconds: Date.parse(row.scheduled_at) / 1000, nanosecond: 0, offset_seconds: 0 };
}
export function combinedAgendaPage(value, query) {
  object(value, [
    'from',
    'until',
    'kind',
    'hearing_status',
    'checked_at',
    'items',
    'complete',
    'next_cursor',
  ]);
  if (
    value.from !== query.from ||
    value.until !== query.until ||
    value.kind !== query.kind ||
    value.hearing_status !== query.hearing_status ||
    !Array.isArray(value.items) ||
    value.items.length > query.limit ||
    typeof value.complete !== 'boolean' ||
    (value.complete ? value.next_cursor !== null : value.next_cursor === null)
  )
    agendaInvalid();
  instant(value.checked_at);
  let previous = query.after;
  const identities = new Set();
  for (const item of value.items) {
    instant(item.at);
    if (query.kind !== 'all' && query.kind !== item.kind) agendaInvalid();
    let expected, id;
    if (item.kind === 'hearing') {
      object(item, ['kind', 'at', 'hearing']);
      expected = hearingValue(item.hearing, query);
      id = item.hearing.id;
    } else if (item.kind === 'deadline') {
      object(item, ['kind', 'at', 'case_title', 'case_reference', 'case_status', 'deadline']);
      const deadline = item.deadline;
      deadlineListRow(deadline, deadline?.case_id);
      caseMetadata(item);
      if (
        deadline.receipt_kind !== 'v2' ||
        deadline.status !== 'active' ||
        deadline.review_state !== 'accepted' ||
        deadline.operational.freshness !== 'current' ||
        deadline.operational.due_at === null ||
        !same(deadline.operational.checked_at, value.checked_at)
      )
        agendaInvalid();
      expected = { ...deadline.operational.due_at, offset_seconds: 0 };
      id = deadline.id;
    } else agendaInvalid();
    if (
      !same(expected, item.at) ||
      item.at.unix_seconds < query.start ||
      item.at.unix_seconds >= query.end
    )
      agendaInvalid();
    const key = agendaKey(item.at, item.kind, id),
      identity = `${item.kind}:${id}`;
    if ((previous && compareAgendaKeys(key, previous) <= 0) || identities.has(identity))
      agendaInvalid();
    identities.add(identity);
    previous = key;
  }
  if (!value.complete) {
    const cursor = agendaCursor(value.next_cursor, query);
    if (
      (query.after && compareAgendaKeys(cursor, query.after) <= 0) ||
      (previous && compareAgendaKeys(cursor, previous) < 0)
    )
      agendaInvalid();
  }
  return value;
}
