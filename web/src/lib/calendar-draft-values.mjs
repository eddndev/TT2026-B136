const pick = (value, names) => Object.fromEntries(names.map((name) => [name, value[name]]));
function rule(value) {
  return {
    ...pick(value, ['classification', 'explanation']),
    source_ids: [...value.source_ids],
  };
}
function values(value) {
  return {
    scope: {
      ...pick(value.scope, [
        'title',
        'jurisdiction',
        'authority',
        'organ',
        'territory',
        'use_description',
      ]),
      entity_codes: [...value.scope.entity_codes],
    },
    coverage: pick(value.coverage, ['from', 'through']),
    sources: value.sources.map((source) =>
      pick(source, [
        'id',
        'title',
        'issuer',
        'official_url',
        'published_on',
        'consulted_on',
        'locator',
      ]),
    ),
    weekly_pattern: value.weekly_pattern.map((row) => ({ weekday: row.weekday, ...rule(row) })),
    exceptions: value.exceptions.map((row) => ({
      ...pick(row, ['id', 'from', 'through']),
      ...rule(row),
    })),
  };
}
function submission(value) {
  if (!value) return null;
  const change = pick(value.command.change, ['action', 'expected_revision']);
  if (value.command.change.values) change.values = values(value.command.change.values);
  if (value.command.change.action !== 'publish') change.reason = value.command.change.reason;
  return {
    ...pick(value, ['actor_id', 'result_revision', 'values_digest', 'submission_digest']),
    command: { ...pick(value.command, ['operation_id', 'calendar_id']), change },
  };
}
export const calendarDraftFields = ['id', 'action', 'base', 'draft', 'mode', 'last'];
export function captureCalendarDraft(state) {
  return {
    id: state.id,
    action: state.action,
    base: state.base
      ? {
          ...pick(state.base, ['id', 'revision', 'status']),
          values: values(state.base.values),
        }
      : null,
    draft: { ...values(state.draft), reason: state.draft.reason },
    mode: ['uncertain', 'conflict', 'exhausted'].includes(state.mode) ? state.mode : 'draft',
    last: submission(state.last),
  };
}
export function validateCalendarDraft(value, descriptor) {
  if (
    !value ||
    typeof value.id !== 'string' ||
    value.action !== descriptor.action ||
    !['publish', 'replace', 'retire'].includes(value.action) ||
    (value.base?.revision ?? 0) !== descriptor.baseRevision ||
    (value.base?.id ?? null) !== descriptor.resourceId ||
    (value.action === 'publish') !== (value.base === null) ||
    (value.base && value.base.id !== value.id) ||
    !['draft', 'uncertain', 'conflict', 'exhausted'].includes(value.mode)
  )
    throw new TypeError('Invalid calendar draft identity.');
  if (value.mode === 'uncertain') {
    const last = value.last;
    if (
      !last ||
      last.command.calendar_id !== value.id ||
      last.command.change.action !== value.action ||
      last.actor_id !== descriptor.principalId ||
      last.result_revision !== descriptor.baseRevision + 1 ||
      last.command.change.expected_revision !== descriptor.baseRevision
    )
      throw new TypeError('Invalid calendar draft submission.');
  }
  captureCalendarDraft(value);
}
