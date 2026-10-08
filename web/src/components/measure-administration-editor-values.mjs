import { measureAdministrationCommand } from '../lib/measure-administration-command.mjs';
import { factSame } from '../lib/procedural-fact-primitives.mjs';
import { factFailure } from '../lib/procedural-fact-errors.mjs';

export function administrationFields(base) {
  const values = base.record.capture.result.values;
  return {
    reason: '',
    conditions: values.conditions,
    validity: structuredClone(values.validity),
    supervisionText: values.supervision.reason ?? values.supervision.statement,
  };
}
export function administrationFormCommand(value, context) {
  let action = { kind: value.action };
  if (value.action === 'correct')
    action.values = {
      conditions: value.fields.conditions,
      validity: structuredClone(value.fields.validity),
      supervision_text: value.fields.supervisionText,
    };
  if (value.action === 'replace_entered_in_error') {
    if (!value.subject) throw new Error('Selecciona la identidad exacta del sujeto.');
    const { id, revision, values_digest } = value.subject;
    action = {
      ...action,
      replacement_id: value.replacementId,
      subject: { id, revision, values_digest },
    };
  }
  return measureAdministrationCommand({
    case_id: value.caseId,
    operation_id: value.operationId,
    target: value.base.reference,
    context: context.expectation,
    reason: value.fields.reason,
    action,
  });
}
export function assertAdministrationBase(base, current) {
  if (!factSame(base.reference, current.reference) || current.validity !== 'valid')
    throw { status: 409, code: 'measure_administration_base_changed' };
}
export function administrationEditorFailure(error) {
  const messages = {
    measure_administrative_stale_head:
      'La medida cambio. Consulta la revision actual antes de iniciar otra rectificacion.',
    measure_administrative_known_dependants:
      'La captura ya tiene referencias posteriores y no admite esta rectificacion. Consulta su historia.',

    measure_administration_base_changed:
      'La medida cambio o fue marcada por error. Conserva el borrador y consulta la revision actual antes de iniciar otra rectificacion.',
    measure_administrative_operation_conflict:
      'La operacion ya fue utilizada. Consulta su resultado exacto.',
    measure_administrative_review_mismatch:
      'Las fuentes cambiaron. Revisa la rectificacion de nuevo.',
    measure_administrative_submission_mismatch:
      'El envio no coincide con la preparacion conservada.',
  };
  return messages[error.code] || factFailure(error);
}
