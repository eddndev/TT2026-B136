import { stageAction, stageDraft, stageSupportFields } from './case-stages.mjs';

const dates = ['known_at', 'accusation_declared_at', 'opening_order_issued_at', 'received_at'];
const texts = ['stage', 'reason', 'receiving_court', 'receipt_reference', 'note'];
const invalid = () => new TypeError('Invalid stage draft.');
export function stageBase(value) {
  return value === null ? null : { stage: value.stage, stage_revision: value.stage_revision };
}
export function stageSupport(value) {
  if (value === null) return null;
  return {
    case_id: value.case_id,
    id: value.id,
    version: value.version,
    digest: value.digest,
    name: value.name,
    uploaded: value.uploaded === true,
  };
}
export function stageRaw(value) {
  return {
    ...Object.fromEntries(texts.map((key) => [key, value[key]])),
    ...Object.fromEntries(
      dates.map((key) => [
        key,
        {
          precision: value[key].precision,
          date: value[key].date,
          time: value[key].time,
          offset: value[key].offset,
        },
      ]),
    ),
    ...Object.fromEntries(
      Object.keys(stageSupportFields).map((key) => [key, stageSupport(value[key])]),
    ),
  };
}
function time(value) {
  return value.precision === 'date'
    ? { precision: value.precision, date: value.date, offset: value.offset }
    : { precision: value.precision, at: value.at };
}
const ref = (value) => ({
  document_id: value.document_id,
  version: value.version,
  digest: value.digest,
});
export function stageSubmission(value) {
  if (value === null) return null;
  const result = { expected_revision: value.expected_revision };
  for (const key of ['stage', 'target', 'reason', 'receiving_court', 'receipt_reference', 'note'])
    if (Object.hasOwn(value, key)) result[key] = value[key];
  for (const key of dates) if (Object.hasOwn(value, key)) result[key] = time(value[key]);
  for (const key of Object.keys(stageSupportFields))
    if (Object.hasOwn(value, key)) result[key] = ref(value[key]);
  return result;
}
export function captureStage(state) {
  return {
    base: stageBase(state.base),
    action: state.action,
    draft: stageRaw(state.draft),
    uncertain: state.uncertain,
    needsReview: state.needsReview,
    exhausted: state.exhausted,
    lastPayload: stageSubmission(state.lastPayload),
    lastSupports: state.lastSupports.map(stageSupport),
  };
}
export function validateStage(value, descriptor) {
  if (
    !value ||
    value.action !== descriptor.action ||
    stageAction(value.base) !== value.action ||
    (value.base?.stage_revision ?? 0) !== descriptor.baseRevision ||
    ['uncertain', 'needsReview', 'exhausted'].some((key) => typeof value[key] !== 'boolean') ||
    (value.uncertain && !value.lastPayload) ||
    !Array.isArray(value.lastSupports)
  )
    throw invalid();
  const raw = stageRaw(value.draft);
  if (
    texts.some((key) => typeof raw[key] !== 'string') ||
    dates.some((key) => Object.values(raw[key]).some((part) => typeof part !== 'string'))
  )
    throw invalid();
  if (value.lastPayload && value.lastPayload.expected_revision !== descriptor.baseRevision)
    throw invalid();
}
export function stageFormState(current, action = stageAction(current)) {
  return {
    base: stageBase(current),
    action,
    draft: stageDraft(current),
    preview: null,
    lastPayload: null,
    lastSupports: [],
    candidate: undefined,
    compared: [],
    busy: false,
    error: '',
    uncertain: false,
    needsReview: false,
    exhausted: false,
    supportIssue: false,
    oldSupports: [],
    blocked: true,
    loaded: false,
    restoredClosed: false,
    incomplete: false,
    blockedByCase: false,
    historyComplete: false,
  };
}

export async function completeStageHistory(api, caseId, admitted) {
  const entries = [];
  let cursor;
  do {
    const page = await api.history({ beforeRevision: cursor });
    if (!admitted()) return null;
    if (
      !Array.isArray(page.entries) ||
      typeof page.has_more !== 'boolean' ||
      (page.has_more && !page.entries.length)
    )
      throw invalid();
    let previous = cursor ?? Infinity;
    for (const entry of page.entries) {
      if (
        entry.case_id !== caseId ||
        !Number.isSafeInteger(entry.stage_revision) ||
        entry.stage_revision < 1 ||
        entry.stage_revision >= previous
      )
        throw invalid();
      previous = entry.stage_revision;
      entries.push(entry);
    }
    if (!page.has_more) return entries;
    if (page.next_before_revision !== previous) throw invalid();
    cursor = previous;
  } while (admitted());
  return null;
}
