import { test, expect } from '@playwright/test';
import { navigate } from './helpers.mjs';
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

test('a report response released after logout cannot restore private detail or download actions', async ({
  page,
}) => {
  const state = await setupReports(page);
  await enterReports(page);
  let intercept;
  const captured = new Promise((resolve) => {
    intercept = resolve;
  });
  state.handle = async (route, url) => {
    if (url.pathname !== `/api/v1/case-reports/${reportId}`) return false;
    intercept(route);
    return true;
  };
  await reports(page)
    .getByRole('button', { name: `Consultar informe ${reportId}`, exact: true })
    .click();
  const route = await captured;
  await expect(details(page)).toHaveAttribute('aria-busy', 'true');
  await page.getByRole('button', { name: 'Cerrar sesi\u00f3n', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  const response = page.waitForResponse((value) => value.url() === route.request().url());
  await route.fulfill({ json: report() });
  await (await response).finished();
  await expect(reports(page)).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^Descargar (PDF|CSV)$/ })).toHaveCount(0);
  expect(state.downloads).toHaveLength(0);
  expect(state.acknowledgments).toHaveLength(0);
});

test('report revocation clears the ready capture and later session expiry returns to login', async ({
  page,
}) => {
  const state = await setupReports(page);
  await enterReports(page);
  await openReport(page);
  await expect(
    details(page).getByRole('button', { name: 'Descargar PDF', exact: true }),
  ).toBeVisible();
  state.handle = async (route, url) => {
    if (url.pathname !== `/api/v1/case-reports/${reportId}`) return false;
    await route.fulfill({ status: 403, json: { error: { code: 'case_report_access_revoked' } } });
    return true;
  };
  await updateReport(page);
  await expect(reports(page).getByRole('alert')).toBeVisible();
  await expect(reports(page).getByRole('button', { name: /^Descargar (PDF|CSV)$/ })).toHaveCount(0);
  await expect(reports(page)).not.toContainText('ab'.repeat(32));
  state.handle = async (route) => {
    await route.fulfill({ status: 401, json: { error: { code: 'invalid_session' } } });
    return true;
  };
  await reports(page).getByRole('button', { name: 'Actualizar informes', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Accede a tu despacho.' })).toBeVisible();
  await expect(reports(page)).toHaveCount(0);
  expect(state.downloads).toHaveLength(0);
});

test('observed report duration remains fixed after notice reading and excludes pending work', async ({
  page,
}, testInfo) => {
  const state = await setupReports(page);
  const queued = pending({
    id: otherReportId,
    operation_id: '82000000-0000-4000-8000-000000000002',
    request_digest: 'ef'.repeat(32),
    requested_at: '2026-09-27T12:00:30Z',
    updated_at: '2026-09-27T12:00:30Z',
  });
  state.records.set(otherReportId, queued);
  await enterReports(page);
  await openReport(page);
  await expect(details(page).getByText('Duraci\u00f3n observada', { exact: true })).toBeVisible();
  await expect(details(page).getByText('1 min', { exact: true })).toBeVisible();
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 1000 });
    await expect(details(page).getByText('1 min', { exact: true })).toBeVisible();
    await page.screenshot({
      path: testInfo.outputPath(`report-duration-${width}.png`),
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await details(page)
    .getByRole('button', { name: 'Marcar aviso como le\u00eddo', exact: true })
    .click();
  await expect(details(page).getByText('Aviso le\u00eddo', { exact: true })).toBeVisible();
  expect(state.acknowledgments).toEqual([reportId]);
  expect(state.records.get(reportId).updated_at).toBe('2026-09-27T12:01:00Z');
  await expect(details(page).getByText('1 min', { exact: true })).toBeVisible();
  await expect(details(page).getByText('2 min', { exact: true })).toHaveCount(0);
  await expect(
    details(page).getByRole('button', { name: 'Descargar PDF', exact: true }),
  ).toBeVisible();

  await openReport(page, otherReportId);
  await expect(details(page).getByText('En cola', { exact: true })).toBeVisible();
  await expect(details(page).getByText('Duraci\u00f3n observada', { exact: true })).toHaveCount(0);
  state.records.set(otherReportId, {
    ...queued,
    state: 'failed',
    failure: 'render_failed',
    updated_at: '2026-09-27T12:01:00Z',
    notice: { kind: 'failed', created_at: '2026-09-27T12:01:00Z', read_at: null },
  });
  await updateReport(page);
  await expect(
    details(page).getByText('No se pudo generar el informe', { exact: true }),
  ).toBeVisible();
  await expect(details(page).getByText('Duraci\u00f3n observada', { exact: true })).toBeVisible();
  await expect(details(page).getByText('30 s', { exact: true })).toBeVisible();
  expect(state.requests).toHaveLength(0);
  expect(state.downloads).toHaveLength(0);
});
