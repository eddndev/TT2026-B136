import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factText as text,
  factMaxRevision,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
} from './resource-hearing-values.mjs';
import { precautionaryHearingValues } from './precautionary-hearing-values.mjs';

function values(row) {
  object(row, [
    'purpose',
    'scheduled_at',
    'modality',
    'venue',
    'note',
    'participants',
    'scheduling_basis',
    'review_targets',
  ]);
  row.venue = text(row.venue, 'Sede', 500, false);
  if (row.note !== null) row.note = text(row.note, 'Nota');
  object(row.scheduling_basis, ['statement', 'support', 'locator']);
  row.scheduling_basis.statement = text(row.scheduling_basis.statement, 'Fundamento');
  row.scheduling_basis.locator = text(row.scheduling_basis.locator, 'Localizador');
  for (const [entries, key] of [
    [row.participants, 'participant_id'],
    [row.review_targets, 'id'],
  ]) {
    if (!Array.isArray(entries) || entries.length > 32) invalid();
    for (const entry of entries) uuid(entry?.[key]);
    entries.sort((left, right) => left[key].localeCompare(right[key]));
  }
  precautionaryHearingValues(row);
}

export function precautionaryHearingCommand(raw) {
  const row = structuredClone(raw);
  object(row, ['case_id', 'operation_id', 'hearing_id', 'change']);
  for (const key of ['case_id', 'operation_id', 'hearing_id']) uuid(row[key]);
  const change = row.change,
    action = change?.action;
  if (!['schedule', 'replace', 'cancel'].includes(action)) invalid();
  object(change, [
    'action',
    ...(action === 'schedule' ? [] : ['expected_revision', 'expected_capture_digest', 'reason']),
    ...(action === 'cancel' ? [] : ['context', 'values']),
  ]);
  if (action !== 'schedule') {
    revision(change.expected_revision, factMaxRevision - 1);
    digest(change.expected_capture_digest);
    change.reason = text(change.reason, 'Motivo');
  }
  if (action !== 'cancel') {
    object(change.context, ['administration_revision', 'stage_revision', 'context_digest']);
    revision(change.context.administration_revision);
    revision(change.context.stage_revision);
    digest(change.context.context_digest);
    values(change.values);
  }
  return row;
}
