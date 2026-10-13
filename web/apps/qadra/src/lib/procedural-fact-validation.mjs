import {
  factInvalid as invalid,
  factObject as object,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
  factLabel as label,
  factSame as same,
  factMaxRevision,
} from './procedural-fact-primitives.mjs';
import { factValues, factDeclaration, factCatalog } from './procedural-fact-values.mjs';
import { factTime } from './procedural-fact-time.mjs';
import { factSources } from './procedural-fact-sources.mjs';
export function factNormalizeCommand(raw) {
  if (!['resolution', 'notification'].includes(raw?.family)) invalid();
  const keys = [
    'family',
    'operation_id',
    'id',
    'change',
    ...(raw.family === 'notification' ? ['resolution_id'] : []),
  ];
  object(raw, keys);
  const action = raw.change?.action;
  if (!['record', 'correct', 'withdraw'].includes(action)) invalid();
  const fields = [
    'action',
    'expected_revision',
    ...(action !== 'withdraw' ? ['values'] : []),
    ...(action !== 'record' ? ['reason'] : []),
  ];
  object(raw.change, fields);
  const result = {
    family: raw.family,
    operation_id: uuid(raw.operation_id),
    id: uuid(raw.id),
    ...(raw.family === 'notification' ? { resolution_id: uuid(raw.resolution_id) } : {}),
    change: { action, expected_revision: raw.change.expected_revision },
  };
  if (action === 'record') {
    if (raw.change.expected_revision !== 0) invalid();
  } else {
    revision(raw.change.expected_revision, factMaxRevision - 1);
    result.change.reason = text(raw.change.reason, 'Motivo');
  }
  if (action !== 'withdraw') result.change.values = factValues(raw.family, raw.change.values);
  if (
    raw.family === 'notification' &&
    result.change.values?.resolution.id !== undefined &&
    result.change.values.resolution.id !== result.resolution_id
  )
    invalid('La resoluci\u00f3n padre es inmutable.');
  return result;
}
export function factAdministration(value, caseId) {
  if (value?.status !== 'active')
    invalid('El contexto capturado no corresponde a una captura activa.');
  label(value.title);
  if (value.reference !== null) label(value.reference);
  if (value.kind === 'unrevised') object(value, ['kind', 'title', 'reference', 'status']);
  else if (value.kind === 'recorded') {
    if (value.case_id !== caseId) invalid();
    revision(value.revision);
    digest(value.values_digest);
    if (
      typeof value.changed_at !== 'string' ||
      !value.changed_at.endsWith('Z') ||
      !Number.isFinite(Date.parse(value.changed_at))
    )
      invalid();
    if (!value.changed_by?.id || !value.changed_by.email) invalid();
  } else invalid('Falta el contexto administrativo capturado.');
  return value;
}
export function factAdministrationFollows(current, previous, caseId) {
  factAdministration(current, caseId);
  factAdministration(previous, caseId);
  if (previous.kind === 'recorded')
    return (
      current.kind === 'recorded' &&
      (current.revision > previous.revision ||
        (current.revision === previous.revision && same(current, previous)))
    );
  return current.kind === 'recorded' || same(current, previous);
}
export function factPreparedValue(value) {
  uuid(value?.case_id);
  if (typeof value.actor_id !== 'string' || !value.actor_id) invalid();
  const command = factNormalizeCommand(value.command);
  if (
    !same(command, value.command) ||
    value.result_revision !== command.change.expected_revision + 1
  )
    invalid();
  digest(value.values_digest);
  digest(value.sources_digest);
  digest(value.submission_digest);
  const normalized = factValues(command.family, value.values);
  if (
    !same(normalized, value.values) ||
    (command.change.values && !same(command.change.values, value.values))
  )
    invalid('Los valores preparados no corresponden al comando.');
  if (command.family === 'notification' && normalized.resolution.id !== command.resolution_id)
    invalid();
  factSources(value.sources, normalized, value.case_id);
  factAdministration(value.observed_administration, value.case_id);
  return value;
}
export function factScope(row, caseId, family, parent, id, exactRevision) {
  if (
    !row ||
    row.case_id !== caseId ||
    row.family !== family ||
    (id !== undefined && row.id !== id) ||
    (family === 'notification' ? row.resolution_id !== parent : Object.hasOwn(row, 'resolution_id'))
  )
    invalid('La respuesta no corresponde al expediente, familia o padre consultados.');
  uuid(row.id);
  revision(row.revision);
  if (exactRevision !== undefined && row.revision !== exactRevision)
    invalid('La revisi\u00f3n consultada no coincide.');
  if (!['recorded', 'withdrawn'].includes(row.status)) invalid();
  return row;
}
export function factMetadata(row) {
  const receipt = row.receipt;
  digest(row.values_digest);
  digest(receipt?.sources_digest);
  digest(receipt?.submission_digest);
  uuid(receipt?.operation_id);
  if (
    !['record', 'correct', 'withdraw'].includes(receipt.action) ||
    receipt.expected_revision !== row.revision - 1 ||
    (receipt.action === 'record') !== (receipt.expected_revision === 0) ||
    row.status !== (receipt.action === 'withdraw' ? 'withdrawn' : 'recorded')
  )
    invalid('El recibo no corresponde a la revisi\u00f3n.');
  if (receipt.action === 'record') {
    if (row.reason !== null) invalid();
  } else text(row.reason, 'Motivo');
  if (
    !row.recorded_by?.id ||
    !row.recorded_by.email ||
    typeof row.recorded_at !== 'string' ||
    !row.recorded_at.endsWith('Z') ||
    !Number.isFinite(Date.parse(row.recorded_at))
  )
    invalid();
  factAdministration(row.recorded_administration, row.case_id);
}
export function factRecordValue(row, caseId, family, parent, id, exactRevision) {
  factScope(row, caseId, family, parent, id, exactRevision);
  factMetadata(row);
  const normalized = factValues(family, row.values);
  if (
    !same(row.values, normalized) ||
    (family === 'notification' && normalized.resolution.id !== parent)
  )
    invalid();
  factSources(row.sources, normalized, caseId);
  return row;
}
export function factListRow(row, caseId, family, parent) {
  factScope(row, caseId, family, parent);
  if (family === 'resolution') {
    factDeclaration(row.class, (v) => factCatalog(v, ['order', 'judgment']));
    factTime(row.issued_at);
  } else {
    if (row.resolution?.id !== parent) invalid();
    revision(row.resolution.revision);
    factDeclaration(row.outcome, (v) => factCatalog(v, ['practiced', 'attempted'], false));
    factTime(row.practiced_at);
  }
}
