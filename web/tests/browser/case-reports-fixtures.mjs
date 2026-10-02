import { createHash } from 'node:crypto';
export const reportId = '81000000-0000-4000-8000-000000000001';
export const otherReportId = '81000000-0000-4000-8000-000000000002';
export const operationId = '82000000-0000-4000-8000-000000000001';
export const lawyerId = '83000000-0000-4000-8000-000000000001';
export const capturedAt = '2026-09-27T12:00:00Z';
export const captureDigest = 'ab'.repeat(32);
// Transport fixtures verify exact download bytes; renderer validity has separate tests.
export const artifactBytes = {
  pdf: Buffer.from('%PDF-1.7\nSynthetic report transport fixture.\n%%EOF\n'),
  csv: Buffer.from('row_type,title\r\ncase,"\'Mu\u00f1oz, defensa"\r\n'),
};
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
export const filters = {
  created_from: '2026-09-01T00:00:00Z',
  created_before: '2026-10-01T00:00:00Z',
  status: 'all',
  assigned_litigator: null,
};
export const report = (change = {}) => ({
  id: reportId,
  operation_id: operationId,
  request_digest: 'cd'.repeat(32),
  scope: 'office',
  filters: { ...filters },
  requested_at: '2026-09-27T11:59:00Z',
  updated_at: capturedAt,
  state: 'ready',
  phase: null,
  retry_at: null,
  failure: null,
  ready: {
    snapshot_digest: captureDigest,
    checked_at: capturedAt,
    artifacts: ['pdf', 'csv'].map((format) => ({
      format,
      bytes: artifactBytes[format].length,
      digest: sha256(artifactBytes[format]),
    })),
  },
  notice: { kind: 'ready', created_at: capturedAt, read_at: null },
  ...change,
});
export const pending = (change = {}) =>
  report({ state: 'queued', ready: null, notice: null, ...change });
export const reportPage = (records, change = {}) => ({
  checked_at: records.reduce(
    (latest, value) => (value.updated_at > latest ? value.updated_at : latest),
    capturedAt,
  ),
  reports: records,
  has_more: false,
  next_after_id: null,
  ...change,
});
