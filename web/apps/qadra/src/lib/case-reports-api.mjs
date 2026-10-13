import { reportLitigatorQuery, reportLitigatorPage } from './case-report-litigators.mjs';
import {
  reportCommand,
  reportType,
  reportQuery,
  reportValue,
  reportPage,
  reportUuid,
  reportInvalid,
  maxReportBytes,
} from './case-report-values.mjs';

export function caseReportsApi(request) {
  let active = true;
  const check = () => {
    if (!active) throw new Error('La consulta de informes ya no est\u00e1 abierta.');
  };
  async function read(path, options) {
    check();
    try {
      const value = await request(path, ...(options ? [options] : []));
      check();
      return value;
    } catch (error) {
      check();
      throw error;
    }
  }
  async function detail(id, suffix = '', options) {
    reportUuid(id);
    const value = reportValue(await read(`/case-reports/${id}${suffix}`, options));
    if (value.id !== id) reportInvalid();
    return value;
  }
  return {
    dispose() {
      active = false;
    },
    async request(raw) {
      check();
      const command = reportCommand(raw);
      const value = reportValue(await read('/case-reports', { method: 'POST', data: command }));
      if (
        value.operation_id !== command.operation_id ||
        reportType(value) !== reportType(command) ||
        Object.keys(command.filters).some((key) => value.filters[key] !== command.filters[key])
      )
        reportInvalid();
      return value;
    },
    async list(raw = {}) {
      check();
      const query = reportQuery(raw),
        params = new URLSearchParams({ limit: String(query.limit) });
      if (query.after_id !== null) params.set('after_id', query.after_id);
      params.set('unread_only', String(query.unread_only));
      return reportPage(await read(`/case-reports?${params}`), query);
    },
    async litigators(raw = {}) {
      check();
      const query = reportLitigatorQuery(raw),
        params = new URLSearchParams({ limit: String(query.limit) });
      if (query.after_id !== null) params.set('after_id', query.after_id);
      if (query.report_type) params.set('report_type', query.report_type);
      return reportLitigatorPage(await read(`/case-reports/litigators?${params}`), query);
    },
    get: (id) => detail(id),
    acknowledge: (id) => detail(id, '/notice-read', { method: 'POST', data: {} }),
    async download(id, format, expected) {
      check();
      reportUuid(id);
      reportValue(expected);
      if (expected.id !== id || expected.state !== 'ready' || !['pdf', 'csv'].includes(format))
        reportInvalid();
      // Capture expectations before awaiting transport; callers cannot replace them mid-download.
      const ready = structuredClone(expected.ready),
        meta = ready.artifacts.find((item) => item.format === format);
      const value = await read(`/case-reports/${id}/download?format=${format}`, {
        binary: 'report',
      });
      const type = format === 'pdf' ? 'application/pdf' : 'text/csv; charset=utf-8';
      // Browsers can omit MIME parameters from Blob.type; the HTTP type remains exact.
      const blobTypes = [type.replaceAll(' ', ''), type.split(';')[0]];
      if (
        !(value.blob instanceof Blob) ||
        value.blob.size !== meta.bytes ||
        value.blob.size > maxReportBytes ||
        value.contentLength !== String(meta.bytes) ||
        value.contentType !== type ||
        !blobTypes.includes(value.blob.type.replaceAll(' ', '')) ||
        value.reportId !== id ||
        value.digest !== meta.digest ||
        value.snapshotDigest !== ready.snapshot_digest
      )
        reportInvalid();
      const bytes = await value.blob.arrayBuffer();
      check();
      const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
      check();
      if ([...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('') !== meta.digest)
        reportInvalid();
      return { blob: value.blob, filename: `report-${id}.${format}` };
    },
  };
}
