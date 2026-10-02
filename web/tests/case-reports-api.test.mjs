import test from 'node:test';
import assert from 'node:assert/strict';
import { caseReportsApi } from '../src/lib/case-reports-api.mjs';
import { createApi } from '../src/lib/api.mjs';
import {
  report,
  pending,
  reportPage,
  reportId,
  otherReportId,
  operationId,
  filters,
  artifactBytes,
  captureDigest,
  sha256,
} from './browser/case-reports-fixtures.mjs';
const command = () => ({ operation_id: operationId, filters: { ...filters } });
const scoped = (value) => caseReportsApi(async () => value);
function download(format, change = {}) {
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
    ...change,
  };
}
test('reports send one exact explicit command and bind the returned operation and filters', async () => {
  const calls = [];
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return pending();
  });
  assert.deepEqual(await api.request(command()), pending());
  assert.deepEqual(calls, [['/case-reports', { method: 'POST', data: command() }]]);
  for (const result of [
    pending({ operation_id: otherReportId }),
    pending({ filters: { ...filters, status: 'closed' } }),
  ])
    await assert.rejects(() => scoped(result).request(command()));
});
test('reports reject malformed dates ranges unknown fields and privileged input before transport', async () => {
  let calls = 0;
  const api = caseReportsApi(async () => {
    calls++;
    return pending();
  });
  for (const value of [
    { ...command(), scope: 'office' },
    { ...command(), operation_id: '../other' },
    ...[
      { created_from: '2026-02-30T00:00:00Z' },
      { created_before: filters.created_from },
      { created_before: '2028-01-01T00:00:00Z' },
      { status: 'anything' },
      { assigned_litigator: '' },
      { requester_id: reportId },
    ].map((change) => ({ ...command(), filters: { ...filters, ...change } })),
  ])
    await assert.rejects(() => api.request(value));
  assert.equal(calls, 0);
});
test('reports preserve typed failure and never retry an uncertain request automatically', async () => {
  let calls = 0;
  const failure = Object.assign(new Error('unavailable'), { status: 503 });
  const api = caseReportsApi(async () => {
    calls++;
    throw failure;
  });
  await assert.rejects(
    () => api.request(command()),
    (error) => error === failure,
  );
  assert.equal(calls, 1);
});
test('report pages use exclusive cursor and unread filter without inventing a count', async () => {
  const calls = [];
  const value = reportPage([report({ id: otherReportId })]);
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return value;
  });
  assert.deepEqual(await api.list({ limit: 1, after_id: reportId, unread_only: true }), value);
  assert.equal(calls[0][0], `/case-reports?limit=1&after_id=${reportId}&unread_only=true`);
  for (const query of [
    { limit: 0 },
    { limit: 101 },
    { after_id: '../x' },
    { unread_only: 'true' },
    { offset: 1 },
  ])
    await assert.rejects(() => api.list(query));
  assert.equal(calls.length, 1);
});
test('report pages reject duplicates stale cursors missing continuation and read notices in unread pages', async () => {
  for (const value of [
    reportPage([report(), report()]),
    reportPage([report()], { has_more: true }),
    reportPage([], { next_after_id: reportId }),
    reportPage([report()], { hidden: 1 }),
  ])
    await assert.rejects(() => scoped(value).list());
  await assert.rejects(() => scoped(reportPage([report()])).list({ after_id: reportId }));
  await assert.rejects(() =>
    scoped(
      reportPage([
        report({
          notice: { kind: 'ready', created_at: report().updated_at, read_at: report().updated_at },
        }),
      ]),
    ).list({ unread_only: true }),
  );
});
test('detail and notice acknowledgement bind the exact report and only acknowledgement mutates', async () => {
  const calls = [];
  const value = report();
  const api = caseReportsApi(async (...args) => {
    calls.push(args);
    return value;
  });
  assert.deepEqual(await api.get(reportId), value);
  assert.deepEqual(await api.acknowledge(reportId), value);
  assert.deepEqual(calls, [
    [`/case-reports/${reportId}`],
    [`/case-reports/${reportId}/notice-read`, { method: 'POST', data: {} }],
  ]);
  await assert.rejects(() => scoped(report({ id: otherReportId })).get(reportId));
});
test('report detail rejects impossible phases artifacts notices and hidden worker state', async () => {
  for (const change of [
    { state: 'unknown' },
    { phase: 'capturing' },
    { ready: null },
    { lease: 'private' },
    { ready: { ...report().ready, artifacts: [report().ready.artifacts[0]] } },
    {
      ready: {
        ...report().ready,
        artifacts: [report().ready.artifacts[0], report().ready.artifacts[0]],
      },
    },
    { notice: null },
    { failure: 'render_failed' },
    { updated_at: '2026-02-30T00:00:00Z' },
  ])
    await assert.rejects(() => scoped(report(change)).get(reportId));
});
test('report detail retains legitimate pending retry failed and revoked states', async () => {
  const variants = [
    pending(),
    pending({ state: 'processing', phase: 'capturing' }),
    pending({ state: 'processing', phase: 'rendering' }),
    pending({ state: 'retry_waiting', retry_at: '2026-09-27T12:01:00Z', phase: 'rendering' }),
    pending({
      state: 'failed',
      failure: 'render_failed',
      notice: { kind: 'failed', created_at: report().updated_at, read_at: null },
    }),
    pending({ state: 'access_revoked', failure: 'access_revoked' }),
  ];
  for (const value of variants) assert.deepEqual(await scoped(value).get(reportId), value);
});
test('downloads retain exact PDF CSV bytes with capture identity and measured digest', async () => {
  for (const format of ['pdf', 'csv']) {
    const calls = [];
    const api = caseReportsApi(async (...args) => {
      calls.push(args);
      return download(format);
    });
    const result = await api.download(reportId, format, report());
    assert.deepEqual(Buffer.from(await result.blob.arrayBuffer()), artifactBytes[format]);
    assert.equal(result.filename, `report-${reportId}.${format}`);
    assert.deepEqual(calls, [
      [`/case-reports/${reportId}/download?format=${format}`, { binary: 'report' }],
    ]);
  }
});
test('downloads reject wrong identity capture type size digest and tampered content', async () => {
  for (const change of [
    { reportId: otherReportId },
    { snapshotDigest: '00'.repeat(32) },
    { contentType: 'text/html' },
    { contentLength: '1' },
    { digest: 'ff'.repeat(32) },
    { blob: new Blob([Buffer.alloc(artifactBytes.pdf.length, 1)], { type: 'application/pdf' }) },
  ])
    await assert.rejects(() => scoped(download('pdf', change)).download(reportId, 'pdf', report()));
  let calls = 0;
  const api = caseReportsApi(async () => {
    calls++;
  });
  for (const args of [
    [reportId, 'exe', report()],
    [reportId, 'pdf', pending()],
    [otherReportId, 'pdf', report()],
  ])
    await assert.rejects(() => api.download(...args));
  assert.equal(calls, 0);
});
test('disposed report clients reject late results and new transport', async () => {
  let release,
    calls = 0;
  const api = caseReportsApi(() => {
    calls++;
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const result = assert.rejects(api.get(reportId));
  api.dispose();
  release(report());
  await result;
  await assert.rejects(() => api.list());
  assert.equal(calls, 1);
});
test('factory report transport retains no-store headers and session replacement isolation', async () => {
  let release;
  const calls = [];
  const api = createApi(async (url, options) => {
    calls.push({ url, ...options });
    if (url.endsWith('/totp'))
      return Response.json({ access_token: 'new', user: { id: reportId } });
    if (url.includes('/download'))
      return new Response(artifactBytes.pdf, {
        headers: {
          'Content-Type': 'application/pdf',
          'Content-Length': String(artifactBytes.pdf.length),
          'X-Report-Id': reportId,
          'X-Report-Digest': sha256(artifactBytes.pdf),
          'X-Report-Snapshot-Digest': captureDigest,
        },
      });
    return new Promise((resolve) => {
      release = resolve;
    });
  });
  const pendingResult = assert.rejects(api.reports().get(reportId));
  await api.mfa('challenge', '123456', 'totp');
  release(Response.json(report()));
  await pendingResult;
  const value = await api.reports().download(reportId, 'pdf', report());
  assert.equal(value.blob.size, artifactBytes.pdf.length);
  assert.equal(calls.at(-1).cache, 'no-store');
  assert.equal(calls.at(-1).headers.Authorization, 'Bearer new');
});

test('factory report downloads accept browser CSV blob media type while retaining exact HTTP charset and bytes', async () => {
  const api = createApi(async () => {
    const response = new Response(artifactBytes.csv, {
      headers: {
        'Content-Type': 'text/csv; charset=utf-8',
        'Content-Length': String(artifactBytes.csv.length),
        'X-Report-Id': reportId,
        'X-Report-Digest': sha256(artifactBytes.csv),
        'X-Report-Snapshot-Digest': captureDigest,
      },
    });
    response.blob = async () => new Blob([artifactBytes.csv], { type: 'text/csv' });
    return response;
  });
  const value = await api.reports().download(reportId, 'csv', report());
  assert.deepEqual(Buffer.from(await value.blob.arrayBuffer()), artifactBytes.csv);
  assert.equal(value.filename, `report-${reportId}.csv`);
  for (const change of [
    { contentType: 'text/csv' },
    { contentType: 'text/csv; charset=iso-8859-1' },
    { blob: new Blob([artifactBytes.csv], { type: 'text/csv; charset=iso-8859-1' }) },
  ])
    await assert.rejects(() => scoped(download('csv', change)).download(reportId, 'csv', report()));
});
