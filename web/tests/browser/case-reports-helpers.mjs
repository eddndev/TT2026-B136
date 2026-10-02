import { expect } from '@playwright/test';
import { setup, login, navigate } from './helpers.mjs';
import { dashboardValue } from './dashboard-helpers.mjs';
import {
  reportId,
  lawyerId,
  report,
  pending,
  reportPage,
  artifactBytes,
  sha256,
} from './case-reports-fixtures.mjs';

export const reports = (page) =>
  page.getByRole('region', { name: 'Informes de expedientes', exact: true });
export const details = (page) =>
  page.getByRole('region', { name: 'Detalle de informe', exact: true });
export const form = (page) => page.getByRole('region', { name: 'Solicitar informe', exact: true });

export async function setupReports(page, role = 'owner') {
  await setup(page, role);
  const scope = role === 'owner' ? 'office' : 'assigned_cases';
  const state = {
    calls: [],
    requests: [],
    acknowledgments: [],
    downloads: [],
    handle: null,
    records: new Map([[reportId, report({ scope })]]),
    picker: {
      scope,
      checked_at: '2026-09-27T12:00:00Z',
      litigators: [{ user_id: lawyerId, email: 'lawyer@example.test' }],
      has_more: false,
      next_after_id: null,
    },
    dashboardCalls: 0,
    dashboard: dashboardValue({
      scope,
      active_cases: 72,
      workload: [{ user_id: lawyerId, email: 'lawyer@example.test', active_cases: 72 }],
    }),
  };
  await page.route('**/api/v1/dashboard', (route) => {
    state.dashboardCalls++;
    return route.fulfill({ json: state.dashboard });
  });
  await page.route('**/api/v1/case-reports**', async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    state.calls.push({ method: request.method(), path: url.pathname, search: url.search });
    if (state.handle && (await state.handle(route, url))) return;
    const path = url.pathname;
    if (path === '/api/v1/case-reports/litigators') return route.fulfill({ json: state.picker });
    if (path === '/api/v1/case-reports') {
      if (request.method() === 'POST') {
        const command = request.postDataJSON();
        state.requests.push(command);
        const value = pending({
          scope,
          operation_id: command.operation_id,
          filters: command.filters,
        });
        state.records.set(reportId, value);
        return route.fulfill({ status: 202, json: value });
      }
      return route.fulfill({ json: reportPage([...state.records.values()]) });
    }
    const id = path.split('/')[4];
    const value = state.records.get(id);
    if (!value)
      return route.fulfill({ status: 404, json: { error: { code: 'case_report_not_found' } } });
    if (path.endsWith('/notice-read')) {
      state.acknowledgments.push(id);
      value.notice = { ...value.notice, read_at: value.notice.read_at || '2026-09-27T12:01:00Z' };
      value.updated_at = value.notice.read_at;
      return route.fulfill({ json: value });
    }
    if (path.endsWith('/download')) {
      const format = url.searchParams.get('format');
      state.downloads.push({ id, format });
      const bytes = artifactBytes[format];
      return route.fulfill({
        body: bytes,
        headers: {
          'content-type': format === 'pdf' ? 'application/pdf' : 'text/csv; charset=utf-8',
          'content-disposition': `attachment; filename="report-${id}.${format}"`,
          'content-length': String(bytes.length),
          'cache-control': 'no-store',
          'x-content-type-options': 'nosniff',
          'x-report-id': id,
          'x-report-digest': sha256(bytes),
          'x-report-snapshot-digest': value.ready.snapshot_digest,
        },
      });
    }
    return route.fulfill({ json: value });
  });
  return state;
}

export async function enterReports(page) {
  await login(page, false, false);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
}

export async function fillFilters(
  page,
  { status = 'all', assigned = '', until = '2026-10-01' } = {},
) {
  await form(page).getByLabel('Creaci\u00f3n desde (UTC)', { exact: true }).fill('2026-09-01');
  await form(page).getByLabel('Creaci\u00f3n hasta (excluida, UTC)', { exact: true }).fill(until);
  await form(page).getByLabel('Estado administrativo', { exact: true }).selectOption(status);
  await form(page).getByLabel('Litigante asignado', { exact: true }).selectOption(assigned);
}

export async function openReport(page, id = reportId) {
  await reports(page)
    .getByRole('button', { name: `Consultar informe ${id}`, exact: true })
    .click();
  await expect(details(page)).toHaveAttribute('aria-busy', 'false');
}

export const updateReport = (page) =>
  details(page).getByRole('button', { name: 'Actualizar informe', exact: true }).click();
