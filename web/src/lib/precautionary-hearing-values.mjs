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
} from './resource-hearing-values.mjs';
import { hearingTimeParts } from './hearing-time.mjs';

function supportReference(value) {
  object(value, ['document_id', 'version', 'digest']);
  uuid(value.document_id);
  revision(value.version);
  digest(value.digest);
}

export function precautionaryHearingValues(value) {
  object(value, [
    'purpose',
    'scheduled_at',
    'modality',
    'venue',
    'note',
    'participants',
    'scheduling_basis',
    'review_targets',
  ]);
  if (
    !['imposition', 'review'].includes(value.purpose) ||
    !['in_person', 'videoconference'].includes(value.modality)
  )
    invalid();
  hearingTimeParts(value.scheduled_at);
  text(value.venue, 500, false);
  if (value.note !== null) text(value.note);
  if (!Array.isArray(value.participants) || value.participants.length > 32) invalid();
  let previous = null;
  for (const person of value.participants) {
    object(person, ['participant_id', 'revision']);
    uuid(person.participant_id);
    revision(person.revision);
    if (previous !== null && person.participant_id <= previous) invalid();
    previous = person.participant_id;
  }
  const basis = value.scheduling_basis;
  object(basis, ['statement', 'support', 'locator']);
  text(basis.statement);
  text(basis.locator);
  supportReference(basis.support);
  const targets = value.review_targets;
  if (
    !Array.isArray(targets) ||
    targets.length > 32 ||
    (value.purpose === 'imposition' ? targets.length !== 0 : targets.length === 0)
  )
    invalid();
  previous = null;
  for (const target of targets) {
    object(target, ['id', 'revision', 'capture_digest']);
    uuid(target.id);
    revision(target.revision);
    digest(target.capture_digest);
    if (previous !== null && target.id <= previous) invalid();
    previous = target.id;
  }
  return value;
}

function participant(value, source, selected, caseId) {
  object(value, ['snapshot', 'overview']);
  const snapshot = value.snapshot,
    overview = value.overview;
  object(snapshot, ['case_id', 'reference', 'values_digest', 'status', 'subject']);
  object(overview, [
    'case_id',
    'id',
    'revision',
    'display_name',
    'procedural_role',
    'organization',
    'directory_status',
    'kind',
    'canonical_format',
    'subject',
  ]);
  if (
    !source ||
    typeof source !== 'object' ||
    Array.isArray(source) ||
    snapshot.case_id !== caseId ||
    overview.case_id !== caseId ||
    source.case_id !== caseId ||
    !same(snapshot.reference, selected) ||
    overview.id !== selected.participant_id ||
    source.id !== selected.participant_id ||
    overview.revision !== selected.revision ||
    source.revision !== selected.revision ||
    snapshot.values_digest !== source.values_digest ||
    snapshot.status !== source.directory_status ||
    overview.directory_status !== snapshot.status ||
    !['active', 'archived'].includes(snapshot.status)
  )
    invalid('La ficha no corresponde al participante historico seleccionado.');
  digest(snapshot.values_digest);
  for (const field of ['display_name', 'procedural_role', 'organization']) {
    if (source[field] !== overview[field]) invalid();
    if (field !== 'organization' || overview[field] !== null) text(overview[field], 200, false);
  }
  if (overview.canonical_format !== source.canonical_format) invalid();
  if (snapshot.subject === null) {
    if (
      overview.subject !== null ||
      source.subject !== null ||
      overview.kind !== null ||
      overview.canonical_format !== 'part1'
    )
      invalid();
  } else {
    const subject = snapshot.subject;
    object(subject, ['id', 'revision', 'values_digest']);
    uuid(subject.id);
    revision(subject.revision);
    digest(subject.values_digest);
    if (
      !same(overview.subject, subject) ||
      !source.subject ||
      source.subject.case_id !== caseId ||
      source.subject.id !== subject.id ||
      source.subject.revision !== subject.revision ||
      source.subject.values_digest !== subject.values_digest ||
      overview.canonical_format !== 'part2' ||
      overview.kind !== source.procedural_role
    )
      invalid();
    text(overview.kind, 200, false);
  }
}

export function precautionaryHearingSources(review) {
  const values = review.resolved_values,
    sources = review.sources;
  object(sources, ['participants', 'support']);
  const support = sources.support;
  object(support, ['document_id', 'version', 'digest', 'name', 'format', 'policy']);
  if (
    !same(values.scheduling_basis.support, {
      document_id: support.document_id,
      version: support.version,
      digest: support.digest,
    }) ||
    !['pdf', 'docx'].includes(support.format) ||
    support.policy !== 'pdf_docx_v1'
  )
    invalid();
  text(support.name, 128, false);
  if (
    !Array.isArray(sources.participants) ||
    !Array.isArray(review.participants) ||
    sources.participants.length !== values.participants.length ||
    review.participants.length !== values.participants.length
  )
    invalid();
  for (let index = 0; index < values.participants.length; index++) {
    participant(
      review.participants[index],
      sources.participants[index],
      values.participants[index],
      review.case_id,
    );
  }
}

export function precautionaryHearingContext(value, caseId) {
  object(value, [
    'case_id',
    'administration',
    'stage',
    'stage_administration',
    'context_digest',
    'expectation',
  ]);
  digest(value.context_digest);
  const { administration, stage, stage_administration: stageAdministration } = value;
  for (const entry of [administration, stageAdministration]) {
    if (!entry || entry.case_id !== caseId || entry.administrative_status !== 'active') invalid();
    revision(entry.revision);
    digest(entry.values_digest);
    text(entry.title, 200, false);
    text(entry.reference, 200, false);
  }
  if (
    !stage ||
    stage.case_id !== caseId ||
    !['initial', 'change'].includes(stage.kind) ||
    !['investigation', 'intermediate', 'trial'].includes(stage.stage)
  )
    invalid();
  revision(stage.stage_revision);
  if (
    value.case_id !== caseId ||
    stage.administration_revision !== stageAdministration.revision ||
    stage.administration_digest !== stageAdministration.values_digest ||
    !same(value.expectation, {
      administration_revision: administration.revision,
      stage_revision: stage.stage_revision,
      context_digest: value.context_digest,
    })
  )
    invalid('El contexto no corresponde al expediente y las revisiones capturadas.');
}
