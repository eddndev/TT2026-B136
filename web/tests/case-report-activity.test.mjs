import test from 'node:test';
import assert from 'node:assert/strict';
import { caseReportsApi } from '../src/lib/case-reports-api.mjs';
import {
  report,
  pending,
  reportPage,
  reportId,
  otherReportId,
  operationId,
  lawyerId,
  filters,
  artifactBytes,
  captureDigest,
  sha256,
} from './browser/case-reports-fixtures.mjs';

const stateCommand = () => ({ operation_id: operationId, filters: { ...filters } });
const activityFilters = (change = {}) => ({
  occurred_from: '2026-09-01T00:00:00Z',
  occurred_before: '2026-10-01T00:00:00Z',
  status: 'all',
  author_litigator: lawyerId,
  ...change,
});
const activityCommand = (change = {}) => ({
  operation_id: operationId,
  report_type: 'litigator_activity',
  filters: activityFilters(),
  ...change,
});
const activityReport = (change = {}) =>
  report({ report_type: 'litigator_activity', filters: activityFilters(), ...change });
const activityPending = (change = {}) =>
  activityReport({ state: 'queued', ready: null, notice: null, ...change });
const scoped = (value) => caseReportsApi(async () => value);

function download(format) {
  const bytes = artifactBytes[format];
  return {
    blob: new Blob([bytes], {
      type: format === 'pdf' ? 'application/pdf' : 'text/csv;charset=utf-8',
    }),
    reportId,
    digest: sha256(bytes),
    snapshotDigest: captureDigest,
    contentLength: String(bytes.length),
    contentType: format === 'pdf' ? 'application/pdf' : 'text/csv; charset=utf-8',
  };
}

test('legacy state reports retain their command and receipt without a report type', async () => {
  const calls = [];
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return pending();
  });
  const value = await api.request(stateCommand());
  assert.deepEqual(value, pending());
  assert.equal(Object.hasOwn(value, 'report_type'), false);
  assert.deepEqual(calls, [['/case-reports', { method: 'POST', data: stateCommand() }]]);
  assert.deepEqual(await scoped(report()).get(reportId), report());
});

test('activity reports send the exact type author and operation period', async () => {
  const calls = [];
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return activityPending();
  });
  assert.deepEqual(await api.request(activityCommand()), activityPending());
  assert.deepEqual(calls, [['/case-reports', { method: 'POST', data: activityCommand() }]]);
  assert.deepEqual(Object.keys(calls[0][1].data.filters), [
    'occurred_from',
    'occurred_before',
    'status',
    'author_litigator',
  ]);
});

test('activity periods retain exact UTC bounds and admit the 366 day boundary', async () => {
  for (const values of [
    activityFilters(),
    activityFilters({
      occurred_from: '2024-01-01T00:00:00Z',
      occurred_before: '2025-01-01T00:00:00Z',
      status: 'closed',
    }),
  ]) {
    const calls = [];
    const expected = activityPending({ filters: values });
    const api = caseReportsApi(async (...args) => {
      calls.push(args);
      return expected;
    });
    assert.deepEqual(await api.request(activityCommand({ filters: values })), expected);
    assert.deepEqual(calls[0][1].data.filters, values);
  }
  let calls = 0;
  const api = caseReportsApi(async () => {
    calls++;
    return activityPending();
  });
  for (const change of [
    { occurred_from: '2026-02-30T00:00:00Z' },
    { occurred_from: '2026-09-01' },
    { occurred_from: '2026-09-01T00:00:00+01:00' },
    { occurred_before: '2026-09-01T00:00:00Z' },
    { occurred_before: '2026-08-31T00:00:00Z' },
    { occurred_before: '2027-09-03T00:00:00Z' },
    { status: 'unknown' },
    { author_litigator: '' },
    { author_litigator: '00000000-0000-0000-0000-000000000000' },
  ])
    await assert.rejects(() => api.request(activityCommand({ filters: activityFilters(change) })));
  assert.equal(calls, 0);
});

test('requests reject mixed state and activity fields before transport', async () => {
  let calls = 0;
  const api = caseReportsApi(async () => {
    calls++;
    return activityPending();
  });
  const missingAuthor = activityFilters();
  delete missingAuthor.author_litigator;
  for (const value of [
    { operation_id: operationId, filters: activityFilters() },
    activityCommand({ filters: { ...filters } }),
    activityCommand({ filters: { ...filters, ...activityFilters() } }),
    stateCommandWithActivityField(),
    activityCommand({ filters: activityFilters({ assigned_litigator: lawyerId }) }),
    activityCommand({ filters: missingAuthor }),
    activityCommand({ report_type: 'not_a_report' }),
    activityCommand({ report_type: null }),
    activityCommand({ requester_id: lawyerId }),
  ])
    await assert.rejects(() => api.request(value));
  assert.equal(calls, 0);
});

function stateCommandWithActivityField() {
  return { ...stateCommand(), filters: { ...filters, occurred_from: filters.created_from } };
}

test('request recovery binds the report type operation and every selected filter', async () => {
  for (const result of [
    pending(),
    activityPending({ operation_id: otherReportId }),
    activityPending({ filters: activityFilters({ author_litigator: otherReportId }) }),
    activityPending({ filters: activityFilters({ status: 'closed' }) }),
    activityPending({ filters: activityFilters({ occurred_from: '2026-09-02T00:00:00Z' }) }),
    activityPending({ filters: activityFilters({ occurred_before: '2026-10-02T00:00:00Z' }) }),
  ])
    await assert.rejects(() => scoped(result).request(activityCommand()));
  await assert.rejects(() => scoped(activityPending()).request(stateCommand()));
  for (const value of [
    report({ filters: activityFilters() }),
    activityReport({ filters: { ...filters } }),
    activityReport({ filters: activityFilters({ created_from: filters.created_from }) }),
    activityReport({ report_type: 'not_a_report' }),
  ])
    await assert.rejects(() => scoped(value).get(reportId));
});

test('an uncertain activity request retries the same operation only explicitly', async () => {
  const calls = [];
  const failure = Object.assign(new Error('response lost after acceptance'), { status: 503 });
  const original = activityCommand();
  const accepted = activityPending();
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    if (calls.length === 1) throw failure;
    return accepted;
  });
  await assert.rejects(
    () => api.request(original),
    (error) => error === failure,
  );
  assert.equal(calls.length, 1);
  assert.deepEqual(await api.request(original), accepted);
  assert.deepEqual(calls, [
    ['/case-reports', { method: 'POST', data: original }],
    ['/case-reports', { method: 'POST', data: original }],
  ]);
  assert.deepEqual(original, activityCommand());
});

test('lists detail and notices retain both report modalities without adding statistics', async () => {
  const activity = activityReport({ id: otherReportId });
  const page = reportPage([report(), activity]);
  const calls = [];
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return args[0].startsWith('/case-reports?') ? page : activity;
  });
  assert.deepEqual(await api.list(), page);
  assert.deepEqual(await api.get(otherReportId), activity);
  assert.deepEqual(await api.acknowledge(otherReportId), activity);
  assert.deepEqual(calls, [
    ['/case-reports?limit=20&unread_only=false'],
    [`/case-reports/${otherReportId}`],
    [`/case-reports/${otherReportId}/notice-read`, { method: 'POST', data: {} }],
  ]);
  assert.deepEqual(Object.keys(activity.ready), ['snapshot_digest', 'checked_at', 'artifacts']);
});

test('activity PDF and CSV downloads preserve the same authorized capture contract', async () => {
  const expected = activityReport();
  for (const format of ['pdf', 'csv']) {
    const calls = [];
    const api = caseReportsApi(async (...args) => {
      calls.push(args);
      return download(format);
    });
    const value = await api.download(reportId, format, expected);
    assert.equal(value.filename, `report-${reportId}.${format}`);
    assert.deepEqual(Buffer.from(await value.blob.arrayBuffer()), artifactBytes[format]);
    assert.deepEqual(calls, [
      [`/case-reports/${reportId}/download?format=${format}`, { binary: 'report' }],
    ]);
  }
});
