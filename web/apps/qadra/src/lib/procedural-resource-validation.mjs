import {
  resourceCommand,
  resourceValues,
  resourceActValues,
  resourceActions,
} from './procedural-resource-values.mjs';
import { resourceSources, resourceSupports } from './procedural-resource-sources.mjs';
import { factAdministration } from './procedural-fact-validation.mjs';
import {
  factInvalid as invalid,
  factObject as object,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factSame as same,
  factText as text,
} from './procedural-fact-primitives.mjs';
function actor(value) {
  object(value, ['id', 'email']);
  uuid(value.id);
  if (typeof value.email !== 'string' || !value.email) invalid();
}
function instant(value) {
  if (typeof value !== 'string' || !value.endsWith('Z') || !Number.isFinite(Date.parse(value)))
    invalid();
}
function stage(value, caseId, administration) {
  object(value, ['case_id', 'current']);
  if (value.case_id !== caseId) invalid('La etapa capturada pertenece a otro expediente.');
  const row = value.current;
  if (row === null) return;
  if (
    row?.case_id !== caseId ||
    !['initial', 'change'].includes(row.kind) ||
    !['investigation', 'intermediate', 'trial'].includes(row.stage)
  )
    invalid();
  object(row, [
    'kind',
    'case_id',
    'stage_revision',
    'stage',
    'administration_revision',
    'administration_digest',
    'recorded_at',
    'recorded_by',
    ...(row.kind === 'change' ? ['from_stage', 'values_digest', 'values', 'supports'] : []),
  ]);
  revision(row.stage_revision);
  revision(row.administration_revision);
  digest(row.administration_digest);
  if (
    administration.kind !== 'recorded' ||
    row.administration_revision > administration.revision ||
    (row.administration_revision === administration.revision &&
      row.administration_digest !== administration.values_digest)
  )
    invalid('La etapa no corresponde al contexto administrativo capturado.');
  actor(row.recorded_by);
  instant(row.recorded_at);
  if (row.kind === 'initial' && (row.stage_revision !== 1 || row.administration_revision !== 1))
    invalid();
  if (row.kind === 'change') {
    digest(row.values_digest);
    if (
      (row.from_stage !== null &&
        !['investigation', 'intermediate', 'trial'].includes(row.from_stage)) ||
      !row.values ||
      typeof row.values !== 'object' ||
      Array.isArray(row.values) ||
      !['adoption', 'to_intermediate', 'to_trial'].includes(row.values.kind) ||
      !Array.isArray(row.supports)
    )
      invalid('La captura del cambio de etapa esta incompleta.');
  }
}
function previous(value, expected) {
  if (!expected) {
    if (value !== null) invalid();
    return;
  }
  object(value, ['revision', 'capture_digest']);
  if (value.revision !== expected) invalid('El predecesor no corresponde a la revisi\u00f3n.');
  revision(value.revision);
  digest(value.capture_digest);
}
function values(raw) {
  const normalized = resourceValues(raw);
  if (!same(raw, normalized)) invalid('Los valores recibidos no est\u00e1n normalizados.');
}
function act(value, action, resultRevision) {
  if (!['record_act', 'correct_act'].includes(action)) {
    if (value !== null) invalid('La revisi\u00f3n contiene un acto inesperado.');
    return;
  }
  object(value, ['id', 'revision', 'values', 'supports', 'previous']);
  uuid(value.id);
  revision(value.revision);
  if (!same(value.values, resourceActValues(value.values))) invalid();
  resourceSupports(value.supports, value.values.evidence);
  if (action === 'record_act') {
    if (value.revision !== 1 || value.previous !== null) invalid();
  } else {
    if (value.revision < 2 || !value.previous || value.previous.revision >= resultRevision)
      invalid();
    previous(value.previous, value.previous?.revision);
  }
}
export function resourcePreparedValue(value) {
  uuid(value?.case_id);
  actor(value.recorded_by);
  const command = resourceCommand(value.command),
    change = command.change;
  if (!same(command, value.command) || value.result_revision !== change.expected_revision + 1)
    invalid();
  values(value.values);
  if (['register', 'correct'].includes(change.action) && !same(change.values, value.values))
    invalid();
  if (value.status !== (change.action === 'archive' ? 'archived' : 'active')) invalid();
  previous(value.previous, change.expected_revision);
  resourceSources(value.sources, value.values, value.case_id);
  act(value.act, change.action, value.result_revision);
  if (
    value.act &&
    (value.act.id !== change.act_id ||
      !same(value.act.values, change.values) ||
      value.act.revision !== (change.expected_act_revision ?? 0) + 1)
  )
    invalid();
  factAdministration(value.observed_administration, value.case_id);
  stage(value.observed_stage, value.case_id, value.observed_administration);
  digest(value.submission_digest);
  return value;
}
export function resourceRecordValue(row, caseId, id, exact) {
  if (
    row?.case_id !== caseId ||
    (id !== undefined && row.id !== id) ||
    (exact !== undefined && row.revision !== exact)
  )
    invalid('La respuesta no corresponde al expediente y revisi\u00f3n consultados.');
  uuid(row.id);
  revision(row.revision);
  actor(row.recorded_by);
  instant(row.recorded_at);
  values(row.values);
  resourceSources(row.sources, row.values, caseId);
  const receipt = row.receipt,
    action = receipt?.action;
  if (
    !Object.hasOwn(resourceActions, action) ||
    receipt.expected_revision !== row.revision - 1 ||
    (action === 'register') !== (row.revision === 1) ||
    row.status !== (action === 'archive' ? 'archived' : 'active')
  )
    invalid();
  uuid(receipt.operation_id);
  previous(receipt.previous, receipt.expected_revision);
  for (const key of ['values_digest', 'sources_digest', 'submission_digest', 'capture_digest'])
    digest(receipt[key]);
  if (['correct', 'correct_act', 'archive', 'reactivate'].includes(action))
    text(row.reason, 'Motivo');
  else if (row.reason !== null) invalid();
  act(row.act, action, row.revision);
  factAdministration(row.recorded_administration, caseId);
  stage(row.recorded_stage, caseId, row.recorded_administration);
  return row;
}
