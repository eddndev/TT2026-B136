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
import { deadlineCalculation, deadlineResponsible } from './deadline-material.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import { deadlineAuthorV2, deadlineReceiptVersion } from './deadline-tracking.mjs';
import { deadlineTrackingContext } from './deadline-tracking-context.mjs';
import { deadlineOperational } from './deadline-operational.mjs';
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
    'version',
  ]);
  uuid(r.operation_id);
  digest(r.review_digest);
  digest(r.capture_digest);
  digest(r.submission_digest);
  if (
    !['register', 'correct', 'set_attention', 'retire', 'reevaluate'].includes(r.action) ||
    r.expected_revision !== row.revision - 1 ||
    (r.action === 'register') !== (row.revision === 1) ||
    (r.action === 'retire') !== (row.status === 'retired')
  )
    invalid('El recibo no corresponde a esta revisi\u00f3n.');
  if (r.action === 'register') {
    if (row.reason !== null) invalid();
  } else text(row.reason);
  deadlineInstant(row.recorded_at);
  deadlineAuthorV2(row.recorded_by);
  deadlineReceiptVersion(r.version, row.case_id);
  const technical = r.action === 'reevaluate';
  if (r.version.kind === 'v1') {
    if (technical || row.recorded_by.kind !== 'user') invalid();
  } else if (
    (r.version.predecessor === null) !== (r.action === 'register') ||
    (row.recorded_by.kind === 'technical') !== technical ||
    (r.version.cause !== null) !== technical
  )
    invalid();
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
    'author',
    'command',
    'result_revision',
    'definition',
    'calculation',
    'responsible',
    'attention',
    'status',
    'tracking',
    'receipt_version',
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
  deadlineAuthorV2(raw.author);
  deadlineReceiptVersion(raw.receipt_version, raw.case_id);
  if (
    raw.author.kind !== 'user' ||
    raw.author.id !== raw.actor_id ||
    raw.receipt_version.kind !== 'v2' ||
    raw.receipt_version.cause !== null ||
    (raw.receipt_version.predecessor === null) !== (change.action === 'register')
  )
    invalid();
  deadlineTrackingContext(
    raw.tracking,
    raw.definition,
    raw.calculation,
    raw.case_id,
    raw.receipt_version,
    change.action,
  );
  if (change.tracking && !same(change.tracking, raw.tracking.policies)) invalid();
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
    'tracking',
    'operational',
  ]);
  deadlineScope(raw, caseId, id, exact);
  deadlineMetadata(raw);
  content(raw, caseId);
  const reviewState = deadlineTrackingContext(
    raw.tracking,
    raw.definition,
    raw.calculation,
    caseId,
    raw.receipt.version,
    raw.receipt.action,
  );
  deadlineOperational(
    raw.operational,
    {
      receiptKind: raw.receipt.version.kind,
      status: raw.status,
      reviewState,
      dueAt: raw.calculation.result.due_at,
    },
    exact !== undefined,
  );
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
    'receipt_kind',
    'review_state',
    'calculation_due_at',
    'calculation_blocked',
    'operational',
  ]);
  deadlineScope(raw, caseId);
  label(raw.title);
  deadlineResponsible(raw.responsible);
  if (
    typeof raw.attention_recorded !== 'boolean' ||
    typeof raw.calculation_blocked !== 'boolean' ||
    raw.calculation_blocked !== (raw.calculation_due_at === null) ||
    !['v1', 'v2'].includes(raw.receipt_kind) ||
    !['accepted', 'pending', 'legacy_undeclared'].includes(raw.review_state) ||
    (raw.receipt_kind === 'v1' && raw.review_state !== 'legacy_undeclared')
  )
    invalid();
  if (raw.calculation_due_at !== null) deadlineInstant(raw.calculation_due_at);
  deadlineOperational(raw.operational, {
    receiptKind: raw.receipt_kind,
    status: raw.status,
    reviewState: raw.review_state,
    dueAt: raw.calculation_due_at,
  });
}
