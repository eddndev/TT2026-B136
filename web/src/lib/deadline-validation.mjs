import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
  factLabel as label,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import {
  deadlineNormalizeCommand,
  deadlineDefinition,
  deadlineAttention,
} from './deadline-values.mjs';
import { deadlineCalculation, deadlineResponsible, deadlineAuthor } from './deadline-material.mjs';
import { deadlineInstant } from './deadline-time.mjs';
export function deadlineScope(raw, caseId, id, exact) {
  uuid(raw?.id);
  revision(raw.revision);
  if (
    raw.case_id !== caseId ||
    (id !== undefined && raw.id !== id) ||
    (exact !== undefined && raw.revision !== exact) ||
    !['active', 'retired'].includes(raw.status)
  )
    invalid('La respuesta no corresponde al expediente, plazo o revisi\u00f3n consultados.');
}
export function deadlineMetadata(row) {
  const r = row.receipt;
  object(r, [
    'operation_id',
    'action',
    'expected_revision',
    'review_digest',
    'capture_digest',
    'submission_digest',
  ]);
  uuid(r.operation_id);
  digest(r.review_digest);
  digest(r.capture_digest);
  digest(r.submission_digest);
  if (
    !['register', 'correct', 'set_attention', 'retire'].includes(r.action) ||
    r.expected_revision !== row.revision - 1 ||
    (r.action === 'register') !== (row.revision === 1) ||
    (r.action === 'retire') !== (row.status === 'retired')
  )
    invalid('El recibo no corresponde a esta revisi\u00f3n.');
  if (r.action === 'register') {
    if (row.reason !== null) invalid();
  } else text(row.reason);
  deadlineInstant(row.recorded_at);
  deadlineAuthor(row.recorded_by);
}
function content(raw, caseId) {
  if (
    !same(deadlineDefinition(raw.definition), raw.definition) ||
    raw.definition.input.selection.case_id !== caseId ||
    !same(deadlineAttention(raw.attention), raw.attention)
  )
    invalid();
  deadlineResponsible(raw.responsible);
  if (raw.responsible.id !== raw.definition.responsible_id) invalid();
  deadlineCalculation(raw.calculation, raw.definition, caseId);
}
export function deadlinePreparedValue(raw) {
  object(raw, [
    'case_id',
    'actor_id',
    'command',
    'result_revision',
    'definition',
    'calculation',
    'responsible',
    'attention',
    'status',
    'review_digest',
    'capture_digest',
    'submission_digest',
  ]);
  uuid(raw.case_id);
  uuid(raw.actor_id);
  const command = deadlineNormalizeCommand(raw.command),
    change = command.change;
  if (
    !same(command, raw.command) ||
    raw.result_revision !== change.expected_revision + 1 ||
    raw.status !== (change.action === 'retire' ? 'retired' : 'active')
  )
    invalid();
  digest(raw.review_digest);
  digest(raw.capture_digest);
  digest(raw.submission_digest);
  content(raw, raw.case_id);
  if (change.definition && !same(change.definition, raw.definition)) invalid();
  if (change.attention && !same(change.attention, raw.attention)) invalid();
  if (change.action === 'register' && raw.attention.status !== 'pending') invalid();
  return raw;
}
export function deadlineRecordValue(raw, caseId, id, exact) {
  object(raw, [
    'id',
    'case_id',
    'revision',
    'definition',
    'calculation',
    'responsible',
    'attention',
    'status',
    'reason',
    'receipt',
    'recorded_at',
    'recorded_by',
  ]);
  deadlineScope(raw, caseId, id, exact);
  deadlineMetadata(raw);
  content(raw, caseId);
  if (raw.receipt.action === 'register' && raw.attention.status !== 'pending') invalid();
  return raw;
}
export function deadlineListRow(raw, caseId) {
  object(raw, [
    'id',
    'case_id',
    'revision',
    'title',
    'status',
    'responsible',
    'attention_recorded',
    'due_at',
    'blocked',
  ]);
  deadlineScope(raw, caseId);
  label(raw.title);
  deadlineResponsible(raw.responsible);
  if (
    typeof raw.attention_recorded !== 'boolean' ||
    typeof raw.blocked !== 'boolean' ||
    raw.blocked !== (raw.due_at === null)
  )
    invalid();
  if (raw.due_at !== null) deadlineInstant(raw.due_at);
}
