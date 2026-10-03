import { resourcePreparedValue } from './procedural-resource-validation.mjs';
const pick = (value, fields) =>
  value == null
    ? value
    : Object.fromEntries(
        fields.filter((key) => Object.hasOwn(value, key)).map((key) => [key, value[key]]),
      );
const time = (value) =>
  pick(value, ['precision', 'year', 'month', 'day', 'hour', 'minute', 'second', 'offset_seconds']);
const declaration = (value) => pick(value, ['kind', 'value', 'reason']);
const support = (value) => pick(value, ['document_id', 'version', 'digest', 'locator']);
const reference = (value) => pick(value, ['id', 'revision']);
export function rawResourceValues(value, act) {
  if (act)
    return {
      ...pick(value, ['kind', 'statement']),
      mode: declaration(value.mode),
      occurred_at: time(value.occurred_at),
      authority: declaration(value.authority),
      evidence: value.evidence.map(support),
    };
  return {
    ...pick(value, ['kind', 'title', 'challenged_part', 'grounds']),
    mode: declaration(value.mode),
    resolution: reference(value.resolution),
    resolution_evidence: support(value.resolution_evidence),
    resolution_reference: declaration(value.resolution_reference),
    issuing_authority: declaration(value.issuing_authority),
    receiving_authority: declaration(value.receiving_authority),
    resolution_at: time(value.resolution_at),
    notification_at: time(value.notification_at),
    appellants: value.appellants.map((row) => ({
      name: row.name,
      role: declaration(row.role),
      participant: reference(row.participant),
    })),
  };
}
function inputs(value) {
  if (!value) return null;
  const rawTime = (entry) => pick(entry, ['precision', 'date', 'clock', 'offset']);
  return {
    resolution: pick(value.resolution, ['selectedId', 'revision']) ?? null,
    resolutionTime: rawTime(value.resolutionTime) ?? null,
    notificationTime: rawTime(value.notificationTime) ?? null,
    actTime: rawTime(value.actTime) ?? null,
    supportRows: value.supportRows ? [...value.supportRows] : null,
    appellantRows: value.appellantRows ? [...value.appellantRows] : null,
    appellants: value.appellants?.map((row) => pick(row, ['name', 'applied']) ?? null) ?? null,
  };
}
export const resourceDraftFields = [
  'id',
  'actId',
  'selectedAct',
  'action',
  'base',
  'draft',
  'mode',
  'last',
  'inputs',
];
export function captureResourceDraft(state) {
  const act = ['record_act', 'correct_act'].includes(state.action);
  if (state.last) resourcePreparedValue(state.last);
  return {
    id: state.id,
    actId: state.actId,
    action: state.action,
    selectedAct: state.selectedAct
      ? pick(state.selectedAct, ['id', 'revision', 'resourceRevision'])
      : null,
    base: state.base ? pick(state.base, ['case_id', 'id', 'revision', 'status']) : null,
    draft: { values: rawResourceValues(state.draft.values, act), reason: state.draft.reason },
    mode: ['uncertain', 'conflict'].includes(state.mode) ? state.mode : 'draft',
    last: state.last ? structuredClone(state.last) : null,
    inputs: inputs(state.inputs),
  };
}
export function validateResourceDraft(value, descriptor) {
  if (
    !value ||
    value.action !== descriptor.action ||
    !['register', 'correct', 'record_act', 'correct_act', 'archive', 'reactivate'].includes(
      value.action,
    ) ||
    typeof value.id !== 'string' ||
    typeof value.actId !== 'string' ||
    (value.base?.id ?? null) !== descriptor.resourceId ||
    (value.base?.revision ?? 0) !== descriptor.baseRevision ||
    (value.base && value.base.case_id !== descriptor.contextId) ||
    (value.action === 'correct_act' &&
      (value.selectedAct?.id !== descriptor.instanceId ||
        value.actId !== descriptor.instanceId ||
        !Number.isSafeInteger(value.selectedAct.resourceRevision) ||
        value.selectedAct.resourceRevision < 1)) ||
    (value.mode === 'uncertain' && !value.last)
  )
    throw new TypeError('Invalid procedural resource draft.');
  if (
    value.last &&
    (value.last.case_id !== descriptor.contextId ||
      value.last.recorded_by.id !== descriptor.principalId ||
      value.last.command.resource_id !== value.id ||
      value.last.command.change.action !== value.action ||
      value.last.result_revision !== descriptor.baseRevision + 1)
  )
    throw new TypeError('Invalid resource submission owner.');
  for (const key of ['supportRows', 'appellantRows']) {
    const rows = value.inputs?.[key];
    if (
      rows &&
      (!Array.isArray(rows) ||
        rows.some((row) => typeof row !== 'string' || !row) ||
        new Set(rows).size !== rows.length)
    )
      throw new TypeError('Invalid resource field owners.');
  }
  captureResourceDraft(value);
}
