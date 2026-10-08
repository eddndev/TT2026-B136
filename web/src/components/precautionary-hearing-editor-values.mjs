import { hearingInstant, hearingTimeParts } from '../lib/hearing-time.mjs';
import { precautionaryHearingCommand } from '../lib/precautionary-hearing-command.mjs';
import { factSame, factInvalid } from '../lib/procedural-fact-primitives.mjs';
import { factFailure } from '../lib/procedural-fact-errors.mjs';

export function initialPrecautionaryFields(base) {
  const values = base?.capture.review.resolved_values;
  return {
    purpose: values?.purpose ?? 'imposition',
    ...(values ? hearingTimeParts(values.scheduled_at) : { date: '', time: '', offset: '-06:00' }),
    modality: values?.modality ?? 'in_person',
    venue: values?.venue ?? '',
    note: values?.note ?? '',
    statement: values?.scheduling_basis.statement ?? '',
    locator: values?.scheduling_basis.locator ?? '',
    reason: '',
  };
}
export function precautionaryFormCommand(value, context) {
  const { fields, base, action, support } = value;
  const change = { action };
  if (action !== 'schedule')
    Object.assign(change, {
      expected_revision: base.capture.review.result_revision,
      expected_capture_digest: base.capture.capture_digest,
      reason: fields.reason,
    });
  if (action !== 'cancel') {
    if (!support) throw new Error('Selecciona el soporte exacto del senalamiento.');
    change.context = context.expectation;
    change.values = {
      purpose: fields.purpose,
      scheduled_at: hearingInstant(fields),
      modality: fields.modality,
      venue: fields.venue,
      note: fields.note.trim() || null,
      participants: value.participants.map((row) => ({
        participant_id: row.id,
        revision: row.revision,
      })),
      scheduling_basis: {
        statement: fields.statement,
        locator: fields.locator,
        support: {
          document_id: support.document_id ?? support.id,
          version: support.version,
          digest: support.digest,
        },
      },
      review_targets: value.reviewTargets.map(({ id, revision, capture_digest }) => ({
        id,
        revision,
        capture_digest,
      })),
    };
  }
  return precautionaryHearingCommand({
    case_id: value.caseId,
    operation_id: value.operationId,
    hearing_id: value.hearingId,
    change,
  });
}
export function assertPrecautionaryBase(base, current) {
  if (
    base &&
    (!current ||
      current.capture.capture_digest !== base.capture.capture_digest ||
      current.capture.review.status !== 'scheduled')
  )
    throw { status: 409, code: 'precautionary_editor_base_changed' };
}
export async function refreshPrecautionarySources(typed, documents, value, admitted) {
  for (const row of value.participants) {
    const exact = await typed.participantRevision(row.id, row.revision);
    if (!admitted()) return false;
    if (!factSame(exact, row)) factInvalid('La ficha exacta del participante cambio.');
  }
  if (value.support) {
    const ref = value.support,
      id = ref.document_id ?? ref.id;
    const scoped = documents.version(id, ref.version);
    try {
      const exact = await scoped.detail();
      if (!admitted()) return false;
      if (
        exact.case_id !== value.caseId ||
        exact.id !== id ||
        exact.version !== ref.version ||
        exact.digest !== ref.digest
      )
        factInvalid('El soporte no coincide con la version conservada.');
    } finally {
      scoped.dispose();
    }
  }
  return admitted();
}
export function precautionaryEditorFailure(error) {
  const messages = {
    precautionary_editor_base_changed:
      'La convocatoria cambio. Cierra este formulario y consulta su revision actual antes de editarla.',
    precautionary_hearing_not_found: 'La convocatoria exacta no esta disponible con esta consulta.',
    precautionary_hearing_operation_conflict:
      'La operacion ya fue utilizada. Consulta su resultado exacto.',
    precautionary_hearing_submission_mismatch: 'El envio cambio. Revisa la convocatoria de nuevo.',
    precautionary_hearing_review_mismatch:
      'Las fuentes cambiaron. Revisa la convocatoria de nuevo.',
  };
  return messages[error.code] || factFailure(error);
}
