import { test, expect } from '@playwright/test';
import { login, navigate } from './helpers.mjs';
import { expire } from './session-inactivity-helpers.mjs';
import {
  reportDraftSetup,
  checkReportRequests,
  reportPath,
} from './session-report-request-fixtures.mjs';
import { lawyerId, pending, reportId } from './case-reports-fixtures.mjs';
import { setupReports, enterReports, reports, details, form } from './case-reports-helpers.mjs';

const type = (page) => form(page).getByLabel('Tipo de informe', { exact: true });
const author = (page) => form(page).getByLabel('Litigante autor', { exact: true });
const assignee = (page) => form(page).getByLabel('Litigante asignado', { exact: true });
const generate = (page) => form(page).getByRole('button', { name: 'Generar informe', exact: true });
const resume = (page) =>
  form(page).getByRole('button', { name: 'Retomar solicitud de informe', exact: true });
const retry = (page) =>
  form(page).getByRole('button', { name: 'Reintentar solicitud', exact: true });
const historical = { user_id: lawyerId, email: 'former.author@example.test' };
const pickerPath = `${reportPath}/litigators`;
async function selectActivity(page) {
  await type(page).selectOption('litigator_activity');
  await expect(author(page)).toContainText(historical.email);
  await author(page).selectOption(lawyerId);
  await form(page).getByLabel('Actividad desde (UTC)', { exact: true }).fill('2026-09-01');
  await form(page)
    .getByLabel('Actividad hasta (excluida, UTC)', { exact: true })
    .fill('2026-10-01');
}
async function returnToReports(page) {
  await login(page, false, false);
  await navigate(page, 'Informes');
  await expect(reports(page)).toHaveAttribute('aria-busy', 'false');
}
const pickerModes = (calls) =>
  calls
    .filter((call) => call.path === pickerPath)
    .map((call) => new URLSearchParams(call.search).get('report_type'));

test('a litigator can select a former author only from the activity directory', async ({
  page,
}) => {
  const state = await setupReports(page, 'litigator');
  state.picker.litigators = [];
  state.handle = async (route, url) => {
    if (url.pathname === pickerPath) {
      const activity = url.searchParams.get('report_type') === 'litigator_activity';
      await route.fulfill({ json: { ...state.picker, litigators: activity ? [historical] : [] } });
      return true;
    }
    if (url.pathname !== reportPath || route.request().method() !== 'POST') return false;
    const command = route.request().postDataJSON();
    state.requests.push(command);
    const value = pending({ scope: 'assigned_cases', ...command });
    state.records.set(reportId, value);
    await route.fulfill({ status: 202, json: value });
    return true;
  };
  await enterReports(page);
  await expect(assignee(page)).not.toContainText(historical.email);
  await type(page).selectOption('litigator_activity');
  await expect(author(page)).toContainText(historical.email);
  await type(page).selectOption('case_state');
  await expect(assignee(page)).not.toContainText(historical.email);
  await selectActivity(page);
  await generate(page).click();
  await expect(details(page)).toContainText('En cola');
  expect(state.requests).toHaveLength(1);
  expect(state.requests[0].report_type).toBe('litigator_activity');
  expect(state.requests[0].filters.author_litigator).toBe(lawyerId);
  expect(pickerModes(state.calls)).toEqual([
    null,
    'litigator_activity',
    null,
    'litigator_activity',
  ]);
});

test.describe('restored report author eligibility', () => {
  test.afterEach(async ({ page }) => checkReportRequests(page));
});
