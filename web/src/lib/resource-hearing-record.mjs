import {
  factObject as object,
  factInvalid as invalid,
  factSame as same,
  factRevision as revision,
} from './procedural-fact-primitives.mjs';
import { factAdministration, factAdministrationFollows } from './procedural-fact-validation.mjs';
import { resourceRecordValue } from './procedural-resource-validation.mjs';
import { profileKinds } from './typed-participant-fields.mjs';
import {
  resourceHearingUuid as uuid,
  resourceHearingDigest as digest,
  resourceHearingText as text,
  resourceHearingUtc as utc,
  resourceHearingReference as reference,
  resourceHearingValues as values,
} from './resource-hearing-values.mjs';

function actor(value) {
  object(value, ['id', 'email']);
  uuid(value.id);
  text(value.email, 320, false);
}
function administration(value, caseId) {
  factAdministration(value, caseId);
  if (value.kind === 'recorded') {
    object(value, [
      'kind',
      'case_id',
      'revision',
      'values_digest',
      'title',
      'reference',
      'status',
      'changed_at',
      'changed_by',
    ]);
    uuid(value.case_id);
    digest(value.values_digest);
    actor(value.changed_by);
    utc(value.changed_at);
  }
}
function source(row, selected, caseId, resourceId, act = false) {
  object(row, [
    'case_id',
    'id',
    'revision',
    'values',
    'status',
    'sources',
    'act',
    'reason',
    'recorded_by',
    'recorded_at',
    'recorded_administration',
    'recorded_stage',
    'receipt',
  ]);
  resourceRecordValue(
    row,
    caseId,
    resourceId,
    act ? selected.resource_revision : selected.revision,
  );
  actor(row.recorded_by);
  utc(row.recorded_at);
  administration(row.recorded_administration, caseId);
  if (
    row.receipt.capture_digest !== selected.capture_digest ||
    (act && (row.act?.id !== selected.id || row.act.revision !== selected.revision))
  )
    invalid();
}
function support(row, expected) {
  object(row, ['document_id', 'version', 'digest', 'name', 'format', 'policy']);
  uuid(row.document_id);
  revision(row.version);
  digest(row.digest);
  text(row.name, 128, false);
  if (
    row.document_id !== expected.document_id ||
    row.version !== expected.version ||
    row.digest !== expected.digest ||
    !['pdf', 'docx'].includes(row.format) ||
    row.policy !== 'pdf_docx_v1'
  )
    invalid();
}
function participants(rows, selected, caseId) {
  if (!Array.isArray(rows) || rows.length !== selected.length) invalid();
  rows.forEach((row, index) => {
    object(row, [
      'case_id',
      'id',
      'revision',
      'values_digest',
      'directory_status',
      'subject',
      'display_name',
      'procedural_role',
      'organization',
      'kind',
    ]);
    uuid(row.case_id);
    uuid(row.id);
    revision(row.revision);
    digest(row.values_digest);
    if (
      row.case_id !== caseId ||
      row.id !== selected[index].participant_id ||
      row.revision !== selected[index].revision ||
      row.directory_status !== 'active'
    )
      invalid();
    text(row.display_name, 200, false);
    text(row.procedural_role, 200, false);
    if (row.organization !== null) text(row.organization, 200, false);
    if (row.subject === null) {
      if (row.kind !== null) invalid();
    } else {
      object(row.subject, ['id', 'revision', 'values_digest']);
      uuid(row.subject.id);
      revision(row.subject.revision);
      digest(row.subject.values_digest);
      if (!profileKinds.some((kind) => kind.key === row.kind)) invalid();
    }
  });
}

export function resourceHearingRecord(row, caseId, resourceId) {
  object(row, [
    'case_id',
    'resource_id',
    'id',
    'revision',
    'operation_id',
    'association_id',
    'expected_resource_revision',
    'resource',
    'act',
    'values',
    'sources',
    'recorded_by',
    'recorded_at',
    'recorded_administration',
    'recorded_resource_head',
    'submission_digest',
    'capture_digest',
  ]);
  for (const key of ['case_id', 'resource_id', 'id', 'operation_id', 'association_id'])
    uuid(row[key]);
  if (row.case_id !== caseId || row.resource_id !== resourceId || row.revision !== 1) invalid();
  revision(row.expected_resource_revision);
  reference(row.resource);
  reference(row.recorded_resource_head);
  if (row.act !== null) reference(row.act, true);
  const head = row.recorded_resource_head;
  if (
    row.resource.id !== resourceId ||
    head.id !== resourceId ||
    head.revision !== row.expected_resource_revision ||
    row.resource.revision > head.revision ||
    (row.act !== null && row.act.resource_revision > head.revision) ||
    (row.resource.revision === head.revision &&
      row.resource.capture_digest !== head.capture_digest) ||
    (row.act?.resource_revision === head.revision && row.act.capture_digest !== head.capture_digest)
  )
    invalid();
  values(row.values);
  digest(row.submission_digest);
  digest(row.capture_digest);
  actor(row.recorded_by);
  const recordedAt = utc(row.recorded_at);
  administration(row.recorded_administration, caseId);
  if (
    row.recorded_administration.kind === 'recorded' &&
    recordedAt < utc(row.recorded_administration.changed_at)
  )
    invalid();
  object(row.sources, ['resource', 'act', 'support', 'participants']);
  source(row.sources.resource, row.resource, caseId, resourceId);
  if (row.act === null) {
    if (row.sources.act !== null) invalid();
  } else {
    source(row.sources.act, row.act, caseId, resourceId, true);
    if (
      row.act.resource_revision === row.resource.revision &&
      !same(row.sources.act, row.sources.resource)
    )
      invalid();
  }
  const selected = row.sources.resource;
  const expectedKind = row.values.kind === 'appeal_arguments' ? 'appeal' : 'revocation';
  if (
    selected.values.kind !== expectedKind ||
    selected.values.mode.kind !== 'known' ||
    selected.values.mode.value !== 'written'
  )
    invalid();
  for (const captured of [selected, row.sources.act].filter(Boolean)) {
    if (
      recordedAt < utc(captured.recorded_at) ||
      !factAdministrationFollows(
        row.recorded_administration,
        captured.recorded_administration,
        caseId,
      )
    )
      invalid();
  }
  support(row.sources.support, row.values.scheduling_basis.support);
  const admitted = [...selected.sources.supports, ...(row.sources.act?.act.supports || [])];
  let found = false;
  for (const captured of admitted) {
    if (
      captured.document_id === row.sources.support.document_id &&
      captured.version === row.sources.support.version
    ) {
      if (!same(captured, row.sources.support)) invalid();
      found = true;
    }
  }
  if (!found) invalid('El soporte no fue admitido en la captura seleccionada.');
  participants(row.sources.participants, row.values.participants, caseId);
  return row;
}
