import { resourceRecordValue } from './procedural-resource-validation.mjs';
import { validateHearing } from './hearings-api.mjs';
import { deadlineRecordValue } from './deadline-validation.mjs';
import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineInstant } from './deadline-time.mjs';

export function resourceActivitySources(value, selection, caseId, resourceId) {
  object(value, ['resource', 'act', 'target']);
  const resource = resourceRecordValue(
    value.resource,
    caseId,
    resourceId,
    selection.resource.revision,
  );
  if (resource.receipt.capture_digest !== selection.resource.capture_digest) invalid();
  if (selection.act === null) {
    if (value.act !== null) invalid();
  } else {
    const ref = selection.act;
    const row = resourceRecordValue(value.act, caseId, resourceId, ref.resource_revision);
    if (
      row.receipt.capture_digest !== ref.capture_digest ||
      row.act?.id !== ref.id ||
      row.act.revision !== ref.revision
    )
      invalid();
    if (row.revision === resource.revision && !same(row, resource)) invalid();
  }
  resourceActivityTargetRecord(value.target, selection.target, caseId, true);
  return value;
}
export function resourceActivityTargetRecord(value, reference, caseId, exact) {
  object(value, ['kind', 'record']);
  if (value.kind !== reference.kind) invalid();
  const row = value.record;
  if (value.kind === 'hearing')
    validateHearing(row, caseId, reference.id, exact ? reference.revision : undefined);
  else deadlineRecordValue(row, caseId, reference.id, exact ? reference.revision : undefined);
  if (row.revision < reference.revision) invalid();
  const field = value.kind === 'hearing' ? 'submission_digest' : 'capture_digest';
  if (exact && row.receipt[field] !== reference[field]) invalid();
  return value;
}
export function resourceActivityCurrent(value, association, checkedAt) {
  deadlineInstant(checkedAt);
  if (checkedAt.offset_seconds !== 0) invalid();
  resourceActivityTargetRecord(value, association.selection.target, association.case_id, false);
  if (value.kind === 'deadline') {
    const checked = value.record.operational.checked_at;
    if (checked !== null && !same(checked, checkedAt))
      invalid('La vigencia pertenece a otra consulta.');
  }
  if (value.record.revision === association.selection.target.revision) {
    const exact = structuredClone(association.sources.target.record);
    const current = structuredClone(value.record);
    if (value.kind === 'deadline') {
      delete exact.operational;
      delete current.operational;
    }
    if (!same(exact, current)) invalid('La misma revision no puede sustituir su captura.');
  }
  return value;
}
