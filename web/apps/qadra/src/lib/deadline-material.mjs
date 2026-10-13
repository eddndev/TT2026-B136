import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factDigest as digest,
  factText as text,
  factLabel as label,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { deadlineSourceReference } from './deadline-values.mjs';
import { deadlineInstant } from './deadline-time.mjs';
import { deadlineProfileScope } from './deadline-profile-values.mjs';
import { deadlineResult } from './deadline-result.mjs';
export function deadlineAuthor(raw) {
  object(raw, ['id', 'email']);
  uuid(raw.id);
  text(raw.email, 'Correo', 320, false);
  return raw;
}
export function deadlineResponsible(raw) {
  object(raw, ['id', 'email', 'role']);
  uuid(raw.id);
  text(raw.email, 'Correo', 320, false);
  if (!['owner', 'litigator', 'paralegal'].includes(raw.role)) invalid();
  return raw;
}
export function deadlineAdministration(raw, caseId) {
  return administration(raw, caseId, false);
}
export function deadlineTrackingAdministration(raw, caseId) {
  return administration(raw, caseId, true);
}
function administration(raw, caseId, allowClosed) {
  const keys = ['kind', 'title', 'reference', 'status'];
  if (raw?.kind === 'recorded') {
    object(raw, [...keys, 'case_id', 'revision', 'values_digest', 'changed_at', 'changed_by']);
    if (raw.case_id !== caseId) invalid();
    revision(raw.revision);
    digest(raw.values_digest);
    deadlineInstant(raw.changed_at);
    deadlineAuthor(raw.changed_by);
  } else {
    object(raw, keys);
    if (raw.kind !== 'unrevised' || raw.status !== 'active') invalid();
  }
  label(raw.title);
  if (raw.reference !== null) label(raw.reference);
  if (raw.status !== 'active' && !(allowClosed && raw.status === 'closed')) invalid();
  return raw;
}
export function deadlineAdministrationFollows(current, previous, caseId) {
  deadlineAdministration(current, caseId);
  deadlineAdministration(previous, caseId);
  if (previous.kind === 'recorded')
    return (
      current.kind === 'recorded' &&
      (current.revision > previous.revision ||
        (current.revision === previous.revision && same(current, previous)))
    );
  return current.kind === 'recorded' || same(current, previous);
}
function source(raw, caseId, head = false) {
  object(raw, [
    'case_id',
    'reference',
    'values_digest',
    'sources_digest',
    'submission_digest',
    'status',
    'href',
  ]);
  if (raw.case_id !== caseId || !same(deadlineSourceReference(raw.reference), raw.reference))
    invalid();
  digest(raw.values_digest);
  digest(raw.submission_digest);
  if (!['recorded', 'withdrawn'].includes(raw.status)) invalid();
  const r = raw.reference,
    base = `/api/v1/cases/${caseId}`;
  let href;
  if (r.family === 'hearing_result') {
    if (raw.sources_digest !== null || (head && r.agreement_id !== null)) invalid();
    href = `${base}/hearings/${r.hearing_id}/results/${r.result_id}/revisions/${r.revision}`;
  } else {
    digest(raw.sources_digest);
    href =
      r.family === 'resolution'
        ? `${base}/resolutions/${r.id}/revisions/${r.revision}`
        : `${base}/resolutions/${r.resolution.id}/notifications/${r.id}/revisions/${r.revision}`;
  }
  if (raw.href !== href)
    invalid('La referencia de fuente no corresponde a su revisi\u00f3n exacta.');
}
function sourcePair(selected, head, caseId, reference) {
  source(selected, caseId);
  source(head, caseId, true);
  const a = selected.reference,
    b = head.reference;
  if (
    !same(a, reference) ||
    a.family !== b.family ||
    b.revision < a.revision ||
    (a.family === 'hearing_result'
      ? a.hearing_id !== b.hearing_id || a.result_id !== b.result_id
      : a.id !== b.id) ||
    (a.family === 'notification' && a.resolution.id !== b.resolution.id)
  )
    invalid();
  if (a.revision === b.revision) {
    const copy = structuredClone(selected);
    if (a.family === 'hearing_result') copy.reference.agreement_id = null;
    if (!same(copy, head))
      invalid('La misma revisi\u00f3n de fuente tiene dos contenidos distintos.');
  }
}
function calendar(raw) {
  object(raw, ['id', 'revision', 'values_digest', 'submission_digest', 'status', 'title', 'href']);
  uuid(raw.id);
  revision(raw.revision);
  digest(raw.values_digest);
  digest(raw.submission_digest);
  label(raw.title);
  if (
    !['published', 'retired'].includes(raw.status) ||
    raw.href !== `/api/v1/judicial-calendars/${raw.id}/revisions/${raw.revision}`
  )
    invalid();
}
export function deadlineCalculation(raw, definition, caseId) {
  object(raw, ['profile', 'material', 'result']);
  const p = raw.profile;
  object(p, [
    'id',
    'revision',
    'algorithm',
    'title',
    'scope',
    'status',
    'definition_digest',
    'submission_digest',
    'href',
  ]);
  uuid(p.id);
  revision(p.revision);
  label(p.title);
  deadlineProfileScope(p.scope, caseId);
  digest(p.definition_digest);
  digest(p.submission_digest);
  if (
    p.algorithm !== 'v1' ||
    p.status !== 'published' ||
    p.id !== definition.profile.id ||
    p.revision !== definition.profile.revision ||
    p.href !== `/api/v1/cases/${caseId}/deadline-profiles/${p.id}/revisions/${p.revision}`
  )
    invalid();
  const m = raw.material;
  object(m, ['case_id', 'administration', 'source', 'source_head', 'calendar', 'calendar_head']);
  if (m.case_id !== caseId) invalid();
  deadlineAdministration(m.administration, caseId);
  const selection = definition.input.selection;
  if (selection.source.kind === 'unknown') {
    if (m.source !== null || m.source_head !== null) invalid();
  } else sourcePair(m.source, m.source_head, caseId, selection.source.value);
  const selected = definition.input.calendar;
  if (selected === null) {
    if (m.calendar !== null || m.calendar_head !== null) invalid();
  } else {
    calendar(m.calendar);
    calendar(m.calendar_head);
    if (
      m.calendar.id !== selected.id ||
      m.calendar.revision !== selected.revision ||
      m.calendar_head.id !== selected.id ||
      m.calendar_head.revision < selected.revision ||
      (m.calendar_head.revision === selected.revision && !same(m.calendar, m.calendar_head))
    )
      invalid();
  }
  deadlineResult(raw.result);
  return raw;
}
