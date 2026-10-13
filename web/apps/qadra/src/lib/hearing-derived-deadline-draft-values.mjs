import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { cloneDraftValue } from './draft-values.mjs';
import { resultRaw, hearingSourceDraft } from './hearing-draft-values.mjs';
import { hearingResultValues } from './hearing-result-values.mjs';
import { deadlineDefinition } from './deadline-values.mjs';
import { deadlinePoliciesCommand } from './deadline-editor-policies.mjs';
import {
  uuid,
  digest,
  derivedCommand,
  derivedResultValues,
} from './hearing-derived-deadline-command.mjs';
import { derivedReady } from './hearing-derived-deadline-record.mjs';

export const hearingDerivedDeadlineDraftFields = [
  'resultId',
  'deadlineId',
  'draft',
  'anchor',
  'continuation',
  'definition',
  'policies',
  'mode',
  'last',
  'inputs',
];

function references(value) {
  uuid(value.resultId);
  uuid(value.deadlineId);
  const anchor = value.anchor;
  uuid(anchor?.hearing_id);
  revision(anchor?.revision);
  digest(anchor?.values_digest);
  digest(anchor?.submission_digest);
  const selected = value.definition?.input?.selection;
  uuid(selected?.case_id);
  const source = selected?.source;
  if (
    source?.kind !== 'known' ||
    source.value?.family !== 'hearing_result' ||
    source.value.hearing_id !== anchor.hearing_id ||
    source.value.result_id !== value.resultId ||
    source.value.revision !== 1
  )
    invalid('El borrador requiere su resultado inicial como fuente exacta.');
  if (source.value.agreement_id !== null) uuid(source.value.agreement_id);
  if (value.continuation !== null) {
    const previous = value.continuation;
    uuid(previous.hearing_id);
    uuid(previous.result_id);
    revision(previous.revision);
    digest(previous.values_digest);
    digest(previous.submission_digest);
    if (previous.result_id === value.resultId) invalid();
  }
}

function retainedReady(value) {
  if (value.mode === 'uncertain' && value.last === null)
    invalid('Falta la revision completa del envio incierto.');
  if (value.last === null) return;
  const last = value.last;
  const command = derivedCommand(
    last.command,
    value.definition.input.selection.case_id,
    value.anchor.hearing_id,
  );
  uuid(last.result?.actor_id);
  derivedReady(last, command, last.result.actor_id);
  if (
    command.result.result_id !== value.resultId ||
    command.deadline.deadline_id !== value.deadlineId ||
    !same(value.anchor, last.result.anchor) ||
    !same(value.continuation, last.result.continuation)
  )
    invalid('La revision conservada no pertenece a este borrador.');
  // Only retained, previously reviewed input is formalized; incomplete edits remain raw.
  const result = derivedResultValues(hearingResultValues(value.draft, 253402300799999));
  if (
    !same(result, command.result.change.values) ||
    !same(deadlineDefinition(value.definition), command.deadline.change.definition) ||
    !same(
      deadlinePoliciesCommand(value.policies, value.definition),
      command.deadline.change.tracking,
    )
  )
    invalid('Los campos no corresponden al envio exacto conservado.');
}

export function captureHearingDerivedDeadlineDraft(state) {
  const value = cloneDraftValue(
    Object.fromEntries(hearingDerivedDeadlineDraftFields.map((key) => [key, state[key]])),
  );
  value.mode = ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft';
  value.draft = resultRaw(value.draft);
  value.anchor = hearingSourceDraft(value.anchor);
  value.continuation = hearingSourceDraft(value.continuation);
  object(value.policies, ['profile', 'source', 'calendar']);
  for (const policy of Object.values(value.policies)) {
    object(policy, ['key', 'value']);
    if (typeof policy.key !== 'string' || typeof policy.value !== 'string') invalid();
  }
  if (value.inputs !== null && (typeof value.inputs !== 'object' || Array.isArray(value.inputs)))
    invalid();
  references(value);
  retainedReady(value);
  return value;
}

export function validateHearingDerivedDeadlineDraft(value, descriptor, principalId) {
  if (
    value.resultId !== descriptor.instanceId ||
    value.anchor.hearing_id !== descriptor.resourceId ||
    value.anchor.revision !== descriptor.baseRevision ||
    value.definition.input.selection.case_id !== descriptor.contextId ||
    (value.last !== null && value.last.result.actor_id !== principalId)
  )
    invalid('El borrador no pertenece a esta audiencia, expediente o identidad.');
}
