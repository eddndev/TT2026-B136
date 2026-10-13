import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factText as text,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { factTime } from './procedural-fact-time.mjs';

function time(value) {
  if (value?.precision === 'unknown') {
    object(value, ['precision', 'reason']);
    value.reason = text(value.reason, 'Motivo');
  } else if (!same(factTime(value), value)) invalid();
}

export function measureAdministrationCommand(raw) {
  const value = structuredClone(raw);
  object(value, ['case_id', 'operation_id', 'target', 'context', 'reason', 'action']);
  uuid(value.case_id);
  uuid(value.operation_id);
  reference(value.target);
  if (value.target.revision === 4294967295) invalid();
  object(value.context, ['administration_revision', 'stage_revision', 'context_digest']);
  revision(value.context.administration_revision);
  revision(value.context.stage_revision);
  digest(value.context.context_digest);
  value.reason = text(value.reason, 'Motivo de rectificacion');
  const action = value.action;
  if (action?.kind === 'correct') {
    object(action, ['kind', 'values']);
    const values = action.values;
    object(values, ['conditions', 'validity', 'supervision_text']);
    values.conditions = text(values.conditions, 'Condiciones');
    values.supervision_text = text(values.supervision_text, 'Supervision');
    object(values.validity, ['start', 'statement', 'end']);
    values.validity.statement = text(values.validity.statement, 'Vigencia');
    time(values.validity.start);
    if (values.validity.end !== null) time(values.validity.end);
  } else if (action?.kind === 'entered_in_error') {
    object(action, ['kind']);
  } else if (action?.kind === 'replace_entered_in_error') {
    object(action, ['kind', 'replacement_id', 'subject']);
    uuid(action.replacement_id);
    if (action.replacement_id === value.target.id) invalid();
    object(action.subject, ['id', 'revision', 'values_digest']);
    uuid(action.subject.id);
    revision(action.subject.revision);
    digest(action.subject.values_digest);
  } else invalid();
  return value;
}
