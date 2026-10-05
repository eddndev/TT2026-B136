import {
  factObject as object,
  factUuid as uuid,
  factDigest as digest,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';
import { agendaInvalid } from './combined-agenda-query.mjs';

export function precautionaryHearingAgendaOverview(row) {
  object(row, [
    'case_id',
    'id',
    'revision',
    'purpose',
    'scheduled_at',
    'modality',
    'status',
    'participant_count',
    'capture_digest',
  ]);
  if (
    uuid(row.case_id) !== row.case_id ||
    uuid(row.id) !== row.id ||
    digest(row.capture_digest) !== row.capture_digest ||
    !['imposition', 'review'].includes(row.purpose) ||
    !['in_person', 'videoconference'].includes(row.modality) ||
    !['scheduled', 'cancelled'].includes(row.status) ||
    !Number.isInteger(row.participant_count) ||
    row.participant_count < 0 ||
    row.participant_count > 32
  )
    agendaInvalid();
  revision(row.revision);
  hearingTimeParts(row.scheduled_at);
  return row;
}
