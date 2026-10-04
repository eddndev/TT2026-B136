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
} from './resource-hearing-values.mjs';

export function resourceHearingActor(value) {
  object(value, ['id', 'email']);
  uuid(value.id);
  text(value.email, 320, false);
}
export function resourceHearingAdministration(value, caseId) {
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
    resourceHearingActor(value.changed_by);
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
  resourceHearingActor(row.recorded_by);
  utc(row.recorded_at);
  resourceHearingAdministration(row.recorded_administration, caseId);
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

export function resourceHearingMaterial(command, resource, act, sources, admin, head) {
  const caseId = command.case_id,
    resourceId = command.resource_id;
  reference(head);
  resourceHearingAdministration(admin, caseId);
  if (
    head.id !== resourceId ||
    head.revision !== command.expected_resource_revision ||
    (command.resource.revision === head.revision &&
      command.resource.capture_digest !== head.capture_digest) ||
    (command.act?.resource_revision === head.revision &&
      command.act.capture_digest !== head.capture_digest)
  )
    invalid();
  source(resource, command.resource, caseId, resourceId);
  if (command.act === null) {
    if (act !== null) invalid();
  } else {
    source(act, command.act, caseId, resourceId, true);
    if (command.act.resource_revision === command.resource.revision && !same(act, resource))
      invalid();
  }
  const expectedKind = command.values.kind === 'appeal_arguments' ? 'appeal' : 'revocation';
  if (
    resource.values.kind !== expectedKind ||
    resource.values.mode.kind !== 'known' ||
    resource.values.mode.value !== 'written'
  )
    invalid();
  for (const captured of [resource, act].filter(Boolean)) {
    if (!factAdministrationFollows(admin, captured.recorded_administration, caseId)) invalid();
  }
  object(sources, ['support', 'participants']);
  support(sources.support, command.values.scheduling_basis.support);
  const admitted = [...resource.sources.supports, ...(act?.act.supports || [])];
  let found = false;
  for (const captured of admitted) {
    if (
      captured.document_id === sources.support.document_id &&
      captured.version === sources.support.version
    ) {
      if (!same(captured, sources.support)) invalid();
      found = true;
    }
  }
  if (!found) invalid('El soporte no fue admitido en la captura seleccionada.');
  participants(sources.participants, command.values.participants, caseId);
}
