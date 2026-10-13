import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
  resourceHearingUtc as utc,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { measureRecordValues } from './measure-record-values.mjs';
import { measureActions } from './measure-presentation.mjs';

function origin(value) {
  object(value, ['operation_id', 'decision_id']);
  uuid(value.operation_id);
  uuid(value.decision_id);
}

function root(value) {
  if (value?.kind === 'judicial') {
    object(value, ['kind', 'origin']);
    origin(value.origin);
  } else {
    object(value, ['kind', 'operation_id', 'measure_id']);
    if (value.kind !== 'administrative') invalid();
    uuid(value.operation_id);
    uuid(value.measure_id);
  }
}

function group(value) {
  object(value, ['operation_id', 'decision_id', 'group_digest']);
  uuid(value.operation_id);
  uuid(value.decision_id);
  digest(value.group_digest);
}

function history(value) {
  object(value, ['records', 'decisions']);
  object(value.records, ['judicial', 'administrative']);
  object(value.records.judicial, ['groups']);
  const arrays = [value.records.judicial.groups, value.records.administrative, value.decisions];
  if (!arrays.every(Array.isArray) || arrays.reduce((count, rows) => count + rows.length, 0) > 256)
    invalid();
}

export function measureRecord(value, caseId, id, selected = null) {
  uuid(caseId);
  uuid(id);
  object(value, [
    'case_id',
    'reference',
    'family',
    'validity',
    'last_action',
    'record_root',
    'judicial_origin',
    'last_judicial',
    'record',
    'record_history',
  ]);
  reference(value.reference);
  if (
    value.case_id !== caseId ||
    value.reference.id !== id ||
    (selected !== null && !same(value.reference, selected)) ||
    !['m1', 'm2', 'c1'].includes(value.family) ||
    !Object.hasOwn(measureActions, value.last_action)
  )
    invalid();
  root(value.record_root);
  origin(value.judicial_origin);
  object(value.last_judicial, ['owner', 'reference']);
  group(value.last_judicial.owner);
  reference(value.last_judicial.reference);
  object(value.record, ['family', 'owner', 'capture']);
  const { family, owner, capture } = value.record;
  const administrative = family === 'c1';
  object(capture, [
    'family',
    'case_id',
    'operation_id',
    'result',
    'actor',
    'recorded_at',
    'capture_digest',
    ...(administrative
      ? ['context', 'support', 'review_digest']
      : ['decision_id', 'decision_digest']),
  ]);
  if (
    family !== value.family ||
    capture.family !== family ||
    capture.case_id !== caseId ||
    capture.capture_digest !== value.reference.capture_digest
  )
    invalid();
  digest(capture.capture_digest);
  utc(capture.recorded_at);
  object(capture.actor, ['id', 'email', 'role']);
  uuid(capture.actor.id);
  text(capture.actor.email, 320, false);
  if (!['owner', 'litigator'].includes(capture.actor.role)) invalid();
  if (administrative) {
    object(owner, ['operation_id', 'capture_digest']);
    uuid(owner.operation_id);
    digest(owner.capture_digest);
    digest(capture.review_digest);
  } else {
    group(owner);
    if (capture.decision_id !== owner.decision_id) invalid();
    digest(capture.decision_digest);
  }
  if (capture.operation_id !== owner.operation_id) invalid();
  const result = capture.result;
  object(result, [
    'id',
    'revision',
    'previous',
    'values',
    'sources',
    'projection',
    ...(family === 'm1' ? ['origin'] : ['record_root', 'judicial_origin']),
    ...(administrative ? ['last_judicial', 'last_action', 'validity'] : ['effect_key', 'action']),
  ]);
  if (result.id !== id || result.revision !== value.reference.revision) invalid();
  if (result.previous !== null) reference(result.previous);
  if (!administrative) uuid(result.effect_key);
  if (
    value.last_action !== (administrative ? result.last_action : result.action) ||
    value.validity !== (administrative ? result.validity : 'valid') ||
    !['valid', 'entered_in_error'].includes(value.validity) ||
    !same(value.judicial_origin, family === 'm1' ? result.origin : result.judicial_origin) ||
    !same(
      value.record_root,
      family === 'm1' ? { kind: 'judicial', origin: result.origin } : result.record_root,
    ) ||
    !same(
      value.last_judicial,
      administrative ? result.last_judicial : { owner, reference: value.reference },
    )
  )
    invalid('La lectura no conserva la declaracion y procedencia de su captura exacta.');
  measureRecordValues(result, caseId);
  // The authorized server validates the complete owning history and its digests.
  history(value.record_history);
  return value;
}
