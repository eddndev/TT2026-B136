import { caseText } from './case-administration.mjs';
import { emptyDate, declaredDate, dateInterval } from './stage-dates.mjs';
export const stageLabels = {
  investigation: 'Investigaci\u00f3n',
  intermediate: 'Intermedia',
  trial: 'Juicio',
};
export const stageAction = (current) =>
  !current
    ? 'adoption'
    : current.stage === 'investigation'
      ? 'intermediate'
      : current.stage === 'intermediate'
        ? 'trial'
        : null;
export function stageDraft(current) {
  return {
    stage: 'investigation',
    known_at: emptyDate(),
    reason: '',
    support: null,
    accusation_declared_at: emptyDate(),
    accusation: null,
    opening_order_issued_at: emptyDate(),
    opening_order: null,
    received_at: emptyDate(),
    receiving_court: '',
    receipt_reference: '',
    receipt_support: null,
    note: '',
  };
}
export function supportRef(record) {
  if (
    !record ||
    !record.id ||
    !Number.isSafeInteger(record.version) ||
    record.version < 1 ||
    !/^[0-9a-f]{64}$/.test(record.digest)
  )
    throw new Error('Selecciona y confirma una versi\u00f3n exacta del documento de soporte.');
  return { document_id: record.id, version: record.version, digest: record.digest };
}
const text = (raw, label, limit, required = false, multiline = false) =>
  caseText(raw, { key: label, label, limit, required, multiline });
export function stagePayload(draft, current, now = Date.now()) {
  const action = stageAction(current),
    expected_revision = current?.stage_revision || 0;
  if (action === 'adoption') {
    if (!stageLabels[draft.stage]) throw new Error('Selecciona una etapa conocida.');
    return {
      expected_revision,
      stage: draft.stage,
      known_at: declaredDate(draft.known_at, now),
      reason: text(draft.reason, 'Motivo de adopci\u00f3n', 1000, true, true),
      support: supportRef(draft.support),
    };
  }
  const note = text(draft.note, 'Nota', 1000, false, true);
  let result;
  if (action === 'intermediate')
    result = {
      expected_revision,
      target: action,
      accusation_declared_at: declaredDate(draft.accusation_declared_at, now),
      accusation: supportRef(draft.accusation),
    };
  else if (action === 'trial') {
    const issued = declaredDate(draft.opening_order_issued_at, now),
      received = declaredDate(draft.received_at, now);
    if (dateInterval(issued)[0] > dateInterval(received)[1])
      throw new Error(
        'La emisi\u00f3n del auto no puede ser posterior a toda la fecha de recepci\u00f3n.',
      );
    result = {
      expected_revision,
      target: action,
      opening_order_issued_at: issued,
      opening_order: supportRef(draft.opening_order),
      received_at: received,
      receiving_court: text(draft.receiving_court, 'Tribunal receptor', 200, true),
    };
    const reference = text(draft.receipt_reference, 'Referencia de recepci\u00f3n', 200);
    if (reference) result.receipt_reference = reference;
    if (draft.receipt_support) result.receipt_support = supportRef(draft.receipt_support);
  } else throw new Error('No hay un avance ordinario disponible desde esta etapa.');
  if (note) result.note = note;
  return result;
}
export const uncertainStage = (error) => !error.status || error.status >= 500;
export const stageSupportFields = {
  support: 'Soporte de adopci\u00f3n',
  accusation: 'Acusaci\u00f3n',
  opening_order: 'Auto de apertura',
  receipt_support: 'Constancia de recepci\u00f3n',
};
