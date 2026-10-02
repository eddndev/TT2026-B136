import { factObject, factUuid, factDigest } from './procedural-fact-primitives.mjs';
import { hearingTimeParts } from './hearing-time.mjs';
export const maxReportBytes = 16 * 1024 * 1024;
export function reportInvalid() {
  throw new Error('No se pudo validar el informe. Actualiza la consulta.');
}
export function reportInstant(value) {
  const match =
    typeof value === 'string' &&
    /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.\d{1,9})?Z$/.exec(value);
  if (!match) reportInvalid();
  hearingTimeParts(`${match[1]}Z`);
  return Date.parse(value);
}
export function reportUuid(value) {
  if (factUuid(value) !== value) reportInvalid();
  return value;
}
export function reportFilters(value) {
  factObject(value, ['created_from', 'created_before', 'status', 'assigned_litigator']);
  const from = reportInstant(value.created_from),
    before = reportInstant(value.created_before);
  if (
    before <= from ||
    before - from > 366 * 86400000 ||
    !['all', 'active', 'closed'].includes(value.status)
  )
    reportInvalid();
  if (value.assigned_litigator !== null) reportUuid(value.assigned_litigator);
  return value;
}
export function reportCommand(value) {
  factObject(value, ['operation_id', 'filters']);
  reportUuid(value.operation_id);
  reportFilters(value.filters);
  return structuredClone(value);
}
export function reportQuery(value = {}) {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !['limit', 'after_id', 'unread_only'].includes(key))
  )
    reportInvalid();
  const result = {
    limit: value.limit ?? 20,
    after_id: value.after_id ?? null,
    unread_only: value.unread_only ?? false,
  };
  if (
    !Number.isInteger(result.limit) ||
    result.limit < 1 ||
    result.limit > 100 ||
    typeof result.unread_only !== 'boolean'
  )
    reportInvalid();
  if (result.after_id !== null) reportUuid(result.after_id);
  return result;
}
export function reportValue(value) {
  factObject(value, [
    'id',
    'operation_id',
    'request_digest',
    'scope',
    'filters',
    'requested_at',
    'updated_at',
    'state',
    'phase',
    'retry_at',
    'failure',
    'ready',
    'notice',
  ]);
  reportUuid(value.id);
  reportUuid(value.operation_id);
  factDigest(value.request_digest);
  reportFilters(value.filters);
  const requested = reportInstant(value.requested_at),
    updated = reportInstant(value.updated_at);
  if (updated < requested || !['office', 'assigned_cases'].includes(value.scope)) reportInvalid();
  const states = ['queued', 'processing', 'retry_waiting', 'ready', 'failed', 'access_revoked'];
  const failures = [
    'temporarily_unavailable',
    'render_unavailable',
    'render_failed',
    'capacity_exceeded',
    'invalid_capture',
    'access_revoked',
  ];
  if (!states.includes(value.state)) reportInvalid();
  if (
    ['processing', 'retry_waiting'].includes(value.state)
      ? !['capturing', 'rendering'].includes(value.phase)
      : value.phase !== null
  )
    reportInvalid();
  if (value.state === 'retry_waiting') {
    if (reportInstant(value.retry_at) < updated) reportInvalid();
  } else if (value.retry_at !== null) reportInvalid();
  if (
    value.state === 'failed'
      ? !failures.includes(value.failure)
      : value.state === 'access_revoked'
        ? value.failure !== 'access_revoked'
        : value.failure !== null
  )
    reportInvalid();
  if (value.state === 'ready') {
    factObject(value.ready, ['snapshot_digest', 'checked_at', 'artifacts']);
    factDigest(value.ready.snapshot_digest);
    const checked = reportInstant(value.ready.checked_at);
    if (
      checked < requested ||
      checked > updated ||
      !Array.isArray(value.ready.artifacts) ||
      value.ready.artifacts.length !== 2
    )
      reportInvalid();
    const formats = new Set();
    for (const artifact of value.ready.artifacts) {
      factObject(artifact, ['format', 'bytes', 'digest']);
      factDigest(artifact.digest);
      if (
        !['pdf', 'csv'].includes(artifact.format) ||
        formats.has(artifact.format) ||
        !Number.isSafeInteger(artifact.bytes) ||
        artifact.bytes < 1 ||
        artifact.bytes > maxReportBytes
      )
        reportInvalid();
      formats.add(artifact.format);
    }
  } else if (value.ready !== null) reportInvalid();
  if (['ready', 'failed'].includes(value.state)) {
    factObject(value.notice, ['kind', 'created_at', 'read_at']);
    const created = reportInstant(value.notice.created_at);
    if (
      value.notice.kind !== value.state ||
      created < (value.ready ? reportInstant(value.ready.checked_at) : requested) ||
      created > updated
    )
      reportInvalid();
    if (value.notice.read_at !== null) {
      const read = reportInstant(value.notice.read_at);
      if (read < created || read > updated) reportInvalid();
    }
  } else if (value.notice !== null) reportInvalid();
  return value;
}
export function reportPage(value, query) {
  factObject(value, ['checked_at', 'reports', 'has_more', 'next_after_id']);
  const checked = reportInstant(value.checked_at);
  if (
    !Array.isArray(value.reports) ||
    value.reports.length > query.limit ||
    typeof value.has_more !== 'boolean'
  )
    reportInvalid();
  let previous = query.after_id;
  for (const report of value.reports) {
    reportValue(report);
    if (
      (previous !== null && report.id <= previous) ||
      reportInstant(report.updated_at) > checked ||
      (query.unread_only && (!report.notice || report.notice.read_at !== null))
    )
      reportInvalid();
    previous = report.id;
  }
  if (
    value.has_more
      ? !value.reports.length || value.next_after_id !== previous
      : value.next_after_id !== null
  )
    reportInvalid();
  return value;
}
