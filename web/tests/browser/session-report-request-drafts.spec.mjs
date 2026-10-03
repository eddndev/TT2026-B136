import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire, signInOther } from './session-inactivity-helpers.mjs';
import { form, fillFilters, reports } from './case-reports-helpers.mjs';
import {
  reportDraftSetup,
  checkReportRequests,
  holdReportRequest,
  accountPath,
  reportPath,
  lawyerId,
} from './session-report-request-fixtures.mjs';

test.afterEach(async ({ page }) => checkReportRequests(page));
const resume = (page) =>
  page.getByRole('button', { name: 'Retomar solicitud de informe', exact: true });
const generate = (page) => form(page).getByRole('button', { name: 'Generar informe', exact: true });
const from = (page) => form(page).getByLabel('Creaci\u00f3n desde (UTC)', { exact: true });
const before = (page) =>
  form(page).getByLabel('Creaci\u00f3n hasta (excluida, UTC)', { exact: true });
async function enter(page) {
  await login(page, false, false);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
}

test('report filters reappear only after fresh account and assignee authority without requesting a report', async ({
  page,
}) => {
  const state = await reportDraftSetup(page);
  await enter(page);
  await fillFilters(page, { status: 'active', assigned: lawyerId });
  await before(page).fill('');
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  const fresh = holdReportRequest(state, 'GET', accountPath);
  const eligible = holdReportRequest(state, 'GET', `${reportPath}/litigators`);
  await resume(page).click();
  await expect.poll(() => fresh.entered).toBe(true);
  await expect(from(page)).not.toHaveValue('2026-09-01');
  await expect(generate(page)).toBeDisabled();
  fresh.release();
  await expect.poll(() => eligible.entered).toBe(true);
  await expect(from(page)).not.toHaveValue('2026-09-01');
  await expect(generate(page)).toBeDisabled();
  eligible.release();
  await expect(from(page)).toHaveValue('2026-09-01');
  await expect(before(page)).toHaveValue('');
  await expect(form(page).getByLabel('Estado administrativo', { exact: true })).toHaveValue(
    'active',
  );
  await expect(form(page).getByLabel('Litigante asignado', { exact: true })).toHaveValue(lawyerId);
  expect(state.reportWrites).toEqual([]);
});

test('a report draft never silently replaces an assignee removed from fresh eligibility', async ({
  page,
}) => {
  const state = await reportDraftSetup(page);
  await enter(page);
  await fillFilters(page, { assigned: lawyerId });
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  state.reportLawyers = [];
  await resume(page).click();
  await expect(form(page).getByRole('alert')).toContainText('litigante');
  await expect(generate(page)).toBeDisabled();
  expect(state.reportWrites).toEqual([]);
  state.reportLawyers = [{ user_id: lawyerId, email: 'lawyer@example.test' }];
  await form(page)
    .getByRole('button', { name: 'Volver a consultar el contexto', exact: true })
    .click();
  await expect(form(page).getByLabel('Litigante asignado', { exact: true })).toHaveValue(lawyerId);
  await expect(from(page)).toHaveValue('2026-09-01');
  expect(state.reportWrites).toEqual([]);
});

test('explicit report draft discard and a different account prevent later resurrection', async ({
  page,
}) => {
  const state = await reportDraftSetup(page);
  await enter(page);
  await fillFilters(page);
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  await page.getByRole('button', { name: 'Descartar solicitud pendiente', exact: true }).click();
  await expect(resume(page)).toHaveCount(0);
  await fillFilters(page, { until: '2026-09-23' });
  await expire(page, state, await form(page).elementHandle());
  await signInOther(page);
  await navigate(page, 'Informes');
  await expect(resume(page)).toHaveCount(0);
  await expire(page, state, await form(page).elementHandle());
  await enter(page);
  await expect(resume(page)).toHaveCount(0);
  expect(state.reportWrites).toEqual([]);
});
