import {
  factObject as object,
  factInvalid as invalid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
  resourceHearingReference as reference,
} from './resource-hearing-values.mjs';
import { precautionaryHearingPrincipal } from './precautionary-hearing-prepared.mjs';
import { precautionaryHearingContext } from './precautionary-hearing-values.mjs';
import { measureRecordValues } from './measure-record-values.mjs';
import { measureActions } from './measure-presentation.mjs';
import { measureAdministrationCommand } from './measure-administration-command.mjs';

function judicialOrigin(value) {
  object(value, ['operation_id', 'decision_id']);
  uuid(value.operation_id);
  uuid(value.decision_id);
}

function result(value, caseId) {
  object(value, [
    'id',
    'revision',
    'previous',
    'record_root',
    'judicial_origin',
    'last_judicial',
    'last_action',
    'validity',
    'values',
    'sources',
    'projection',
  ]);
  uuid(value.id);
  revision(value.revision);
  reference(value.previous);
  judicialOrigin(value.judicial_origin);
  const root = value.record_root;
  if (root?.kind === 'judicial') {
    object(root, ['kind', 'origin']);
    judicialOrigin(root.origin);
  } else {
    object(root, ['kind', 'operation_id', 'measure_id']);
    if (root.kind !== 'administrative') invalid();
    uuid(root.operation_id);
    uuid(root.measure_id);
  }
  object(value.last_judicial, ['owner', 'reference']);
  reference(value.last_judicial.reference);
  const owner = value.last_judicial.owner;
  object(owner, ['operation_id', 'decision_id', 'group_digest']);
  uuid(owner.operation_id);
  uuid(owner.decision_id);
  digest(owner.group_digest);
  if (
    !Object.hasOwn(measureActions, value.last_action) ||
    !['valid', 'entered_in_error'].includes(value.validity)
  )
    invalid();
  measureRecordValues(value, caseId);
}

function support(value) {
  object(value, ['document_id', 'version', 'digest', 'name', 'format', 'policy']);
  uuid(value.document_id);
  revision(value.version);
  digest(value.digest);
  text(value.name, 128, false);
  if (!['pdf', 'docx'].includes(value.format) || value.policy !== 'pdf_docx_v1') invalid();
}

export function measureAdministrationPrepared(value) {
  object(value, [
    'case_id',
    'actor',
    'command',
    'context',
    'support',
    'result',
    'replacement',
    'submission_digest',
    'review_digest',
  ]);
  uuid(value.case_id);
  precautionaryHearingPrincipal(value.actor);
  digest(value.submission_digest);
  digest(value.review_digest);
  const command = measureAdministrationCommand(value.command);
  if (command.case_id !== value.case_id || !same(command, value.command)) invalid();
  precautionaryHearingContext(value.context, value.case_id);
  if (!same(command.context, value.context.expectation)) invalid();
  support(value.support);
  result(value.result, value.case_id);
  const { target, action } = command,
    current = value.result;
  if (
    current.id !== target.id ||
    current.revision !== target.revision + 1 ||
    !same(current.previous, target)
  )
    invalid();
  if (action.kind === 'correct') {
    const supervision = current.values.supervision;
    if (
      current.validity !== 'valid' ||
      value.replacement !== null ||
      current.values.conditions !== action.values.conditions ||
      !same(current.values.validity, action.values.validity) ||
      action.values.supervision_text !==
        (supervision.kind === 'known' ? supervision.statement : supervision.reason)
    )
      invalid();
  } else if (action.kind === 'entered_in_error') {
    if (current.validity !== 'entered_in_error' || value.replacement !== null) invalid();
  } else {
    const replacement = value.replacement;
    result(replacement, value.case_id);
    if (
      current.validity !== 'entered_in_error' ||
      replacement.validity !== 'valid' ||
      replacement.id !== action.replacement_id ||
      replacement.revision !== 1 ||
      !same(replacement.previous, target) ||
      !same(replacement.values.subject, action.subject) ||
      !same(replacement.record_root, {
        kind: 'administrative',
        operation_id: command.operation_id,
        measure_id: action.replacement_id,
      }) ||
      !same(replacement.judicial_origin, current.judicial_origin) ||
      !same(replacement.last_judicial, current.last_judicial)
    )
      invalid();
  }
  return value;
}
