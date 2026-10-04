import { alertRecord, alertInstant } from './alerts.mjs';
import { resourceHearingCreation } from './resource-hearing-unit.mjs';

export function resourceHearingAlert(creation = resourceHearingCreation()) {
  const hearing = creation.hearing;
  const activity = Date.parse(hearing.values.scheduled_at) / 1000;
  const row = alertRecord('upcoming');
  row.subject = {
    kind: 'resource_hearing',
    case_id: hearing.case_id,
    resource_id: hearing.resource_id,
    id: hearing.id,
  };
  row.subject_title = 'Audiencia del recurso declarada';
  row.origin = { revision: hearing.revision, evidence_digest: hearing.capture_digest };
  row.kind = { kind: 'upcoming', lead_hours: 24, activity_at: alertInstant(activity) };
  row.trigger_at = alertInstant(activity - 86400);
  row.created_at = alertInstant(activity - 86400 + 3, 123456789);
  return row;
}
