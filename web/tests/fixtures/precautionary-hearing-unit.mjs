import { resourceHearingCreation } from './resource-hearing-unit.mjs';
import { alertRecord, alertInstant } from './alerts.mjs';

export const clone = (value) => structuredClone(value);
export const precautionaryCaseId = '10000000-0000-4000-8000-000000000001';
export const precautionaryHearingId = 'f1000000-0000-4000-8000-000000000001';
const operationId = (revision) => `f2000000-0000-4000-8000-${String(revision).padStart(12, '0')}`;

function context(actor) {
  const administration = {
    case_id: precautionaryCaseId,
    revision: 1,
    title: 'Expediente cautelar',
    reference: 'CAUTELAR-1',
    administrative_status: 'active',
    profile: {
      nuc: 'NUC-1',
      nuc_authority: 'Fiscalia',
      judicial_case_number: 'CJ-1',
      judicial_authority: 'Juzgado de control',
      offenses: ['Delito declarado'],
      general_information: null,
      complementary_identifiers: null,
    },
    values_digest: 'a'.repeat(64),
    changed_at: '2026-01-01T00:00:00Z',
    changed_by: { id: actor.id, email: actor.email },
  };
  return {
    case_id: precautionaryCaseId,
    administration,
    stage: {
      kind: 'initial',
      case_id: precautionaryCaseId,
      stage_revision: 1,
      stage: 'investigation',
      administration_revision: 1,
      administration_digest: administration.values_digest,
      recorded_at: administration.changed_at,
      recorded_by: clone(administration.changed_by),
    },
    stage_administration: clone(administration),
    context_digest: 'b'.repeat(64),
    expectation: {
      administration_revision: 1,
      stage_revision: 1,
      context_digest: 'b'.repeat(64),
    },
  };
}

function participant(row, actor) {
  const { values_digest, kind, ...overview } = row;
  return {
    source: {
      ...overview,
      values_digest,
      legal_status: 'Interviniente declarado',
      changed_at: '2026-01-01T00:00:00Z',
      changed_by: { id: actor.id, email: actor.email },
      canonical_format: 'part1',
      profile: null,
      credential_origin: null,
      submission_digest: null,
      submission_revision: null,
    },
    projection: {
      snapshot: {
        case_id: row.case_id,
        reference: { participant_id: row.id, revision: row.revision },
        values_digest,
        status: row.directory_status,
        subject: null,
      },
      overview: { ...overview, kind, canonical_format: 'part1' },
    },
  };
}

export function precautionaryHearingOperation({ revision = 3, participantCount = 2 } = {}) {
  const source = resourceHearingCreation({ participantCount }).hearing;
  const actor = { ...source.recorded_by, role: 'owner' };
  const capturedContext = context(actor);
  const people = source.sources.participants.map((row) => participant(row, actor));
  const values = {
    purpose: 'imposition',
    scheduled_at: '2026-01-03T09:00:00-06:00',
    modality: 'in_person',
    venue: 'Sala cautelar',
    note: 'Programacion declarada',
    participants: clone(source.values.participants),
    scheduling_basis: {
      statement: 'Senalamiento declarado con soporte exacto',
      support: clone(source.values.scheduling_basis.support),
      locator: 'Pagina 1',
    },
    review_targets: [],
  };
  const captures = [];
  for (let index = 1; index <= revision; index++) {
    if (index === 2) {
      values.scheduled_at = '2026-01-04T09:00:00-06:00';
      values.venue = 'Sala cautelar reprogramada';
    }
    const change =
      index === 1
        ? { action: 'schedule', context: clone(capturedContext.expectation), values: clone(values) }
        : {
            action: index === 3 ? 'cancel' : 'replace',
            expected_revision: index - 1,
            expected_capture_digest: captures[index - 2].capture_digest,
            ...(index === 3
              ? {}
              : {
                  context: clone(capturedContext.expectation),
                  values: clone(values),
                }),
            reason: index === 3 ? 'Cancelacion declarada' : 'Nuevo senalamiento declarado',
          };
    captures.push({
      review: {
        case_id: precautionaryCaseId,
        actor: clone(actor),
        command: {
          case_id: precautionaryCaseId,
          operation_id: operationId(index),
          hearing_id: precautionaryHearingId,
          change,
        },
        resolved_values: clone(values),
        result_revision: index,
        status: index === 3 ? 'cancelled' : 'scheduled',
        scheduling_context: clone(capturedContext),
        observed_context: clone(capturedContext),
        sources: {
          participants: people.map((row) => clone(row.source)),
          support: clone(source.sources.support),
        },
        participants: people.map((row) => clone(row.projection)),
        submission_digest: String(index).repeat(64),
        review_digest: String(index + 3).repeat(64),
      },
      recorded_at: `2026-01-0${index + 1}T12:00:00Z`,
      capture_digest: String(index + 6).repeat(64),
    });
  }
  const first = captures[0];
  return {
    capture: clone(captures.at(-1)),
    history: {
      origin: {
        case_id: precautionaryCaseId,
        hearing_id: precautionaryHearingId,
        operation_id: first.review.command.operation_id,
        revision: 1,
        submission_digest: first.review.submission_digest,
        review_digest: first.review.review_digest,
        capture_digest: first.capture_digest,
      },
      captures,
      record_history: { records: { judicial: { groups: [] }, administrative: [] }, decisions: [] },
    },
  };
}

export function precautionaryHearingOverview(value = precautionaryHearingOperation()) {
  const { capture } = value,
    review = capture.review,
    values = review.resolved_values;
  return {
    case_id: review.case_id,
    id: review.command.hearing_id,
    revision: review.result_revision,
    purpose: values.purpose,
    scheduled_at: values.scheduled_at,
    modality: values.modality,
    status: review.status,
    participant_count: values.participants.length,
    capture_digest: capture.capture_digest,
  };
}

export function precautionaryHearingAlert(value = precautionaryHearingOperation({ revision: 2 })) {
  const selected = precautionaryHearingOverview(value),
    row = alertRecord('upcoming');
  const at = Date.parse(selected.scheduled_at) / 1000;
  row.subject = { kind: 'precautionary_hearing', case_id: selected.case_id, id: selected.id };
  row.subject_title = 'Audiencia cautelar declarada';
  row.origin = { revision: selected.revision, evidence_digest: selected.capture_digest };
  row.kind = { kind: 'upcoming', lead_hours: 24, activity_at: alertInstant(at) };
  row.trigger_at = alertInstant(at - 86400);
  row.created_at = alertInstant(at - 86400 + 3);
  return row;
}
