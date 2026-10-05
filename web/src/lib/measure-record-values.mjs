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
import { factTime } from './procedural-fact-time.mjs';
import { subjectValues } from './typed-participant-values.mjs';
import { measureKinds } from './measure-presentation.mjs';

function declaredTime(value) {
  if (value?.precision === 'unknown') {
    object(value, ['precision', 'reason']);
    text(value.reason);
  } else if (!same(factTime(value), value)) invalid();
}

function subject(reference, source, projection, caseId) {
  object(reference, ['id', 'revision', 'values_digest']);
  uuid(reference.id);
  revision(reference.revision);
  digest(reference.values_digest);
  object(source, [
    'case_id',
    'id',
    'revision',
    'values',
    'values_digest',
    'changed_at',
    'changed_by',
  ]);
  object(projection, ['case_id', 'id', 'revision', 'kind', 'display_name']);
  if (
    source.case_id !== caseId ||
    projection.case_id !== caseId ||
    source.id !== reference.id ||
    projection.id !== reference.id ||
    source.revision !== reference.revision ||
    projection.revision !== reference.revision ||
    source.values_digest !== reference.values_digest ||
    !same(subjectValues(source.values), source.values) ||
    projection.kind !== source.values.kind
  )
    invalid('La identidad visible no corresponde a la fuente exacta de la medida.');
  const name =
    source.values.kind === 'institutional_body'
      ? source.values.name
      : source.values.name.state === 'known'
        ? source.values.name.value
        : source.values.name.label;
  text(projection.display_name, 200, false);
  if (projection.display_name !== name) invalid();
}

function supervision(value, source, projection, caseId) {
  if (value?.kind === 'unknown') {
    object(value, ['kind', 'reason']);
    text(value.reason);
    if (source !== null || projection !== null) invalid();
    return;
  }
  object(value, ['kind', 'participant', 'statement']);
  if (value.kind !== 'known') invalid();
  text(value.statement);
  object(value.participant, ['participant_id', 'revision']);
  uuid(value.participant.participant_id);
  revision(value.participant.revision);
  object(projection, ['snapshot', 'overview']);
  const { snapshot, overview } = projection;
  if (
    !source ||
    !snapshot ||
    !overview ||
    source.case_id !== caseId ||
    snapshot.case_id !== caseId ||
    overview.case_id !== caseId ||
    source.id !== value.participant.participant_id ||
    overview.id !== source.id ||
    source.revision !== value.participant.revision ||
    overview.revision !== source.revision ||
    !same(snapshot.reference, value.participant) ||
    snapshot.values_digest !== source.values_digest ||
    snapshot.status !== source.directory_status ||
    overview.directory_status !== snapshot.status
  )
    invalid();
  digest(snapshot.values_digest);
  for (const field of ['display_name', 'procedural_role', 'organization']) {
    if (overview[field] !== source[field]) invalid();
    if (overview[field] !== null) text(overview[field], 200, false);
  }
}

export function measureRecordValues(result, caseId) {
  const { values, sources, projection } = result;
  object(values, ['subject', 'kind', 'conditions', 'validity', 'supervision']);
  if (!Object.hasOwn(measureKinds, values.kind)) invalid();
  text(values.conditions);
  object(values.validity, ['start', 'statement', 'end']);
  text(values.validity.statement);
  declaredTime(values.validity.start);
  if (values.validity.end !== null) declaredTime(values.validity.end);
  object(sources, ['subject', 'supervisor']);
  object(projection, ['subject', 'supervisor']);
  subject(values.subject, sources.subject, projection.subject, caseId);
  supervision(values.supervision, sources.supervisor, projection.supervisor, caseId);
}
