import { measureDecisionCommand } from '../lib/measure-decision-command.mjs';
import { factFailure } from '../lib/procedural-fact-errors.mjs';
import { factSame } from '../lib/procedural-fact-primitives.mjs';

export function newMeasureProposal() {
  return {
    id: crypto.randomUUID(),
    subject: null,
    supervisor: null,
    values: {
      subject: null,
      kind: '',
      conditions: '',
      validity: { start: { precision: '' }, statement: '', end: null },
      supervision: { kind: 'unknown', reason: '' },
    },
  };
}
export function newMeasureEffect() {
  return {
    key: crypto.randomUUID(),
    action: 'impose',
    previous: null,
    proposal: newMeasureProposal(),
    predecessors: [],
    successors: [],
  };
}
export function proposalFromRecord(record) {
  const result = record.record.capture.result;
  return {
    id: result.id,
    values: structuredClone(result.values),
    subject: structuredClone(result.sources.subject),
    supervisor: structuredClone(result.sources.supervisor),
  };
}
function proposal(value) {
  if (!value.subject) throw new Error('Selecciona la identidad exacta del sujeto.');
  const subject = value.subject;
  const values = structuredClone(value.values);
  values.subject = {
    id: subject.id,
    revision: subject.revision,
    values_digest: subject.values_digest,
  };
  if (values.supervision.kind === 'known') {
    if (!value.supervisor) throw new Error('Selecciona la ficha exacta de supervision.');
    values.supervision.participant = {
      participant_id: value.supervisor.id,
      revision: value.supervisor.revision,
    };
  }
  return { id: value.id, values };
}
function prior(value) {
  if (!value) throw new Error('Selecciona la medida exacta.');
  return structuredClone(value.reference);
}
function effect(row) {
  if (row.action === 'impose') return { action: row.action, proposal: proposal(row.proposal) };
  if (row.action === 'substitute')
    return {
      action: row.action,
      predecessors: row.predecessors.map(prior),
      successors: row.successors.map(proposal),
    };
  const value = { action: row.action, previous: prior(row.previous) };
  if (row.action === 'modify') value.values = proposal(row.proposal).values;
  return value;
}
export function decisionFormCommand(value, context) {
  const { fields, support, anchor } = value;
  if (!support) throw new Error('Selecciona el soporte exacto de la decision.');
  if (!anchor && value.inputs?.anchor?.kind && value.inputs.anchor.kind !== 'independent')
    throw new Error('Selecciona la revision exacta de la audiencia de origen.');
  let selected = null;
  if (anchor?.kind === 'initial') {
    const row = anchor.record;
    selected = {
      kind: 'initial',
      hearing_id: row.id,
      revision: row.revision,
      values_digest: row.values_digest,
      submission_digest: row.receipt.submission_digest,
    };
  } else if (anchor?.kind === 'precautionary') {
    const row = anchor.record.capture;
    selected = {
      kind: 'precautionary',
      hearing_id: row.review.command.hearing_id,
      revision: row.review.result_revision,
      capture_digest: row.capture_digest,
    };
  }
  return measureDecisionCommand({
    case_id: value.caseId,
    operation_id: value.operationId,
    decision_id: value.decisionId,
    context: context.expectation,
    values: {
      authority: fields.authority,
      declared_at: fields.declaredAt,
      justification: fields.justification,
      locator: fields.locator,
      support: {
        document_id: support.document_id ?? support.id,
        version: support.version,
        digest: support.digest,
      },
    },
    anchor: selected,
    outcome:
      fields.outcome === 'no_measure_change'
        ? { kind: 'no_measure_change', statement: fields.statement }
        : { kind: 'changes', effects: value.effects.map(effect) },
  });
}
export async function checkDecisionPredecessors(scoped, effects, admitted) {
  const rows = effects.flatMap((row) =>
    row.action === 'substitute'
      ? row.predecessors
      : ['confirm', 'modify', 'revoke', 'cease'].includes(row.action)
        ? [row.previous]
        : [],
  );
  for (const selected of rows) {
    if (!selected) throw new Error('Selecciona la medida exacta.');
    const current = await scoped.get(selected.reference.id);
    if (!admitted()) return false;
    if (!factSame(current.reference, selected.reference))
      throw { status: 409, code: 'measure_editor_base_changed' };
  }
  return admitted();
}
export function decisionEditorFailure(error) {
  if (error.status === 413) return 'La decision supera el limite de 4 MiB.';
  const messages = {
    measure_editor_base_changed:
      'La medida cambio. Conserva el borrador y selecciona expresamente la revision que corresponde.',
    measure_decision_operation_conflict:
      'La operacion ya fue utilizada. Consulta su resultado exacto.',
    measure_decision_review_mismatch: 'Las fuentes cambiaron. Revisa la decision de nuevo.',
    measure_decision_submission_mismatch: 'El envio no coincide con la preparacion conservada.',
  };
  return messages[error.code] || factFailure(error);
}
