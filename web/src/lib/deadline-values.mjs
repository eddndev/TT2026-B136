import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factText as text,
  factLabel as label,
  factMaxRevision,
} from './procedural-fact-primitives.mjs';
import { factDeclaration } from './procedural-fact-values.mjs';
import { factTime } from './procedural-fact-time.mjs';
export function deadlineReference(raw) {
  object(raw, ['id', 'revision']);
  return { id: uuid(raw.id), revision: revision(raw.revision) };
}
export function deadlineSourceReference(raw) {
  const family = raw?.family;
  if (family === 'resolution' || family === 'notification') {
    object(raw, ['family', 'id', 'revision', ...(family === 'notification' ? ['resolution'] : [])]);
    return {
      family,
      id: uuid(raw.id),
      revision: revision(raw.revision),
      ...(family === 'notification' ? { resolution: deadlineReference(raw.resolution) } : {}),
    };
  }
  object(raw, ['family', 'hearing_id', 'result_id', 'revision', 'agreement_id']);
  if (family !== 'hearing_result') invalid('Selecciona una fuente temporal exacta.');
  return {
    family,
    hearing_id: uuid(raw.hearing_id),
    result_id: uuid(raw.result_id),
    revision: revision(raw.revision),
    agreement_id: raw.agreement_id === null ? null : uuid(raw.agreement_id),
  };
}
function temporalQualification(raw) {
  object(raw, ['purpose', 'at', 'statement', 'locator']);
  if (!['hearing_end', 'ordered_period_start'].includes(raw.purpose)) invalid();
  return {
    purpose: raw.purpose,
    at: factTime(raw.at),
    statement: text(raw.statement),
    locator: label(raw.locator),
  };
}
function boolean(raw) {
  return factDeclaration(raw, (value) => {
    if (typeof value !== 'boolean')
      invalid('Declara si el supuesto aplica o explica lo desconocido.');
    return value;
  });
}
function applicability(raw) {
  object(raw, ['statement', 'locator', 'scope_applies', 'unresolved_incident', 'conditions']);
  if (!Array.isArray(raw.conditions) || raw.conditions.length > 16) invalid();
  const conditions = raw.conditions.map((row) => {
    object(row, ['id', 'applies', 'locator']);
    return { id: uuid(row.id), applies: boolean(row.applies), locator: label(row.locator) };
  });
  if (new Set(conditions.map((row) => row.id)).size !== conditions.length)
    invalid('Las condiciones no deben repetirse.');
  return {
    statement: text(raw.statement),
    locator: label(raw.locator),
    scope_applies: boolean(raw.scope_applies),
    unresolved_incident: boolean(raw.unresolved_incident),
    conditions,
  };
}
export function deadlineDefinition(raw) {
  object(raw, ['title', 'profile', 'responsible_id', 'input']);
  const input = raw.input;
  object(input, ['selection', 'calendar', 'ordered_quantity', 'qualification']);
  object(input.selection, ['case_id', 'source', 'qualification']);
  return {
    title: label(raw.title),
    profile: deadlineReference(raw.profile),
    responsible_id: uuid(raw.responsible_id),
    input: {
      selection: {
        case_id: uuid(input.selection.case_id),
        source: factDeclaration(input.selection.source, deadlineSourceReference),
        qualification:
          input.selection.qualification === null
            ? null
            : temporalQualification(input.selection.qualification),
      },
      calendar: input.calendar === null ? null : deadlineReference(input.calendar),
      ordered_quantity: input.ordered_quantity === null ? null : revision(input.ordered_quantity),
      qualification: applicability(input.qualification),
    },
  };
}
export function deadlineAttention(raw) {
  if (raw?.status === 'pending') {
    object(raw, ['status']);
    return { status: 'pending' };
  }
  object(raw, ['status', 'occurred_at', 'statement', 'locator']);
  if (raw.status !== 'recorded') invalid();
  return {
    status: 'recorded',
    occurred_at: factTime(raw.occurred_at),
    statement: text(raw.statement),
    locator: label(raw.locator),
  };
}
export function deadlineNormalizeCommand(raw) {
  object(raw, ['operation_id', 'deadline_id', 'change']);
  const action = raw.change?.action;
  if (!['register', 'correct', 'set_attention', 'retire'].includes(action)) invalid();
  const fields = [
    'action',
    'expected_revision',
    ...(['register', 'correct'].includes(action) ? ['definition'] : []),
    ...(action === 'set_attention' ? ['attention'] : []),
    ...(action !== 'register' ? ['reason'] : []),
  ];
  object(raw.change, fields);
  const change = { action, expected_revision: raw.change.expected_revision };
  if (action === 'register') {
    if (change.expected_revision !== 0) invalid();
  } else {
    revision(change.expected_revision, factMaxRevision - 1);
    change.reason = text(raw.change.reason, 'Motivo');
  }
  if (['register', 'correct'].includes(action))
    change.definition = deadlineDefinition(raw.change.definition);
  if (action === 'set_attention') change.attention = deadlineAttention(raw.change.attention);
  return { operation_id: uuid(raw.operation_id), deadline_id: uuid(raw.deadline_id), change };
}
