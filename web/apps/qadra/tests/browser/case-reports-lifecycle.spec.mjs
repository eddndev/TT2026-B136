import { test, expect } from '@playwright/test';
import { navigate, signOut } from './helpers.mjs';
import { reportId, otherReportId, report, pending, reportPage } from './case-reports-fixtures.mjs';
import {
  setupReports,
  enterReports,
  openReport,
  updateReport,
  reports,
  details,
} from './case-reports-helpers.mjs';

test('report listing survives navigation and paginates durable results without guessing complete totals', async ({
  page,
}) => {
  const state = await setupReports(page);
  const second = report({
    id: otherReportId,
    operation_id: '82000000-0000-4000-8000-000000000002',
    request_digest: 'ef'.repeat(32),
  });
  state.records.set(otherReportId, second);
  state.handle = async (route, url) => {
    if (url.pathname !== '/api/v1/case-reports' || route.request().method() !== 'GET') return false;
    const after = url.searchParams.get('after_id');
    await route.fulfill({
      json: after
        ? reportPage([second])
        : reportPage([report()], {
            has_more: true,
            next_after_id: reportId,
          }),
    });
    return true;
  };
  await enterReports(page);
  await expect(
    reports(page).getByRole('button', { name: `Consultar informe ${otherReportId}`, exact: true }),
  ).toHaveCount(0);
  await reports(page).getByRole('button', { name: 'Siguientes informes', exact: true }).click();
  await expect(
    reports(page).getByRole('button', { name: `Consultar informe ${otherReportId}`, exact: true }),
  ).toBeVisible();
  expect(
    state.calls.some((call) => new URLSearchParams(call.search).get('after_id') === reportId),
  ).toBe(true);
  await expect(reports(page)).not.toContainText(/Total de informes:\s*[12]/);
  await navigate(page, 'Inicio');
  await navigate(page, 'Informes');
  await openReport(page);
  await expect(details(page)).toContainText('Aviso sin leer');
  expect(state.requests).toHaveLength(0);
  expect(state.acknowledgments).toHaveLength(0);
});
