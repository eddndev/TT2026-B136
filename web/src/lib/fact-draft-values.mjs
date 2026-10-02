const pick = (value, keys) =>
  value == null
    ? value
    : Object.fromEntries(
        keys.filter((key) => Object.hasOwn(value, key)).map((key) => [key, value[key]]),
      );
const time = (value) =>
  pick(value, ['precision', 'year', 'month', 'day', 'hour', 'minute', 'second', 'offset_seconds']);
const person = (value) => pick(value, ['kind', 'id', 'revision', 'label', 'description']);
const declaration = (
  value,
  project = (entry) => (typeof entry === 'object' ? pick(entry, ['kind', 'label']) : entry),
) => {
  if (!value) return value;
  return {
    ...pick(value, ['kind', 'reason']),
    ...(Object.hasOwn(value, 'value') ? { value: project(value.value) } : {}),
  };
};
function provenance(value) {
  if (!value) return value;
  const result = pick(value, ['kind', 'note', 'locator']);
  if (Object.hasOwn(value, 'reference'))
    result.reference =
      typeof value.reference === 'object'
        ? pick(value.reference, ['hearing_id', 'result_id', 'revision', 'agreement_id'])
        : value.reference;
  if (Object.hasOwn(value, 'support'))
    result.support = pick(value.support, ['document_id', 'version', 'digest', 'locator']);
  return result;
}
export function rawFactValues(value, family) {
  const common = {
    ...pick(value, ['subtype', 'summary']),
    provenance: provenance(value.provenance),
  };
  if (family === 'resolution')
    return {
      ...common,
      class: declaration(value.class),
      issuer: declaration(value.issuer),
      issued_at: time(value.issued_at),
    };
  const representation = pick(value.representation, ['kind', 'reason', 'scope']);
  for (const field of ['represented', 'representative'])
    if (Object.hasOwn(value.representation, field))
      representation[field] = person(value.representation[field]);
  if (Object.hasOwn(value.representation, 'provenance'))
    representation.provenance = provenance(value.representation.provenance);
  return {
    ...common,
    resolution: pick(value.resolution, ['id', 'revision']),
    ...Object.fromEntries(
      ['character', 'medium', 'context', 'outcome'].map((key) => [key, declaration(value[key])]),
    ),
    practiced_at: time(value.practiced_at),
    received_at: time(value.received_at),
    stated_effect:
      value.stated_effect == null
        ? value.stated_effect
        : {
            ...pick(value.stated_effect, ['statement', 'locator']),
            at: time(value.stated_effect.at),
          },
    intended_recipient: declaration(value.intended_recipient, person),
    actual_receiver: declaration(value.actual_receiver, person),
    representation,
  };
}
function sources(value) {
  const parent = value.resolution;
  return {
    resolution:
      parent == null
        ? parent
        : {
            ...pick(parent, [
              'case_id',
              'id',
              'revision',
              'values_digest',
              'submission_digest',
              'status',
              'summary',
            ]),
            class: declaration(parent.class),
            issuer: declaration(parent.issuer),
            issued_at: time(parent.issued_at),
          },
    participants: value.participants.map((row) => ({
      ...pick(row, [
        'case_id',
        'id',
        'revision',
        'values_digest',
        'directory_status',
        'display_name',
        'procedural_role',
        'organization',
        'kind',
      ]),
      subject: pick(row.subject, ['id', 'revision', 'values_digest']),
    })),
    hearing_results: value.hearing_results.map((row) => ({
      ...pick(row, [
        'case_id',
        'hearing_id',
        'result_id',
        'revision',
        'agreement_id',
        'values_digest',
        'submission_digest',
        'status',
        'occurrence',
        'summary',
      ]),
      event_time: time(row.event_time),
      agreement: pick(row.agreement, ['id', 'text']),
    })),
    direct_supports: value.direct_supports.map((row) =>
      pick(row, ['document_id', 'version', 'digest', 'name', 'format', 'policy']),
    ),
  };
}
function base(value, family) {
  return value == null
    ? null
    : {
        ...pick(value, [
          'case_id',
          'family',
          'id',
          'resolution_id',
          'revision',
          'status',
          'values_digest',
        ]),
        values: rawFactValues(value.values, family),
        sources: sources(value.sources),
      };
}
function last(value, family) {
  if (!value) return null;
  const change = pick(value.command.change, ['action', 'expected_revision', 'reason']);
  if (Object.hasOwn(value.command.change, 'values'))
    change.values = rawFactValues(value.command.change.values, family);
  const administration = pick(value.observed_administration, [
    'kind',
    'case_id',
    'revision',
    'title',
    'reference',
    'status',
    'values_digest',
    'changed_at',
  ]);
  if (Object.hasOwn(value.observed_administration, 'changed_by'))
    administration.changed_by = pick(value.observed_administration.changed_by, ['id', 'email']);
  return {
    ...pick(value, [
      'case_id',
      'actor_id',
      'result_revision',
      'values_digest',
      'sources_digest',
      'submission_digest',
    ]),
    command: { ...pick(value.command, ['family', 'operation_id', 'id', 'resolution_id']), change },
    values: rawFactValues(value.values, family),
    sources: sources(value.sources),
    observed_administration: administration,
  };
}
const filter = (value) => pick(value, ['name', 'applied']);
const timeInput = (value) => pick(value, ['precision', 'date', 'clock', 'offset']);
const resultInput = (value) => pick(value, ['hearingId', 'resultId', 'revision', 'agreement']);
function inputs(value) {
  if (!value) return null;
  return {
    times: Object.fromEntries(
      ['issued_at', 'practiced_at', 'received_at', 'stated_effect'].map((key) => [
        key,
        timeInput(value.times?.[key]) ?? null,
      ]),
    ),
    recipient: filter(value.recipient) ?? null,
    receiver: filter(value.receiver) ?? null,
    represented: filter(value.represented) ?? null,
    representative: filter(value.representative) ?? null,
    provenance: resultInput(value.provenance) ?? null,
    representation: resultInput(value.representation) ?? null,
    parent: pick(value.parent, ['revision']) ?? null,
  };
}
export const factDraftFields = [
  'id',
  'family',
  'parentId',
  'action',
  'base',
  'draft',
  'mode',
  'last',
  'inputs',
];
export function captureFactDraft(state) {
  return {
    id: state.id,
    family: state.family,
    parentId: state.parentId,
    action: state.action,
    base: base(state.base, state.family),
    draft: { values: rawFactValues(state.draft.values, state.family), reason: state.draft.reason },
    mode: ['uncertain', 'conflict', 'exhausted'].includes(state.mode) ? state.mode : 'draft',
    last: last(state.last, state.family),
    inputs: inputs(state.inputs),
  };
}
export function validateFactDraft(value, descriptor, family, parentId) {
  if (
    !value ||
    value.family !== family ||
    value.parentId !== parentId ||
    value.action !== descriptor.action ||
    !['record', 'correct', 'withdraw'].includes(value.action) ||
    typeof value.id !== 'string' ||
    (value.base?.revision ?? 0) !== descriptor.baseRevision ||
    (value.base?.id ?? null) !== descriptor.resourceId ||
    (value.mode === 'uncertain' &&
      (!value.last ||
        value.last.command.id !== value.id ||
        value.last.command.family !== family ||
        (value.last.command.resolution_id ?? null) !== parentId ||
        value.last.command.change.action !== value.action ||
        value.last.case_id !== descriptor.contextId ||
        value.last.actor_id !== descriptor.principalId ||
        value.last.result_revision !== descriptor.baseRevision + 1))
  )
    throw new TypeError('Invalid procedural fact draft.');
  captureFactDraft(value);
}
