import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { form, fillFilters, reports, details } from './case-reports-helpers.mjs';
import { reportId, otherReportId } from './case-reports-fixtures.mjs';
import {
  reportDraftSetup,
  checkReportRequests,
  holdReportRequest,
  reportPath,
} from './session-report-request-fixtures.mjs';

test.afterEach(async ({ page }) => checkReportRequests(page));

test('a report confirmed while an older detail is loading remains selected after that late read', async ({
  page,
}) => {
  const state = await reportDraftSetup(page);
  await login(page, false, false);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
  await fillFilters(page);
  state.reportWrite = { commit: true };
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  await expect(details(page).getByText(reportId, { exact: true })).toBeVisible();
  expect(state.reportWrites).toHaveLength(1);
  const { requester: ownerA, ...receiptA } = [...state.reportRecords.values()][0];
  expect(ownerA).toBe(state.current.user.id);

  await fillFilters(page, { until: '2026-09-28' });
  const writeB = holdReportRequest(state, 'POST', reportPath);
  state.reportWrite = { commit: true };
  await form(page).getByRole('button', { name: 'Generar informe', exact: true }).click();
  await expect.poll(() => writeB.entered).toBe(true);
  expect(state.reportWrites).toHaveLength(2);
  const { requester: ownerB, ...receiptB } = [...state.reportRecords.values()].find(
    (value) => value.id === otherReportId,
  );
  expect(ownerB).toBe(ownerA);
  expect(receiptB.operation_id).not.toBe(receiptA.operation_id);
  const readA = holdReportRequest(state, 'GET', `${reportPath}/${reportId}`);
  await reports(page)
    .getByRole('button', { name: `Consultar informe ${reportId}`, exact: true })
    .click();
  await expect.poll(() => readA.entered).toBe(true);
  await expect(details(page)).toHaveAttribute('aria-busy', 'true');

  const acceptedB = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === reportPath && response.request().method() === 'POST',
  );
  writeB.release();
  const responseB = await acceptedB;
  expect(responseB.status()).toBe(202);
  expect(await responseB.json()).toEqual(receiptB);
  await expect(
    reports(page).getByRole('button', {
      name: `Consultar informe ${otherReportId}`,
      exact: true,
    }),
  ).toBeVisible();

  const lateA = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === `${reportPath}/${reportId}` &&
      response.request().method() === 'GET',
  );
  readA.release();
  const responseA = await lateA;
  expect(responseA.status()).toBe(200);
  expect(await responseA.json()).toEqual(receiptA);
  await responseA.finished();
  await expect(details(page)).toHaveAttribute('aria-busy', 'false');
  await expect(details(page).getByText(otherReportId, { exact: true })).toBeVisible();
  await expect(details(page).getByText(reportId, { exact: true })).toHaveCount(0);
  await expect(details(page)).toContainText('2026-09-28T00:00:00Z');
  expect(state.reportWrites).toHaveLength(2);
  expect(state.reportRecords.size).toBe(2);
});
