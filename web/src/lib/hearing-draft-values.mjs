const pick = (value, names) =>
  Object.fromEntries(
    names.filter((key) => Object.hasOwn(value, key)).map((key) => [key, value[key]]),
  );
const ref = (value) =>
  value == null
    ? null
    : pick(value, ['case_id', 'id', 'document_id', 'version', 'digest', 'name', 'uploaded']);
const time = (value) => pick(value, ['precision', 'date', 'time', 'offset', 'at']);
const person = (value) => pick(value, ['participant_id', 'revision', 'capacity', 'observation']);
function selectorsDraft(value, result) {
  if (!value) return null;
  const field = result ? 'attendees' : 'participants';
  return {
    [field]:
      value[field] == null
        ? null
        : pick(value[field], result ? ['name', 'applied'] : ['name', 'query']),
  };
}
export const hearingContextDraft = (value) =>
  value == null
    ? null
    : pick(value, [
        'case_id',
        'case_revision',
        'case_values_digest',
        'administrative_status',
        'profile_complete',
        'stage_revision',
        'stage',
        'stage_values_digest',
      ]);
export function hearingSourceDraft(value) {
  if (value == null) return null;
  const result = pick(value, [
    'hearing_id',
    'result_id',
    'revision',
    'values_digest',
    'submission_digest',
    'status',
    'kind',
    'scheduled_at',
  ]);
  if (value.scheduling_context)
    result.scheduling_context = pick(value.scheduling_context, [
      'administration_revision',
      'administration_digest',
      'stage_revision',
      'stage',
      'stage_digest',
    ]);
  return result;
}
export function hearingRaw(value) {
  return {
    ...pick(value, ['kind', 'modality', 'venue', 'note', 'statement', 'reason']),
    time: time(value.time),
    participants: value.participants.map(person),
    support: ref(value.support),
  };
}
export function resultRaw(value) {
  return {
    ...pick(value, ['occurrence', 'extent', 'summary', 'reason']),
    time: time(value.time),
    attendees: value.attendees.map(person),
    agreements: value.agreements.map((row) => pick(row, ['id', 'text'])),
    provenance: {
      ...pick(value.provenance, ['kind', 'reference']),
      support: ref(value.provenance.support),
    },
  };
}
function values(value, result) {
  if (result)
    return {
      ...pick(value, ['occurrence', 'extent', 'summary']),
      event_time: time(value.event_time),
      attendees: value.attendees.map(person),
      agreements: value.agreements.map((row) => pick(row, ['id', 'text'])),
      provenance: {
        ...pick(value.provenance, ['kind', 'reference']),
        support: ref(value.provenance.support),
      },
    };
  return {
    ...pick(value, ['kind', 'scheduled_at', 'modality', 'venue', 'note']),
    participants: value.participants.map(person),
    conviction_basis: value.conviction_basis
      ? {
          statement: value.conviction_basis.statement,
          support: ref(value.conviction_basis.support),
        }
      : null,
  };
}
export function hearingBase(value, result = false) {
  if (value == null) return null;
  return {
    ...pick(value, ['case_id', 'id', 'hearing_id', 'revision', 'status', 'values_digest']),
    values: values(value.values, result),
    ...(result
      ? {
          anchor: hearingSourceDraft(value.anchor),
          continuation: hearingSourceDraft(value.continuation),
        }
      : {}),
  };
}
export function hearingLast(value, result = false) {
  if (value == null) return null;
  const change = pick(value.command.change, [
    'action',
    'expected_revision',
    'expected_case_revision',
    'expected_stage_revision',
    'anchor_revision',
    'reason',
  ]);
  if (value.command.change.values) change.values = values(value.command.change.values, result);
  if (Object.hasOwn(value.command.change, 'continuation'))
    change.continuation = value.command.change.continuation
      ? pick(value.command.change.continuation, ['result_id', 'revision'])
      : null;
  return {
    ...pick(value, [
      'case_id',
      'actor_id',
      'result_revision',
      'values_digest',
      'submission_digest',
    ]),
    command: { ...pick(value.command, ['operation_id', 'hearing_id', 'result_id']), change },
    ...(result
      ? {
          anchor: hearingSourceDraft(value.anchor),
          continuation: hearingSourceDraft(value.continuation),
        }
      : {}),
  };
}
export const hearingDraftFields = [
  'id',
  'action',
  'base',
  'draft',
  'context',
  'anchor',
  'source',
  'mode',
  'last',
  'selectors',
];
export function captureHearingDraft(state, result = false) {
  return {
    id: state.id,
    action: state.action,
    base: hearingBase(state.base, result),
    draft: result ? resultRaw(state.draft) : hearingRaw(state.draft),
    context: hearingContextDraft(state.context),
    anchor: hearingSourceDraft(state.anchor),
    source: hearingSourceDraft(state.source),
    mode: ['uncertain', 'conflict', 'exhausted'].includes(state.mode) ? state.mode : 'draft',
    last: hearingLast(state.last, result),
    selectors: selectorsDraft(state.selectors, result),
  };
}
export function validateHearingDraft(value, descriptor, result) {
  const actions = result ? ['record', 'correct', 'withdraw'] : ['schedule', 'replace', 'cancel'];
  if (
    !value ||
    !actions.includes(value.action) ||
    value.action !== descriptor.action ||
    typeof value.id !== 'string' ||
    (value.base?.revision ?? 0) !== descriptor.baseRevision ||
    (descriptor.resourceId && value.base?.id !== descriptor.resourceId) ||
    (value.mode === 'uncertain' &&
      (!value.last ||
        value.last.command.change.action !== value.action ||
        value.last.result_revision !== descriptor.baseRevision + 1))
  )
    throw new TypeError('Invalid hearing draft.');
  captureHearingDraft(value, result);
}
